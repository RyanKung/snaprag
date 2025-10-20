use std::sync::Arc;

use tracing::debug;
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

    /// Start sync with parallel workers per shard (fail-fast strategy)
    /// If any worker fails, all workers stop and progress is saved
    pub async fn start_with_range_and_workers(
        &self,
        from_block: u64,
        to_block: u64,
        workers_per_shard: u32,
    ) -> Result<()> {
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use tokio::sync::Semaphore;

        info!(
            "Starting parallel sync: {} shards × {} workers = {} total workers (fail-fast mode)",
            self.config.shard_ids.len(),
            workers_per_shard,
            self.config.shard_ids.len() * workers_per_shard as usize
        );

        // Get Snapchain info to determine actual max heights per shard
        let shard_max_heights: std::collections::HashMap<u32, u64> = match self.client.get_info().await {
            Ok(info) => {
                info.shard_infos.iter()
                    .map(|s| (s.shard_id, s.max_height))
                    .collect()
            }
            Err(_) => std::collections::HashMap::new(),
        };

        let mut all_shard_handles = vec![];

        // For each shard, create a coordinator that manages workers
        for &shard_id in &self.config.shard_ids {
            // Determine the actual range for this shard
            let shard_to_block = if to_block == u64::MAX {
                shard_max_heights.get(&shard_id).copied().unwrap_or(to_block)
            } else {
                to_block
            };

            // Check if we should resume from last saved progress
            let last_saved_height = self.database.get_last_processed_height(shard_id).await.unwrap_or(0);
            let shard_from_block = if from_block == 0 && last_saved_height > 0 {
                info!(
                    "📍 Shard {} resuming from last saved height {}",
                    shard_id, last_saved_height
                );
                last_saved_height
            } else {
                from_block
            };

            let total_blocks = shard_to_block.saturating_sub(shard_from_block);

            info!(
                "🔄 Shard {}: spawning tasks with max {} concurrent workers ({} total blocks)",
                shard_id, workers_per_shard, total_blocks
            );

            let client = self.client.clone();
            let database = self.database.clone();
            let config = self.config.clone();

            // Spawn shard coordinator
            let handle = tokio::spawn(async move {
                // 🎯 Semaphore: Limit concurrent tasks to workers_per_shard
                let semaphore = Arc::new(Semaphore::new(workers_per_shard as usize));
                let current_block = Arc::new(AtomicU64::new(shard_from_block));
                let should_stop = Arc::new(AtomicBool::new(false));
                // Track completed batches to find minimum continuous progress
                let completed_batches = Arc::new(tokio::sync::Mutex::new(std::collections::BTreeSet::new()));
                let mut task_handles = vec![];
                
                // Spawn a background task to periodically save progress
                let progress_saver_db = database.clone();
                let progress_saver_batches = completed_batches.clone();
                let progress_saver_stop = should_stop.clone();
                let progress_saver = tokio::spawn(async move {
                    let mut last_saved_progress = shard_from_block;
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        
                        if progress_saver_stop.load(Ordering::SeqCst) {
                            break;
                        }
                        
                        // Calculate continuous progress
                        let batches = progress_saver_batches.lock().await;
                        let mut continuous_progress = shard_from_block;
                        let batch_size_u64 = config.batch_size as u64;
                        
                        while batches.contains(&continuous_progress) {
                            continuous_progress += batch_size_u64;
                        }
                        drop(batches);
                        
                        // Only update if progress changed
                        if continuous_progress > last_saved_progress {
                            if let Err(e) = progress_saver_db.update_last_processed_height(shard_id, continuous_progress).await {
                                warn!("Failed to save progress: {}", e);
                            } else {
                                debug!("💾 Shard {} progress saved: {}", shard_id, continuous_progress);
                                last_saved_progress = continuous_progress;
                            }
                        }
                    }
                });

                // Spawn tasks dynamically until we reach the end or stop signal
                loop {
                    // Check if we should stop (another worker failed)
                    if should_stop.load(Ordering::SeqCst) {
                        warn!("Shard {}: stopping task spawning due to failure in another worker", shard_id);
                        break;
                    }

                    let batch_start = current_block.fetch_add(config.batch_size as u64, Ordering::SeqCst);
                    
                    if batch_start >= shard_to_block {
                        break; // No more batches to process
                    }

                    let batch_end = (batch_start + config.batch_size as u64 - 1).min(shard_to_block);
                    
                    // Acquire semaphore permit (will wait if at max workers)
                    let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
                        crate::SnapRagError::Custom(format!("Semaphore error: {}", e))
                    })?;

                    let client = client.clone();
                    let database = database.clone();
                    let should_stop_shared = should_stop.clone();
                    let completed_batches_shared = completed_batches.clone();

                    // Spawn task for this batch with retry logic
                    let task = tokio::spawn(async move {
                        let _permit = permit; // Hold permit until task completes

                        const MAX_RETRIES: u32 = 3;
                        let mut attempt = 0;

                        loop {
                            attempt += 1;
                            
                            // Check stop signal before each attempt
                            if should_stop_shared.load(Ordering::SeqCst) {
                                return Ok::<(u64, u64), crate::SnapRagError>((0, 0));
                            }

                            let request = crate::sync::client::proto::ShardChunksRequest {
                                shard_id,
                                start_block_number: batch_start,
                                stop_block_number: Some(batch_end),
                            };

                            match client.get_shard_chunks(request).await {
                                Ok(response) => {
                                    let chunks = response.shard_chunks;

                                    if !chunks.is_empty() {
                                        match ShardProcessor::new(database.as_ref().clone())
                                            .process_chunks_batch(&chunks, shard_id)
                                            .await
                                        {
                                            Ok(_) => {
                                                let messages_in_batch: u64 = chunks
                                                    .iter()
                                                    .map(|c| {
                                                        c.transactions
                                                            .iter()
                                                            .map(|tx| tx.user_messages.len() as u64)
                                                            .sum::<u64>()
                                                    })
                                                    .sum();

                                                tracing::debug!(
                                                    "Shard {} batch {}-{}: {} blocks, {} msgs",
                                                    shard_id, batch_start, batch_end, chunks.len(), messages_in_batch
                                                );

                                                // ✅ Success - mark batch as completed (progress saver will handle DB update)
                                                completed_batches_shared.lock().await.insert(batch_start);

                                                return Ok::<(u64, u64), crate::SnapRagError>((
                                                    chunks.len() as u64,
                                                    messages_in_batch,
                                                ));
                                            }
                                            Err(e) if attempt < MAX_RETRIES => {
                                                warn!(
                                                    "Batch {}-{} processing failed (attempt {}/{}): {}, retrying...",
                                                    batch_start, batch_end, attempt, MAX_RETRIES, e
                                                );
                                                tokio::time::sleep(tokio::time::Duration::from_secs(attempt as u64)).await;
                                                continue; // Retry
                                            }
                                            Err(e) => {
                                                error!(
                                                    "🔴 Batch {}-{} failed after {} attempts: {}",
                                                    batch_start, batch_end, MAX_RETRIES, e
                                                );
                                                // Signal all workers to stop
                                                should_stop_shared.store(true, Ordering::SeqCst);
                                                return Err(e);
                                            }
                                        }
                                    } else {
                                        return Ok((0, 0)); // Empty batch
                                    }
                                }
                                Err(e) if e.to_string().contains("no more chunks") => {
                                    return Ok((0, 0)); // Skip empty batches
                                }
                                Err(e) if attempt < MAX_RETRIES => {
                                    warn!(
                                        "Batch {}-{} fetch failed (attempt {}/{}): {}, retrying...",
                                        batch_start, batch_end, attempt, MAX_RETRIES, e
                                    );
                                    tokio::time::sleep(tokio::time::Duration::from_secs(attempt as u64)).await;
                                    continue; // Retry
                                }
                                Err(e) => {
                                    error!(
                                        "🔴 Batch {}-{} fetch failed after {} attempts: {}",
                                        batch_start, batch_end, MAX_RETRIES, e
                                    );
                                    // Signal all workers to stop
                                    should_stop_shared.store(true, Ordering::SeqCst);
                                    return Err(e);
                                }
                            }
                        }
                    });

                    task_handles.push((batch_start, task));
                }

                // Wait for all tasks to complete
                let total_tasks = task_handles.len();
                info!("Shard {}: waiting for {} tasks to complete...", shard_id, total_tasks);

                let mut total_blocks = 0u64;
                let mut total_messages = 0u64;
                let mut completed_tasks = 0usize;
                let mut failed_tasks = 0usize;

                let mut task_handles_iter = task_handles.into_iter();
                while let Some((batch_start, task)) = task_handles_iter.next() {
                    match task.await {
                        Ok(Ok((blocks, messages))) => {
                            total_blocks += blocks;
                            total_messages += messages;
                            completed_tasks += 1;

                            if completed_tasks % 100 == 0 {
                                let progress_pct = (completed_tasks as f64 / total_tasks as f64 * 100.0).min(100.0);
                                info!(
                                    "Shard {}: {}/{} tasks ({:.1}%), {} blocks, {} msgs total",
                                    shard_id, completed_tasks, total_tasks, progress_pct, total_blocks, total_messages
                                );
                            }
                        }
                        Ok(Err(e)) => {
                            error!("🔴 Shard {} batch {} failed: {}", shard_id, batch_start, e);
                            
                            // Signal all workers to stop (already set by the failed worker, but set again to be safe)
                            should_stop.store(true, Ordering::SeqCst);
                            
                            // Calculate and save final continuous progress
                            let batches = completed_batches.lock().await;
                            let mut continuous_progress = shard_from_block;
                            let batch_size_u64 = config.batch_size as u64;
                            while batches.contains(&continuous_progress) {
                                continuous_progress += batch_size_u64;
                            }
                            drop(batches);
                            
                            if let Err(save_err) = database.update_last_processed_height(shard_id, continuous_progress).await {
                                error!("Failed to save progress: {}", save_err);
                            } else {
                                info!("💾 Saved final progress at block {} before stopping", continuous_progress);
                            }
                            
                            // Wait for remaining spawned tasks to finish gracefully
                            let remaining = task_handles_iter.len();
                            if remaining > 0 {
                                info!("⏳ Waiting for {} remaining tasks to finish...", remaining);
                                for (_batch_start, task) in task_handles_iter {
                                    let _ = task.await;  // Ignore individual errors, we're already failing
                                }
                                info!("✅ All tasks stopped gracefully");
                            }
                            
                            // Stop progress saver
                            let _ = progress_saver.await;
                            
                            // Fail fast - stop entire shard sync
                            return Err(e);
                        }
                        Err(e) => {
                            error!("🔴 Shard {} batch {} panicked: {}", shard_id, batch_start, e);
                            
                            // Signal all workers to stop
                            should_stop.store(true, Ordering::SeqCst);
                            
                            // Calculate and save final continuous progress
                            let batches = completed_batches.lock().await;
                            let mut continuous_progress = shard_from_block;
                            let batch_size_u64 = config.batch_size as u64;
                            while batches.contains(&continuous_progress) {
                                continuous_progress += batch_size_u64;
                            }
                            drop(batches);
                            
                            if let Err(save_err) = database.update_last_processed_height(shard_id, continuous_progress).await {
                                error!("Failed to save progress: {}", save_err);
                            } else {
                                info!("💾 Saved final progress at block {} before stopping", continuous_progress);
                            }
                            
                            // Wait for remaining spawned tasks to finish gracefully
                            let remaining = task_handles_iter.len();
                            if remaining > 0 {
                                info!("⏳ Waiting for {} remaining tasks to finish...", remaining);
                                for (_batch_start, task) in task_handles_iter {
                                    let _ = task.await;
                                }
                                info!("✅ All tasks stopped gracefully");
                            }
                            
                            // Stop progress saver
                            let _ = progress_saver.await;
                            
                            return Err(crate::SnapRagError::Custom(format!("Task panicked: {}", e)));
                        }
                    }
                }

                if failed_tasks > 0 {
                    // This shouldn't happen due to fail-fast, but just in case
                    warn!(
                        "Shard {}: {} tasks failed (unexpected - should have stopped early)",
                        shard_id, failed_tasks
                    );
                }

                info!(
                    "✅ Shard {} completed: {} blocks, {} messages",
                    shard_id, total_blocks, total_messages
                );
                
                // Stop the progress saver and wait for it
                should_stop.store(true, Ordering::SeqCst);
                let _ = progress_saver.await;

                Ok::<(), crate::SnapRagError>(())
            });

            all_shard_handles.push((shard_id, handle));
        }

        // Wait for all shards to complete
        info!("⏳ Waiting for {} shards to complete...", all_shard_handles.len());

        for (shard_id, handle) in all_shard_handles {
            match handle.await {
                Ok(Ok(())) => {
                    info!("✅ Shard {} finished successfully", shard_id);
                }
                Ok(Err(e)) => {
                    error!("❌ Shard {} failed: {}", shard_id, e);
                    return Err(e);
                }
                Err(e) => {
                    error!("❌ Shard {} panicked: {}", shard_id, e);
                    return Err(crate::SnapRagError::Custom(format!(
                        "Shard {} panicked: {}",
                        shard_id, e
                    )));
                }
            }
        }

        info!("🎉 All shards completed successfully!");
        info!("⚠️  Note: Run 'snaprag sync start' again (without --workers) to fill any gaps from failed batches");
        Ok(())
    }
}
