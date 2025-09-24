//! Types for snapchain synchronization

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Snapchain HTTP endpoint
    pub snapchain_http_endpoint: String,
    /// Snapchain gRPC endpoint
    pub snapchain_grpc_endpoint: String,
    /// Shard IDs to sync (0 = block shard, 1+ = user shards)
    pub shard_ids: Vec<u32>,
    /// Starting block height for sync (None = from genesis)
    pub start_block_height: Option<u64>,
    /// Batch size for processing blocks
    pub batch_size: u32,
    /// Enable real-time sync after catchup
    pub enable_realtime_sync: bool,
    /// Enable historical sync from genesis
    pub enable_historical_sync: bool,
    /// Sync interval in milliseconds
    pub sync_interval_ms: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            snapchain_http_endpoint: "http://localhost:3381".to_string(),
            snapchain_grpc_endpoint: "http://localhost:3383".to_string(),
            shard_ids: vec![0, 1, 2], // Block shard (0) + user shards (1, 2)
            start_block_height: None,
            batch_size: 100,
            enable_realtime_sync: true,
            enable_historical_sync: true,
            sync_interval_ms: 1000,
        }
    }
}

impl SyncConfig {
    /// Create SyncConfig from AppConfig
    pub fn from_app_config(app_config: &crate::AppConfig) -> Self {
        Self {
            snapchain_http_endpoint: app_config.snapchain_http_endpoint().to_string(),
            snapchain_grpc_endpoint: app_config.snapchain_grpc_endpoint().to_string(),
            shard_ids: app_config.shard_ids().clone(),
            start_block_height: None,
            batch_size: app_config.sync_batch_size(),
            enable_realtime_sync: app_config.realtime_sync_enabled(),
            enable_historical_sync: app_config.historical_sync_enabled(),
            sync_interval_ms: app_config.sync_interval_ms(),
        }
    }
}

/// Sync state for tracking progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Current sync status
    pub status: SyncStatus,
    /// Last synced block height per shard
    pub last_synced_heights: HashMap<u32, u64>,
    /// Total blocks processed per shard
    pub total_blocks_processed: HashMap<u32, u64>,
    /// Total messages processed per shard
    pub total_messages_processed: HashMap<u32, u64>,
    /// Last sync timestamp
    pub last_sync_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Sync errors encountered
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    /// Initial state
    NotStarted,
    /// Syncing historical blocks
    CatchingUp,
    /// Syncing real-time events
    Realtime,
    /// Sync paused
    Paused,
    /// Sync completed (for one-time sync)
    Completed,
    /// Sync failed
    Failed,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: SyncStatus::NotStarted,
            last_synced_heights: HashMap::new(),
            total_blocks_processed: HashMap::new(),
            total_messages_processed: HashMap::new(),
            last_sync_timestamp: None,
            errors: Vec::new(),
        }
    }
}

impl SyncState {
    /// Create a new sync state
    pub fn new() -> Self {
        Self::default()
    }
}

/// Block processing result
#[derive(Debug, Clone)]
pub struct BlockProcessResult {
    pub shard_id: u32,
    pub block_height: u64,
    pub transactions_processed: u32,
    pub messages_processed: u32,
    pub user_data_updates: u32,
    pub profile_updates: u32,
    pub processing_time_ms: u64,
}

/// Sync statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub total_blocks: u64,
    pub total_messages: u64,
    pub total_users: u64,
    pub average_blocks_per_second: f64,
    pub average_messages_per_second: f64,
}

impl Default for SyncStats {
    fn default() -> Self {
        Self {
            start_time: chrono::Utc::now(),
            end_time: None,
            total_blocks: 0,
            total_messages: 0,
            total_users: 0,
            average_blocks_per_second: 0.0,
            average_messages_per_second: 0.0,
        }
    }
}
