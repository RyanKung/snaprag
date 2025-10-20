use std::sync::Arc;

use tracing::error;
use tracing::info;
use tracing::warn;

use super::state::ChunkProcessStats;
use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::client::SnapchainClient;
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
    client: SnapchainClient,
    database: Arc<Database>,
    state: Arc<tokio::sync::RwLock<SyncState>>,
    state_manager: Arc<tokio::sync::RwLock<SyncStateManager>>,
    lock_manager: SyncLockManager,
}

impl LifecycleManager {
    pub fn new(
        config: SyncConfig,
        client: SnapchainClient,
        database: Arc<Database>,
        state: Arc<tokio::sync::RwLock<SyncState>>,
        state_manager: Arc<tokio::sync::RwLock<SyncStateManager>>,
        lock_manager: SyncLockManager,
    ) -> Self {
        Self {
            config,
            client,
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
        info!("Starting full historical data sync from genesis...");

        // Update status
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("HistoricalSync")?;
        }

        // For now, we just log that this is not yet fully implemented
        // The actual implementation requires spawning parallel tasks which needs restructuring
        warn!("Full historical sync requires manual use of 'snaprag sync start --from-block 0 --to-block <latest>'");
        Ok(())
    }

    async fn start_historical_sync_with_range(
        &self,
        from_block: u64,
        to_block: u64,
        lock: &mut SyncLockFile,
    ) -> Result<()> {
        info!(
            "Starting historical data sync from block {} to block {}...",
            from_block, to_block
        );

        // Update status
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("RangeSync")?;
        }

        // For simplicity, sync each shard sequentially
        for &shard_id in &self.config.shard_ids {
            info!(
                "📥 Syncing shard {} (blocks {}-{})",
                shard_id, from_block, to_block
            );

            let mut current_block = from_block;
            let mut total_messages = 0u64;
            let mut total_blocks = 0u64;

            while current_block <= to_block {
                let remaining = to_block.saturating_sub(current_block).saturating_add(1);
                let batch = self.config.batch_size.min(remaining as u32);

                match self
                    .poll_batch_internal(shard_id, current_block, batch)
                    .await
                {
                    Ok(stats) => {
                        total_blocks += stats.blocks_processed();
                        total_messages += stats.messages_processed();

                        let processed_block = stats.last_block_number().unwrap_or(current_block);
                        current_block = processed_block.saturating_add(1);

                        lock.update_progress(Some(shard_id), Some(current_block));
                        self.lock_manager.update_lock(lock.clone())?;

                        info!(
                            "Shard {}: processed {} blocks, {} messages (current: {})",
                            shard_id, total_blocks, total_messages, current_block
                        );

                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) if e.to_string().contains("no more chunks") => {
                        info!("Shard {}: reached end at block {}", shard_id, current_block);
                        break;
                    }
                    Err(e) => {
                        error!(
                            "Shard {} sync error at block {}: {}",
                            shard_id, current_block, e
                        );
                        return Err(e);
                    }
                }
            }

            info!(
                "✅ Shard {} completed: {} blocks, {} messages",
                shard_id, total_blocks, total_messages
            );
        }

        Ok(())
    }

    async fn start_full_realtime_sync(&self) -> Result<()> {
        info!("Real-time sync not yet implemented in refactored service");
        warn!("Use 'snaprag sync start --from-block <last_block>' for now");
        Ok(())
    }

    // Helper method for batch polling
    async fn poll_batch_internal(
        &self,
        shard_id: u32,
        from_block: u64,
        batch_size: u32,
    ) -> Result<ChunkProcessStats> {
        use crate::sync::client::proto::ShardChunksRequest;

        let request = ShardChunksRequest {
            shard_id,
            start_block_number: from_block,
            stop_block_number: Some(from_block + batch_size as u64 - 1),
        };

        let response = self.database.clone();
        let chunks = self.client.clone();

        // This should use coordinator, but for now use direct implementation
        let shard_chunks_response = chunks.get_shard_chunks(request).await?;
        let chunks_to_process = shard_chunks_response.shard_chunks;

        if chunks_to_process.is_empty() {
            return Err(crate::SnapRagError::Custom("no more chunks".to_string()));
        }

        let processor = ShardProcessor::new(response.as_ref().clone());
        let mut stats = ChunkProcessStats::default();

        processor
            .process_chunks_batch(&chunks_to_process, shard_id)
            .await?;

        for chunk in chunks_to_process {
            let block_number = Self::extract_block_number(&chunk);
            let message_count = Self::count_chunk_messages(&chunk);
            stats.record_chunk(block_number, message_count);
        }

        Ok(stats)
    }

    fn extract_block_number(chunk: &crate::sync::client::proto::ShardChunk) -> Option<u64> {
        chunk
            .header
            .as_ref()
            .and_then(|header| header.height.as_ref())
            .map(|height| height.block_number)
    }

    fn count_chunk_messages(chunk: &crate::sync::client::proto::ShardChunk) -> u64 {
        chunk
            .transactions
            .iter()
            .map(|tx| tx.user_messages.len() as u64)
            .sum()
    }
}
