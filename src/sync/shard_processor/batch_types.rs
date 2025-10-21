use crate::models::ShardBlockInfo;

/// Cast batch item for bulk insert
#[derive(Debug, Clone)]
pub struct CastBatchItem {
    pub fid: i64,
    pub text: Option<String>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub parent_hash: Option<Vec<u8>>,
    pub root_parent_hash: Option<Vec<u8>>,
    pub embeds: Option<serde_json::Value>,
    pub mentions: Option<serde_json::Value>,
    pub shard_block_info: ShardBlockInfo,
}

/// Link batch item for bulk insert
#[derive(Debug, Clone)]
pub struct LinkBatchItem {
    pub fid: i64,
    pub target_fid: i64,
    pub link_type: String,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub shard_block_info: ShardBlockInfo,
}

/// Reaction batch item for bulk insert
#[derive(Debug, Clone)]
pub struct ReactionBatchItem {
    pub fid: i64,
    pub target_cast_hash: Vec<u8>,
    pub target_fid: Option<i64>,
    pub reaction_type: i16,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub shard_block_info: ShardBlockInfo,
}

/// Verification batch item for bulk insert
#[derive(Debug, Clone)]
pub struct VerificationBatchItem {
    pub fid: i64,
    pub address: Vec<u8>,
    pub claim_signature: Option<Vec<u8>>,
    pub block_hash: Option<Vec<u8>>,
    pub verification_type: Option<i16>,
    pub chain_id: Option<i32>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub shard_block_info: ShardBlockInfo,
}

/// Profile update batch item for bulk insert
#[derive(Debug, Clone)]
pub struct ProfileUpdateBatchItem {
    pub fid: i64,
    pub field_name: String,
    pub value: Option<String>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
}

/// Onchain event batch item for bulk insert
#[derive(Debug, Clone)]
pub struct OnchainEventBatchItem {
    pub fid: i64,
    pub event_type: i32,
    pub chain_id: i32,
    pub block_number: i32,
    pub block_hash: Option<Vec<u8>>,
    pub block_timestamp: i64,
    pub tx_hash: Option<Vec<u8>>,
    pub log_index: Option<i32>,
    pub event_data: serde_json::Value,
}

/// Username proof batch item for bulk insert
#[derive(Debug, Clone)]
pub struct UsernameProofBatchItem {
    pub fid: i64,
    pub username: String,
    pub owner: Vec<u8>,
    pub signature: Vec<u8>,
    pub username_type: i16,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub shard_block_info: ShardBlockInfo,
}

/// Frame action batch item for bulk insert
#[derive(Debug, Clone)]
pub struct FrameActionBatchItem {
    pub fid: i64,
    pub url: String,
    pub button_index: Option<i32>,
    pub cast_hash: Option<Vec<u8>>,
    pub cast_fid: Option<i64>,
    pub input_text: Option<String>,
    pub state: Option<Vec<u8>>,
    pub transaction_id: Option<Vec<u8>>,
    pub timestamp: i64,
    pub message_hash: Vec<u8>,
    pub shard_block_info: ShardBlockInfo,
}

/// Link remove batch item
#[derive(Debug, Clone)]
pub struct LinkRemoveBatchItem {
    pub fid: i64,
    pub target_fid: i64,
    pub removed_at: i64,
    pub removed_message_hash: Vec<u8>,
}

/// Reaction remove batch item
#[derive(Debug, Clone)]
pub struct ReactionRemoveBatchItem {
    pub fid: i64,
    pub target_cast_hash: Vec<u8>,
    pub removed_at: i64,
    pub removed_message_hash: Vec<u8>,
}

/// Verification remove batch item
#[derive(Debug, Clone)]
pub struct VerificationRemoveBatchItem {
    pub fid: i64,
    pub address: Vec<u8>,
    pub removed_at: i64,
    pub removed_message_hash: Vec<u8>,
}

