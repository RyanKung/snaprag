use std::collections::HashMap;
use std::collections::HashSet;

use tracing::error;
use tracing::info;
use tracing::warn;

use crate::database::Database;
use crate::models::ShardBlockInfo;
use crate::models::UserActivityTimeline;
use crate::models::UserDataChange;
use crate::models::UserProfile;
use crate::models::UserProfileSnapshot;
use crate::sync::client::proto::Message as FarcasterMessage;
use crate::sync::client::proto::MessageData;
use crate::sync::client::proto::ShardChunk;
use crate::sync::client::proto::Transaction;
use crate::sync::client::proto::ValidatorMessage;
use crate::Result;

/// Batched data for bulk insert
struct BatchedData {
    casts: Vec<(
        i64,
        Option<String>,
        i64,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
    )>,
    activities: Vec<(i64, String, Option<serde_json::Value>, i64, Option<Vec<u8>>)>,
    fids_to_ensure: HashSet<i64>,
}

/// Processor for handling shard chunks and extracting user data
pub struct ShardProcessor {
    database: Database,
    // Cache for FIDs that have been ensured to exist in this batch
    fid_cache: std::sync::Mutex<std::collections::HashSet<i64>>,
}

impl ShardProcessor {
    /// Create a new shard processor
    pub fn new(database: Database) -> Self {
        Self {
            database,
            fid_cache: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// Clear the FID cache (call this between batches)
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.fid_cache.lock() {
            cache.clear();
        }
    }

    /// Process multiple chunks in a single batch for maximum performance
    pub async fn process_chunks_batch(&self, chunks: &[ShardChunk], shard_id: u32) -> Result<()> {
        // Collect all data from all chunks
        let mut batched = BatchedData {
            casts: Vec::new(),
            activities: Vec::new(),
            fids_to_ensure: HashSet::new(),
        };

        for chunk in chunks {
            let header = chunk
                .header
                .as_ref()
                .ok_or_else(|| crate::SnapRagError::Custom("Missing chunk header".to_string()))?;

            let height = header
                .height
                .as_ref()
                .ok_or_else(|| crate::SnapRagError::Custom("Missing header height".to_string()))?;

            let block_number = height.block_number;
            let timestamp = header.timestamp;

            tracing::debug!(
                "Processing shard {} block {} with {} transactions",
                shard_id,
                block_number,
                chunk.transactions.len()
            );

            // Process each transaction and collect data
            for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
                self.collect_transaction_data(
                    transaction,
                    shard_id,
                    block_number,
                    timestamp,
                    tx_idx,
                    &mut batched,
                )
                .await?;
            }
        }

        // Single batch insert for all chunks
        self.flush_batched_data(batched).await?;

        // Update sync progress for the last chunk
        if let Some(last_chunk) = chunks.last() {
            if let Some(header) = &last_chunk.header {
                if let Some(height) = &header.height {
                    self.database
                        .update_last_processed_height(shard_id, height.block_number)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Process a shard chunk and extract all user data
    pub async fn process_chunk(&self, chunk: &ShardChunk, shard_id: u32) -> Result<()> {
        let header = chunk
            .header
            .as_ref()
            .ok_or_else(|| crate::SnapRagError::Custom("Missing chunk header".to_string()))?;

        let height = header
            .height
            .as_ref()
            .ok_or_else(|| crate::SnapRagError::Custom("Missing header height".to_string()))?;

        let block_number = height.block_number;
        let timestamp = header.timestamp;

        tracing::debug!(
            "Processing shard {} block {} with {} transactions",
            shard_id,
            block_number,
            chunk.transactions.len()
        );

        // Collect all data for batch insert
        let mut batched = BatchedData {
            casts: Vec::new(),
            activities: Vec::new(),
            fids_to_ensure: HashSet::new(),
        };

        // Process each transaction and collect data
        for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
            self.collect_transaction_data(
                transaction,
                shard_id,
                block_number,
                timestamp,
                tx_idx,
                &mut batched,
            )
            .await?;
        }

        // Batch insert all collected data
        self.flush_batched_data(batched).await?;

        // Update sync progress
        self.database
            .update_last_processed_height(shard_id, block_number)
            .await?;

        Ok(())
    }

    /// Flush batched data to database
    async fn flush_batched_data(&self, batched: BatchedData) -> Result<()> {
        tracing::debug!(
            "Flushing batch: {} FIDs, {} casts, {} activities",
            batched.fids_to_ensure.len(),
            batched.casts.len(),
            batched.activities.len()
        );

        // Start a transaction for the entire batch
        let mut tx = self.database.pool().begin().await?;

        // Batch insert FIDs
        if !batched.fids_to_ensure.is_empty() {
            let now = chrono::Utc::now();

            // Build dynamic query for batch insert
            let mut query = String::from(
                "INSERT INTO user_profiles (fid, last_updated_timestamp, last_updated_at) VALUES ",
            );

            let params_per_row = 3;
            let fids: Vec<i64> = batched.fids_to_ensure.iter().copied().collect();
            let value_clauses: Vec<String> = (0..fids.len())
                .map(|i| {
                    let base = i * params_per_row;
                    format!("(${}, ${}, ${})", base + 1, base + 2, base + 3)
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(" ON CONFLICT (fid) DO NOTHING");

            let mut q = sqlx::query(&query);
            for _fid in &fids {
                q = q.bind(_fid).bind(0i64).bind(now);
            }

            let result = q.execute(&mut *tx).await?;
            let profiles_created = result.rows_affected();

            if profiles_created > 0 {
                tracing::debug!("Created {} new profiles", profiles_created);
            }
        }

        // Batch insert casts
        if !batched.casts.is_empty() {
            tracing::debug!("Batch inserting {} casts", batched.casts.len());

            // Build dynamic query
            let mut query = String::from(
                "INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions) VALUES "
            );

            let params_per_row = 8;
            let value_clauses: Vec<String> = (0..batched.casts.len())
                .map(|i| {
                    let base = i * params_per_row;
                    format!(
                        "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5,
                        base + 6,
                        base + 7,
                        base + 8
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(
                " ON CONFLICT (message_hash) DO UPDATE SET \
                fid = EXCLUDED.fid, \
                text = EXCLUDED.text, \
                timestamp = EXCLUDED.timestamp, \
                parent_hash = EXCLUDED.parent_hash, \
                root_hash = EXCLUDED.root_hash, \
                embeds = EXCLUDED.embeds, \
                mentions = EXCLUDED.mentions",
            );

            let mut q = sqlx::query(&query);
            for (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions) in
                batched.casts
            {
                q = q
                    .bind(fid)
                    .bind(text)
                    .bind(timestamp)
                    .bind(message_hash)
                    .bind(parent_hash)
                    .bind(root_hash)
                    .bind(embeds)
                    .bind(mentions);
            }

            q.execute(&mut *tx).await?;
        }

        // Batch insert activities
        if !batched.activities.is_empty() {
            tracing::debug!("Batch inserting {} activities", batched.activities.len());

            // Build dynamic query
            let mut query = String::from(
                "INSERT INTO user_activity_timeline (fid, activity_type, activity_data, timestamp, message_hash) VALUES "
            );

            let params_per_row = 5;
            let value_clauses: Vec<String> = (0..batched.activities.len())
                .map(|i| {
                    let base = i * params_per_row;
                    format!(
                        "(${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));

            let mut q = sqlx::query(&query);
            for (fid, activity_type, activity_data, timestamp, message_hash) in batched.activities {
                q = q
                    .bind(fid)
                    .bind(activity_type)
                    .bind(activity_data)
                    .bind(timestamp)
                    .bind(message_hash);
            }

            q.execute(&mut *tx).await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(())
    }

    /// Collect data from a single transaction
    async fn collect_transaction_data(
        &self,
        transaction: &Transaction,
        shard_id: u32,
        block_number: u64,
        timestamp: u64,
        tx_index: usize,
        batched: &mut BatchedData,
    ) -> Result<()> {
        let fid = transaction.fid;

        // Skip system transactions (FID = 0)
        if fid == 0 {
            return Ok(());
        }

        // Create shard block info for tracking
        let shard_block_info = ShardBlockInfo::new(shard_id, block_number, fid as u64, timestamp);

        // Collect user messages data
        for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
            self.collect_message_data(message, &shard_block_info, msg_idx, batched)
                .await?;
        }

        // Process system messages (on-chain events, fname transfers, etc.)
        for system_msg in &transaction.system_messages {
            self.process_system_message(system_msg, &shard_block_info, batched)
                .await?;
        }

        Ok(())
    }

    /// Collect data from a single user message
    async fn collect_message_data(
        &self,
        message: &FarcasterMessage,
        shard_block_info: &ShardBlockInfo,
        msg_index: usize,
        batched: &mut BatchedData,
    ) -> Result<()> {
        let data = message
            .data
            .as_ref()
            .ok_or_else(|| crate::SnapRagError::Custom("Missing message data".to_string()))?;

        let message_type = data.r#type;
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;
        let message_hash = message.hash.clone();

        // Ensure FID will be created for ALL message types
        batched.fids_to_ensure.insert(fid);

        match message_type {
            1 => {
                // CastAdd - collect cast data
                self.collect_cast_add(data, &message_hash, batched).await?;
            }
            2 => {
                // CastRemove - collect activity
                batched.activities.push((
                    fid,
                    "cast_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "cast_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            3 => {
                // ReactionAdd - collect activity
                batched.activities.push((
                    fid,
                    "reaction_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "reaction_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            4 => {
                // ReactionRemove - collect activity
                batched.activities.push((
                    fid,
                    "reaction_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "reaction_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            5 => {
                // LinkAdd - collect activity
                batched.activities.push((
                    fid,
                    "link_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "link_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            6 => {
                // LinkRemove - collect activity
                batched.activities.push((
                    fid,
                    "link_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "link_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            7 => {
                // VerificationAddEthAddress - collect activity
                batched.activities.push((
                    fid,
                    "verification_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "verification_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            8 => {
                // VerificationRemove - collect activity
                batched.activities.push((
                    fid,
                    "verification_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "verification_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));
            }
            11 => {
                // UserDataAdd - collect activity (profile updates handled separately)
                batched.activities.push((
                    fid,
                    "user_data_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "user_data_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                ));

                // Also update profile fields directly in the same transaction
                // Parse and store user data updates
                if let Some(body) = &data.body {
                    if let Some(user_data_body) = body.get("user_data_body") {
                        if let Some(data_type) = user_data_body.get("type").and_then(|v| v.as_i64())
                        {
                            if let Some(value) =
                                user_data_body.get("value").and_then(|v| v.as_str())
                            {
                                // Store for later batch processing
                                // For now, we'll need to handle this in flush_batched_data
                                tracing::debug!(
                                    "UserDataAdd type {} for FID {}: {}",
                                    data_type,
                                    fid,
                                    value
                                );
                            }
                        }
                    }
                }
            }
            _ => {
                tracing::debug!("Unknown message type {} for FID {}", message_type, fid);
            }
        }

        Ok(())
    }

    /// Collect cast data for batch insert
    async fn collect_cast_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        batched: &mut BatchedData,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        // FID already ensured in collect_message_data, no need to insert again

        // Parse cast data from the body
        let mut text = None;
        let mut parent_hash = None;
        let root_hash = None;
        let mut embeds = None;
        let mut mentions = None;

        if let Some(body) = &data.body {
            if let Some(cast_add_body) = body.get("cast_add_body") {
                // Extract text
                if let Some(text_value) = cast_add_body.get("text") {
                    text = text_value.as_str().map(|s| s.to_string());
                }

                // Extract parent cast info
                if let Some(parent) = cast_add_body.get("parent") {
                    if let Some(parent_cast_id) = parent.get("parent_cast_id") {
                        if let Some(parent_hash_value) = parent_cast_id.get("hash") {
                            if let Some(hash_str) = parent_hash_value.as_str() {
                                parent_hash = hex::decode(hash_str).ok();
                            }
                        }
                    }
                }

                // Extract embeds
                if let Some(embeds_value) = cast_add_body.get("embeds") {
                    embeds = Some(embeds_value.clone());
                }

                // Extract mentions
                if let Some(mentions_value) = cast_add_body.get("mentions") {
                    mentions = Some(mentions_value.clone());
                }
            }
        }

        // Collect cast for batch insert
        batched.casts.push((
            fid,
            text.clone(),
            timestamp,
            message_hash.to_vec(),
            parent_hash,
            root_hash,
            embeds,
            mentions,
        ));

        // Collect activity for batch insert
        batched.activities.push((
            fid,
            "cast_add".to_string(),
            Some(serde_json::json!({
                "message_type": "cast_add",
                "timestamp": timestamp
            })),
            timestamp,
            Some(message_hash.to_vec()),
        ));

        Ok(())
    }

    /// Process a single transaction (fallback for non-batched types)
    async fn process_transaction(
        &self,
        transaction: &Transaction,
        shard_id: u32,
        block_number: u64,
        timestamp: u64,
        tx_index: usize,
    ) -> Result<()> {
        let fid = transaction.fid;

        // Skip system transactions (FID = 0)
        if fid == 0 {
            return Ok(());
        }

        // Create shard block info for tracking
        let shard_block_info = ShardBlockInfo::new(shard_id, block_number, fid as u64, timestamp);

        // Process user messages in this transaction
        for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
            self.process_user_message(message, &shard_block_info, msg_idx)
                .await?;
        }

        Ok(())
    }

    /// Process a single user message
    async fn process_user_message(
        &self,
        message: &FarcasterMessage,
        shard_block_info: &ShardBlockInfo,
        msg_index: usize,
    ) -> Result<()> {
        let data = message
            .data
            .as_ref()
            .ok_or_else(|| crate::SnapRagError::Custom("Missing message data".to_string()))?;

        let message_type = data.r#type;
        let fid = data.fid;
        let timestamp = data.timestamp;
        let message_hash = message.hash.clone();

        match message_type {
            1 => {
                // CastAdd
                self.process_cast_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            2 => {
                // CastRemove
                self.process_cast_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            3 => {
                // ReactionAdd
                self.process_reaction_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            4 => {
                // ReactionRemove
                self.process_reaction_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            5 => {
                // LinkAdd (Follow)
                self.process_link_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            6 => {
                // LinkRemove (Unfollow)
                self.process_link_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            7 => {
                // VerificationAddEthAddress
                self.process_verification_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            8 => {
                // VerificationRemove
                self.process_verification_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            11 => {
                // UserDataAdd
                self.process_user_data_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            _ => {
                // Log unhandled message types (12=UsernameProof, 13=FrameAction, etc.)
                // These are less common, so we log them but don't fail
                if message_type > 0 {
                    warn!("Unhandled message type: {} for FID {}", message_type, fid);
                }
            }
        }

        Ok(())
    }

    /// Process UserDataAdd message (profile creation/update)
    async fn process_user_data_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        tracing::debug!(
            "Found UserDataAdd message for FID {} at timestamp {}",
            fid,
            timestamp
        );

        // Parse user data from the body
        let mut data_type = 0;
        let mut value = "user_data_add".to_string();

        if let Some(body) = &data.body {
            if let Some(user_data_body) = body.get("user_data_body") {
                // Extract data type
                if let Some(type_value) = user_data_body.get("type") {
                    data_type = type_value.as_i64().unwrap_or(0) as i16;
                }

                // Extract value
                if let Some(value_value) = user_data_body.get("value") {
                    if let Some(value_str) = value_value.as_str() {
                        value = value_str.to_string();
                    }
                }
            }
        }

        // Ensure user profile exists first
        self.ensure_user_profile_exists(fid, timestamp).await?;

        // Update user profile fields based on data_type
        // Farcaster UserDataType: 1=PFP, 2=DISPLAY_NAME, 3=BIO, 5=URL, 6=USERNAME
        match data_type {
            1 => {
                // Profile picture
                self.update_profile_field(fid, "pfp_url", Some(value.clone()), timestamp)
                    .await?;
            }
            2 => {
                // Display name
                self.update_profile_field(fid, "display_name", Some(value.clone()), timestamp)
                    .await?;
            }
            3 => {
                // Bio
                self.update_profile_field(fid, "bio", Some(value.clone()), timestamp)
                    .await?;
            }
            5 => {
                // Website URL
                self.update_profile_field(fid, "website_url", Some(value.clone()), timestamp)
                    .await?;
            }
            6 => {
                // Username
                self.update_profile_field(fid, "username", Some(value.clone()), timestamp)
                    .await?;
            }
            _ => {
                tracing::debug!("Unknown user data type {} for FID {}", data_type, fid);
            }
        }

        // Insert user data into database
        self.database
            .upsert_user_data(
                fid,
                data_type,
                value.clone(),
                timestamp,
                message_hash.to_vec(),
            )
            .await?;

        // Record user data change
        self.database
            .record_user_data_change(
                fid,
                data_type,
                None, // old_value
                value,
                timestamp,
                message_hash.to_vec(),
            )
            .await?;

        Ok(())
    }

    /// Update a specific field in user profile
    async fn update_profile_field(
        &self,
        fid: i64,
        field_name: &str,
        value: Option<String>,
        timestamp: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now();

        // Build dynamic SQL based on field name
        let sql = format!(
            r#"
            UPDATE user_profiles 
            SET {} = $1, last_updated_timestamp = $2, last_updated_at = $3
            WHERE fid = $4
            "#,
            field_name
        );

        sqlx::query(&sql)
            .bind(value)
            .bind(timestamp)
            .bind(now)
            .bind(fid)
            .execute(self.database.pool())
            .await?;

        Ok(())
    }

    /// Process CastAdd message
    async fn process_cast_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        tracing::debug!(
            "Found CastAdd message for FID {} at timestamp {}",
            fid,
            timestamp
        );

        // Ensure user profile exists (create minimal profile if not)
        self.ensure_user_profile_exists(fid, timestamp).await?;

        // Parse cast data from the body
        let mut text = None;
        let mut parent_hash = None;
        let root_hash = None;
        let mut embeds = None;
        let mut mentions = None;

        if let Some(body) = &data.body {
            if let Some(cast_add_body) = body.get("cast_add_body") {
                // Extract text
                if let Some(text_value) = cast_add_body.get("text") {
                    text = text_value.as_str().map(|s| s.to_string());
                }

                // Extract parent cast info
                if let Some(parent) = cast_add_body.get("parent") {
                    if let Some(parent_cast_id) = parent.get("parent_cast_id") {
                        if let Some(parent_hash_value) = parent_cast_id.get("hash") {
                            if let Some(hash_str) = parent_hash_value.as_str() {
                                parent_hash = hex::decode(hash_str).ok();
                            }
                        }
                    }
                }

                // Extract embeds
                if let Some(embeds_value) = cast_add_body.get("embeds") {
                    embeds = Some(embeds_value.clone());
                }

                // Extract mentions
                if let Some(mentions_value) = cast_add_body.get("mentions") {
                    mentions = Some(mentions_value.clone());
                }
            }
        }

        // Insert cast into database
        self.database
            .upsert_cast(
                fid,
                text,
                timestamp,
                message_hash.to_vec(),
                parent_hash,
                root_hash,
                embeds,
                mentions,
            )
            .await?;

        // Record cast activity
        self.database
            .record_user_activity(
                fid,
                "cast_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "cast_add",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Process CastRemove message
    async fn process_cast_remove(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        tracing::debug!(
            "Found CastRemove message for FID {} at timestamp {}",
            fid,
            timestamp
        );

        // Record cast removal activity
        self.database
            .record_user_activity(
                fid,
                "cast_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "cast_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Process ReactionAdd message
    async fn process_reaction_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        tracing::debug!(
            "Found ReactionAdd message for FID {} at timestamp {}",
            fid,
            timestamp
        );

        // Parse reaction data from the body
        let mut target_fid = None;
        let mut reaction_type = None;

        if let Some(body) = &data.body {
            if let Some(reaction_body) = body.get("reaction_body") {
                // Extract target cast info
                if let Some(target) = reaction_body.get("target") {
                    if let Some(cast_id) = target.get("cast_id") {
                        if let Some(target_fid_value) = cast_id.get("fid") {
                            target_fid = target_fid_value.as_u64().map(|f| f as i64);
                        }
                    }
                }

                // Extract reaction type
                if let Some(type_value) = reaction_body.get("type") {
                    reaction_type = type_value.as_i64();
                }
            }
        }

        // Record reaction activity with parsed data
        let mut activity_data = serde_json::json!({
            "message_type": "reaction_add",
            "timestamp": timestamp
        });

        if let Some(tfid) = target_fid {
            activity_data["target_fid"] = serde_json::Value::Number(serde_json::Number::from(tfid));
        }
        if let Some(rt) = reaction_type {
            activity_data["reaction_type"] =
                serde_json::Value::Number(serde_json::Number::from(rt));
        }

        self.database
            .record_user_activity(
                fid,
                "reaction_add".to_string(),
                Some(activity_data),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Process ReactionRemove message
    async fn process_reaction_remove(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        tracing::debug!(
            "Found ReactionRemove message for FID {} at timestamp {}",
            fid,
            timestamp
        );

        // Record reaction removal activity
        self.database
            .record_user_activity(
                fid,
                "reaction_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "reaction_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Process LinkAdd message (follow relationship)
    async fn process_link_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        // Parse link data from the body
        let mut target_fid = None;
        let mut link_type = "follow".to_string();

        if let Some(body) = &data.body {
            if let Some(link_body) = body.get("link_body") {
                // Extract target FID
                if let Some(target_value) = link_body.get("targetFid") {
                    target_fid = target_value.as_i64();
                }
                // Extract link type if available
                if let Some(type_value) = link_body.get("type") {
                    if let Some(type_str) = type_value.as_str() {
                        link_type = type_str.to_string();
                    }
                }
            }
        }

        if let Some(target_fid) = target_fid {
            // Ensure both source and target profiles exist
            self.ensure_user_profile_exists(fid, timestamp).await?;
            self.ensure_user_profile_exists(target_fid, timestamp)
                .await?;

            // Insert link into database
            self.database
                .upsert_link(
                    fid,
                    target_fid,
                    &link_type,
                    timestamp,
                    message_hash.to_vec(),
                )
                .await?;

            tracing::debug!(
                "Recorded link: FID {} → FID {} ({})",
                fid,
                target_fid,
                link_type
            );
        }

        Ok(())
    }

    /// Process LinkRemove message (unfollow)
    async fn process_link_remove(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        _shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        // Record link removal activity
        self.database
            .record_user_activity(
                fid,
                "link_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "link_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Process VerificationAddEthAddress message
    async fn process_verification_add(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        _shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        // Parse verification data
        if let Some(body) = &data.body {
            if let Some(verification_body) = body.get("verificationAddAddressBody") {
                if let Some(address) = verification_body.get("address") {
                    if let Some(eth_address) = address.as_str() {
                        // Update user profile with verified Ethereum address
                        self.update_profile_field(
                            fid,
                            "primary_address_ethereum",
                            Some(eth_address.to_string()),
                            timestamp,
                        )
                        .await?;

                        tracing::debug!(
                            "Recorded ETH verification for FID {}: {}",
                            fid,
                            eth_address
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Process VerificationRemove message
    async fn process_verification_remove(
        &self,
        data: &MessageData,
        message_hash: &[u8],
        _shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        let fid = data.fid as i64;
        let timestamp = data.timestamp as i64;

        // Record verification removal
        self.database
            .record_user_activity(
                fid,
                "verification_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "verification_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
            )
            .await?;

        Ok(())
    }

    /// Ensure a user profile exists for the given FID
    /// Creates a minimal profile if it doesn't exist
    /// Uses cache to avoid redundant database checks
    async fn ensure_user_profile_exists(&self, fid: i64, timestamp: i64) -> Result<()> {
        // Check cache first
        {
            if let Ok(cache) = self.fid_cache.lock() {
                if cache.contains(&fid) {
                    return Ok(());
                }
            }
        }

        // Create minimal user profile using direct SQL with ON CONFLICT DO NOTHING
        // This is fast because it doesn't query first - just inserts if not exists
        let now = chrono::Utc::now();
        match sqlx::query(
            r#"
            INSERT INTO user_profiles (
                fid, last_updated_timestamp, last_updated_at
            )
            VALUES ($1, $2, $3)
            ON CONFLICT (fid) DO NOTHING
            "#,
        )
        .bind(fid)
        .bind(timestamp)
        .bind(now)
        .execute(self.database.pool())
        .await
        {
            Ok(result) => {
                // Add to cache
                if let Ok(mut cache) = self.fid_cache.lock() {
                    cache.insert(fid);
                }

                if result.rows_affected() > 0 {
                    tracing::debug!("Created new profile for FID {}", fid);
                }
            }
            Err(e) => {
                warn!("Failed to ensure profile for FID {}: {}", fid, e);
            }
        }

        Ok(())
    }

    /// Process system message (on-chain events, fname transfers, etc.)
    async fn process_system_message(
        &self,
        system_msg: &ValidatorMessage,
        shard_block_info: &ShardBlockInfo,
        batched: &mut BatchedData,
    ) -> Result<()> {
        // Process on-chain events
        if let Some(on_chain_event) = &system_msg.on_chain_event {
            self.process_on_chain_event(on_chain_event, shard_block_info, batched)
                .await?;
        }

        // Process fname transfers
        if let Some(fname_transfer) = &system_msg.fname_transfer {
            self.process_fname_transfer(fname_transfer, shard_block_info, batched)
                .await?;
        }

        Ok(())
    }

    /// Process on-chain event (FID registration, storage, signers, etc.)
    async fn process_on_chain_event(
        &self,
        event: &crate::sync::client::proto::OnChainEvent,
        _shard_block_info: &ShardBlockInfo,
        batched: &mut BatchedData,
    ) -> Result<()> {
        let fid = event.fid as i64;
        let event_type = event.r#type;

        // Ensure FID exists
        batched.fids_to_ensure.insert(fid);

        // Use current timestamp as fallback for activity timestamp
        let timestamp = chrono::Utc::now().timestamp();

        match event_type {
            3 => {
                // EVENT_TYPE_ID_REGISTER - FID注册
                tracing::info!(
                    "🆕 FID Registration: FID {} registered at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push((
                    fid,
                    "fid_register".to_string(),
                    Some(serde_json::json!({
                        "event_type": "id_register",
                        "block_number": event.block_number,
                        "transaction_hash": hex::encode(&event.transaction_hash),
                        "log_index": event.log_index,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                ));
            }
            4 => {
                // EVENT_TYPE_STORAGE_RENT - 存储租赁
                tracing::debug!(
                    "💾 Storage Rent: FID {} purchased storage at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push((
                    fid,
                    "storage_rent".to_string(),
                    Some(serde_json::json!({
                        "event_type": "storage_rent",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                ));
            }
            1 => {
                // EVENT_TYPE_SIGNER - 密钥管理
                tracing::debug!(
                    "🔑 Signer Event: FID {} signer event at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push((
                    fid,
                    "signer_event".to_string(),
                    Some(serde_json::json!({
                        "event_type": "signer",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                ));
            }
            5 => {
                // EVENT_TYPE_TIER_PURCHASE - 订阅购买
                tracing::debug!(
                    "⭐ Tier Purchase: FID {} at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push((
                    fid,
                    "tier_purchase".to_string(),
                    Some(serde_json::json!({
                        "event_type": "tier_purchase",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                ));
            }
            _ => {
                tracing::debug!("Unknown on-chain event type {} for FID {}", event_type, fid);
            }
        }

        Ok(())
    }

    /// Process fname transfer
    async fn process_fname_transfer(
        &self,
        fname_transfer: &crate::sync::client::proto::FnameTransfer,
        _shard_block_info: &ShardBlockInfo,
        batched: &mut BatchedData,
    ) -> Result<()> {
        let from_fid = fname_transfer.from_fid as i64;
        let to_fid = fname_transfer.to_fid as i64;

        tracing::debug!(
            "📛 Fname Transfer: {} transferred fname from FID {} to FID {}",
            fname_transfer.id,
            from_fid,
            to_fid
        );

        // Skip FID=0 (system/initialization data)
        // Only process valid FIDs
        if from_fid > 0 {
            batched.fids_to_ensure.insert(from_fid);
        }
        if to_fid > 0 {
            batched.fids_to_ensure.insert(to_fid);
        }

        // Only record activities for valid FIDs (> 0)
        let timestamp = chrono::Utc::now().timestamp();

        if from_fid > 0 {
            batched.activities.push((
                from_fid,
                "fname_transfer_out".to_string(),
                Some(serde_json::json!({
                    "transfer_id": fname_transfer.id,
                    "to_fid": to_fid,
                })),
                timestamp,
                None,
            ));
        }

        if to_fid > 0 {
            batched.activities.push((
                to_fid,
                "fname_transfer_in".to_string(),
                Some(serde_json::json!({
                    "transfer_id": fname_transfer.id,
                    "from_fid": from_fid,
                })),
                timestamp,
                None,
            ));
        }

        Ok(())
    }
}
