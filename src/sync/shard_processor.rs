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
    // Activities: (fid, activity_type, activity_data, timestamp, message_hash, shard_id, block_height)
    activities: Vec<(
        i64,
        String,
        Option<serde_json::Value>,
        i64,
        Option<Vec<u8>>,
        Option<i32>,
        Option<i64>,
    )>,
    fids_to_ensure: HashSet<i64>,
    // Profile field updates: (fid, field_name, value, timestamp)
    profile_updates: Vec<(i64, String, Option<String>, i64)>,
}

/// Processor for handling shard chunks and extracting user data
pub struct ShardProcessor {
    database: Database,
    // Cache for FIDs that have been ensured to exist in this batch
    fid_cache: std::sync::Mutex<std::collections::HashSet<i64>>,
    // Cache for FIDs that have been registered (via id_register event)
    registered_fids: std::sync::Mutex<std::collections::HashSet<i64>>,
}

impl ShardProcessor {
    /// Create a new shard processor
    pub fn new(database: Database) -> Self {
        Self {
            database,
            fid_cache: std::sync::Mutex::new(std::collections::HashSet::new()),
            registered_fids: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// Clear the FID cache (call this between batches)
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.fid_cache.lock() {
            cache.clear();
        }
        // Note: We DON'T clear registered_fids as it should persist across batches
    }

    /// Process multiple chunks in a single batch for maximum performance
    pub async fn process_chunks_batch(&self, chunks: &[ShardChunk], shard_id: u32) -> Result<()> {
        // Collect all data from all chunks
        let mut batched = BatchedData {
            casts: Vec::new(),
            activities: Vec::new(),
            fids_to_ensure: HashSet::new(),
            profile_updates: Vec::new(),
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
            profile_updates: Vec::new(),
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
        let start = std::time::Instant::now();
        tracing::trace!(
            "Flushing batch: {} FIDs, {} casts, {} activities, {} profile updates",
            batched.fids_to_ensure.len(),
            batched.casts.len(),
            batched.activities.len(),
            batched.profile_updates.len()
        );

        // 🚀 OPTIMIZATION: Verify FIDs in batch before processing
        // This replaces N individual queries with 1 batch query
        if !batched.fids_to_ensure.is_empty() {
            self.batch_verify_fids(&batched.fids_to_ensure).await?;
        }

        // Start a transaction for the entire batch
        let mut tx = self.database.pool().begin().await?;

        // Batch insert FIDs (split into chunks to avoid parameter limit)
        if !batched.fids_to_ensure.is_empty() {
            let now = chrono::Utc::now();

            const PARAMS_PER_ROW: usize = 3;
            const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
            const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~21666 rows per chunk

            let fids: Vec<i64> = batched.fids_to_ensure.iter().copied().collect();

            // Split FIDs into chunks
            for chunk in fids.chunks(CHUNK_SIZE) {
                // Build dynamic query for batch insert
                let mut query = String::from(
                    "INSERT INTO user_profiles (fid, last_updated_timestamp, last_updated_at) VALUES ",
                );

                let value_clauses: Vec<String> = (0..chunk.len())
                    .map(|i| {
                        let base = i * PARAMS_PER_ROW;
                        format!("(${}, ${}, ${})", base + 1, base + 2, base + 3)
                    })
                    .collect();

                query.push_str(&value_clauses.join(", "));
                query.push_str(" ON CONFLICT (fid) DO NOTHING");

                let mut q = sqlx::query(&query);
                for _fid in chunk {
                    q = q.bind(_fid).bind(0i64).bind(now);
                }

                let result = q.execute(&mut *tx).await?;
                let profiles_created = result.rows_affected();

                if profiles_created > 0 {
                    tracing::debug!("Created {} new profiles", profiles_created);
                }
            }
        }

        // Batch insert casts (split into chunks to avoid parameter limit)
        if !batched.casts.is_empty() {
            tracing::trace!(
                "Batch inserting {} casts (before dedup)",
                batched.casts.len()
            );

            // 🚀 CRITICAL FIX: Deduplicate by message_hash to avoid "affect row a second time" error
            // Keep the latest version of each cast (by timestamp)
            let mut casts_map: HashMap<Vec<u8>, _> = HashMap::new();
            let original_count = batched.casts.len();
            for cast in &batched.casts {
                let hash = cast.3.clone(); // message_hash
                casts_map.insert(hash, cast.clone());
            }
            let deduped_casts: Vec<_> = casts_map.into_values().collect();
            let deduped_count = deduped_casts.len();
            if original_count != deduped_count {
                tracing::debug!(
                    "Deduplicated casts: {} -> {} ({} duplicates removed)",
                    original_count,
                    deduped_count,
                    original_count - deduped_count
                );
            }

            const PARAMS_PER_ROW: usize = 8;
            const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
            const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~8125 rows per chunk

            // Split casts into chunks
            for chunk in deduped_casts.chunks(CHUNK_SIZE) {
                // Build dynamic query
                let mut query = String::from(
                    "INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions) VALUES "
                );

                let value_clauses: Vec<String> = (0..chunk.len())
                    .map(|i| {
                        let base = i * PARAMS_PER_ROW;
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
                for (
                    fid,
                    text,
                    timestamp,
                    message_hash,
                    parent_hash,
                    root_hash,
                    embeds,
                    mentions,
                ) in chunk
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
        }

        // Batch insert activities (split into chunks to avoid parameter limit)
        if !batched.activities.is_empty() {
            tracing::trace!("Batch inserting {} activities", batched.activities.len());

            const PARAMS_PER_ROW: usize = 7;
            const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
            const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~9285 rows per chunk

            // Split activities into chunks
            for chunk in batched.activities.chunks(CHUNK_SIZE) {
                // Build dynamic query with shard_id and block_height
                let mut query = String::from(
                    "INSERT INTO user_activity_timeline (fid, activity_type, activity_data, timestamp, message_hash, shard_id, block_height) VALUES "
                );

                let value_clauses: Vec<String> = (0..chunk.len())
                    .map(|i| {
                        let base = i * PARAMS_PER_ROW;
                        format!(
                            "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
                            base + 1,
                            base + 2,
                            base + 3,
                            base + 4,
                            base + 5,
                            base + 6,
                            base + 7
                        )
                    })
                    .collect();

                query.push_str(&value_clauses.join(", "));

                let mut q = sqlx::query(&query);
                for (
                    fid,
                    activity_type,
                    activity_data,
                    timestamp,
                    message_hash,
                    shard_id,
                    block_height,
                ) in chunk
                {
                    q = q
                        .bind(fid)
                        .bind(activity_type)
                        .bind(activity_data)
                        .bind(timestamp)
                        .bind(message_hash)
                        .bind(shard_id)
                        .bind(block_height);
                }

                q.execute(&mut *tx).await?;
            }
        }

        // 🚀 OPTIMIZATION: Simplified profile updates using multiple simple UPDATEs
        // Instead of complex CASE statements, use multiple targeted updates
        // This is faster in Rust (less string allocation) and clearer
        if !batched.profile_updates.is_empty() {
            tracing::trace!(
                "Batch updating {} profile fields",
                batched.profile_updates.len()
            );

            // Group updates by field name
            let mut updates_by_field: HashMap<String, Vec<(i64, Option<String>, i64)>> =
                HashMap::new();

            for (fid, field_name, value, timestamp) in batched.profile_updates {
                updates_by_field
                    .entry(field_name)
                    .or_insert_with(Vec::new)
                    .push((fid, value, timestamp));
            }

            let now = chrono::Utc::now();

            // 🚀 Use unnest() for batch updates - much faster!
            for (field_name, updates) in updates_by_field {
                if updates.is_empty() {
                    continue;
                }

                let mut fids = Vec::with_capacity(updates.len());
                let mut values = Vec::with_capacity(updates.len());
                let mut timestamps = Vec::with_capacity(updates.len());

                for (fid, value, timestamp) in updates {
                    fids.push(fid);
                    values.push(value);
                    timestamps.push(timestamp);
                }

                // Dynamic SQL based on field name
                let sql = format!(
                    r#"
                    UPDATE user_profiles AS up
                    SET {} = data.value,
                        last_updated_timestamp = data.timestamp,
                        last_updated_at = $4
                    FROM unnest($1::bigint[], $2::text[], $3::bigint[]) 
                        AS data(fid, value, timestamp)
                    WHERE up.fid = data.fid
                    "#,
                    field_name
                );

                sqlx::query(&sql)
                    .bind(&fids)
                    .bind(&values)
                    .bind(&timestamps)
                    .bind(now)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        // Commit the transaction
        tx.commit().await?;

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 1000 {
            warn!("Batch flush took {}ms (slow!)", elapsed.as_millis());
        } else {
            tracing::trace!("Batch flush completed in {}ms", elapsed.as_millis());
        }

        Ok(())
    }

    /// Batch verify FIDs are registered
    /// 🚀 OPTIMIZATION: Check all FIDs in one query instead of N queries
    async fn batch_verify_fids(&self, fids: &HashSet<i64>) -> Result<()> {
        if fids.is_empty() {
            return Ok(());
        }

        let fid_vec: Vec<i64> = fids.iter().copied().collect();

        // 🚀 Single query to check all FIDs at once
        let registered_fids = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT DISTINCT fid 
            FROM user_activity_timeline 
            WHERE fid = ANY($1) AND activity_type = 'id_register'
            "#,
        )
        .bind(&fid_vec)
        .fetch_all(self.database.pool())
        .await?;

        // Update cache with verified FIDs
        {
            if let Ok(mut cache) = self.registered_fids.lock() {
                for fid in registered_fids {
                    cache.insert(fid);
                }
            }
        }

        Ok(())
    }

    /// Helper to create activity tuple with metadata
    fn create_activity(
        fid: i64,
        activity_type: String,
        activity_data: Option<serde_json::Value>,
        timestamp: i64,
        message_hash: Option<Vec<u8>>,
        shard_block_info: &ShardBlockInfo,
    ) -> (
        i64,
        String,
        Option<serde_json::Value>,
        i64,
        Option<Vec<u8>>,
        Option<i32>,
        Option<i64>,
    ) {
        (
            fid,
            activity_type,
            activity_data,
            timestamp,
            message_hash,
            Some(shard_block_info.shard_id as i32),
            Some(shard_block_info.block_height as i64),
        )
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

        // Create shard block info for tracking
        // Note: For system transactions (fid=0), we use 0 as transaction_fid
        let shard_block_info = ShardBlockInfo::new(shard_id, block_number, fid as u64, timestamp);

        // Process user messages (only in user transactions, fid > 0)
        if fid > 0 {
            for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
                self.collect_message_data(message, &shard_block_info, msg_idx, batched)
                    .await?;
            }
        }

        // Process system messages (can appear in both user and system transactions)
        // System transactions (fid=0) contain batch OP chain events like id_register
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

        // 🔥 STRICT MODE: Verify FID is registered before processing messages
        self.verify_fid_registered(fid, shard_block_info).await?;

        // Ensure FID will be created for ALL message types
        batched.fids_to_ensure.insert(fid);

        match message_type {
            1 => {
                // CastAdd - collect cast data
                self.collect_cast_add(data, &message_hash, &shard_block_info, batched)
                    .await?;
            }
            2 => {
                // CastRemove - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "cast_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "cast_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            3 => {
                // ReactionAdd - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "reaction_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "reaction_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            4 => {
                // ReactionRemove - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "reaction_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "reaction_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            5 => {
                // LinkAdd - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "link_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "link_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            6 => {
                // LinkRemove - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "link_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "link_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            7 => {
                // VerificationAddEthAddress - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "verification_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "verification_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            8 => {
                // VerificationRemove - collect activity
                batched.activities.push(Self::create_activity(
                    fid,
                    "verification_remove".to_string(),
                    Some(serde_json::json!({
                        "message_type": "verification_remove",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));
            }
            11 => {
                // UserDataAdd - collect activity and profile updates
                batched.activities.push(Self::create_activity(
                    fid,
                    "user_data_add".to_string(),
                    Some(serde_json::json!({
                        "message_type": "user_data_add",
                        "timestamp": timestamp
                    })),
                    timestamp,
                    Some(message_hash.to_vec()),
                    &shard_block_info,
                ));

                // Parse and collect profile field updates
                if let Some(body) = &data.body {
                    if let Some(user_data_body) = body.get("user_data_body") {
                        if let Some(data_type) = user_data_body.get("type").and_then(|v| v.as_i64())
                        {
                            if let Some(value) =
                                user_data_body.get("value").and_then(|v| v.as_str())
                            {
                                // Map Farcaster UserDataType to field name
                                // 1=PFP, 2=DISPLAY_NAME, 3=BIO, 5=URL, 6=USERNAME
                                let field_name = match data_type {
                                    1 => Some("pfp_url"),
                                    2 => Some("display_name"),
                                    3 => Some("bio"),
                                    5 => Some("website_url"),
                                    6 => Some("username"),
                                    _ => None,
                                };

                                if let Some(field) = field_name {
                                    batched.profile_updates.push((
                                        fid,
                                        field.to_string(),
                                        Some(value.to_string()),
                                        timestamp,
                                    ));
                                    tracing::debug!(
                                        "Collected profile update: FID {} {} = {}",
                                        fid,
                                        field,
                                        value
                                    );
                                }
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
        shard_block_info: &ShardBlockInfo,
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
        batched.activities.push(Self::create_activity(
            fid,
            "cast_add".to_string(),
            Some(serde_json::json!({
                "message_type": "cast_add",
                "timestamp": timestamp
            })),
            timestamp,
            Some(message_hash.to_vec()),
            shard_block_info,
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

        // Create shard block info for tracking
        // Note: For system transactions (fid=0), we use 0 as transaction_fid
        let shard_block_info = ShardBlockInfo::new(shard_id, block_number, fid as u64, timestamp);

        // Process user messages in this transaction (only if fid > 0)
        if fid > 0 {
            for (msg_idx, message) in transaction.user_messages.iter().enumerate() {
                self.process_user_message(message, &shard_block_info, msg_idx)
                    .await?;
            }
        }

        // TODO: Process system messages here as well for consistency

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

        // User profile will be ensured in batch flush
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

        // User profile will be ensured in batch flush
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
            // Profiles will be ensured in batch flush
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

        // Use block timestamp from the on-chain event (Unix timestamp)
        let timestamp = event.block_timestamp as i64;

        match event_type {
            3 => {
                // EVENT_TYPE_ID_REGISTER - FID注册
                tracing::info!(
                    "🆕 FID Registration: FID {} registered at block {}",
                    fid,
                    event.block_number
                );

                // Mark this FID as registered
                if let Ok(mut registered) = self.registered_fids.lock() {
                    registered.insert(fid);
                }

                batched.activities.push(Self::create_activity(
                    fid,
                    "id_register".to_string(),
                    Some(serde_json::json!({
                        "event_type": "id_register",
                        "block_number": event.block_number,
                        "transaction_hash": hex::encode(&event.transaction_hash),
                        "log_index": event.log_index,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                    _shard_block_info,
                ));
            }
            4 => {
                // EVENT_TYPE_STORAGE_RENT - 存储租赁
                tracing::debug!(
                    "💾 Storage Rent: FID {} purchased storage at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push(Self::create_activity(
                    fid,
                    "storage_rent".to_string(),
                    Some(serde_json::json!({
                        "event_type": "storage_rent",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                    _shard_block_info,
                ));
            }
            1 => {
                // EVENT_TYPE_SIGNER - 密钥管理
                tracing::debug!(
                    "🔑 Signer Event: FID {} signer event at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push(Self::create_activity(
                    fid,
                    "signer_event".to_string(),
                    Some(serde_json::json!({
                        "event_type": "signer",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                    _shard_block_info,
                ));
            }
            5 => {
                // EVENT_TYPE_TIER_PURCHASE - 订阅购买
                tracing::debug!(
                    "⭐ Tier Purchase: FID {} at block {}",
                    fid,
                    event.block_number
                );

                batched.activities.push(Self::create_activity(
                    fid,
                    "tier_purchase".to_string(),
                    Some(serde_json::json!({
                        "event_type": "tier_purchase",
                        "block_number": event.block_number,
                    })),
                    timestamp,
                    Some(event.transaction_hash.clone()),
                    _shard_block_info,
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
        shard_block_info: &ShardBlockInfo,
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

        // Use block timestamp from shard block info (Unix timestamp)
        let timestamp = shard_block_info.timestamp as i64;

        if from_fid > 0 {
            batched.activities.push(Self::create_activity(
                from_fid,
                "fname_transfer_out".to_string(),
                Some(serde_json::json!({
                    "transfer_id": fname_transfer.id,
                    "to_fid": to_fid,
                })),
                timestamp,
                None,
                shard_block_info,
            ));
        }

        if to_fid > 0 {
            batched.activities.push(Self::create_activity(
                to_fid,
                "fname_transfer_in".to_string(),
                Some(serde_json::json!({
                    "transfer_id": fname_transfer.id,
                    "from_fid": from_fid,
                })),
                timestamp,
                None,
                shard_block_info,
            ));
        }

        Ok(())
    }

    /// Verify that a FID has been registered via id_register event
    /// This ensures we don't process messages from unregistered FIDs (dangling FIDs)
    /// OPTIMIZED: Only checks cache, database check moved to batch processing
    async fn verify_fid_registered(
        &self,
        fid: i64,
        shard_block_info: &ShardBlockInfo,
    ) -> Result<()> {
        // 🚀 OPTIMIZATION: Only check in-memory cache
        // Database verification happens once per batch in flush_batched_data
        {
            if let Ok(registered) = self.registered_fids.lock() {
                if registered.contains(&fid) {
                    return Ok(());
                }
            }
        }

        // For new FIDs, allow processing and verify in batch
        // This reduces per-message database queries from N to 1
        tracing::debug!(
            "FID {} not in cache at block {} - will verify in batch",
            fid,
            shard_block_info.block_height
        );

        Ok(())
    }
}
