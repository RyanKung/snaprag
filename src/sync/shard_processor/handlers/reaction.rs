/// Reaction message handlers (Add and Remove)

use crate::models::ShardBlockInfo;
use crate::Result;

use super::super::types::BatchedData;

/// Handle ReactionAdd message (type 3)
pub(super) fn handle_reaction_add(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) {
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

/// Handle ReactionRemove message (type 4)
pub(super) fn handle_reaction_remove(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    batched: &mut BatchedData,
) {
    if let Some(reaction_body) = body.get("reaction_body") {
        let reaction_type = reaction_body
            .get("type")
            .and_then(|v| v.as_i64())
            .map(|v| v as i16)
            .unwrap_or(1);

        // Try to get target cast hash
        if let Some(target_cast_id) = reaction_body.get("target_cast_id") {
            if let Some(target_hash_str) = target_cast_id.get("hash").and_then(|v| v.as_str()) {
                if let Ok(target_hash) = hex::decode(target_hash_str) {
                    batched.reaction_removes.push((
                        fid,
                        target_hash,
                        timestamp,
                        message_hash.to_vec(),
                    ));

                    tracing::debug!(
                        "Collected reaction remove: FID {} unliked cast (type: {})",
                        fid,
                        reaction_type
                    );
                }
            }
        }
        // Handle target_url removes
        else if let Some(target_url) = reaction_body.get("target_url").and_then(|v| v.as_str()) {
            let url_hash = format!("url_{}", target_url).as_bytes().to_vec();
            batched.reaction_removes.push((
                fid,
                url_hash,
                timestamp,
                message_hash.to_vec(),
            ));
            tracing::debug!(
                "Collected URL reaction remove: FID {} unliked URL {} (type: {})",
                fid,
                target_url,
                reaction_type
            );
        }
    }
}

