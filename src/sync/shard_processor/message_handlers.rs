use super::cast_handlers::collect_cast_add;
use super::types::BatchedData;
// Removed: activities tracking disabled
// use super::utils::create_activity;
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
            // CastRemove - no action needed (soft delete handled in casts table)
        }
        3 => {
            // ReactionAdd - collect reaction data
            if let Some(body) = &data.body {
                if let Some(reaction_body) = body.get("reaction_body") {
                    let reaction_type = reaction_body
                        .get("type")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1) as i16; // 1=like, 2=recast

                    // Handle target_cast_id (reaction to a cast)
                    if let Some(target) = reaction_body.get("target_cast_id") {
                        let target_fid = target.get("fid").and_then(|v| v.as_i64());

                        if let Some(target_hash_str) = target.get("hash").and_then(|v| v.as_str()) {
                            if let Ok(target_hash) = hex::decode(target_hash_str) {
                                batched.reactions.push((
                                    fid,
                                    target_hash,
                                    target_fid,
                                    reaction_type,
                                    timestamp,
                                    message_hash.to_vec(),
                                    shard_block_info.clone(),
                                ));

                                tracing::debug!(
                                    "Collected reaction: FID {} -> cast (type: {})",
                                    fid,
                                    reaction_type
                                );
                            } else {
                                tracing::warn!("Failed to decode reaction target hash for FID {}", fid);
                            }
                        }
                    } 
                    // Handle target_url (reaction to external URL)
                    else if let Some(target_url) = reaction_body.get("target_url").and_then(|v| v.as_str()) {
                        // For URL reactions, use URL hash as target_cast_hash
                        let url_hash = format!("url_{}", target_url).as_bytes().to_vec();
                        
                        batched.reactions.push((
                            fid,
                            url_hash,
                            None, // No target_fid for URLs
                            reaction_type,
                            timestamp,
                            message_hash.to_vec(),
                            shard_block_info.clone(),
                        ));

                        tracing::debug!(
                            "Collected URL reaction: FID {} -> {} (type: {})",
                            fid,
                            target_url,
                            reaction_type
                        );
                    } else {
                        tracing::warn!("ReactionAdd for FID {} has no target_cast_id or target_url", fid);
                    }
                }
            }
        }
        4 => {
            // ReactionRemove - collect activity
        }
        5 => {
            // LinkAdd - collect link data
            tracing::trace!("Processing LinkAdd message for FID {}, body present: {}", fid, data.body.is_some());
            if let Some(body) = &data.body {
                tracing::trace!("Body keys: {:?}", body.as_object().map(|o| o.keys().collect::<Vec<_>>()));
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
        }
        6 => {
            // LinkRemove - collect activity (we don't remove from links table, just log)
        }
        7 => {
            // VerificationAddEthAddress - collect verification data
            if let Some(body) = &data.body {
                if let Some(verification_body) = body.get("verification_add_eth_address_body") {
                    if let Some(address_str) =
                        verification_body.get("address").and_then(|v| v.as_str())
                    {
                        if let Ok(address) = hex::decode(address_str) {
                            let claim_signature = verification_body
                                .get("claim_signature")
                                .and_then(|v| v.as_str())
                                .and_then(|s| hex::decode(s).ok());

                            let block_hash = verification_body
                                .get("block_hash")
                                .and_then(|v| v.as_str())
                                .and_then(|h| hex::decode(h).ok());

                            let verification_type = verification_body
                                .get("verification_type")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i16);

                            let chain_id = verification_body
                                .get("chain_id")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32);

                            batched.verifications.push((
                                fid,
                                address,
                                claim_signature,
                                block_hash,
                                verification_type,
                                chain_id,
                                timestamp,
                                message_hash.to_vec(),
                                shard_block_info.clone(),
                            ));

                            tracing::debug!("Collected verification: FID {} verified address", fid);
                        }
                    }
                }
            }

            // Also collect activity
        }
        8 => {
            // VerificationRemove - collect activity
        }
        11 => {
            // UserDataAdd - collect activity and profile updates

            // Parse and collect profile field updates
            if let Some(body) = &data.body {
                if let Some(user_data_body) = body.get("user_data_body") {
                    if let Some(data_type) = user_data_body.get("type").and_then(|v| v.as_i64()) {
                        if let Some(value) = user_data_body.get("value").and_then(|v| v.as_str()) {
                            // Map Farcaster UserDataType to field name (complete mapping)
                            // See: https://docs.farcaster.xyz/reference/contracts/reference/id-registry
                            let field_name = match data_type {
                                1 => Some("pfp_url"),
                                2 => Some("display_name"),
                                3 => Some("bio"),
                                5 => Some("website_url"),
                                6 => Some("username"),
                                7 => Some("location"),
                                8 => Some("twitter_username"),
                                9 => Some("github_username"),
                                10 => Some("banner_url"),
                                11 => Some("primary_address_ethereum"),
                                12 => Some("primary_address_solana"),
                                13 => Some("profile_token"),
                                _ => {
                                    tracing::warn!("Unknown UserDataType {} for FID {}", data_type, fid);
                                    None
                                }
                            };

                            if let Some(field) = field_name {
                                batched.profile_updates.push((
                                    fid,
                                    field.to_string(),
                                    Some(value.to_string()),
                                    timestamp,
                                    message_hash.to_vec(),
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
