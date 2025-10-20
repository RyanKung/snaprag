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

        // 🚀 Parallel shard sync: Process all configured shards simultaneously
        info!(
            "Starting parallel sync of {} shards...",
            self.config.shard_ids.len()
        );

        let mut handles = vec![];

        for &shard_id in &self.config.shard_ids {
            info!("🔄 Spawning sync task for shard {}", shard_id);

            // Clone necessary resources for each shard task
            let client = self.client.clone();
            let database = self.database.clone();
            let state_manager = self.state_manager.clone();
            let config = self.config.clone();
            let lock_manager = self.lock_manager.clone();

            // Spawn parallel task for this shard
            let handle = tokio::spawn(async move {
                // Check if we should resume from last saved progress
                let last_saved_height = database.get_last_processed_height(shard_id).await.unwrap_or(0);

                // Resume from last saved height if it's greater than requested from_block
                let resume_from = if from_block == 0 && last_saved_height > 0 {
                    info!(
                        "📍 Resuming shard {} from last saved height {} (instead of {})",
                        shard_id, last_saved_height, from_block
                    );
                    last_saved_height
                } else if last_saved_height > from_block && last_saved_height < to_block {
                    info!(
                        "📍 Progress found for shard {}: resuming from block {} (was at {})",
                        shard_id, last_saved_height, from_block
                    );
                    last_saved_height
                } else {
                    from_block
                };

                info!(
                    "📥 Starting sync for shard {} from block {} to {}",
                    shard_id, resume_from, to_block
                );

                let mut current_block = resume_from;
                let mut total_messages = 0u64;
                let mut total_blocks = 0u64;

                while current_block <= to_block {
                    let remaining = to_block.saturating_sub(current_block).saturating_add(1);
                    let batch = config.batch_size.min(remaining as u32);

                    // Create request and fetch chunks
                    let request = crate::sync::client::proto::ShardChunksRequest {
                        shard_id,
                        start_block_number: current_block,
                        stop_block_number: Some(current_block + batch as u64 - 1),
                    };

                    match client.get_shard_chunks(request).await {
                        Ok(response) => {
                            let chunks = response.shard_chunks;
                            
                            if chunks.is_empty() {
                                info!("Shard {}: no more chunks at block {}", shard_id, current_block);
                                break;
                            }

                            let processor = ShardProcessor::new(database.as_ref().clone());
                            processor.process_chunks_batch(&chunks, shard_id).await?;

                            // Update stats
                            let messages_in_batch: u64 = chunks.iter()
                                .map(|c| c.transactions.iter().map(|tx| tx.user_messages.len() as u64).sum::<u64>())
                                .sum();
                            
                            total_blocks += chunks.len() as u64;
                            total_messages += messages_in_batch;

                            // Find max block number processed
                            let max_block = chunks.iter()
                                .filter_map(|c| c.header.as_ref())
                                .filter_map(|h| h.height.as_ref())
                                .map(|height| height.block_number)
                                .max()
                                .unwrap_or(current_block);

                            current_block = max_block.saturating_add(1);

                            // Save progress to database
                            database.update_last_processed_height(shard_id, current_block).await?;

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
                            error!("Shard {} sync error at block {}: {}", shard_id, current_block, e);
                            return Err(e);
                        }
                    }
                }

                info!("✅ Shard {} completed: {} blocks, {} messages", shard_id, total_blocks, total_messages);
                Ok::<(), crate::SnapRagError>(())
            });

            handles.push((shard_id, handle));
        }

        // Wait for all shards to complete
        info!(
            "⏳ Waiting for {} shard sync tasks to complete...",
            handles.len()
        );

        for (shard_id, handle) in handles {
            match handle.await {
                Ok(Ok(())) => {
                    info!("✅ Shard {} sync completed", shard_id);
                }
                Ok(Err(e)) => {
                    error!("❌ Shard {} sync failed: {}", shard_id, e);
                    return Err(e);
                }
                Err(e) => {
                    error!("❌ Shard {} task panicked: {}", shard_id, e);
                    return Err(crate::SnapRagError::Custom(format!(
                        "Shard {} sync task panicked: {}",
                        shard_id, e
                    )));
                }
            }
        }

        info!("🎉 Parallel sync completed across {} shards", self.config.shard_ids.len());
        Ok(())
    }

    async fn start_full_realtime_sync(&self) -> Result<()> {
        info!("Real-time sync not yet implemented in refactored service");
        warn!("Use 'snaprag sync start --from-block <last_block>' for now");
        Ok(())
    }
}
