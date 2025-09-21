use snaprag::database::Database;
use snaprag::models::*;
use snaprag::AppConfig;
use snaprag::Result;
use sqlx::PgPool;

async fn setup_test_db() -> Result<Database> {
    // Load configuration from config.toml
    let config = AppConfig::load()?;
    
    // Create database connection pool with config
    let pool = PgPool::connect(&config.database_url()).await?;
        
    let db = Database::new(pool);
    
    // Initialize schema
    db.init_schema().await?;
    
    Ok(db)
}

#[tokio::test]
async fn test_create_user_profile() -> Result<()> {
    let db = setup_test_db().await?;
    
    let create_request = CreateUserProfileRequest {
        fid: 9999,
        username: Some("testuser".to_string()),
        display_name: Some("Test User".to_string()),
        bio: Some("Test bio".to_string()),
        pfp_url: None,
        banner_url: None,
        location: None,
        website_url: None,
        twitter_username: None,
        github_username: None,
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        message_hash: vec![1, 2, 3, 4, 5],
        timestamp: 1640995200,
    };

    let profile = db.create_user_profile(create_request).await?;
    assert_eq!(profile.fid, 9999);
    assert_eq!(profile.username, Some("testuser".to_string()));
    assert_eq!(profile.bio, Some("Test bio".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_update_user_profile() -> Result<()> {
    let db = setup_test_db().await?;
    
    // Create initial profile
    let create_request = CreateUserProfileRequest {
        fid: 9998,
        username: Some("testuser2".to_string()),
        display_name: Some("Test User 2".to_string()),
        bio: Some("Original bio".to_string()),
        pfp_url: None,
        banner_url: None,
        location: None,
        website_url: None,
        twitter_username: None,
        github_username: None,
        primary_address_ethereum: None,
        primary_address_solana: None,
        profile_token: None,
        message_hash: vec![1, 2, 3, 4, 5],
        timestamp: 1640995200,
    };

    let _profile = db.create_user_profile(create_request).await?;

    // Update bio
    let update_request = UpdateUserProfileRequest {
        fid: 9998,
        data_type: UserDataType::Bio,
        new_value: "Updated bio".to_string(),
        message_hash: vec![6, 7, 8, 9, 10],
        timestamp: 1640995800,
    };

    let updated_profile = db.update_user_profile(update_request).await?;
    assert_eq!(updated_profile.bio, Some("Updated bio".to_string()));

    // Check that snapshots were created
    let snapshots = db.get_profile_snapshots(ProfileSnapshotQuery {
        fid: 9998,
        start_timestamp: None,
        end_timestamp: None,
        limit: Some(10),
        offset: None,
    }).await?;
    
    assert_eq!(snapshots.len(), 2); // Initial + update

    // Check that changes were recorded
    let changes = db.get_user_data_changes(9998, Some(UserDataType::Bio as i32), Some(10), None).await?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].old_value, Some("Original bio".to_string()));
    assert_eq!(changes[0].new_value, "Updated bio".to_string());

    Ok(())
}

#[tokio::test]
async fn test_username_proof() -> Result<()> {
    let db = setup_test_db().await?;
    
    let proof = db.upsert_username_proof(
        9997,
        "testuser3".to_string(),
        UsernameType::Fname,
        "0x1234567890123456789012345678901234567890".to_string(),
        vec![1, 2, 3, 4, 5],
        1640995200,
    ).await?;

    assert_eq!(proof.fid, 9997);
    assert_eq!(proof.username, "testuser3");
    assert_eq!(proof.username_type, UsernameType::Fname as i32);

    // Test getting the proof
    let retrieved_proof = db.get_username_proof(9997, UsernameType::Fname).await?;
    assert!(retrieved_proof.is_some());
    assert_eq!(retrieved_proof.unwrap().username, "testuser3");

    Ok(())
}

#[tokio::test]
async fn test_user_activity() -> Result<()> {
    let db = setup_test_db().await?;
    
    let activity = db.record_user_activity(
        9996,
        "cast".to_string(),
        Some(serde_json::json!({
            "text": "Test cast",
            "mentions": [1001, 1002]
        })),
        1640995200,
        Some(vec![1, 2, 3, 4, 5]),
    ).await?;

    assert_eq!(activity.fid, 9996);
    assert_eq!(activity.activity_type, "cast");
    assert_eq!(activity.timestamp, 1640995200);

    // Test getting activities
    let activities = db.get_user_activity_timeline(
        9996,
        Some("cast".to_string()),
        None,
        None,
        Some(10),
        None,
    ).await?;

    assert_eq!(activities.len(), 1);
    assert_eq!(activities[0].activity_type, "cast");

    Ok(())
}
