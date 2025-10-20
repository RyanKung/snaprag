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
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new database instance from configuration
    pub async fn from_config(config: &crate::config::AppConfig) -> Result<Self> {
        let threshold_ms = (config.slow_query_threshold_secs() * 1000.0) as i32;

        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout()))
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    // Try to set slow query logging, but don't fail if no permission
                    // This requires ALTER SYSTEM or SUPERUSER privileges
                    if let Err(e) = sqlx::query(&format!(
                        "SET log_min_duration_statement = {}",
                        threshold_ms
                    ))
                    .execute(&mut *conn)
                    .await
                    {
                        tracing::debug!(
                            "Could not set log_min_duration_statement (needs elevated privileges): {}",
                            e
                        );
                        // Continue anyway - this is optional
                    }
                    Ok(())
                })
            });

        let pool = pool_options.connect(config.database_url()).await?;
        Ok(Self::new(pool))
    }

    /// Run database migrations
    /// Note: Migrations are currently managed manually via SQL files in /migrations
    /// Future enhancement: Could integrate with sqlx migrations or refinery
    pub async fn migrate(&self) -> Result<()> {
        Ok(())
    }

    /// Get a reference to the database pool for raw queries
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
