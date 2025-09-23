use super::*;
use crate::errors::SnapRagError;
use crate::models::{
    CreateUserProfileRequest, RecordUserActivityRequest, RecordUserDataChangeRequest,
    UpdateUserProfileRequest, UserDataType, UserProfile,
};
use crate::Result;
use uuid::Uuid;

#[tokio::test]
async fn test_user_profile_create_and_read() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99999i64; // Use a high FID to avoid conflicts

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Create a test user profile
    let request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: test_fid,
        username: Some("test_user".to_string()),
        display_name: Some("Test User".to_string()),
        bio: Some("Test bio for CRUD testing".to_string()),
        pfp_url: Some("https://example.com/test-avatar.jpg".to_string()),
        banner_url: None,
        location: Some("Test City".to_string()),
        website_url: Some("https://testuser.com".to_string()),
        twitter_username: Some("testuser".to_string()),
        github_username: Some("testuser".to_string()),
        primary_address_ethereum: Some("0x1234567890123456789012345678901234567890".to_string()),
        primary_address_solana: Some("TestSolanaAddress123456789".to_string()),
        profile_token: None,
        created_at: chrono::Utc::now().timestamp(),
        message_hash: Some(vec![1, 2, 3, 4, 5]),
    };

    // Create the profile
    database.create_user_profile(request).await?;

    // Verify the profile was created
    assert!(verify_user_profile_exists(&database, test_fid).await?);

    // Read the profile data
    let profile_data = get_user_profile_data(&database, test_fid).await?;
    assert!(profile_data.is_some());

    let (username, display_name, bio) = profile_data.unwrap();
    assert_eq!(username, "test_user");
    assert_eq!(display_name, "Test User");
    assert_eq!(bio, "Test bio for CRUD testing");

    // Clean up test data
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_user_profile_update() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99998i64; // Use a different high FID

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Create initial profile
    let initial_timestamp = chrono::Utc::now().timestamp();
    let initial_request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: test_fid,
        username: Some("initial_user".to_string()),
        display_name: Some("Initial User".to_string()),
        bio: Some("Initial bio".to_string()),
        pfp_url: Some("https://example.com/initial-avatar.jpg".to_string()),
        banner_url: None,
        location: None,
        website_url: None,
        twitter_username: None,
        github_username: None,
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        created_at: initial_timestamp,
        message_hash: Some(vec![1, 2, 3, 4, 5]),
    };

    database.create_user_profile(initial_request).await?;

    // Update the profile with a different timestamp
    let update_timestamp = initial_timestamp + 1; // Ensure different timestamp
    let update_request = UpdateUserProfileRequest {
        fid: test_fid,
        data_type: UserDataType::Display,
        new_value: "Updated Display Name".to_string(),
        message_hash: vec![6, 7, 8, 9, 10],
        timestamp: update_timestamp,
    };

    database.update_user_profile(update_request).await?;

    // Verify the update
    let profile_data = get_user_profile_data(&database, test_fid).await?;
    assert!(profile_data.is_some());

    let (username, display_name, bio) = profile_data.unwrap();
    assert_eq!(username, "initial_user"); // Should remain unchanged
    assert_eq!(display_name, "Updated Display Name"); // Should be updated
    assert_eq!(bio, "Initial bio"); // Should remain unchanged

    // Clean up test data
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_user_profile_upsert() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99997i64; // Use another different high FID

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Test upsert with new profile
    let profile = UserProfile {
        id: Uuid::new_v4(),
        fid: test_fid,
        username: Some("upsert_user".to_string()),
        display_name: Some("Upsert User".to_string()),
        bio: Some("Upsert bio".to_string()),
        pfp_url: Some("https://example.com/upsert-avatar.jpg".to_string()),
        banner_url: None,
        location: None,
        website_url: Some("https://upsertuser.com".to_string()),
        twitter_username: None,
        github_username: None,
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        last_updated_at: chrono::Utc::now(),
        shard_id: Some(1),
        block_height: Some(1000),
        transaction_fid: Some(12345),
        last_updated_timestamp: chrono::Utc::now().timestamp(),
        profile_embedding: None,
        bio_embedding: None,
        interests_embedding: None,
    };

    // First upsert (should create)
    database.upsert_user_profile(&profile).await?;

    // Verify it was created
    assert!(verify_user_profile_exists(&database, test_fid).await?);

    let profile_data = get_user_profile_data(&database, test_fid).await?;
    assert!(profile_data.is_some());
    let (username, display_name, bio) = profile_data.unwrap();
    assert_eq!(username, "upsert_user");
    assert_eq!(display_name, "Upsert User");
    assert_eq!(bio, "Upsert bio");

    // Second upsert with updated data (should update)
    let updated_profile = UserProfile {
        id: Uuid::new_v4(), // Different ID
        fid: test_fid,
        username: Some("updated_upsert_user".to_string()),
        display_name: Some("Updated Upsert User".to_string()),
        bio: Some("Updated upsert bio".to_string()),
        pfp_url: Some("https://example.com/updated-upsert-avatar.jpg".to_string()),
        banner_url: None,
        location: None,
        website_url: Some("https://updatedupsertuser.com".to_string()),
        twitter_username: None,
        github_username: None,
        shard_id: Some(1),
        block_height: Some(1001),
        transaction_fid: Some(12346),
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        last_updated_at: chrono::Utc::now(),
        last_updated_timestamp: chrono::Utc::now().timestamp(),
        profile_embedding: None,
        bio_embedding: None,
        interests_embedding: None,
    };

    database.upsert_user_profile(&updated_profile).await?;

    // Verify it was updated
    let updated_profile_data = get_user_profile_data(&database, test_fid).await?;
    assert!(updated_profile_data.is_some());
    let (updated_username, updated_display_name, updated_bio) = updated_profile_data.unwrap();
    assert_eq!(updated_username, "updated_upsert_user");
    assert_eq!(updated_display_name, "Updated Upsert User");
    assert_eq!(updated_bio, "Updated upsert bio");

    // Clean up test data
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_user_data_changes_crud() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99996i64; // Use another different high FID

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Record a user data change
    let change_request = RecordUserDataChangeRequest {
        fid: test_fid,
        data_type: UserDataType::Bio,
        old_value: Some("Old bio".to_string()),
        new_value: "New bio for testing".to_string(),
        message_hash: vec![21, 22, 23, 24, 25],
        timestamp: chrono::Utc::now().timestamp(),
    };

    database
        .record_user_data_change(
            change_request.fid,
            change_request.data_type as i16,
            change_request.old_value,
            change_request.new_value,
            change_request.timestamp,
            change_request.message_hash,
        )
        .await?;

    // Verify the change was recorded
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_data_changes WHERE fid = $1",
        test_fid
    )
    .fetch_one(database.pool())
    .await?;

    assert!(result.count.unwrap_or(0) > 0);

    // Get the change details
    let change_result = sqlx::query!(
        "SELECT data_type, old_value, new_value FROM user_data_changes WHERE fid = $1",
        test_fid
    )
    .fetch_one(database.pool())
    .await?;

    assert_eq!(change_result.data_type, 3); // USER_DATA_TYPE_BIO
    assert_eq!(change_result.old_value, Some("Old bio".to_string()));
    assert_eq!(change_result.new_value, "New bio for testing".to_string());

    // Clean up test data
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_user_activity_recording() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99995i64; // Use another different high FID

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Record user activity
    let activity_request = RecordUserActivityRequest {
        fid: test_fid,
        activity_type: "test_activity".to_string(),
        activity_data: serde_json::json!({
            "test_field": "test_value",
            "nested": {
                "field": "nested_value"
            }
        }),
        timestamp: chrono::Utc::now().timestamp(),
        message_hash: Some(vec![26, 27, 28, 29, 30]),
    };

    database
        .record_user_activity(
            activity_request.fid,
            activity_request.activity_type,
            Some(activity_request.activity_data),
            activity_request.timestamp,
            activity_request.message_hash,
        )
        .await?;

    // Verify the activity was recorded
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_activity_timeline WHERE fid = $1",
        test_fid
    )
    .fetch_one(database.pool())
    .await?;

    assert!(result.count.unwrap_or(0) > 0);

    // Get the activity details
    let activity_result = sqlx::query!(
        "SELECT activity_type, activity_data FROM user_activity_timeline WHERE fid = $1",
        test_fid
    )
    .fetch_one(database.pool())
    .await?;

    assert_eq!(activity_result.activity_type, "test_activity");

    // Verify the JSON data was stored correctly
    let activity_data: Option<serde_json::Value> = activity_result.activity_data;
    assert!(activity_data.is_some());
    let activity_data = activity_data.unwrap();
    assert_eq!(activity_data["test_field"], "test_value");
    assert_eq!(activity_data["nested"]["field"], "nested_value");

    // Clean up test data
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_database_transaction_rollback() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid = 99994i64; // Use another different high FID

    // Clean up any existing test data
    cleanup_test_data(&database, test_fid).await?;

    // Start a transaction
    let mut tx = database.pool().begin().await?;

    // Create a profile within the transaction
    let request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: test_fid,
        username: Some("transaction_user".to_string()),
        display_name: Some("Transaction User".to_string()),
        bio: Some("Transaction bio".to_string()),
        pfp_url: None,
        banner_url: None,
        location: None,
        website_url: None,
        twitter_username: None,
        github_username: None,
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        created_at: chrono::Utc::now().timestamp(),
        message_hash: Some(vec![31, 32, 33, 34, 35]),
    };

    // Execute the insert within the transaction
    sqlx::query!(
        "INSERT INTO user_profiles (id, fid, username, display_name, bio, last_updated_timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
        request.id,
        request.fid,
        request.username,
        request.display_name,
        request.bio,
        request.created_at
    )
    .execute(&mut *tx)
    .await?;

    // Verify it exists within the transaction
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_profiles WHERE fid = $1",
        test_fid
    )
    .fetch_one(&mut *tx)
    .await?;

    assert!(result.count.unwrap_or(0) > 0);

    // Rollback the transaction
    tx.rollback().await?;

    // Verify it doesn't exist after rollback
    let final_result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_profiles WHERE fid = $1",
        test_fid
    )
    .fetch_one(database.pool())
    .await?;

    assert_eq!(final_result.count.unwrap_or(0), 0);

    // Clean up test data (should be no-op, but good practice)
    cleanup_test_data(&database, test_fid).await?;

    Ok(())
}

#[tokio::test]
async fn test_database_concurrent_operations() -> Result<()> {
    let database = create_test_database().await?;
    let test_fid_base = 99990i64; // Base FID for concurrent tests

    // Clean up any existing test data
    for i in 0..5 {
        cleanup_test_data(&database, test_fid_base + i).await?;
    }

    // Create multiple profiles concurrently
    let mut handles = Vec::new();

    for i in 0..5 {
        let db = database.clone();
        let fid = test_fid_base + i;

        let handle = tokio::spawn(async move {
            let request = CreateUserProfileRequest {
                id: Uuid::new_v4(),
                fid,
                username: Some(format!("concurrent_user_{}", i)),
                display_name: Some(format!("Concurrent User {}", i)),
                bio: Some(format!("Concurrent bio {}", i)),
                pfp_url: None,
                banner_url: None,
                location: None,
                website_url: None,
                twitter_username: None,
                github_username: None,
                primary_address_ethereum: None,
                primary_address_solana: None,
                profile_token: None,
                created_at: chrono::Utc::now().timestamp(),
                message_hash: Some(vec![
                    i as u8,
                    (i + 1) as u8,
                    (i + 2) as u8,
                    (i + 3) as u8,
                    (i + 4) as u8,
                ]),
            };

            db.create_user_profile(request).await
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle
            .await
            .map_err(|e| SnapRagError::Custom(e.to_string()))??;
    }

    // Verify all profiles were created
    for i in 0..5 {
        let fid = test_fid_base + i;
        assert!(verify_user_profile_exists(&database, fid).await?);

        let profile_data = get_user_profile_data(&database, fid).await?;
        assert!(profile_data.is_some());
        let (username, display_name, bio) = profile_data.unwrap();
        assert_eq!(username, format!("concurrent_user_{}", i));
        assert_eq!(display_name, format!("Concurrent User {}", i));
        assert_eq!(bio, format!("Concurrent bio {}", i));
    }

    // Clean up all test data
    for i in 0..5 {
        cleanup_test_data(&database, test_fid_base + i).await?;
    }

    Ok(())
}

// Helper functions for testing

async fn verify_user_profile_exists(database: &Database, fid: i64) -> Result<bool> {
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM user_profiles WHERE fid = $1",
        fid
    )
    .fetch_one(database.pool())
    .await?;

    Ok(result.count.unwrap_or(0) > 0)
}

async fn get_user_profile_data(
    database: &Database,
    fid: i64,
) -> Result<Option<(String, String, String)>> {
    let result = sqlx::query!(
        "SELECT username, display_name, bio FROM user_profiles WHERE fid = $1",
        fid
    )
    .fetch_one(database.pool())
    .await?;

    match (result.username, result.display_name, result.bio) {
        (Some(username), Some(display_name), Some(bio)) => Ok(Some((username, display_name, bio))),
        _ => Ok(None),
    }
}

async fn cleanup_test_data(database: &Database, fid: i64) -> Result<()> {
    // Clean up in reverse order of dependencies
    sqlx::query!("DELETE FROM user_activity_timeline WHERE fid = $1", fid)
        .execute(database.pool())
        .await?;

    sqlx::query!("DELETE FROM user_data_changes WHERE fid = $1", fid)
        .execute(database.pool())
        .await?;

    sqlx::query!("DELETE FROM user_profile_snapshots WHERE fid = $1", fid)
        .execute(database.pool())
        .await?;

    sqlx::query!("DELETE FROM user_profiles WHERE fid = $1", fid)
        .execute(database.pool())
        .await?;

    Ok(())
}
