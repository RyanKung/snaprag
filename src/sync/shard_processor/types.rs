use std::collections::HashSet;

/// Batched data for bulk insert
pub struct BatchedData {
    pub casts: Vec<(
        i64,
        Option<String>,
        i64,
        Vec<u8>,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<serde_json::Value>,
        Option<serde_json::Value>,
    )>,
    // Activities: (fid, activity_type, activity_data, timestamp, message_hash, shard_id, block_height)
    pub activities: Vec<(
        i64,
        String,
        Option<serde_json::Value>,
        i64,
        Option<Vec<u8>>,
        Option<i32>,
        Option<i64>,
    )>,
    pub fids_to_ensure: HashSet<i64>,
    // Profile field updates: (fid, field_name, value, timestamp)
    pub profile_updates: Vec<(i64, String, Option<String>, i64)>,
}

impl BatchedData {
    pub fn new() -> Self {
        Self {
            casts: Vec::new(),
            activities: Vec::new(),
            fids_to_ensure: HashSet::new(),
            profile_updates: Vec::new(),
        }
    }
}
