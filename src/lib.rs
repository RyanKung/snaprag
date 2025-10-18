//! SnapRAG - A Farcaster data synchronization and RAG (Retrieval-Augmented Generation) library
//!
//! SnapRAG provides comprehensive tools for:
//! - Synchronizing Farcaster data from snapchain
//! - Storing and querying user profiles, casts, and relationships
//! - Vector embeddings and semantic search capabilities
//! - RAG functionality for natural language queries
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use snaprag::{SnapRag, AppConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration
//!     let config = AppConfig::load()?;
//!     
//!     // Create SnapRAG instance
//!     let snaprag = SnapRag::new(&config).await?;
//!     
//!     // Initialize database schema
//!     snaprag.init_database().await?;
//!     
//!     // Start synchronization
//!     snaprag.start_sync().await?;
//!     
//!     // Query data
//!     let profiles = snaprag.search_profiles("developer").await?;
//!     println!("Found {} profiles", profiles.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod api;
pub mod cli;
pub mod config;
pub mod database;
pub mod embeddings;
pub mod errors;
pub mod generated;
pub mod grpc_client;
pub mod llm;
pub mod logging;
pub mod models;
pub mod rag;
pub mod sync;

/// Farcaster epoch constant (January 1, 2021 UTC in milliseconds)
pub const FARCASTER_EPOCH: u64 = 1609459200000;

/// Convert Farcaster timestamp (seconds since Farcaster epoch) to Unix timestamp (seconds since Unix epoch)
pub fn farcaster_to_unix_timestamp(farcaster_timestamp: u64) -> u64 {
    farcaster_timestamp + (FARCASTER_EPOCH / 1000)
}

/// Convert Unix timestamp (seconds since Unix epoch) to Farcaster timestamp (seconds since Farcaster epoch)
pub fn unix_to_farcaster_timestamp(unix_timestamp: u64) -> u64 {
    unix_timestamp - (FARCASTER_EPOCH / 1000)
}

#[cfg(test)]
pub mod tests;

// Re-export commonly used types
use std::sync::Arc;

pub use config::AppConfig;
pub use database::Database;
// Re-export embedding stats types
pub use embeddings::backfill::BackfillStats as ProfileBackfillStats;
// Re-export embeddings functionality
pub use embeddings::{
    backfill_cast_embeddings,
    backfill_embeddings as backfill_profile_embeddings,
    CastBackfillStats,
    EmbeddingService,
};
pub use errors::*;
// Re-export LLM functionality
pub use llm::{
    ChatMessage,
    LlmService,
    StreamingResponse,
};
pub use models::*;
// Re-export RAG functionality
pub use rag::{
    CastContextAssembler,
    CastRetriever,
    ContextAssembler,
    RagQuery,
    RagResponse,
    RagService,
    RetrievalMethod,
    Retriever,
    SearchResult,
};
pub use sync::lazy_loader::LazyLoader;
pub use sync::service::SyncService;
use tracing::info;

/// Main SnapRAG client for high-level operations
pub struct SnapRag {
    config: AppConfig,
    database: Arc<Database>,
    sync_service: Option<Arc<SyncService>>,
    lazy_loader: Option<Arc<LazyLoader>>,
}

impl SnapRag {
    /// Create a new SnapRAG instance
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let database = Arc::new(Database::from_config(config).await?);
        Ok(Self {
            config: config.clone(),
            database,
            sync_service: None,
            lazy_loader: None,
        })
    }

    /// Create SnapRAG instance with lazy loading enabled
    pub async fn new_with_lazy_loading(config: &AppConfig) -> Result<Self> {
        let database = Arc::new(Database::from_config(config).await?);
        let snapchain_client = Arc::new(sync::SnapchainClient::from_config(config).await?);
        let lazy_loader = Some(Arc::new(LazyLoader::new(
            database.clone(),
            snapchain_client,
        )));

        Ok(Self {
            config: config.clone(),
            database,
            sync_service: None,
            lazy_loader,
        })
    }

    /// Initialize the database schema
    pub async fn init_database(&self) -> Result<()> {
        self.database.init_schema().await?;
        info!("Database schema initialized");
        Ok(())
    }

    /// Get the database instance for direct access
    pub fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Get reference to lazy loader
    pub fn lazy_loader(&self) -> Option<&Arc<LazyLoader>> {
        self.lazy_loader.as_ref()
    }

    /// Get user profile with automatic lazy loading
    pub async fn get_user_profile_smart(&self, fid: i64) -> Result<Option<UserProfile>> {
        // Try database first
        if let Some(profile) = self.database.get_user_profile(fid).await? {
            return Ok(Some(profile));
        }

        // If lazy loader is available, try lazy loading
        if let Some(loader) = &self.lazy_loader {
            match loader.fetch_user_profile(fid as u64).await {
                Ok(profile) => Ok(Some(profile)),
                Err(e) => {
                    tracing::warn!("Failed to lazy load profile {}: {}", fid, e);
                    Ok(None) // Graceful degradation
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Get user casts with automatic lazy loading
    pub async fn get_user_casts_smart(&self, fid: i64, limit: Option<i64>) -> Result<Vec<Cast>> {
        // Try database first
        let existing_casts = self.database.get_casts_by_fid(fid, limit, Some(0)).await?;

        if !existing_casts.is_empty() {
            return Ok(existing_casts);
        }

        // If lazy loader is available, try lazy loading
        if let Some(loader) = &self.lazy_loader {
            match loader.fetch_user_casts(fid as u64).await {
                Ok(casts) => {
                    // Return limited results if requested
                    if let Some(lim) = limit {
                        Ok(casts.into_iter().take(lim as usize).collect())
                    } else {
                        Ok(casts)
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to lazy load casts for {}: {}", fid, e);
                    Ok(Vec::new())
                }
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Override sync configuration from command-line arguments
    pub fn override_sync_config(
        &mut self,
        shard_ids: Vec<u32>,
        batch_size: Option<u32>,
        interval_ms: Option<u64>,
    ) -> Result<()> {
        if !shard_ids.is_empty() {
            self.config.sync.shard_ids = shard_ids;
        }
        if let Some(batch) = batch_size {
            self.config.sync.batch_size = batch;
        }
        if let Some(interval) = interval_ms {
            self.config.sync.sync_interval_ms = interval;
        }
        Ok(())
    }

    /// Start data synchronization
    pub async fn start_sync(&mut self) -> Result<()> {
        let sync_service = Arc::new(SyncService::new(&self.config, self.database.clone()).await?);
        sync_service.start().await?;
        self.sync_service = Some(sync_service);
        Ok(())
    }

    /// Start synchronization with a specific block range
    pub async fn start_sync_with_range(&mut self, from_block: u64, to_block: u64) -> Result<()> {
        let sync_service = Arc::new(SyncService::new(&self.config, self.database.clone()).await?);
        sync_service.start_with_range(from_block, to_block).await?;
        self.sync_service = Some(sync_service);
        Ok(())
    }

    /// Stop synchronization
    pub async fn stop_sync(&self, force: bool) -> Result<()> {
        use crate::sync::lock_file::SyncLockManager;

        // Always use lock file approach since stop command runs in a different process
        let lock_manager = SyncLockManager::new();

        if lock_manager.lock_exists() {
            match lock_manager.read_lock() {
                Ok(lock) => {
                    let pid = lock.pid;
                    tracing::info!("Found running sync process with PID: {}", pid);

                    // Send signal to kill the process
                    #[cfg(unix)]
                    {
                        let signal = if force { 9 } else { 15 }; // SIGKILL or SIGTERM
                        tracing::info!("Sending signal {} to process {}", signal, pid);

                        let result = unsafe { libc::kill(pid as libc::pid_t, signal) };

                        if result == 0 {
                            tracing::info!("✅ Signal sent successfully");
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                            // Verify process is gone
                            let check = unsafe { libc::kill(pid as libc::pid_t, 0) };
                            if check == 0 {
                                tracing::warn!("Process still running, sending SIGKILL");
                                unsafe {
                                    libc::kill(pid as libc::pid_t, 9);
                                }
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            }
                        } else {
                            let errno = std::io::Error::last_os_error();
                            tracing::warn!("Failed to send signal: {}", errno);
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        tracing::warn!("Process termination not supported on this platform");
                    }

                    // Remove lock file
                    lock_manager.remove_lock()?;
                }
                Err(e) => {
                    tracing::warn!("Failed to read lock file: {}", e);
                    lock_manager.remove_lock()?;
                }
            }
        } else {
            tracing::info!("No sync process found");
        }

        Ok(())
    }

    /// Get sync status
    pub fn get_sync_status(&self) -> Result<Option<crate::sync::lock_file::SyncLockFile>> {
        // Always try to read from lock file first, regardless of whether this instance has a sync_service
        // This allows status commands to see sync processes started by other instances
        use crate::sync::lock_file::SyncLockManager;
        let lock_manager = SyncLockManager::new();

        if lock_manager.lock_exists() {
            match lock_manager.read_lock() {
                Ok(lock) => Ok(Some(lock)),
                Err(_) => {
                    // Fallback to sync_service if lock file read failed
                    if let Some(sync_service) = &self.sync_service {
                        sync_service.get_sync_status()
                    } else {
                        Ok(None)
                    }
                }
            }
        } else if let Some(sync_service) = &self.sync_service {
            sync_service.get_sync_status()
        } else {
            Ok(None)
        }
    }

    /// Search user profiles
    pub async fn search_profiles(&self, query: &str) -> Result<Vec<models::UserProfile>> {
        let search_query = models::UserProfileQuery {
            fid: None,
            username: None,
            display_name: None,
            bio: None,
            location: None,
            twitter_username: None,
            github_username: None,
            limit: Some(20),
            offset: None,
            start_timestamp: None,
            end_timestamp: None,
            sort_by: Some(models::ProfileSortBy::LastUpdated),
            sort_order: Some(models::SortOrder::Desc),
            search_term: Some(query.to_string()),
        };
        self.database.list_user_profiles(search_query).await
    }

    /// Get user profile by FID
    pub async fn get_profile(&self, fid: i64) -> Result<Option<models::UserProfile>> {
        let query = models::UserProfileQuery {
            fid: Some(fid),
            username: None,
            display_name: None,
            bio: None,
            location: None,
            twitter_username: None,
            github_username: None,
            limit: Some(1),
            offset: None,
            start_timestamp: None,
            end_timestamp: None,
            sort_by: None,
            sort_order: None,
            search_term: None,
        };
        let profiles = self.database.list_user_profiles(query).await?;
        Ok(profiles.into_iter().next())
    }

    /// Get statistics
    pub async fn get_statistics(&self) -> Result<models::StatisticsResult> {
        let stats_query = models::StatisticsQuery {
            start_date: None,
            end_date: None,
            group_by: None,
        };
        self.database.get_statistics(stats_query).await
    }

    /// List casts
    pub async fn list_casts(&self, limit: Option<i64>) -> Result<Vec<models::Cast>> {
        let cast_query = models::CastQuery {
            fid: None,
            text_search: None,
            parent_hash: None,
            root_hash: None,
            has_mentions: None,
            has_embeds: None,
            start_timestamp: None,
            end_timestamp: None,
            limit,
            offset: None,
            sort_by: Some(models::CastSortBy::Timestamp),
            sort_order: Some(models::SortOrder::Desc),
        };
        self.database.list_casts(cast_query).await
    }

    /// List follows
    pub async fn list_follows(
        &self,
        fid: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<models::Link>> {
        let link_query = models::LinkQuery {
            fid,
            target_fid: None,
            link_type: Some("follow".to_string()),
            start_timestamp: None,
            end_timestamp: None,
            limit,
            offset: None,
            sort_by: Some(models::LinkSortBy::Timestamp),
            sort_order: Some(models::SortOrder::Desc),
        };
        self.database.list_links(link_query).await
    }

    /// Get user activity timeline
    pub async fn get_user_activity(
        &self,
        fid: i64,
        limit: i64,
        offset: i64,
        activity_type: Option<String>,
    ) -> Result<Vec<models::UserActivityTimeline>> {
        self.database
            .get_user_activity_timeline(fid, activity_type, None, None, Some(limit), Some(offset))
            .await
    }

    /// Create a RAG service for natural language queries
    pub async fn create_rag_service(&self) -> Result<RagService> {
        RagService::new(&self.config).await
    }

    /// Create an embedding service for vector generation
    pub fn create_embedding_service(&self) -> Result<Arc<EmbeddingService>> {
        Ok(Arc::new(EmbeddingService::new(&self.config)?))
    }

    /// Create an LLM service for text generation
    pub fn create_llm_service(&self) -> Result<Arc<LlmService>> {
        Ok(Arc::new(LlmService::new(&self.config)?))
    }

    /// Semantic search for profiles
    pub async fn semantic_search_profiles(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>> {
        let embedding_service = self.create_embedding_service()?;
        let retriever = Retriever::new(self.database.clone(), embedding_service);
        retriever.semantic_search(query, limit, threshold).await
    }

    /// Semantic search for casts
    pub async fn semantic_search_casts(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<models::CastSearchResult>> {
        let embedding_service = self.create_embedding_service()?;
        let cast_retriever = CastRetriever::new(self.database.clone(), embedding_service);
        cast_retriever
            .semantic_search(query, limit, threshold)
            .await
    }

    /// Get cast thread (parent chain + root + children)
    pub async fn get_cast_thread(
        &self,
        message_hash: Vec<u8>,
        depth: usize,
    ) -> Result<database::CastThread> {
        self.database.get_cast_thread(message_hash, depth).await
    }

    /// Backfill profile embeddings
    pub async fn backfill_profile_embeddings(
        &self,
        limit: Option<usize>,
    ) -> Result<ProfileBackfillStats> {
        let embedding_service = self.create_embedding_service()?;
        embeddings::backfill_embeddings(self.database.clone(), embedding_service).await
    }

    /// Backfill cast embeddings
    pub async fn backfill_cast_embeddings(
        &self,
        limit: Option<usize>,
    ) -> Result<CastBackfillStats> {
        let embedding_service = self.create_embedding_service()?;
        embeddings::backfill_cast_embeddings(self.database.clone(), embedding_service, limit).await
    }
}
