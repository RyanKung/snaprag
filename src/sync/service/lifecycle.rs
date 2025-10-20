use std::sync::Arc;

use tracing::info;
use tracing::warn;

use super::state::ChunkProcessStats;
use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::lock_file::SyncLockFile;
use crate::sync::lock_file::SyncLockManager;
use crate::sync::lock_file::SyncRange;
use crate::sync::shard_processor::ShardProcessor;
use crate::sync::state_manager::SyncStateManager;
use crate::sync::types::SyncConfig;
use crate::sync::types::SyncState;
use crate::Result;

/// Lifecycle management for sync service
pub struct LifecycleManager {
    config: SyncConfig,
    database: Arc<Database>,
    state: Arc<tokio::sync::RwLock<SyncState>>,
    state_manager: Arc<tokio::sync::RwLock<SyncStateManager>>,
    lock_manager: SyncLockManager,
}

impl LifecycleManager {
    pub fn new(
        config: SyncConfig,
        database: Arc<Database>,
        state: Arc<tokio::sync::RwLock<SyncState>>,
        state_manager: Arc<tokio::sync::RwLock<SyncStateManager>>,
        lock_manager: SyncLockManager,
    ) -> Self {
        Self {
            config,
            database,
            state,
            state_manager,
            lock_manager,
        }
    }

    /// Start the sync service
    pub async fn start(&self) -> Result<()> {
        info!("Starting SnapRAG sync service...");
        info!("Sync configuration: {:?}", self.config);

        // Always start with historical sync for full data import
        if self.config.enable_historical_sync {
            info!("Starting full historical data sync from genesis...");
            self.start_full_historical_sync().await?;
        }

        // Then start real-time sync for new data
        if self.config.enable_realtime_sync {
            info!("Starting real-time sync for new data...");
            self.start_full_realtime_sync().await?;
        }

        Ok(())
    }

    /// Start the sync service with a specific block range
    pub async fn start_with_range(&self, from_block: u64, to_block: u64) -> Result<()> {
        // Validate range parameters
        if from_block > to_block {
            return Err(crate::SnapRagError::Custom(format!(
                "Invalid range: from_block ({}) cannot be greater than to_block ({})",
                from_block, to_block
            )));
        }

        info!(
            "Starting SnapRAG sync service with range {} to {}...",
            from_block, to_block
        );
        info!("Sync configuration: {:?}", self.config);

        // Create lock file for this sync process
        let sync_range = SyncRange {
            from_block,
            to_block: if to_block == u64::MAX {
                None
            } else {
                Some(to_block)
            },
        };
        let mut lock = self.lock_manager.create_lock("running", Some(sync_range))?;

        // Start historical sync with the specified range
        if self.config.enable_historical_sync {
            info!(
                "Starting historical data sync from block {} to block {}...",
                from_block, to_block
            );
            self.start_historical_sync_with_range(from_block, to_block, &mut lock)
                .await?;
        }

        // Update lock file to completed status
        lock.update_status("completed");
        self.lock_manager.update_lock(lock)?;

        // Note: We don't start real-time sync when using a range, as it's typically for testing
        info!("Range sync completed. Use 'sync start' without range for continuous sync.");

        Ok(())
    }

    /// Stop the sync service
    pub async fn stop(&self, force: bool) -> Result<()> {
        info!("Stopping SnapRAG sync service...");

        // Update state to stopping
        {
            let mut state = self.state.write().await;
            state.status = crate::sync::types::SyncStatus::Paused;
        }

        // Update lock file to stopped status
        if let Ok(lock) = self.lock_manager.read_lock() {
            let mut lock = lock;
            if force {
                lock.update_status("force_stopped");
            } else {
                lock.update_status("stopped");
            }
            let _ = self.lock_manager.update_lock(lock);
        }

        info!("Sync service stopped");
        Ok(())
    }

    /// Get sync status
    pub fn get_sync_status(&self) -> Result<Option<SyncLockFile>> {
        match self.lock_manager.read_lock() {
            Ok(lock) => Ok(Some(lock)),
            Err(_) => Ok(None),
        }
    }

    // Private methods for historical sync
    async fn start_full_historical_sync(&self) -> Result<()> {
        // Implementation will be moved from original service.rs
        todo!("Move implementation from original service.rs")
    }

    async fn start_historical_sync_with_range(
        &self,
        from_block: u64,
        to_block: u64,
        lock: &mut SyncLockFile,
    ) -> Result<()> {
        // Implementation will be moved from original service.rs
        todo!("Move implementation from original service.rs")
    }

    async fn start_full_realtime_sync(&self) -> Result<()> {
        // Implementation will be moved from original service.rs
        todo!("Move implementation from original service.rs")
    }
}
