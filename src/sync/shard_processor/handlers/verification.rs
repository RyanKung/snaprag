use super::super::types::BatchedData;
/// Verification message handlers (Add and Remove, ETH and Solana)
use crate::models::ShardBlockInfo;
use crate::Result;

/// Handle `VerificationAdd` message (type 7) - supports both ETH and Solana
pub(super) fn handle_verification_add(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) {
    // Try ETH address verification
    if let Some(verification_body) = body.get("verification_add_eth_address_body") {
        if let Some(address_str) = verification_body.get("address").and_then(|v| v.as_str()) {
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
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|v| i16::try_from(v).ok());

                let chain_id = verification_body
                    .get("chain_id")
                    .and_then(serde_json::Value::as_i64)
                    .and_then(|v| i32::try_from(v).ok());

                batched.verifications.push((
                    fid,
                    address,
                    claim_signature,
                    block_hash,
                    verification_type,
                    chain_id,
                    timestamp,
                    message_hash.to_vec(),
                    None, // removed_at (None for Add)
                    None, // removed_message_hash
                    shard_block_info.clone(),
                ));

                tracing::debug!("Collected ETH verification: FID {} verified address", fid);
            } else {
                tracing::warn!("Failed to decode ETH address for FID {}", fid);
            }
        }
    }
    // Try Solana address verification
    else if let Some(verification_body) = body.get("verification_add_solana_address_body") {
        if let Some(address_str) = verification_body.get("address").and_then(|v| v.as_str()) {
            // Solana addresses are base58 encoded, store as-is
            let address = address_str.as_bytes().to_vec();

            let claim_signature = verification_body
                .get("claim_signature")
                .and_then(|v| v.as_str())
                .and_then(|s| hex::decode(s).ok());

            let block_hash = verification_body
                .get("block_hash")
                .and_then(|v| v.as_str())
                .and_then(|h| hex::decode(h).ok());

            batched.verifications.push((
                fid,
                address,
                claim_signature,
                block_hash,
                Some(2),   // verification_type=2 for Solana
                Some(900), // chain_id=900 for Solana (standard)
                timestamp,
                message_hash.to_vec(),
                None, // removed_at (None for Add)
                None, // removed_message_hash
                shard_block_info.clone(),
            ));

            tracing::debug!(
                "Collected Solana verification: FID {} verified address {}",
                fid,
                address_str
            );
        } else {
            tracing::warn!("Solana verification missing address for FID {}", fid);
        }
    } else {
        tracing::warn!(
            "VerificationAdd for FID {} has unknown verification body type",
            fid
        );
    }
}

/// Handle `VerificationRemove` message (type 8)
/// Pure INSERT mode: Creates a new record with removed_at set
pub(super) fn handle_verification_remove(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) {
    if let Some(verification_body) = body.get("verification_remove_body") {
        if let Some(address_str) = verification_body.get("address").and_then(|v| v.as_str()) {
            // Try hex decode first (ETH address), fallback to bytes (Solana)
            let address =
                hex::decode(address_str).unwrap_or_else(|_| address_str.as_bytes().to_vec());

            // Pure INSERT mode: Insert new record with removed_at set
            batched.verifications.push((
                fid,
                address,
                None, // claim_signature (not provided in remove)
                None, // block_hash
                None, // verification_type (unknown for remove)
                None, // chain_id
                timestamp,
                message_hash.to_vec(),
                Some(timestamp), // removed_at - marks this as a remove event
                Some(message_hash.to_vec()), // removed_message_hash
                shard_block_info.clone(),
            ));

            tracing::debug!("Collected verification remove: FID {} removed address", fid);
        } else {
            tracing::warn!("VerificationRemove for FID {} missing address", fid);
        }
    } else {
        tracing::warn!(
            "VerificationRemove for FID {} has no verification_remove_body",
            fid
        );
    }
}
