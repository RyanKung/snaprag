use std::collections::HashSet;

/// Batched data for bulk insert
#[derive(Default)]
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
    // Links: (fid, target_fid, link_type, timestamp, message_hash, removed_at, removed_message_hash, shard_block_info)
    // Support both LinkAdd (removed_at=None) and LinkRemove (removed_at=Some) in same INSERT
    pub links: Vec<(
        i64,                         // fid
        i64,                         // target_fid
        String,                      // link_type
        i64,                         // timestamp
        Vec<u8>,                     // message_hash
        Option<i64>,                 // removed_at (None for Add, Some for Remove)
        Option<Vec<u8>>,             // removed_message_hash
        crate::models::ShardBlockInfo, // shard_block_info
    )>,
    // Reactions: (fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, removed_at, removed_message_hash, shard_block_info)
    pub reactions: Vec<(
        i64,                         // fid
        Vec<u8>,                     // target_cast_hash
        Option<i64>,                 // target_fid
        i16,                         // reaction_type
        i64,                         // timestamp
        Vec<u8>,                     // message_hash
        Option<i64>,                 // removed_at (None for Add, Some for Remove)
        Option<Vec<u8>>,             // removed_message_hash
        crate::models::ShardBlockInfo, // shard_block_info
    )>,
    // Verifications: (fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, removed_at, removed_message_hash, shard_block_info)
    pub verifications: Vec<(
        i64,                         // fid
        Vec<u8>,                     // address
        Option<Vec<u8>>,             // claim_signature
        Option<Vec<u8>>,             // block_hash
        Option<i16>,                 // verification_type
        Option<i32>,                 // chain_id
        i64,                         // timestamp
        Vec<u8>,                     // message_hash
        Option<i64>,                 // removed_at (None for Add, Some for Remove)
        Option<Vec<u8>>,             // removed_message_hash
        crate::models::ShardBlockInfo, // shard_block_info
    )>,
    // ❌ Removed: activities field (user_activity_timeline table dropped for performance)
    pub fids_to_ensure: HashSet<i64>,
    // Profile field updates: (fid, field_name, value, timestamp, message_hash)
    pub profile_updates: Vec<(i64, String, Option<String>, i64, Vec<u8>)>,
    // Onchain events: (fid, event_type, chain_id, block_number, block_hash, block_timestamp, tx_hash, log_index, event_data)
    pub onchain_events: Vec<(i64, i32, i32, i32, Option<Vec<u8>>, i64, Option<Vec<u8>>, Option<i32>, serde_json::Value)>,
    // Remove events: (fid, identifier, removed_at, removed_message_hash)
    // For links: identifier = target_fid (as i64)
    // For reactions: identifier = target_cast_hash (as Vec<u8>)  
    // For verifications: identifier = address (as Vec<u8>)
    pub link_removes: Vec<(i64, i64, i64, Vec<u8>)>,
    pub reaction_removes: Vec<(i64, Vec<u8>, i64, Vec<u8>)>,
    pub verification_removes: Vec<(i64, Vec<u8>, i64, Vec<u8>)>,
    // Username proofs: (fid, username, owner, signature, username_type, timestamp, message_hash, shard_block_info)
    pub username_proofs: Vec<(i64, String, Vec<u8>, Vec<u8>, i16, i64, Vec<u8>, crate::models::ShardBlockInfo)>,
    // Frame actions: (fid, url, button_index, cast_hash, cast_fid, input_text, state, transaction_id, timestamp, message_hash, shard_block_info)
    pub frame_actions: Vec<(i64, String, Option<i32>, Option<Vec<u8>>, Option<i64>, Option<String>, Option<Vec<u8>>, Option<Vec<u8>>, i64, Vec<u8>, crate::models::ShardBlockInfo)>,
}

impl BatchedData {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
