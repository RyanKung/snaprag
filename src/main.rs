use snaprag::database::Database;
use snaprag::models::*;
use snaprag::AppConfig;
use snaprag::Result;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from config.toml
    let config = AppConfig::load()?;

    println!("📋 Loaded configuration:");
    println!(
        "  Database URL: {}",
        mask_database_url(&config.database_url())
    );
    println!("  Max connections: {}", config.max_connections());
    println!("  Embedding dimension: {}", config.embedding_dimension());
    println!(
        "  Vector indexes enabled: {}",
        config.vector_indexes_enabled()
    );

    // Create database connection pool with config
    let pool = PgPool::connect(&config.database_url()).await?;

    let db = Database::new(pool);

    // Initialize database schema
    db.init_schema().await?;
    println!("Database schema initialized successfully!");

    // Example usage
    example_usage(&db).await?;

    Ok(())
}

async fn example_usage(db: &Database) -> Result<()> {
    println!("Running example usage...");

    // Create a user profile
    let create_request = CreateUserProfileRequest {
        id: uuid::Uuid::new_v4(),
        fid: 12345,
        username: Some("alice".to_string()),
        display_name: Some("Alice Smith".to_string()),
        bio: Some("Blockchain enthusiast and developer".to_string()),
        pfp_url: Some("https://example.com/avatar.jpg".to_string()),
        banner_url: None,
        location: Some("San Francisco, CA".to_string()),
        website_url: Some("https://alice.dev".to_string()),
        twitter_username: Some("alice_dev".to_string()),
        github_username: Some("alice-github".to_string()),
        primary_address_ethereum: Some("0x1234567890123456789012345678901234567890".to_string()),
        primary_address_solana: None,
        profile_token: None,
        message_hash: Some(vec![1, 2, 3, 4, 5]),
        created_at: 1640995200, // 2022-01-01 00:00:00 UTC
    };

    let profile = db.create_user_profile(create_request).await?;
    println!("Created user profile: {:?}", profile);

    // Update user profile
    let update_request = UpdateUserProfileRequest {
        fid: 12345,
        data_type: UserDataType::Bio,
        new_value: "Senior blockchain developer and DeFi researcher".to_string(),
        message_hash: vec![6, 7, 8, 9, 10],
        timestamp: 1640995800, // 10 minutes later
    };

    let updated_profile = db.update_user_profile(update_request).await?;
    println!("Updated user profile: {:?}", updated_profile);

    // Get profile snapshots
    let snapshot_query = ProfileSnapshotQuery {
        fid: 12345,
        start_timestamp: None,
        end_timestamp: None,
        limit: Some(10),
        offset: None,
    };

    let snapshots = db.get_profile_snapshots(snapshot_query).await?;
    println!("Found {} profile snapshots", snapshots.len());

    // Get user data changes
    let changes = db
        .get_user_data_changes(12345, Some(UserDataType::Bio as i16), Some(10), None)
        .await?;
    println!("Found {} bio changes", changes.len());

    // Create username proof
    let proof = db
        .upsert_username_proof(
            12345,
            "alice".to_string(),
            UsernameType::Fname,
            "0x1234567890123456789012345678901234567890".to_string(),
            vec![11, 12, 13, 14, 15],
            1640995200,
        )
        .await?;
    println!("Created username proof: {:?}", proof);

    // Record user activity
    let activity = db
        .record_user_activity(
            12345,
            "cast".to_string(),
            Some(serde_json::json!({
                "text": "Hello, Farcaster!",
                "mentions": [67890]
            })),
            1640995200,
            Some(vec![16, 17, 18, 19, 20]),
        )
        .await?;
    println!("Recorded user activity: {:?}", activity);

    // Query user profiles
    let query = UserProfileQuery {
        fid: None,
        username: Some("alice".to_string()),
        display_name: None,
        limit: Some(10),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
    };

    let profiles = db.list_user_profiles(query).await?;
    println!("Found {} profiles matching query", profiles.len());

    println!("Example usage completed successfully!");
    Ok(())
}

/// Mask database URL for logging (hide password)
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            format!(
                "{}://{}@{}:{}",
                parsed.scheme(),
                parsed.username(),
                host,
                parsed.port().unwrap_or(5432)
            )
        } else {
            "***masked***".to_string()
        }
    } else {
        "***invalid***".to_string()
    }
}
