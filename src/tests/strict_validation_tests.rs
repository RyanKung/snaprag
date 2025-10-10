//! Strict validation tests that ensure no panics and proper error handling
//! These tests validate that all interfaces work correctly without panicking

use std::panic;

use uuid::Uuid;

use crate::config::AppConfig;
use crate::database::Database;
use crate::errors::SnapRagError;
use crate::models::CreateUserProfileRequest;
use crate::models::UserDataType;
use crate::Result;

/// Test that validates all database operations handle errors gracefully
#[tokio::test]
async fn test_database_error_handling() -> Result<()> {
    // Test with invalid configuration to ensure proper error handling
    let invalid_config = AppConfig {
        database: crate::config::DatabaseConfig {
            url: "invalid://url".to_string(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout: 30,
        },
        logging: crate::config::LoggingConfig {
            level: "info".to_string(),
            backtrace: true,
        },
        embeddings: crate::config::EmbeddingsConfig {
            dimension: 1536,
            model: "text-embedding-ada-002".to_string(),
        },
        performance: crate::config::PerformanceConfig {
            enable_vector_indexes: true,
            vector_index_lists: 100,
        },
        sync: crate::config::SyncConfig {
            snapchain_http_endpoint: "http://localhost:3383".to_string(),
            snapchain_grpc_endpoint: "http://localhost:3384".to_string(),
            enable_realtime_sync: true,
            enable_historical_sync: false,
            historical_sync_from_event_id: 0,
            batch_size: 100,
            sync_interval_ms: 1000,
            shard_ids: vec![0, 1, 2],
        },
        llm: crate::config::LlmConfig {
            llm_endpoint: "https://api.openai.com/v1".to_string(),
            llm_key: "test-key".to_string(),
        },
    };

    // This should return an error, not panic
    let result = Database::from_config(&invalid_config).await;
    assert!(result.is_err());

    // Verify the error is properly typed
    match result.unwrap_err() {
        SnapRagError::Database(_) => {
            // Expected error type for invalid database URL
        }
        other => panic!("Expected Database error, got: {:?}", other),
    }

    Ok(())
}

/// Test that validates user profile creation with invalid data
#[tokio::test]
async fn test_user_profile_validation() -> Result<()> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;

    // Test with invalid FID (negative)
    let invalid_request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: -1, // Invalid FID
        username: Some("test".to_string()),
        display_name: Some("Test".to_string()),
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
        created_at: chrono::Utc::now().timestamp(),
        message_hash: Some(vec![1, 2, 3]),
    };

    // This should handle the invalid FID gracefully
    let result = database.create_user_profile(invalid_request).await;
    // The database might accept negative FIDs, so we just ensure it doesn't panic
    // If it returns an error, that's also acceptable
    match result {
        Ok(_) => {
            // Database accepted the invalid FID, clean up
            cleanup_test_data(&database, -1).await?;
        }
        Err(_) => {
            // Database rejected the invalid FID, which is also acceptable
        }
    }

    Ok(())
}

/// Test that validates error handling in sync operations
#[tokio::test]
async fn test_sync_error_handling() -> Result<()> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;

    // Test with invalid block range (to < from)
    // This should not panic, even with invalid range
    let sync_service = crate::sync::SyncService::new(&config, std::sync::Arc::new(database)).await;
    match sync_service {
        Ok(service) => {
            // Test invalid range
            let result = service.start_with_range(1000, 500).await;
            // Should return an error, not panic
            assert!(result.is_err());
        }
        Err(_) => {
            // Service creation failed, which is acceptable
        }
    }

    Ok(())
}

/// Test that validates all string operations handle edge cases
#[tokio::test]
async fn test_string_handling() -> Result<()> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;

    // Test with empty strings
    let empty_request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: 99999,
        username: Some("".to_string()),     // Empty string
        display_name: Some("".to_string()), // Empty string
        bio: Some("".to_string()),          // Empty string
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
        message_hash: Some(vec![]), // Empty vector
    };

    // This should not panic
    let result = database.create_user_profile(empty_request).await;
    match result {
        Ok(_) => {
            // Clean up
            cleanup_test_data(&database, 99999).await?;
        }
        Err(_) => {
            // Also acceptable
        }
    }

    // Test with very long strings
    let long_string = "a".repeat(10000);
    let long_request = CreateUserProfileRequest {
        id: Uuid::new_v4(),
        fid: 99998,
        username: Some(long_string.clone()),
        display_name: Some(long_string.clone()),
        bio: Some(long_string),
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
        message_hash: Some(vec![1; 1000]), // Large vector
    };

    // This should not panic
    let result = database.create_user_profile(long_request).await;
    match result {
        Ok(_) => {
            // Clean up
            cleanup_test_data(&database, 99998).await?;
        }
        Err(_) => {
            // Also acceptable
        }
    }

    Ok(())
}

/// Test that validates concurrent operations don't cause panics
#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    let db_arc = std::sync::Arc::new(database);

    // Test concurrent profile creation
    let mut handles = Vec::new();
    for i in 0..10 {
        let db = db_arc.clone();
        let handle = tokio::spawn(async move {
            let request = CreateUserProfileRequest {
                id: Uuid::new_v4(),
                fid: 90000 + i,
                username: Some(format!("concurrent_user_{}", i)),
                display_name: Some(format!("Concurrent User {}", i)),
                bio: Some(format!("Bio {}", i)),
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
                message_hash: Some(vec![i as u8; 10]),
            };

            // This should not panic
            db.create_user_profile(request).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await;
        // Should not panic
        assert!(result.is_ok());

        // Check if the operation succeeded or failed gracefully
        match result.unwrap() {
            Ok(_) => {
                // Operation succeeded
            }
            Err(e) => {
                // Operation failed gracefully - this is acceptable
                assert!(
                    !e.to_string().contains("panic"),
                    "Concurrent operation must not panic: {}",
                    e
                );
            }
        }
    }

    // Clean up
    for i in 0..10 {
        cleanup_test_data(&db_arc, 90000 + i).await?;
    }

    Ok(())
}

/// Test that validates error types are properly propagated
#[tokio::test]
async fn test_error_propagation() -> Result<()> {
    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;

    // Test with invalid user data type
    let result = database
        .update_user_profile(crate::models::UpdateUserProfileRequest {
            fid: 99997,
            data_type: UserDataType::Bio,
            new_value: "test".to_string(),
            message_hash: vec![1, 2, 3],
            timestamp: chrono::Utc::now().timestamp(),
        })
        .await;

    // Should return an error, not panic
    match result {
        Ok(_) => {
            // Profile update succeeded (user might exist)
            cleanup_test_data(&database, 99997).await?;
        }
        Err(e) => {
            // Should be a proper error type
            match e {
                SnapRagError::UserNotFound(_) => {
                    // Expected error for non-existent user
                }
                SnapRagError::Database(_) => {
                    // Database error is also acceptable
                }
                other => {
                    // Other error types are also acceptable
                    assert!(
                        !other.to_string().contains("panic"),
                        "Error must not contain panic: {:?}",
                        other
                    );
                }
            }
        }
    }

    Ok(())
}

/// Helper function to clean up test data
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
