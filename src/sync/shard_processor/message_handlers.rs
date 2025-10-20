use super::cast_handlers::collect_cast_add;
use super::types::BatchedData;
use super::utils::create_activity;
use crate::models::ShardBlockInfo;
use crate::sync::client::proto::Message as FarcasterMessage;
use crate::sync::client::proto::Transaction;
use crate::Result;

/// Collect transaction data and route to appropriate handlers
pub(super) async fn collect_transaction_data(
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
            collect_message_data(message, &shard_block_info, msg_idx, batched).await?;
        }
    }

    // Process system messages (can appear in both user and system transactions)
    // System transactions (fid=0) contain batch OP chain events like id_register
    for system_msg in &transaction.system_messages {
        process_system_message(system_msg, &shard_block_info, batched).await?;
    }

    Ok(())
}

/// Collect message data and route to appropriate handlers
pub(super) async fn collect_message_data(
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
            collect_cast_add(data, &message_hash, shard_block_info, batched).await?;
        }
        2 => {
            // CastRemove - collect activity
            batched.activities.push(create_activity(
                fid,
                "cast_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "cast_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        3 => {
            // ReactionAdd - collect activity
            batched.activities.push(create_activity(
                fid,
                "reaction_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "reaction_add",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        4 => {
            // ReactionRemove - collect activity
            batched.activities.push(create_activity(
                fid,
                "reaction_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "reaction_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        5 => {
            // LinkAdd - collect link data
            if let Some(body) = &data.body {
                if let Some(link_body) = body.get("link_body") {
                    let link_type = link_body
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("follow");
                    let target_fid = link_body
                        .get("target_fid")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    if target_fid > 0 {
                        batched.links.push((
                            fid,
                            target_fid,
                            link_type.to_string(),
                            timestamp,
                            message_hash.to_vec(),
                            shard_block_info.clone(),
                        ));

                        tracing::debug!(
                            "Collected link: FID {} -> {} ({})",
                            fid,
                            target_fid,
                            link_type
                        );
                    }
                }
            }

            // Also collect activity
            batched.activities.push(create_activity(
                fid,
                "link_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "link_add",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        6 => {
            // LinkRemove - collect activity (we don't remove from links table, just log)
            batched.activities.push(create_activity(
                fid,
                "link_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "link_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        7 => {
            // VerificationAddEthAddress - collect activity
            batched.activities.push(create_activity(
                fid,
                "verification_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "verification_add",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        8 => {
            // VerificationRemove - collect activity
            batched.activities.push(create_activity(
                fid,
                "verification_remove".to_string(),
                Some(serde_json::json!({
                    "message_type": "verification_remove",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));
        }
        11 => {
            // UserDataAdd - collect activity and profile updates
            batched.activities.push(create_activity(
                fid,
                "user_data_add".to_string(),
                Some(serde_json::json!({
                    "message_type": "user_data_add",
                    "timestamp": timestamp
                })),
                timestamp,
                Some(message_hash.to_vec()),
                shard_block_info,
            ));

            // Parse and collect profile field updates
            if let Some(body) = &data.body {
                if let Some(user_data_body) = body.get("user_data_body") {
                    if let Some(data_type) = user_data_body.get("type").and_then(|v| v.as_i64()) {
                        if let Some(value) = user_data_body.get("value").and_then(|v| v.as_str()) {
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

/// Process system messages
pub(super) async fn process_system_message(
    system_msg: &crate::sync::client::proto::ValidatorMessage,
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) -> Result<()> {
    // System messages are typically id_register events
    // We'll handle them in the batch processing phase
    tracing::debug!(
        "Processing system message in shard {}",
        shard_block_info.shard_id
    );
    Ok(())
}
