use crate::database::Database;
use crate::models::{
    ShardBlockInfo, UserActivityTimeline, UserDataChange, UserProfile, UserProfileSnapshot,
};
use crate::sync::client::proto::{
    Message as FarcasterMessage, MessageData, ShardChunk, Transaction,
};
use crate::Result;
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Processor for handling shard chunks and extracting user data
pub struct ShardProcessor {
    database: Database,
}

impl ShardProcessor {
    /// Create a new shard processor
    pub fn new(database: Database) -> Self {
        Self { database }
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

        info!(
            "Processing shard {} block {} with {} transactions",
            shard_id,
            block_number,
            chunk.transactions.len()
        );

        // Process each transaction in the chunk
        for (tx_idx, transaction) in chunk.transactions.iter().enumerate() {
            self.process_transaction(transaction, shard_id, block_number, timestamp, tx_idx)
                .await?;
        }

        // Update sync progress
        self.database
            .update_last_processed_height(shard_id, block_number)
            .await?;

        Ok(())
    }

    /// Process a single transaction
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
            11 => {
                // UserDataAdd
                // Handle profile creation/update
                self.process_user_data_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            1 => {
                // CastAdd
                // Handle cast creation
                self.process_cast_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            2 => {
                // CastRemove
                // Handle cast removal
                self.process_cast_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            3 => {
                // ReactionAdd
                // Handle reaction addition
                self.process_reaction_add(data, &message_hash, shard_block_info)
                    .await?;
            }
            4 => {
                // ReactionRemove
                // Handle reaction removal
                self.process_reaction_remove(data, &message_hash, shard_block_info)
                    .await?;
            }
            _ => {
                // Log unhandled message types
                warn!("Unhandled message type: {} for FID {}", message_type, fid);
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

        // For now, just log that we found a user data add message
        // The actual parsing would require understanding the protobuf body structure
        info!(
            "Found UserDataAdd message for FID {} at timestamp {}",
            fid, timestamp
        );

        // Record basic user data change
        self.database
            .record_user_data_change(
                fid,
                0,                           // data_type - would need to parse from body
                None,                        // old_value
                "user_data_add".to_string(), // new_value
                timestamp,
                message_hash.to_vec(),
            )
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

        info!(
            "Found CastAdd message for FID {} at timestamp {}",
            fid, timestamp
        );

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

        info!(
            "Found CastRemove message for FID {} at timestamp {}",
            fid, timestamp
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

        info!(
            "Found ReactionAdd message for FID {} at timestamp {}",
            fid, timestamp
        );

        // Record reaction activity
        self.database
            .record_user_activity(
                fid,
                "reaction_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "reaction_add",
                    "timestamp": timestamp
                })),
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

        info!(
            "Found ReactionRemove message for FID {} at timestamp {}",
            fid, timestamp
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
}
