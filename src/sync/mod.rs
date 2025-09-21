//! Snapchain synchronization module
//!
//! This module provides functionality to sync data from snapchain nodes,
//! including block-by-block synchronization and real-time event streaming.

pub mod client;
pub mod service;
pub mod state_manager;
pub mod types;

pub use client::SnapchainClient;
pub use service::SyncService;
pub use state_manager::{SyncStateManager, SyncStats};
pub use types::*;
