pub mod database_tests;
pub mod grpc_shard_chunks_test;
pub mod integration_sync_test;
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
    // Clean up user profiles
    sqlx::query!("DELETE FROM user_profiles WHERE fid = $1", test_fid)
        .execute(database.pool())
        .await?;

    // Clean up user data changes
    sqlx::query!("DELETE FROM user_data_changes WHERE fid = $1", test_fid)
        .execute(database.pool())
        .await?;

    // Clean up user activities
    sqlx::query!("DELETE FROM user_activities WHERE fid = $1", test_fid)
        .execute(database.pool())
        .await?;

    // Clean up user activity timeline
    sqlx::query!(
        "DELETE FROM user_activity_timeline WHERE fid = $1",
        test_fid
    )
    .execute(database.pool())
    .await?;

    Ok(())
}

/// Test helper to verify data exists in database
pub async fn verify_user_profile_exists(database: &Database, fid: i64) -> Result<bool> {
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_profiles WHERE fid = $1",
        fid
    )
    .fetch_one(database.pool())
    .await?;

    Ok(result.count.unwrap_or(0) > 0)
}

/// Test helper to get user profile data
pub async fn get_user_profile_data(
    database: &Database,
    fid: i64,
) -> Result<Option<(String, String, String)>> {
    let result = sqlx::query!(
        "SELECT username, display_name, bio FROM user_profiles WHERE fid = $1",
        fid
    )
    .fetch_optional(database.pool())
    .await?;

    if let Some(row) = result {
        Ok(Some((
            row.username.unwrap_or_default(),
            row.display_name.unwrap_or_default(),
            row.bio.unwrap_or_default(),
        )))
    } else {
        Ok(None)
    }
}
