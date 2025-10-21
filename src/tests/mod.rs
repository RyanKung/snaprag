pub mod database_tests;
pub mod deterministic_blocks_test;
pub mod event_sourcing_test;
pub mod grpc_shard_chunks_test;
pub mod integration_sync_test;
pub mod rag_integration_test;
pub mod strict_test_config;
pub mod strict_test_runner;
pub mod strict_test_validation;
pub mod strict_validation_tests;

use crate::config::AppConfig;
use crate::database::Database;
use crate::Result;

/// Test helper to create a test database connection
pub async fn create_test_database() -> Result<Database> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    Ok(database)
}

/// Test helper to clean up test data
pub async fn cleanup_test_data(database: &Database, test_fid: i64) -> Result<()> {
    // Clean up user profile changes (event-sourcing table)
    sqlx::query("DELETE FROM user_profile_changes WHERE fid = $1")
        .bind(test_fid)
        .execute(database.pool())
        .await?;

    // Clean up user data changes
    sqlx::query("DELETE FROM user_data_changes WHERE fid = $1")
        .bind(test_fid)
        .execute(database.pool())
        .await?;

    Ok(())
}

/// Test helper to verify data exists in database
pub async fn verify_user_profile_exists(database: &Database, fid: i64) -> Result<bool> {
    let result: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM user_profiles WHERE fid = $1"
    )
    .bind(fid)
    .fetch_one(database.pool())
    .await?;

    Ok(result.0 > 0)
}

/// Test helper to get user profile data
pub async fn get_user_profile_data(
    database: &Database,
    fid: i64,
) -> Result<Option<(String, String, String)>> {
    let result: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT username, display_name, bio FROM user_profiles WHERE fid = $1"
    )
    .bind(fid)
    .fetch_optional(database.pool())
    .await?;

    if let Some((username, display_name, bio)) = result {
        Ok(Some((
            username.unwrap_or_default(),
            display_name.unwrap_or_default(),
            bio.unwrap_or_default(),
        )))
    } else {
        Ok(None)
    }
}
