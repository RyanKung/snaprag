use crate::config::AppConfig;
use crate::database::Database;
use crate::models::*;
use crate::sync::client::proto;
use crate::sync::client::SnapchainClient;
use crate::sync::types::{SyncConfig, SyncState, SyncStatus};
use crate::{Result, SnapRagError};
use base64::Engine as _;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

/// Main sync service that coordinates synchronization with snapchain
pub struct SyncService {
    config: SyncConfig,
    client: SnapchainClient,
    database: Arc<Database>,
    state: Arc<RwLock<SyncState>>,
}

impl SyncService {
    /// Create a new sync service
    pub async fn new(app_config: &AppConfig, database: Arc<Database>) -> Result<Self> {
        let sync_config = SyncConfig::from_app_config(app_config);
        let client = SnapchainClient::from_config(app_config).await?;

        // Load or create initial sync state
        let state = Arc::new(RwLock::new(SyncState::new()));

        Ok(Self {
            config: sync_config,
            client,
            database,
            state,
        })
    }

    /// Start the sync service
    pub async fn start(&self) -> Result<()> {
        info!("Starting SnapRAG sync service...");
        info!("Sync configuration: {:?}", self.config);

        // Start historical sync if enabled
        if self.config.enable_historical_sync {
            self.start_historical_sync().await?;
        }

        // Start real-time sync if enabled
        if self.config.enable_realtime_sync {
            self.start_realtime_sync().await?;
        }

        Ok(())
    }

    /// Start historical synchronization using ReplicationService API
    async fn start_historical_sync(&self) -> Result<()> {
        info!("Starting historical sync using ReplicationService API...");

        let mut state = self.state.write().await;
        state.status = SyncStatus::CatchingUp;
        drop(state);

        // Sync each shard using replication service
        for &shard_id in &self.config.shard_ids {
            info!("Starting historical sync for shard {}", shard_id);
            self.sync_shard_historical_replication(shard_id).await?;
        }

        let mut state = self.state.write().await;
        state.status = SyncStatus::Realtime;
        info!("Historical sync completed, switching to real-time sync");

        Ok(())
    }

    /// Start real-time synchronization using Subscribe API
    async fn start_realtime_sync(&self) -> Result<()> {
        info!("Starting real-time sync...");

        let mut state = self.state.write().await;
        state.status = SyncStatus::Realtime;
        drop(state);

        // Subscribe to events for each shard
        for &shard_id in &self.config.shard_ids {
            if shard_id == 0 {
                continue; // Skip block shard
            }

            // Spawn a task for each shard subscription
            let client = self.client.clone();
            let database = self.database.clone();
            let config = self.config.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::subscribe_shard_events(shard_id, client, database, config).await
                {
                    error!("Error in shard {} subscription: {}", shard_id, e);
                }
            });
        }

        // Keep the service running
        loop {
            sleep(Duration::from_millis(self.config.sync_interval_ms)).await;

            // Update sync statistics
            self.update_sync_stats().await;
        }
    }

    /// Sync historical data for a specific shard using ReplicationService
    async fn sync_shard_historical_replication(&self, shard_id: u32) -> Result<()> {
        info!(
            "Syncing historical data for shard {} using ReplicationService",
            shard_id
        );

        // Get shard snapshot metadata to find available snapshots
        let snapshot_request =
            crate::sync::client::proto::GetShardSnapshotMetadataRequest { shard_id };

        let snapshots = self
            .client
            .get_shard_snapshot_metadata(snapshot_request)
            .await?;
        info!(
            "Found {} snapshots for shard {}",
            snapshots.snapshots.len(),
            shard_id
        );

        if snapshots.snapshots.is_empty() {
            warn!("No snapshots available for shard {}", shard_id);
            return Ok(());
        }

        // Process each snapshot
        for snapshot in snapshots.snapshots {
            info!(
                "Processing snapshot at height {} for shard {}",
                snapshot.height, shard_id
            );
            self.sync_snapshot_data(shard_id, snapshot.height).await?;
        }

        info!("Completed historical sync for shard {}", shard_id);
        Ok(())
    }

    /// Sync data from a specific snapshot using trie iteration
    async fn sync_snapshot_data(&self, shard_id: u32, height: u64) -> Result<()> {
        info!(
            "Syncing snapshot data for shard {} at height {}",
            shard_id, height
        );

        let mut page_token: Option<String> = None;
        let mut total_messages = 0;
        let mut total_users = 0;
        let mut processed_fids = std::collections::HashSet::new();

        // Iterate through trie virtual shards (typically 0-255 for user data)
        for virtual_shard in 0..256u32 {
            info!(
                "Processing trie virtual shard {} for shard {} at height {}",
                virtual_shard, shard_id, height
            );

            loop {
                let request = crate::sync::client::proto::GetShardTransactionsRequest {
                    shard_id,
                    height,
                    trie_virtual_shard: virtual_shard,
                    page_token: page_token.clone(),
                };

                let response = self.client.get_shard_transactions(request).await?;

                if response.trie_messages.is_empty() {
                    break; // No more messages in this virtual shard
                }

                // Process each message in the response
                for trie_entry in response.trie_messages {
                    if let Some(user_message) = trie_entry.user_message {
                        self.process_user_message_comprehensive(&user_message)
                            .await?;
                        total_messages += 1;

                        // Track unique users
                        if let Some(fid) = user_message.data.as_ref().map(|d| d.fid) {
                            if processed_fids.insert(fid) {
                                total_users += 1;
                            }
                        }
                    }

                    if let Some(on_chain_event) = trie_entry.on_chain_event {
                        self.process_onchain_event(&on_chain_event).await?;
                    }

                    if let Some(fname_transfer) = trie_entry.fname_transfer {
                        self.process_fname_transfer(&fname_transfer).await?;
                    }
                }

                // Update page token for next iteration
                page_token = response.next_page_token.clone();
                if page_token.is_none() {
                    break; // No more pages in this virtual shard
                }
            }
        }

        info!(
            "Processed {} messages and {} unique users from snapshot at height {} for shard {}",
            total_messages, total_users, height, shard_id
        );
        Ok(())
    }

    /// Subscribe to real-time events for a specific shard
    async fn subscribe_shard_events(
        shard_id: u32,
        client: SnapchainClient,
        database: Arc<Database>,
        config: SyncConfig,
    ) -> Result<()> {
        info!("Starting real-time subscription for shard {}", shard_id);
        let mut retry_delay = Duration::from_millis(config.sync_interval_ms.max(100));

        loop {
            let last_height = Self::get_last_processed_height_static(&database, shard_id).await?;

            let request = proto::ShardChunksRequest {
                shard_id,
                start_block_number: last_height.saturating_add(1),
                stop_block_number: None,
            };

            match client.get_shard_chunks(request).await {
                Ok(response) => {
                    let mut highest_seen = last_height;

                    for chunk in response.shard_chunks {
                        if let Err(err) = Self::process_shard_chunk_static(&chunk, &database).await
                        {
                            error!("Failed processing shard {} chunk: {err}");
                            continue;
                        }

                        if let Some(block_number) = chunk
                            .header
                            .as_ref()
                            .and_then(|header| header.height.as_ref())
                            .map(|height| height.block_number)
                        {
                            highest_seen = highest_seen.max(block_number);
                        }
                    }

                    if highest_seen > last_height {
                        Self::update_last_processed_height_static(
                            &database,
                            shard_id,
                            highest_seen,
                        )
                        .await?;
                    } else {
                        let subscribe_request = proto::SubscribeRequest {
                            event_types: vec![],
                            from_id: None,
                            shard_index: Some(shard_id),
                        };

                        match client.subscribe(subscribe_request).await {
                            Ok(events) => {
                                let mut event_high = highest_seen;

                                for event in events {
                                    if let Some(message) = event.message {
                                        if let Err(err) = Self::process_user_message_static(
                                            &message,
                                            &database,
                                            event.block_number,
                                            shard_id,
                                            event.fid,
                                        )
                                        .await
                                        {
                                            error!(
                                                "Failed processing hub event for shard {}: {err}",
                                                shard_id
                                            );
                                            continue;
                                        }

                                        event_high = event_high.max(event.block_number);
                                    }
                                }

                                if event_high > last_height {
                                    Self::update_last_processed_height_static(
                                        &database, shard_id, event_high,
                                    )
                                    .await?;
                                }
                            }
                            Err(err) => {
                                warn!("Subscribe fallback failed for shard {}: {err}", shard_id)
                            }
                        }
                    }

                    retry_delay = Duration::from_millis(config.sync_interval_ms.max(100));
                }
                Err(err) => {
                    error!("Error fetching shard {} chunks: {err}", shard_id);
                    sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(Duration::from_secs(60));
                    continue;
                }
            }

            sleep(Duration::from_millis(config.sync_interval_ms)).await;
        }
    }

    /// Process a shard chunk and extract user messages
    async fn process_shard_chunk(&self, chunk: &proto::ShardChunk) -> Result<()> {
        if let Some(header) = &chunk.header {
            if let Some(height) = &header.height {
                info!(
                    "Processing shard chunk at height {} for shard {}",
                    height.block_number, height.shard_index
                );
            }
        }

        // Process each transaction in the chunk
        for transaction in &chunk.transactions {
            self.process_transaction(transaction).await?;
        }

        Ok(())
    }

    /// Process a single transaction
    async fn process_transaction(&self, transaction: &proto::Transaction) -> Result<()> {
        // Process user messages
        for message in &transaction.user_messages {
            self.process_user_message(message).await?;
        }

        // Process system messages (on-chain events, etc.)
        for system_message in &transaction.system_messages {
            self.process_system_message(system_message).await?;
        }

        Ok(())
    }

    /// Process a user message comprehensively (cast, reaction, profile, etc.)
    async fn process_user_message_comprehensive(&self, message: &proto::Message) -> Result<()> {
        // Check if message already processed
        let message_hash = message.hash.clone();
        if self.database.is_message_processed(&message_hash).await? {
            return Ok(());
        }

        // Extract message data
        let message_data = match &message.data {
            Some(data) => data,
            None => {
                warn!("Message without data, skipping");
                return Ok(());
            }
        };

        let fid = message_data.fid as i64;
        let timestamp = message_data.timestamp as i64;
        let message_type = message_data.r#type;

        // Process based on message type
        match message_type {
            1 => self.process_cast_add(message, fid, timestamp).await?, // CAST_ADD
            2 => self.process_cast_remove(message, fid, timestamp).await?, // CAST_REMOVE
            3 => self.process_reaction_add(message, fid, timestamp).await?, // REACTION_ADD
            4 => {
                self.process_reaction_remove(message, fid, timestamp)
                    .await?
            } // REACTION_REMOVE
            5 => self.process_link_add(message, fid, timestamp).await?, // LINK_ADD
            6 => self.process_link_remove(message, fid, timestamp).await?, // LINK_REMOVE
            7 => {
                self.process_verification_add(message, fid, timestamp)
                    .await?
            } // VERIFICATION_ADD_ETH_ADDRESS
            8 => {
                self.process_verification_remove(message, fid, timestamp)
                    .await?
            } // VERIFICATION_REMOVE
            11 => self.process_user_data_add(message, fid, timestamp).await?, // USER_DATA_ADD
            12 => self.process_username_proof(message, fid, timestamp).await?, // USERNAME_PROOF
            13 => self.process_frame_action(message, fid, timestamp).await?, // FRAME_ACTION
            14 => {
                self.process_link_compact_state(message, fid, timestamp)
                    .await?
            } // LINK_COMPACT_STATE
            _ => {
                info!("Unknown message type: {}, skipping", message_type);
                return Ok(());
            }
        }

        // Record the processed message
        self.database
            .record_processed_message(
                message_hash,
                1, // Default shard for user messages
                0, // Block height not available in this context
                fid as u64,
                &format!("MessageType::{}", message_type),
                fid as u64,
                timestamp,
                Self::compute_content_hash(message),
            )
            .await?;

        Ok(())
    }

    /// Process a user message (legacy method for compatibility)
    async fn process_user_message(&self, message: &proto::Message) -> Result<()> {
        info!(
            "Processing user message: {:?}",
            message.data.as_ref().map(|d| &d.r#type)
        );
        Ok(())
    }

    /// Process a system message (on-chain events, etc.)
    async fn process_system_message(&self, system_message: &proto::ValidatorMessage) -> Result<()> {
        if let Some(event) = &system_message.on_chain_event {
            self.process_onchain_event(event).await?;
        }

        if let Some(transfer) = &system_message.fname_transfer {
            self.process_fname_transfer(transfer).await?;
        }

        Ok(())
    }

    /// Get the last processed height for a shard
    async fn get_last_processed_height(&self, shard_id: u32) -> Result<u64> {
        self.database.get_last_processed_height(shard_id).await
    }

    /// Update the last processed height for a shard
    async fn update_last_processed_height(&self, shard_id: u32, height: u64) -> Result<()> {
        self.database
            .update_last_processed_height(shard_id, height)
            .await
    }

    /// Update sync statistics
    async fn update_sync_stats(&self) {
        match self.database.get_sync_stats().await {
            Ok(stats) => {
                let mut state = self.state.write().await;
                state.last_sync_timestamp = Some(Utc::now());

                for shard_stat in stats {
                    let shard_id = shard_stat.shard_id as u32;
                    state
                        .total_blocks_processed
                        .insert(shard_id, shard_stat.total_blocks as u64);
                    state
                        .total_messages_processed
                        .insert(shard_id, shard_stat.total_messages as u64);

                    if let Some(height) = shard_stat.last_processed_height {
                        state.last_synced_heights.insert(shard_id, height as u64);
                    }
                }
            }
            Err(err) => warn!("Failed to refresh sync statistics: {err}"),
        }
    }

    // Static versions for use in spawned tasks
    async fn get_last_processed_height_static(database: &Database, shard_id: u32) -> Result<u64> {
        database.get_last_processed_height(shard_id).await
    }

    async fn update_last_processed_height_static(
        database: &Database,
        shard_id: u32,
        height: u64,
    ) -> Result<()> {
        database
            .update_last_processed_height(shard_id, height)
            .await
    }

    async fn process_shard_chunk_static(
        chunk: &proto::ShardChunk,
        database: &Database,
    ) -> Result<()> {
        // Process each transaction in the chunk
        for transaction in &chunk.transactions {
            if let Some(header) = &chunk.header {
                if let Some(height) = &header.height {
                    Self::process_transaction_static(
                        transaction,
                        database,
                        height.block_number,
                        height.shard_index,
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn process_transaction_static(
        transaction: &proto::Transaction,
        database: &Database,
        block_height: u64,
        shard_id: u32,
    ) -> Result<()> {
        // Process user messages
        for message in &transaction.user_messages {
            Self::process_user_message_static(
                message,
                database,
                block_height,
                shard_id,
                transaction.fid,
            )
            .await?;
        }

        // Process system messages
        for system_message in &transaction.system_messages {
            Self::process_system_message_static(
                system_message,
                database,
                block_height,
                shard_id,
                transaction.fid,
            )
            .await?;
        }

        Ok(())
    }

    async fn process_user_message_static(
        message: &proto::Message,
        database: &Database,
        block_height: u64,
        shard_id: u32,
        transaction_fid: u64,
    ) -> Result<()> {
        // Check if message already processed
        let message_hash = message.hash.clone();
        if database.is_message_processed(&message_hash).await? {
            return Ok(());
        }

        // Extract message data
        let message_type = message
            .data
            .as_ref()
            .map(|d| format!("{:?}", d.r#type))
            .unwrap_or_else(|| "unknown".to_string());
        let fid = message.data.as_ref().map(|d| d.fid).unwrap_or(0);
        let timestamp = message.data.as_ref().map(|d| d.timestamp).unwrap_or(0);

        // Record the processed message
        database
            .record_processed_message(
                message_hash,
                shard_id,
                block_height,
                transaction_fid,
                &message_type,
                fid as u64,
                timestamp as i64,
                Self::compute_content_hash(message),
            )
            .await?;

        info!(
            "Processed {} message for FID {} at height {}",
            message_type, fid, block_height
        );
        Ok(())
    }

    async fn process_system_message_static(
        system_message: &proto::ValidatorMessage,
        database: &Database,
        block_height: u64,
        shard_id: u32,
        transaction_fid: u64,
    ) -> Result<()> {
        if let Some(event) = &system_message.on_chain_event {
            Self::handle_onchain_event(database, event, block_height, shard_id).await?;
        }

        if let Some(transfer) = &system_message.fname_transfer {
            Self::handle_fname_transfer(database, transfer, block_height, transaction_fid).await?;
        }

        Ok(())
    }

    fn compute_content_hash(message: &proto::Message) -> Option<Vec<u8>> {
        let data = message.data.as_ref()?;
        let mut hasher = Sha256::new();

        hasher.update(&message.hash);
        hasher.update(data.r#type.to_le_bytes());
        hasher.update(data.fid.to_le_bytes());
        hasher.update(data.timestamp.to_le_bytes());

        if let Some(body) = &data.body {
            match serde_json::to_vec(body) {
                Ok(bytes) => hasher.update(bytes),
                Err(err) => warn!("Failed to serialize message body for hashing: {err}"),
            }
        }

        Some(hasher.finalize().to_vec())
    }

    fn extract_message_body(message: &proto::Message) -> Option<serde_json::Value> {
        message.data.as_ref()?.body.clone()
    }

    fn value_to_string(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(num) => Some(num.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            serde_json::Value::Object(map) => {
                if let Some(inner) = map.get("value").and_then(Self::value_to_string) {
                    return Some(inner);
                }
                if let Some(inner) = map.get("text").and_then(Self::value_to_string) {
                    return Some(inner);
                }
                if let Some(inner) = map.get("url").and_then(Self::value_to_string) {
                    return Some(inner);
                }
                Some(value.to_string())
            }
            _ => None,
        }
    }

    fn value_to_i64(value: &serde_json::Value) -> Option<i64> {
        match value {
            serde_json::Value::Number(num) => {
                if let Some(i) = num.as_i64() {
                    Some(i)
                } else {
                    num.as_u64().map(|u| u as i64)
                }
            }
            serde_json::Value::String(s) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    fn parse_user_data_type(value: &serde_json::Value) -> Option<UserDataType> {
        if let Some(num) = Self::value_to_i64(value) {
            let data_type = UserDataType::from(num as i16);
            return if data_type == UserDataType::None {
                None
            } else {
                Some(data_type)
            };
        }

        if let Some(s) = Self::value_to_string(value) {
            let normalized = s.to_ascii_lowercase();
            let mapped = match normalized.as_str() {
                "pfp" | "profile_picture" | "user_data_type_pfp" => UserDataType::Pfp,
                "display" | "display_name" | "user_data_type_display" => UserDataType::Display,
                "bio" | "user_data_type_bio" => UserDataType::Bio,
                "url" | "website" | "user_data_type_url" => UserDataType::Url,
                "username" | "user_data_type_username" => UserDataType::Username,
                "location" | "user_data_type_location" => UserDataType::Location,
                "twitter" | "twitter_username" | "user_data_type_twitter" => UserDataType::Twitter,
                "github" | "github_username" | "user_data_type_github" => UserDataType::Github,
                "banner" | "banner_url" | "user_data_type_banner" => UserDataType::Banner,
                "primary_address_ethereum" | "primaryethereum" | "eth_address" => {
                    UserDataType::PrimaryAddressEthereum
                }
                "primary_address_solana" | "primarysolana" => UserDataType::PrimaryAddressSolana,
                "profile_token" | "token" => UserDataType::ProfileToken,
                _ => UserDataType::None,
            };

            return if mapped == UserDataType::None {
                None
            } else {
                Some(mapped)
            };
        }

        None
    }

    fn parse_username_type(value: &serde_json::Value) -> Option<UsernameType> {
        if let Some(num) = Self::value_to_i64(value) {
            return Some(UsernameType::from(num as i32));
        }

        if let Some(s) = Self::value_to_string(value) {
            let normalized = s.to_ascii_lowercase();
            let mapped = match normalized.as_str() {
                "fname" | "username_type_fname" | "farcaster" => UsernameType::Fname,
                "ens_l1" | "ens" | "username_type_ens_l1" => UsernameType::EnsL1,
                "basename" | "username_type_basename" => UsernameType::Basename,
                _ => UsernameType::None,
            };

            return Some(mapped);
        }

        None
    }

    fn decode_bytes(value: &serde_json::Value) -> Option<Vec<u8>> {
        let string_value = Self::value_to_string(value)?;
        let trimmed = string_value.trim();

        if trimmed.is_empty() {
            return Some(Vec::new());
        }

        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
            return Some(bytes);
        }

        if let Some(stripped) = trimmed.strip_prefix("0x") {
            if let Ok(bytes) = hex::decode(stripped) {
                return Some(bytes);
            }
        }

        if let Ok(bytes) = hex::decode(trimmed) {
            return Some(bytes);
        }

        Some(trimmed.as_bytes().to_vec())
    }

    async fn record_activity(
        database: &Database,
        fid: i64,
        activity_type: &str,
        activity_data: serde_json::Value,
        timestamp: i64,
        message_hash: Option<Vec<u8>>,
    ) -> Result<()> {
        database
            .record_user_activity(
                fid,
                activity_type.to_string(),
                Some(activity_data),
                timestamp,
                message_hash,
            )
            .await?;

        Ok(())
    }

    fn initialize_profile_request(
        fid: i64,
        timestamp: i64,
        message_hash: Vec<u8>,
    ) -> CreateUserProfileRequest {
        CreateUserProfileRequest {
            fid,
            username: None,
            display_name: None,
            bio: None,
            pfp_url: None,
            banner_url: None,
            location: None,
            website_url: None,
            twitter_username: None,
            github_username: None,
            primary_address_ethereum: None,
            primary_address_solana: None,
            profile_token: None,
            message_hash,
            timestamp,
        }
    }

    fn apply_user_data_to_profile_request(
        request: &mut CreateUserProfileRequest,
        data_type: UserDataType,
        value: &str,
    ) {
        match data_type {
            UserDataType::Pfp => request.pfp_url = Some(value.to_string()),
            UserDataType::Display => request.display_name = Some(value.to_string()),
            UserDataType::Bio => request.bio = Some(value.to_string()),
            UserDataType::Url => request.website_url = Some(value.to_string()),
            UserDataType::Username => request.username = Some(value.to_string()),
            UserDataType::Location => request.location = Some(value.to_string()),
            UserDataType::Twitter => request.twitter_username = Some(value.to_string()),
            UserDataType::Github => request.github_username = Some(value.to_string()),
            UserDataType::Banner => request.banner_url = Some(value.to_string()),
            UserDataType::PrimaryAddressEthereum => {
                request.primary_address_ethereum = Some(value.to_string())
            }
            UserDataType::PrimaryAddressSolana => {
                request.primary_address_solana = Some(value.to_string())
            }
            UserDataType::ProfileToken => request.profile_token = Some(value.to_string()),
            UserDataType::None => {}
        }
    }

    fn extract_user_data_details(body: &serde_json::Value) -> Option<(UserDataType, String)> {
        let object = body.as_object()?;
        let data_type_value = object
            .get("type")
            .or_else(|| object.get("user_data_type"))
            .or_else(|| object.get("userDataType"))?;

        let data_type = Self::parse_user_data_type(data_type_value)?;

        let candidate_keys = [
            "value",
            "stringValue",
            "text",
            "url",
            "username",
            "display",
            "bio",
        ];

        let mut extracted_value = None;
        for key in candidate_keys.iter() {
            if let Some(val) = object.get(*key).and_then(Self::value_to_string) {
                extracted_value = Some(val);
                break;
            }
        }

        let value = extracted_value
            .or_else(|| Self::value_to_string(body))
            .unwrap_or_else(|| serde_json::to_string(body).unwrap_or_default());

        Some((data_type, value))
    }

    async fn handle_onchain_event(
        database: &Database,
        event: &proto::OnChainEvent,
        block_height: u64,
        shard_id: u32,
    ) -> Result<()> {
        let activity_data = serde_json::json!({
            "event_type": event.r#type,
            "block_number": event.block_number,
            "transaction_hash": hex::encode(&event.transaction_hash),
            "block_hash": hex::encode(&event.block_hash),
            "log_index": event.log_index,
            "shard_id": shard_id,
        });

        if event.fid != 0 {
            Self::record_activity(
                database,
                event.fid as i64,
                "onchain_event",
                activity_data,
                block_height as i64,
                None,
            )
            .await?;
        }

        Ok(())
    }

    async fn handle_fname_transfer(
        database: &Database,
        transfer: &proto::FnameTransfer,
        block_height: u64,
        transaction_fid: u64,
    ) -> Result<()> {
        let inbound_activity = serde_json::json!({
            "transfer_id": transfer.id,
            "from_fid": transfer.from_fid,
            "to_fid": transfer.to_fid,
            "transaction_fid": transaction_fid,
            "direction": "in",
            "block_height": block_height,
        });

        if transfer.to_fid != 0 {
            Self::record_activity(
                database,
                transfer.to_fid as i64,
                "fname_transfer_in",
                inbound_activity,
                block_height as i64,
                None,
            )
            .await?;
        }

        if transfer.from_fid != 0 {
            let outbound_activity = serde_json::json!({
                "transfer_id": transfer.id,
                "from_fid": transfer.from_fid,
                "to_fid": transfer.to_fid,
                "transaction_fid": transaction_fid,
                "direction": "out",
                "block_height": block_height,
            });

            Self::record_activity(
                database,
                transfer.from_fid as i64,
                "fname_transfer_out",
                outbound_activity,
                block_height as i64,
                None,
            )
            .await?;
        }

        Ok(())
    }

    // Message type specific processing methods

    /// Process CastAdd message
    async fn process_cast_add(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("CastAdd message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let text = body.get("text").and_then(Self::value_to_string);
        let target_hash = body.get("targetHash").and_then(Self::value_to_string);
        let mentions = body.get("mentions").cloned();
        let embeds = body.get("embeds").cloned();

        let activity_data = serde_json::json!({
            "message_type": "cast_add",
            "text": text,
            "target_hash": target_hash,
            "mentions": mentions,
            "embeds": embeds,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "cast_add",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed CastAdd for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process CastRemove message
    async fn process_cast_remove(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("CastRemove message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let target_hash = body.get("targetHash").and_then(Self::value_to_string);

        let activity_data = serde_json::json!({
            "message_type": "cast_remove",
            "target_hash": target_hash,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "cast_remove",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed CastRemove for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process ReactionAdd message
    async fn process_reaction_add(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("ReactionAdd message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let reaction_type = body.get("type").and_then(Self::value_to_string);
        let target_hash = body.get("targetHash").and_then(Self::value_to_string);
        let target_fid = body.get("targetFid").and_then(Self::value_to_i64);
        let target_url = body.get("targetUrl").and_then(Self::value_to_string);

        let activity_data = serde_json::json!({
            "message_type": "reaction_add",
            "reaction_type": reaction_type,
            "target_hash": target_hash,
            "target_fid": target_fid,
            "target_url": target_url,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "reaction_add",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed ReactionAdd for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process ReactionRemove message
    async fn process_reaction_remove(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("ReactionRemove message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let reaction_type = body.get("type").and_then(Self::value_to_string);
        let target_hash = body.get("targetHash").and_then(Self::value_to_string);
        let target_fid = body.get("targetFid").and_then(Self::value_to_i64);

        let activity_data = serde_json::json!({
            "message_type": "reaction_remove",
            "reaction_type": reaction_type,
            "target_hash": target_hash,
            "target_fid": target_fid,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "reaction_remove",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed ReactionRemove for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process LinkAdd message
    async fn process_link_add(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("LinkAdd message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let link_type = body.get("type").and_then(Self::value_to_string);
        let target_fid = body.get("targetFid").and_then(Self::value_to_i64);
        let display_timestamp = body.get("displayTimestamp").and_then(Self::value_to_i64);

        let activity_data = serde_json::json!({
            "message_type": "link_add",
            "link_type": link_type,
            "target_fid": target_fid,
            "display_timestamp": display_timestamp,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "link_add",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed LinkAdd for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process LinkRemove message
    async fn process_link_remove(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("LinkRemove message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let link_type = body.get("type").and_then(Self::value_to_string);
        let target_fid = body.get("targetFid").and_then(Self::value_to_i64);

        let activity_data = serde_json::json!({
            "message_type": "link_remove",
            "link_type": link_type,
            "target_fid": target_fid,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "link_remove",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed LinkRemove for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process VerificationAdd message
    async fn process_verification_add(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("VerificationAdd message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let address = body.get("address").and_then(Self::value_to_string);
        let signature = body
            .get("signature")
            .and_then(Self::decode_bytes)
            .unwrap_or_default();
        let block_hash = body.get("blockHash").and_then(Self::value_to_string);
        let timestamp_ms = body.get("timestamp").and_then(Self::value_to_i64);

        let activity_data = serde_json::json!({
            "message_type": "verification_add",
            "address": address,
            "signature": base64::engine::general_purpose::STANDARD.encode(&signature),
            "block_hash": block_hash,
            "body_timestamp": timestamp_ms,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "verification_add",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed VerificationAdd for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process VerificationRemove message
    async fn process_verification_remove(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("VerificationRemove message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let address = body.get("address").and_then(Self::value_to_string);

        let activity_data = serde_json::json!({
            "message_type": "verification_remove",
            "address": address,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "verification_remove",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed VerificationRemove for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process UserDataAdd message (profile updates)
    async fn process_user_data_add(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("UserDataAdd message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let (data_type, value) = match Self::extract_user_data_details(&body) {
            Some(details) => details,
            None => {
                warn!("Unable to parse user data body for fid={}", fid);
                return Ok(());
            }
        };

        if value.is_empty() {
            warn!(
                "Empty user data value for fid={}, type={:?}",
                fid, data_type
            );
            return Ok(());
        }

        let message_hash = message.hash.clone();

        if self.database.get_user_profile(fid).await?.is_some() {
            match self
                .database
                .update_user_profile(UpdateUserProfileRequest {
                    fid,
                    data_type,
                    new_value: value.clone(),
                    message_hash: message_hash.clone(),
                    timestamp,
                })
                .await
            {
                Ok(_) => {}
                Err(SnapRagError::UserNotFound(_)) => {
                    let mut request =
                        Self::initialize_profile_request(fid, timestamp, message_hash.clone());
                    Self::apply_user_data_to_profile_request(&mut request, data_type, &value);
                    self.database.create_user_profile(request).await?;
                }
                Err(err) => return Err(err),
            }
        } else {
            let mut request =
                Self::initialize_profile_request(fid, timestamp, message_hash.clone());
            Self::apply_user_data_to_profile_request(&mut request, data_type, &value);
            self.database.create_user_profile(request).await?;
        }

        let activity_data = serde_json::json!({
            "message_type": "user_data_add",
            "data_type": data_type as i32,
            "value": value,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "user_data_update",
            activity_data,
            timestamp,
            Some(message_hash),
        )
        .await?;

        info!(
            "Processed UserDataAdd for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process UsernameProof message
    async fn process_username_proof(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("UsernameProof message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let name = body
            .get("name")
            .and_then(Self::value_to_string)
            .unwrap_or_default();
        let username_type_value = body.get("type").or_else(|| body.get("usernameType"));
        let username_type = username_type_value
            .and_then(Self::parse_username_type)
            .unwrap_or(UsernameType::None);
        let owner = body
            .get("owner")
            .and_then(Self::value_to_string)
            .unwrap_or_default();
        let signature = body
            .get("signature")
            .and_then(Self::decode_bytes)
            .unwrap_or_default();
        let proof_timestamp = body
            .get("timestamp")
            .and_then(Self::value_to_i64)
            .unwrap_or(timestamp);
        let proof_fid = body.get("fid").and_then(Self::value_to_i64).unwrap_or(fid);

        self.database
            .upsert_username_proof(
                proof_fid,
                name.clone(),
                username_type,
                owner.clone(),
                signature.clone(),
                proof_timestamp,
            )
            .await?;

        if !name.is_empty() {
            let message_hash = message.hash.clone();
            match self
                .database
                .update_user_profile(UpdateUserProfileRequest {
                    fid,
                    data_type: UserDataType::Username,
                    new_value: name.clone(),
                    message_hash: message_hash.clone(),
                    timestamp,
                })
                .await
            {
                Ok(_) => {}
                Err(SnapRagError::UserNotFound(_)) => {
                    let mut request =
                        Self::initialize_profile_request(fid, timestamp, message_hash);
                    request.username = Some(name.clone());
                    self.database.create_user_profile(request).await?;
                }
                Err(err) => return Err(err),
            }
        }

        let activity_data = serde_json::json!({
            "message_type": "username_proof",
            "name": name,
            "username_type": username_type as i32,
            "owner": owner,
            "signature": base64::engine::general_purpose::STANDARD.encode(&signature),
            "proof_timestamp": proof_timestamp,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "username_proof",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed UsernameProof for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process FrameAction message
    async fn process_frame_action(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("FrameAction message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let url = body.get("url").and_then(Self::value_to_string);
        let button_index = body.get("buttonIndex").and_then(Self::value_to_i64);

        let activity_data = serde_json::json!({
            "message_type": "frame_action",
            "url": url,
            "button_index": button_index,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "frame_action",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed FrameAction for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process LinkCompactState message
    async fn process_link_compact_state(
        &self,
        message: &proto::Message,
        fid: i64,
        timestamp: i64,
    ) -> Result<()> {
        let body = match Self::extract_message_body(message) {
            Some(body) => body,
            None => {
                warn!("LinkCompactState message missing body; fid={}", fid);
                return Ok(());
            }
        };

        let link_type = body.get("type").and_then(Self::value_to_string);
        let target_fids = body.get("targetFids").cloned();

        let activity_data = serde_json::json!({
            "message_type": "link_compact_state",
            "link_type": link_type,
            "target_fids": target_fids,
            "raw": body,
        });

        Self::record_activity(
            self.database.as_ref(),
            fid,
            "link_compact_state",
            activity_data,
            timestamp,
            Some(message.hash.clone()),
        )
        .await?;

        info!(
            "Processed LinkCompactState for FID {} at timestamp {}",
            fid, timestamp
        );
        Ok(())
    }

    /// Process OnChainEvent
    async fn process_onchain_event(
        &self,
        event: &crate::sync::client::proto::OnChainEvent,
    ) -> Result<()> {
        Self::handle_onchain_event(self.database.as_ref(), event, event.block_number, 0).await?;
        info!(
            "Processed OnChainEvent type={} fid={} block={}",
            event.r#type, event.fid, event.block_number
        );
        Ok(())
    }

    /// Process FnameTransfer
    async fn process_fname_transfer(
        &self,
        transfer: &crate::sync::client::proto::FnameTransfer,
    ) -> Result<()> {
        Self::handle_fname_transfer(self.database.as_ref(), transfer, 0, 0).await?;
        info!(
            "Processed FnameTransfer id={} from={} to={}",
            transfer.id, transfer.from_fid, transfer.to_fid
        );
        Ok(())
    }
}

impl Clone for SyncService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            client: self.client.clone(),
            database: self.database.clone(),
            state: self.state.clone(),
        }
    }
}
