use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::client::proto;
use crate::sync::client::SnapchainClient;
use crate::sync::lock_file::SyncLockFile;
use crate::sync::lock_file::SyncLockManager;
use crate::sync::lock_file::SyncRange;
use crate::sync::shard_processor::ShardProcessor;
use crate::sync::state_manager::SyncStateManager;
use crate::sync::types::SyncConfig;
use crate::sync::types::SyncState;
use crate::Result;

/// Main sync service that coordinates synchronization with snapchain
pub struct SyncService {
    config: SyncConfig,
    client: SnapchainClient,
    database: Arc<Database>,
    state: Arc<RwLock<SyncState>>,
    state_manager: Arc<RwLock<SyncStateManager>>,
    lock_manager: SyncLockManager,
}

#[derive(Debug, Default, Clone)]
pub struct ChunkProcessStats {
    blocks_processed: u64,
    messages_processed: u64,
    last_block_number: Option<u64>,
}

impl ChunkProcessStats {
    pub fn blocks_processed(&self) -> u64 {
        self.blocks_processed
    }

    pub fn messages_processed(&self) -> u64 {
        self.messages_processed
    }

    pub fn last_block_number(&self) -> Option<u64> {
        self.last_block_number
    }

    fn record_chunk(&mut self, block_number: Option<u64>, message_count: u64) {
        self.blocks_processed += 1;
        self.messages_processed += message_count;
        if let Some(block_number) = block_number {
            let updated = match self.last_block_number {
                Some(current) => current.max(block_number),
                None => block_number,
            };
            self.last_block_number = Some(updated);
        }
    }
}

fn extract_block_number(chunk: &proto::ShardChunk) -> Option<u64> {
    chunk
        .header
        .as_ref()
        .and_then(|header| header.height.as_ref())
        .map(|height| height.block_number)
}

fn count_chunk_messages(chunk: &proto::ShardChunk) -> u64 {
    chunk
        .transactions
        .iter()
        .map(|tx| tx.user_messages.len() as u64)
        .sum()
}

async fn process_shard_chunks(
    database: &Arc<Database>,
    shard_id: u32,
    chunks: Vec<proto::ShardChunk>,
) -> Result<ChunkProcessStats> {
    if chunks.is_empty() {
        return Ok(ChunkProcessStats::default());
    }

    let processor = ShardProcessor::new(database.as_ref().clone());
    let mut stats = ChunkProcessStats::default();

    // Process all chunks in batch
    processor.process_chunks_batch(&chunks, shard_id).await?;

    // Record stats
    for chunk in chunks {
        let block_number = extract_block_number(&chunk);
        let message_count = count_chunk_messages(&chunk);
        stats.record_chunk(block_number, message_count);
    }

    Ok(stats)
}

impl SyncService {
    /// Create a new sync service
    pub async fn new(app_config: &AppConfig, database: Arc<Database>) -> Result<Self> {
        let sync_config = SyncConfig::from_app_config(app_config);
        let client = SnapchainClient::from_config(app_config).await?;

        // Load or create initial sync state
        let state = Arc::new(RwLock::new(SyncState::new()));

        // Initialize state manager with persistent storage
        let mut state_manager = SyncStateManager::new("snaprag_sync_state.json");
        state_manager.load()?;
        let state_manager = Arc::new(RwLock::new(state_manager));

        Ok(Self {
            config: sync_config,
            client,
            database,
            state,
            state_manager,
            lock_manager: SyncLockManager::new(),
        })
    }

    /// Poll once for a single block (for testing)
    pub async fn poll_once(&self, shard_id: u32, block_number: u64) -> Result<ChunkProcessStats> {
        info!("Polling once for shard {} block {}", shard_id, block_number);

        let request = proto::ShardChunksRequest {
            shard_id,
            start_block_number: block_number,
            stop_block_number: Some(block_number + 1),
        };

        match self.client.get_shard_chunks(request).await {
            Ok(response) => {
                let chunk_count = response.shard_chunks.len();
                info!(
                    "Received {} chunks for shard {} block {}",
                    chunk_count, shard_id, block_number
                );

                let mut stats =
                    process_shard_chunks(&self.database, shard_id, response.shard_chunks).await?;

                if stats.blocks_processed == 0 {
                    stats.blocks_processed = 1;
                    stats.last_block_number = Some(block_number);
                }

                let processed_block = stats.last_block_number.unwrap_or(block_number);

                info!(
                    "Poll once completed for shard {} block {} (messages: {})",
                    shard_id, processed_block, stats.messages_processed
                );

                Ok(stats)
            }
            Err(err) => {
                error!(
                    "Failed to get shard chunks for shard {} at block {}: {}",
                    shard_id, block_number, err
                );
                Err(err)
            }
        }
    }

    /// Poll a batch of blocks at once
    pub async fn poll_batch(
        &self,
        shard_id: u32,
        from_block: u64,
        batch_size: u32,
    ) -> Result<ChunkProcessStats> {
        let to_block = from_block + batch_size as u64;

        info!(
            "📦 Processing blocks {} to {} (batch size: {})",
            from_block,
            to_block - 1,
            batch_size
        );

        let request = proto::ShardChunksRequest {
            shard_id,
            start_block_number: from_block,
            stop_block_number: Some(to_block),
        };

        match self.client.get_shard_chunks(request).await {
            Ok(response) => {
                let chunk_count = response.shard_chunks.len();
                info!("   ↳ Fetched {} chunks from server", chunk_count);

                let stats =
                    process_shard_chunks(&self.database, shard_id, response.shard_chunks).await?;

                info!(
                    "   ✓ Completed blocks {} to {} → {} messages, {} blocks processed",
                    from_block,
                    to_block - 1,
                    stats.messages_processed,
                    stats.blocks_processed
                );

                Ok(stats)
            }
            Err(err) => {
                error!(
                    "   ✗ Failed to fetch blocks {}-{}: {}",
                    from_block,
                    to_block - 1,
                    err
                );
                Err(err)
            }
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

    /// Start full historical data synchronization from genesis
    async fn start_full_historical_sync(&self) -> Result<()> {
        info!("Starting full historical data sync from genesis...");

        // Update status
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("HistoricalSync")?;
        }

        // Get node info to understand the data structure
        let info = self.client.get_info().await?;
        info!(
            "Node info: version={}, num_shards={}, total_messages={}",
            info.version,
            info.num_shards,
            info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0)
        );

        // Discover all FIDs first to understand the scope
        info!("Discovering all FIDs for historical sync...");
        let all_fids = self.client.get_all_fids().await?;
        info!("Found {} unique FIDs to sync historically", all_fids.len());

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

            // Spawn parallel task for this shard
            let handle = tokio::spawn(async move {
                let service = SyncService {
                    config,
                    client,
                    database,
                    state: Arc::new(RwLock::new(SyncState::default())),
                    state_manager,
                    lock_manager: SyncLockManager::new(),
                };

                info!("📥 Starting full historical sync for shard {}", shard_id);
                service.sync_shard_full_historical(shard_id).await
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
                    info!("✅ Shard {} sync completed successfully", shard_id);
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

        // Update status to completed
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("HistoricalSyncCompleted")?;
        }

        info!(
            "🎉 Parallel historical sync completed! Processed {} unique FIDs across {} shards",
            all_fids.len(),
            self.config.shard_ids.len()
        );
        Ok(())
    }

    /// Start historical data synchronization with a specific block range
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

        // Get node info to understand the data structure
        let info = self.client.get_info().await?;
        info!(
            "Node info: version={}, num_shards={}, total_messages={}",
            info.version,
            info.num_shards,
            info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0)
        );

        // 🚀 Parallel shard sync with range: Process all configured shards simultaneously
        info!(
            "Starting parallel range sync of {} shards from block {} to {}...",
            self.config.shard_ids.len(),
            from_block,
            to_block
        );

        let mut handles = vec![];

        for &shard_id in &self.config.shard_ids {
            info!(
                "🔄 Spawning range sync task for shard {} (blocks {}-{})",
                shard_id, from_block, to_block
            );

            // Clone necessary resources for each shard task
            let client = self.client.clone();
            let database = self.database.clone();
            let state_manager = self.state_manager.clone();
            let config = self.config.clone();
            let lock_manager = self.lock_manager.clone(); // 🔒 Share the same lock_manager (with mutex)

            // Spawn parallel task for this shard
            let handle = tokio::spawn(async move {
                let service = SyncService {
                    config,
                    client,
                    database,
                    state: Arc::new(RwLock::new(SyncState::default())),
                    state_manager,
                    lock_manager: lock_manager.clone(),
                };

                // Create a lock for this shard
                let mut shard_lock = SyncLockFile::new(
                    "running",
                    Some(SyncRange {
                        from_block,
                        to_block: Some(to_block),
                    }),
                );
                shard_lock.update_progress(Some(shard_id), Some(from_block));

                info!("📥 Starting range sync for shard {}", shard_id);
                service
                    .sync_shard_with_range(shard_id, from_block, to_block, &mut shard_lock)
                    .await
            });

            handles.push((shard_id, handle));
        }

        // Wait for all shards to complete
        info!(
            "⏳ Waiting for {} shard range sync tasks to complete...",
            handles.len()
        );

        for (shard_id, handle) in handles {
            match handle.await {
                Ok(Ok(())) => {
                    info!(
                        "✅ Shard {} range sync completed (blocks {}-{})",
                        shard_id, from_block, to_block
                    );
                }
                Ok(Err(e)) => {
                    error!("❌ Shard {} range sync failed: {}", shard_id, e);
                    return Err(e);
                }
                Err(e) => {
                    error!("❌ Shard {} range sync task panicked: {}", shard_id, e);
                    return Err(crate::SnapRagError::Custom(format!(
                        "Shard {} range sync task panicked: {}",
                        shard_id, e
                    )));
                }
            }
        }

        info!(
            "🎉 Parallel range sync completed! Processed blocks {} to {} across {} shards",
            from_block,
            to_block,
            self.config.shard_ids.len()
        );
        Ok(())
    }

    /// Start full real-time synchronization for all new data
    async fn start_full_realtime_sync(&self) -> Result<()> {
        info!("Starting full real-time sync for all new data...");

        // Update status
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("RealtimeSync")?;
        }

        // Get node info to understand current state
        let info = self.client.get_info().await?;
        info!(
            "Starting real-time sync: {} shards, {} total messages",
            info.num_shards,
            info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0)
        );

        // Discover all FIDs for comprehensive monitoring
        info!("Discovering all FIDs for comprehensive real-time monitoring...");
        let all_fids = self.client.get_all_fids().await?;
        info!("Found {} FIDs for real-time monitoring", all_fids.len());

        // Start monitoring all shards for new data (shard-based approach)
        for shard_id in 0..info.num_shards {
            let client = self.client.clone();
            let database = self.database.clone();
            let state_manager = self.state_manager.clone();
            let config = self.config.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::monitor_shard_realtime(shard_id, client, database, state_manager, config)
                        .await
                {
                    error!(
                        "Error monitoring shard {} for real-time updates: {}",
                        shard_id, e
                    );
                }
            });
        }

        // Also start FID-based monitoring for comprehensive coverage
        // Limit concurrent FID monitoring to prevent resource exhaustion
        const MAX_CONCURRENT_FIDS: usize = 200;
        let fids_to_monitor = if all_fids.len() > MAX_CONCURRENT_FIDS {
            info!(
                "Limiting FID monitoring to first {} FIDs to prevent resource exhaustion",
                MAX_CONCURRENT_FIDS
            );
            all_fids.into_iter().take(MAX_CONCURRENT_FIDS).collect()
        } else {
            all_fids
        };

        for _fid in fids_to_monitor {
            let _client = self.client.clone();
            let _database = self.database.clone();
            let _state_manager = self.state_manager.clone();
            let _config = self.config.clone();

            // FID monitoring removed - using shard-based monitoring instead
        }

        // Keep the service running and print status periodically
        let mut status_counter = 0;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(
                self.config.sync_interval_ms,
            ))
            .await;

            // Print status every 30 iterations (every 30 seconds with default interval)
            status_counter += 1;
            if status_counter >= 30 {
                // Status printing removed - simplified sync service
                status_counter = 0;
            }
        }
    }

    /// Monitor a specific shard for real-time updates
    async fn monitor_shard_realtime(
        shard_id: u32,
        client: SnapchainClient,
        database: Arc<Database>,
        state_manager: Arc<RwLock<SyncStateManager>>,
        config: SyncConfig,
    ) -> Result<()> {
        info!("Starting real-time monitoring for shard {}", shard_id);
        let retry_delay = tokio::time::Duration::from_millis(config.sync_interval_ms.max(100));

        let mut last_processed_height = {
            let sm = state_manager.read().await;
            sm.get_last_processed_height(shard_id)
        };

        loop {
            // Check for new chunks in this shard
            let request = proto::ShardChunksRequest {
                shard_id,
                start_block_number: last_processed_height,
                stop_block_number: Some(last_processed_height + 10), // Small batch for real-time
                ..Default::default()
            };

            match client.get_shard_chunks(request).await {
                Ok(response) => {
                    let chunks = response.shard_chunks;
                    if !chunks.is_empty() {
                        let chunk_count = chunks.len();
                        info!(
                            "Shard {}: found {} new chunks at height {}",
                            shard_id, chunk_count, last_processed_height
                        );

                        let stats = process_shard_chunks(&database, shard_id, chunks).await?;

                        if stats.blocks_processed > 0 {
                            let next_height = stats
                                .last_block_number
                                .map(|block| block.saturating_add(1))
                                .unwrap_or_else(|| {
                                    last_processed_height.saturating_add(stats.blocks_processed)
                                });

                            {
                                let mut sm = state_manager.write().await;
                                sm.increment_blocks_processed(shard_id, stats.blocks_processed)?;
                                sm.increment_messages_processed(
                                    shard_id,
                                    stats.messages_processed,
                                )?;
                                sm.update_last_processed_height(shard_id, next_height)?;
                            }

                            info!(
                                "Shard {}: processed {} blocks ({} messages), next height: {}",
                                shard_id,
                                stats.blocks_processed,
                                stats.messages_processed,
                                next_height
                            );

                            last_processed_height = next_height;
                        }
                    }
                }
                Err(err) => {
                    error!(
                        "Failed to check for new chunks in shard {}: {}",
                        shard_id, err
                    );
                    let mut sm = state_manager.write().await;
                    sm.add_error(format!("Shard {} real-time sync error: {}", shard_id, err))?;
                }
            }

            tokio::time::sleep(retry_delay).await;
        }
    }

    /// Sync full historical data for a specific shard
    async fn sync_shard_full_historical(&self, shard_id: u32) -> Result<()> {
        info!("Starting full historical sync for shard {}", shard_id);

        // Get last processed height for this shard
        let mut last_height = {
            let state_manager = self.state_manager.read().await;
            state_manager.get_last_processed_height(shard_id)
        };

        info!("Shard {}: starting from height {}", shard_id, last_height);

        let mut total_messages = 0u64;
        let mut total_blocks = 0u64;

        loop {
            // Use poll_batch for better performance
            match self
                .poll_batch(shard_id, last_height, self.config.batch_size)
                .await
            {
                Ok(stats) => {
                    let blocks_delta = stats.blocks_processed;
                    let messages_delta = stats.messages_processed;
                    let processed_block = stats.last_block_number.unwrap_or(last_height);
                    if processed_block == u64::MAX {
                        warn!(
                            "Shard {}: reached maximum block number {}, stopping historical sync",
                            shard_id, processed_block
                        );
                        break;
                    }
                    let next_height = processed_block.saturating_add(1);

                    total_blocks += blocks_delta;
                    total_messages += messages_delta;

                    {
                        let mut state_manager = self.state_manager.write().await;
                        state_manager.update_last_processed_height(shard_id, next_height)?;
                        state_manager.increment_messages_processed(shard_id, messages_delta)?;
                        state_manager.increment_blocks_processed(shard_id, blocks_delta)?;
                    }

                    last_height = next_height;

                    info!(
                        "Shard {}: processed {} blocks ({} messages), totals -> blocks: {}, messages: {}, next height: {}",
                        shard_id,
                        blocks_delta,
                        messages_delta,
                        total_blocks,
                        total_messages,
                        next_height
                    );

                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(err) => {
                    // Check if it's a "no more data" error or a real error
                    if err.to_string().contains("no more chunks")
                        || err.to_string().contains("empty")
                    {
                        info!(
                            "Shard {}: no more chunks at height {}, sync complete",
                            shard_id, last_height
                        );
                        break;
                    }

                    // Check for critical database errors that require immediate stop
                    let err_str = err.to_string();
                    if err_str.contains("does not exist")
                        || err_str.contains("schema")
                        || err_str.contains("column")
                        || err_str.contains("relation")
                    {
                        error!(
                            "🔥 CRITICAL DATABASE ERROR at shard {} block {}: {}",
                            shard_id, last_height, err
                        );
                        panic!(
                            "Fatal database schema error: {}. Please fix the database schema and restart.",
                            err
                        );
                    }

                    error!(
                        "Failed to process shard {} block {}: {}",
                        shard_id, last_height, err
                    );
                    let mut state_manager = self.state_manager.write().await;
                    state_manager.add_error(format!(
                        "Shard {} block processing error at height {}: {}",
                        shard_id, last_height, err
                    ))?;

                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                }
            }
        }

        info!(
            "Shard {} sync completed: {} messages, {} blocks",
            shard_id, total_messages, total_blocks
        );
        Ok(())
    }

    /// Sync a specific shard with a block range
    async fn sync_shard_with_range(
        &self,
        shard_id: u32,
        from_block: u64,
        to_block: u64,
        lock: &mut SyncLockFile,
    ) -> Result<()> {
        // Check if we should resume from last saved progress
        let last_saved_height = self.database.get_last_processed_height(shard_id).await?;

        // Resume from last saved height if it's greater than requested from_block
        // This allows automatic resume when sync is restarted
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
            "Starting range sync for shard {} from block {} to block {}",
            shard_id, resume_from, to_block
        );

        let mut current_block = resume_from;
        let mut total_messages = 0u64;
        let mut total_blocks = 0u64;

        while current_block <= to_block {
            // Use poll_batch to fetch multiple blocks at once
            let remaining = to_block.saturating_sub(current_block).saturating_add(1);
            let batch_size = std::cmp::min(self.config.batch_size as u64, remaining) as u32;

            match self.poll_batch(shard_id, current_block, batch_size).await {
                Ok(stats) => {
                    let blocks_delta = stats.blocks_processed;
                    let messages_delta = stats.messages_processed;
                    let processed_block = stats.last_block_number.unwrap_or(current_block);

                    if processed_block == u64::MAX {
                        warn!(
                            "Shard {}: reached maximum block number {}, stopping range sync",
                            shard_id, processed_block
                        );
                        break;
                    }

                    let next_block = processed_block.saturating_add(1);

                    total_blocks += blocks_delta;
                    total_messages += messages_delta;

                    {
                        let mut state_manager = self.state_manager.write().await;
                        state_manager.update_last_processed_height(shard_id, next_block)?;
                        state_manager.increment_messages_processed(shard_id, messages_delta)?;
                        state_manager.increment_blocks_processed(shard_id, blocks_delta)?;
                    }

                    lock.update_progress(Some(shard_id), Some(next_block));
                    lock.increment_processed(blocks_delta, messages_delta);
                    self.lock_manager.update_lock(lock.clone())?;

                    info!(
                        "📊 Progress: {} blocks ({} messages) | Total: {} blocks, {} messages | Next: {}",
                        blocks_delta,
                        messages_delta,
                        total_blocks,
                        total_messages,
                        next_block
                    );

                    current_block = next_block;

                    // Small delay to prevent overwhelming the node
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(err) => {
                    // Check for critical database errors that require immediate stop
                    let err_str = err.to_string();
                    if err_str.contains("does not exist")
                        || err_str.contains("schema")
                        || err_str.contains("column")
                        || err_str.contains("relation")
                    {
                        error!(
                            "🔥 CRITICAL DATABASE ERROR at shard {} block {}: {}",
                            shard_id, current_block, err
                        );
                        panic!(
                            "Fatal database schema error: {}. Please fix the database schema and restart.",
                            err
                        );
                    }

                    error!(
                        "Failed to process shard {} block {}: {}",
                        shard_id, current_block, err
                    );
                    let mut state_manager = self.state_manager.write().await;
                    state_manager.add_error(format!(
                        "Shard {} block processing error at block {}: {}",
                        shard_id, current_block, err
                    ))?;

                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                }
            }
        }

        info!(
            "Shard {} range sync completed: {} messages, {} blocks (blocks {} to {})",
            shard_id, total_messages, total_blocks, from_block, to_block
        );
        Ok(())
    }

    /// Stop the sync service and save current state
    pub async fn stop(&self, force: bool) -> Result<()> {
        info!("Stopping SnapRAG sync service...");

        if self.lock_manager.lock_exists() {
            match self.lock_manager.read_lock() {
                Ok(mut lock) => {
                    let pid = lock.pid;

                    if force {
                        info!("Force stopping sync process (PID: {})", pid);
                        lock.update_status("force_stopped");
                    } else {
                        info!("Gracefully stopping sync process (PID: {})", pid);
                        lock.update_status("stopped");
                    }

                    // Save final state to lock file
                    self.lock_manager.update_lock(lock)?;

                    // Save state to persistent storage
                    {
                        let mut state_manager = self.state_manager.write().await;
                        state_manager.save()?;
                    }

                    // 🚀 CRITICAL FIX: Actually kill the process
                    let current_pid = std::process::id();
                    info!("Current PID: {}, Target PID: {}", current_pid, pid);

                    if pid != current_pid {
                        // Killing a different process
                        let signal = if force { 9 } else { 15 }; // SIGKILL or SIGTERM
                        info!("⚡ Sending signal {} to process {}", signal, pid);

                        #[cfg(unix)]
                        {
                            // Use libc to send signal
                            let result = unsafe { libc::kill(pid as libc::pid_t, signal) };

                            info!("Kill result: {}", result);

                            if result == 0 {
                                info!("✅ Signal sent successfully to process {}", pid);
                                // Wait for process to terminate
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                                // Verify process is gone
                                let check = unsafe { libc::kill(pid as libc::pid_t, 0) };
                                if check == 0 {
                                    warn!("⚠️  Process {} still running after signal, sending SIGKILL", pid);
                                    unsafe {
                                        libc::kill(pid as libc::pid_t, 9);
                                    }
                                    tokio::time::sleep(tokio::time::Duration::from_millis(500))
                                        .await;
                                } else {
                                    info!("✅ Process {} terminated successfully", pid);
                                }
                            } else {
                                let errno = std::io::Error::last_os_error();
                                warn!("❌ Failed to send signal to process {} (result: {}, errno: {})", pid, result, errno);
                            }
                        }

                        #[cfg(not(unix))]
                        {
                            warn!("Process termination not supported on this platform");
                        }
                    } else {
                        info!("Cannot stop self - exiting after cleanup");
                    }

                    // Remove lock file
                    self.lock_manager.remove_lock()?;

                    info!("Sync service stopped successfully");
                }
                Err(e) => {
                    warn!("Failed to read lock file during stop: {}", e);
                    self.lock_manager.remove_lock()?;
                }
            }
        } else {
            info!("No active sync process found");
        }

        Ok(())
    }

    /// Get sync status from lock file
    pub fn get_sync_status(&self) -> Result<Option<SyncLockFile>> {
        if self.lock_manager.lock_exists() {
            Ok(Some(self.lock_manager.read_lock()?))
        } else {
            Ok(None)
        }
    }

    // All unused methods have been removed
}
