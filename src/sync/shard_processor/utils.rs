use std::collections::HashSet;

use super::types::BatchedData;
use crate::models::ShardBlockInfo;
use crate::Result;

/// Create activity data for batch insertion
pub(super) fn create_activity(
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
        Some(shard_block_info.shard_id.min(i32::MAX as u32) as i32),
        Some(shard_block_info.block_height.min(i64::MAX as u64) as i64),
    )
}

/// Batch verify FIDs are registered
pub(super) async fn batch_verify_fids(
    database: &crate::database::Database,
    registered_fids: &std::sync::Mutex<std::collections::HashSet<i64>>,
    fids: &HashSet<i64>,
) -> Result<()> {
    if fids.is_empty() {
        return Ok(());
    }

    let fid_vec: Vec<i64> = fids.iter().copied().collect();

    // 🚀 Single query to check all FIDs at once
    let verified_fids = sqlx::query_scalar::<_, i64>(
        r"
        SELECT DISTINCT fid 
        FROM user_activity_timeline 
        WHERE fid = ANY($1) AND activity_type = 'id_register'
        ",
    )
    .bind(&fid_vec)
    .fetch_all(database.pool())
    .await?;

    // Update cache with verified FIDs
    {
        if let Ok(mut cache) = registered_fids.lock() {
            for fid in verified_fids {
                cache.insert(fid);
            }
        }
    }

    Ok(())
}
