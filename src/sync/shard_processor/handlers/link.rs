/// Link message handlers (Add and Remove)

use crate::models::ShardBlockInfo;
use crate::Result;

use super::super::types::BatchedData;

/// Handle LinkAdd message (type 5)
pub(super) fn handle_link_add(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) {
    tracing::trace!("Processing LinkAdd message for FID {}, body present: {}", fid, body.is_object());
    
    if let Some(link_body) = body.get("link_body") {
        tracing::trace!("Body keys: {:?}", link_body.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        
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

/// Handle LinkRemove message (type 6)
pub(super) fn handle_link_remove(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    batched: &mut BatchedData,
) {
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
            batched.link_removes.push((
                fid,
                target_fid,
                timestamp,
                message_hash.to_vec(),
            ));

            tracing::debug!(
                "Collected link remove: FID {} unfollowed {} ({})",
                fid,
                target_fid,
                link_type
            );
        }
    }
}

