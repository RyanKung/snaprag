/// System message handler (`OnChain` Events)

use crate::models::ShardBlockInfo;
use crate::Result;

use super::super::types::BatchedData;

/// Process system messages (`OnChainEvents`)
pub(in crate::sync::shard_processor) async fn process_system_message(
    system_msg: &crate::sync::client::proto::ValidatorMessage,
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) -> Result<()> {
    // System messages contain OnChainEvents (id_register, storage_rent, etc.)
    if let Some(onchain_event) = &system_msg.on_chain_event {
        let fid = i64::try_from(onchain_event.fid).unwrap_or(0);
        let event_type = onchain_event.r#type;
        let chain_id = i32::try_from(onchain_event.chain_id).unwrap_or(0);
        let block_number = i32::try_from(onchain_event.block_number).unwrap_or(0);
        let block_timestamp = i64::try_from(onchain_event.block_timestamp).unwrap_or(0);
        let block_hash = if onchain_event.block_hash.is_empty() {
            None
        } else {
            Some(onchain_event.block_hash.clone())
        };
        let transaction_hash = if onchain_event.transaction_hash.is_empty() {
            None
        } else {
            Some(onchain_event.transaction_hash.clone())
        };
        let log_index = if onchain_event.log_index > 0 {
            Some(i32::try_from(onchain_event.log_index).unwrap_or(0))
        } else {
            None
        };

        // Serialize event body to JSON for storage
        let event_data = serde_json::to_value(onchain_event).unwrap_or(serde_json::Value::Null);

        batched.onchain_events.push((
            fid,
            event_type,
            chain_id,
            block_number,
            block_hash,
            block_timestamp,
            transaction_hash,
            log_index,
            event_data,
        ));

        tracing::debug!(
            "Collected onchain event: FID {} type {} (block {})",
            fid,
            event_type,
            block_number
        );
    } else {
        tracing::warn!(
            "System message in shard {} has no onchain_event",
            shard_block_info.shard_id
        );
    }
    
    Ok(())
}

