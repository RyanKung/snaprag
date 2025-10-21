/// UsernameProof message handler

use crate::models::ShardBlockInfo;
use crate::Result;

use super::super::types::BatchedData;

/// Handle UsernameProof message (type 12)
pub(super) fn handle_username_proof(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) {
    if let Some(username_proof_body) = body.get("username_proof_body") {
        let username = username_proof_body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        if let Some(owner_str) = username_proof_body.get("owner").and_then(|v| v.as_str()) {
            if let Ok(owner) = hex::decode(owner_str) {
                let signature = username_proof_body
                    .get("signature")
                    .and_then(|v| v.as_str())
                    .and_then(|s| hex::decode(s).ok())
                    .unwrap_or_default();
                
                let username_type = username_proof_body
                    .get("type")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i16)
                    .unwrap_or(1); // Default to FNAME
                
                batched.username_proofs.push((
                    fid,
                    username,
                    owner,
                    signature,
                    username_type,
                    timestamp,
                    message_hash.to_vec(),
                    shard_block_info.clone(),
                ));
                
                tracing::debug!(
                    "Collected username proof: FID {} -> @{}",
                    fid,
                    username_proof_body.get("name").and_then(|v| v.as_str()).unwrap_or("")
                );
            } else {
                tracing::warn!("Failed to decode owner address for USERNAME_PROOF FID {}", fid);
            }
        }
    }
}

