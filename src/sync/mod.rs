//! Snapchain synchronization module
//!
//! This module provides functionality to sync data from snapchain nodes,
//! including block-by-block synchronization and real-time event streaming.

pub mod client;
pub mod lock_file;
pub mod service;
pub mod shard_processor;
pub mod state_manager;
pub mod types;

pub use client::SnapchainClient;
pub use lock_file::{SyncLockFile, SyncLockManager, SyncProgress, SyncRange};
pub use service::SyncService;
pub use shard_processor::ShardProcessor;
pub use state_manager::{SyncStateManager, SyncStats};
pub use types::*;
