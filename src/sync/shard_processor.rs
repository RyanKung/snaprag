use std::collections::HashMap;

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
use crate::Result;

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

        info!(
            "Found UserDataAdd message for FID {} at timestamp {}",
            fid, timestamp
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
