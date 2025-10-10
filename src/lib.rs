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
pub use errors::*;
pub use models::*;
pub use sync::service::SyncService;
use tracing::info;

/// Main SnapRAG client for high-level operations
pub struct SnapRag {
    config: AppConfig,
    database: Arc<Database>,
    sync_service: Option<Arc<SyncService>>,
}

impl SnapRag {
    /// Create a new SnapRAG instance
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let database = Arc::new(Database::from_config(config).await?);
        Ok(Self {
            config: config.clone(),
            database,
            sync_service: None,
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
        if let Some(sync_service) = &self.sync_service {
            sync_service.stop(force).await?;
        }
        Ok(())
    }

    /// Get sync status
    pub fn get_sync_status(&self) -> Result<Option<crate::sync::lock_file::SyncLockFile>> {
        if let Some(sync_service) = &self.sync_service {
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
}
