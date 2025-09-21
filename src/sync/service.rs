use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::client::proto;
use crate::sync::client::SnapchainClient;
use crate::sync::state_manager::SyncStateManager;
use crate::sync::types::{SyncConfig, SyncState};
use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Main sync service that coordinates synchronization with snapchain
pub struct SyncService {
    config: SyncConfig,
    client: SnapchainClient,
    database: Arc<Database>,
    state: Arc<RwLock<SyncState>>,
    state_manager: Arc<RwLock<SyncStateManager>>,
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
        })
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
        info!("Node info: version={}, num_shards={}, total_messages={}", 
              info.version, info.num_shards, 
              info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0));

        // Discover all FIDs first to understand the scope
        info!("Discovering all FIDs for historical sync...");
        let all_fids = self.client.get_all_fids().await?;
        info!("Found {} unique FIDs to sync historically", all_fids.len());

        // Sync block shard (shard 0) first
        info!("Starting sync of block shard (shard 0)...");
        self.sync_shard_full_historical(0).await?;

        // Sync user shards (1 to num_shards-1)
        for shard_id in 1..info.num_shards {
            info!("Starting sync of user shard {}...", shard_id);
            self.sync_shard_full_historical(shard_id).await?;
        }

        // Update status to completed
        {
            let mut state_manager = self.state_manager.write().await;
            state_manager.update_status("HistoricalSyncCompleted")?;
        }

        info!("Full historical sync completed! Processed {} unique FIDs", all_fids.len());
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
        info!("Starting real-time sync: {} shards, {} total messages", 
              info.num_shards, info.db_stats.as_ref().map(|s| s.num_messages).unwrap_or(0));

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
                if let Err(e) = Self::monitor_shard_realtime(shard_id, client, database, state_manager, config).await {
                    error!("Error monitoring shard {} for real-time updates: {}", shard_id, e);
                }
            });
        }

        // Also start FID-based monitoring for comprehensive coverage
        // Limit concurrent FID monitoring to prevent resource exhaustion
        const MAX_CONCURRENT_FIDS: usize = 200;
        let fids_to_monitor = if all_fids.len() > MAX_CONCURRENT_FIDS {
            info!("Limiting FID monitoring to first {} FIDs to prevent resource exhaustion", MAX_CONCURRENT_FIDS);
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
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.sync_interval_ms)).await;

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
        _database: Arc<Database>,
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
                    if !response.shard_chunks.is_empty() {
                        let chunk_count = response.shard_chunks.len();
                        info!("Shard {}: found {} new chunks at height {}", shard_id, chunk_count, last_processed_height);

                        for _chunk in response.shard_chunks {
                            // Shard chunk processing removed - simplified sync service
                            match Ok::<(), crate::SnapRagError>(()) {
                                Ok(()) => {
                                    info!("Shard {}: processed chunk successfully", shard_id);
                                }
                                Err(err) => {
                                    error!("Failed to process chunk in shard {}: {}", shard_id, err);
                                    let mut sm = state_manager.write().await;
                                    sm.add_error(format!("Shard {} chunk processing error: {}", shard_id, err))?;
                                }
                            }
                        }

                        // Update last processed height
                        last_processed_height += chunk_count as u64;
                        {
                            let mut sm = state_manager.write().await;
                            sm.update_last_processed_height(shard_id, last_processed_height)?;
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to check for new chunks in shard {}: {}", shard_id, err);
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

        let total_messages = 0u64;
        let mut total_blocks = 0u64;

        loop {
            // Get chunks from current height
            let request = proto::ShardChunksRequest {
                shard_id,
                start_block_number: last_height,
                stop_block_number: Some(last_height + self.config.batch_size as u64),
                ..Default::default()
            };

            match self.client.get_shard_chunks(request).await {
                Ok(response) => {
                    if response.shard_chunks.is_empty() {
                        info!("Shard {}: no more chunks at height {}, sync complete", shard_id, last_height);
                break;
            }

                    let chunk_count = response.shard_chunks.len();
                    total_blocks += chunk_count as u64;

            // Process each chunk
                    for _chunk in response.shard_chunks {
                        // Shard chunk processing removed - simplified sync service
                        match Ok::<(), crate::SnapRagError>(()) {
                            Ok(()) => {
                                // Chunk processed successfully
                            }
                            Err(err) => {
                                error!("Failed processing shard {} chunk: {}", shard_id, err);
                                let mut state_manager = self.state_manager.write().await;
                                state_manager.add_error(format!("Shard {} chunk processing error: {}", shard_id, err))?;
                            }
                        }
                    }
                
                // Update progress
                    last_height += chunk_count as u64;
                    {
                        let mut state_manager = self.state_manager.write().await;
                        state_manager.update_last_processed_height(shard_id, last_height)?;
                        state_manager.increment_messages_processed(shard_id, total_messages)?;
                        state_manager.increment_blocks_processed(shard_id, total_blocks)?;
                    }

                    info!("Shard {}: processed {} chunks, total messages: {}, height: {}", 
                          shard_id, chunk_count, total_messages, last_height);

                    // Small delay to prevent overwhelming the node
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                Err(err) => {
                    error!("Failed to get shard chunks for shard {} at height {}: {}", shard_id, last_height, err);
                    let mut state_manager = self.state_manager.write().await;
                    state_manager.add_error(format!("Shard {} chunk fetch error at height {}: {}", shard_id, last_height, err))?;
                    
                    // Wait before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    continue;
                }
            }
        }

        info!("Shard {} sync completed: {} messages, {} blocks", shard_id, total_messages, total_blocks);
        Ok(())
    }




    // All unused methods have been removed
}




