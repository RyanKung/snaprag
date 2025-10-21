use super::types::BatchedData;
// Removed: activities tracking disabled
// use super::utils::create_activity;
use crate::models::ShardBlockInfo;
use crate::sync::client::proto::MessageData;
use crate::Result;

/// Collect cast add data for batch processing
pub(super) async fn collect_cast_add(
    data: &MessageData,
    message_hash: &[u8],
    shard_block_info: &ShardBlockInfo,
    batched: &mut BatchedData,
) -> Result<()> {
    let fid = data.fid as i64;
    let timestamp = data.timestamp as i64;

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

    // Collect cast for batch insert
    batched.casts.push((
        fid,
        text.clone(),
        timestamp,
        message_hash.to_vec(),
        parent_hash,
        root_hash,
        embeds,
        mentions,
        shard_block_info.clone(),
    ));

    // ❌ Removed: activities tracking disabled (user_activity_timeline dropped)

    Ok(())
}
