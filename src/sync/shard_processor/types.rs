use std::collections::HashSet;

/// Batched data for bulk insert
pub struct BatchedData {
    // Casts: (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions, shard_block_info)
    pub casts: Vec<(
        i64,
        Option<String>,
        i64,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
        crate::models::ShardBlockInfo,
    )>,
    // Links: (fid, target_fid, link_type, timestamp, message_hash, shard_block_info)
    pub links: Vec<(
        i64,
        i64,
        String,
        i64,
        Vec<u8>,
        crate::models::ShardBlockInfo,
    )>,
    // Reactions: (fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, shard_block_info)
    pub reactions: Vec<(
        i64,
        Vec<u8>,
        Option<i64>,
        i16,
        i64,
        Vec<u8>,
        crate::models::ShardBlockInfo,
    )>,
    // Verifications: (fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, shard_block_info)
    pub verifications: Vec<(
        i64,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<i16>,
        Option<i32>,
        i64,
        Vec<u8>,
        crate::models::ShardBlockInfo,
    )>,
    // ❌ Removed: activities field (user_activity_timeline table dropped for performance)
    pub fids_to_ensure: HashSet<i64>,
    // Profile field updates: (fid, field_name, value, timestamp)
    pub profile_updates: Vec<(i64, String, Option<String>, i64)>,
}

impl BatchedData {
    pub fn new() -> Self {
        Self {
            casts: Vec::new(),
            links: Vec::new(),
            reactions: Vec::new(),
            verifications: Vec::new(),
            fids_to_ensure: HashSet::new(),
            profile_updates: Vec::new(),
        }
    }
}
