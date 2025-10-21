use sqlx::PgPool;

use crate::models::*;
use crate::Result;
use crate::SnapRagError;

// Re-export submodules
mod casts;
mod links;
mod schema;
mod sync;
mod user_activity;
mod user_data;
mod user_data_changes;
mod user_profiles;
mod user_snapshots;
mod username_proofs;

// Re-export types
pub use casts::CastThread;
pub use sync::SyncStats;

/// Database connection pool wrapper
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    #[must_use] 
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new database instance from configuration
    pub async fn from_config(config: &crate::config::AppConfig) -> Result<Self> {
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout()));

        let pool = pool_options.connect(config.database_url()).await?;
        
        tracing::info!(
            "Database pool configured: max_connections={}, min_connections={}",
            config.max_connections(),
            config.min_connections()
        );
        
        Ok(Self::new(pool))
    }

    /// Run database migrations
    /// Note: Migrations are currently managed manually via SQL files in /migrations
    /// Future enhancement: Could integrate with sqlx migrations or refinery
    pub const fn migrate(&self) -> Result<()> {
        Ok(())
    }

    /// Get a reference to the database pool for raw queries
    #[must_use] 
    pub const fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
