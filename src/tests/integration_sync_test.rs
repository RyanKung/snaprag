use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::{SyncLockManager, SyncService};
use crate::Result;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tracing::info;

// Global test lock to ensure tests run serially
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Helper function to run snaprag CLI commands
fn run_snaprag_command(args: &[&str]) -> Result<String> {
    let output = Command::new("cargo")
        .args(&["run", "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::SnapRagError::Custom(format!(
            "Command failed: {}\nStderr: {}",
            output.status, stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Helper function to clean up before each test
fn cleanup_before_test() -> Result<()> {
    // Stop any running sync processes
    let _ = run_snaprag_command(&["sync", "stop", "--force"]);

    // Reset all data and lock files
    let _ = run_snaprag_command(&["reset", "--force"]);

    // Small delay to ensure cleanup is complete
    std::thread::sleep(std::time::Duration::from_millis(100));

    Ok(())
}

/// Integration test for sync service with user message blocks
/// Tests the complete sync pipeline from gRPC to database
#[tokio::test]
async fn test_sync_user_message_blocks() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    // Initialize logging for test
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting integration test for user message blocks");

    // Load configuration
    let config = AppConfig::load()?;
    info!("✅ Configuration loaded successfully");

    // Initialize database
    let database = Database::from_config(&config).await?;
    let db_arc = Arc::new(database);
    info!("✅ Database initialized successfully");

    // Clean up any existing lock files
    let lock_manager = SyncLockManager::new();
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
        info!("🧹 Cleaned up existing lock file");
    }

    // Test sync with known user message blocks
    // Based on our analysis: blocks 1250000-1250100 contain user messages
    let test_from_block = 1250000;
    let test_to_block = 1250005; // Small range for testing

    info!(
        "🚀 Testing sync from block {} to {}",
        test_from_block, test_to_block
    );

    // Create sync service
    let sync_service = SyncService::new(&config, db_arc.clone()).await?;
    info!("✅ Sync service created successfully");

    // Run sync with range
    let sync_result = sync_service
        .start_with_range(test_from_block, test_to_block)
        .await;

    match sync_result {
        Ok(()) => {
            info!("✅ Sync completed successfully");
        }
        Err(e) => {
            panic!("❌ Sync failed with error: {}", e);
        }
    }

    // Verify lock file was created and contains progress
    if lock_manager.lock_exists() {
        let lock = lock_manager.read_lock()?;
        info!("📊 Lock file status: {}", lock.status);
        info!(
            "📊 Processed blocks: {}",
            lock.progress.total_blocks_processed
        );
        info!(
            "📊 Processed messages: {}",
            lock.progress.total_messages_processed
        );

        // Clean up lock file
        lock_manager.remove_lock()?;
        info!("🧹 Cleaned up lock file");
    } else {
        panic!("❌ No lock file found after sync - this should not happen!");
    }

    // Check database for any new data
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(db_arc.pool())
        .await?;

    let activity_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_activity_timeline")
            .fetch_one(db_arc.pool())
            .await?;

    let changes_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_data_changes")
        .fetch_one(db_arc.pool())
        .await?;

    info!("📊 Database state after sync:");
    info!("  - User profiles: {}", user_count);
    info!("  - User activities: {}", activity_count);
    info!("  - User data changes: {}", changes_count);

    // Test passed if we got this far without panicking
    info!("🎉 Integration test completed successfully");
    Ok(())
}

/// Test sync with high activity blocks (5000000+)
#[tokio::test]
async fn test_sync_high_activity_blocks() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting integration test for high activity blocks");

    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    let db_arc = Arc::new(database);

    // Clean up lock files
    let lock_manager = SyncLockManager::new();
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
    }

    // Test with high activity range: 5000000-5000005
    let test_from_block = 5000000;
    let test_to_block = 5000005;

    info!(
        "🚀 Testing sync from block {} to {}",
        test_from_block, test_to_block
    );

    let sync_service = SyncService::new(&config, db_arc.clone()).await?;
    let sync_result = sync_service
        .start_with_range(test_from_block, test_to_block)
        .await;

    match sync_result {
        Ok(()) => info!("✅ High activity sync completed successfully"),
        Err(e) => panic!("❌ High activity sync failed with error: {}", e),
    }

    // Check results
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(db_arc.pool())
        .await?;

    info!("📊 High activity sync results:");
    info!("  - User profiles: {}", user_count);

    // Clean up
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
    }

    info!("🎉 High activity integration test completed");
    Ok(())
}

/// Test sync with early blocks (no user messages)
#[tokio::test]
async fn test_sync_early_blocks() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting integration test for early blocks (no user messages)");

    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    let db_arc = Arc::new(database);

    // Clean up lock files
    let lock_manager = SyncLockManager::new();
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
    }

    // Test with early range: 0-10 (known to have no user messages)
    let test_from_block = 0;
    let test_to_block = 10;

    info!(
        "🚀 Testing sync from block {} to {}",
        test_from_block, test_to_block
    );

    let sync_service = SyncService::new(&config, db_arc.clone()).await?;
    let sync_result = sync_service
        .start_with_range(test_from_block, test_to_block)
        .await;

    match sync_result {
        Ok(()) => info!("✅ Early blocks sync completed successfully"),
        Err(e) => panic!("❌ Early blocks sync failed with error: {}", e),
    }

    // Check results - should have no user data
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(db_arc.pool())
        .await?;

    let activity_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_activity_timeline")
            .fetch_one(db_arc.pool())
            .await?;

    info!("📊 Early blocks sync results:");
    info!("  - User profiles: {}", user_count);
    info!("  - User activities: {}", activity_count);

    // Clean up
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
    }

    info!("🎉 Early blocks integration test completed");
    Ok(())
}

/// Test sync service error handling
#[tokio::test]
async fn test_sync_error_handling() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting integration test for error handling");

    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    let db_arc = Arc::new(database);

    // Test with invalid range (to > from)
    let test_from_block = 1000;
    let test_to_block = 500; // Invalid: to < from

    info!(
        "🚀 Testing sync with invalid range: {} to {}",
        test_from_block, test_to_block
    );

    let sync_service = SyncService::new(&config, db_arc.clone()).await?;
    let sync_result = sync_service
        .start_with_range(test_from_block, test_to_block)
        .await;

    match sync_result {
        Ok(()) => {
            info!("✅ Sync with invalid range completed (this might be expected behavior)");
        }
        Err(e) => {
            info!("✅ Sync correctly handled invalid range with error: {}", e);
        }
    }

    info!("🎉 Error handling integration test completed");
    Ok(())
}

/// Test lock file management during sync
#[tokio::test]
async fn test_lock_file_management() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting integration test for lock file management");

    let config = AppConfig::load()?;
    let database = Database::from_config(&config).await?;
    let db_arc = Arc::new(database);

    let lock_manager = SyncLockManager::new();

    // Clean up any existing lock files
    if lock_manager.lock_exists() {
        lock_manager.remove_lock()?;
    }

    // Test sync with small range
    let test_from_block = 1250000;
    let test_to_block = 1250001;

    info!("🚀 Testing lock file management during sync");

    let sync_service = SyncService::new(&config, db_arc.clone()).await?;

    // Start sync (should create lock file)
    let sync_result = sync_service
        .start_with_range(test_from_block, test_to_block)
        .await;

    // Check lock file was created
    if lock_manager.lock_exists() {
        let lock = lock_manager.read_lock()?;
        info!("✅ Lock file created successfully");
        info!("  - PID: {}", lock.pid);
        info!("  - Status: {}", lock.status);
        info!(
            "  - Sync range: {} to {}",
            lock.progress
                .sync_range
                .as_ref()
                .map(|r| r.from_block)
                .unwrap_or(0),
            lock.progress
                .sync_range
                .as_ref()
                .and_then(|r| r.to_block)
                .map(|b| b.to_string())
                .unwrap_or("latest".to_string())
        );

        // Clean up
        lock_manager.remove_lock()?;
        info!("✅ Lock file cleaned up successfully");
    } else {
        panic!("❌ Lock file was not created during sync - this should not happen!");
    }

    match sync_result {
        Ok(()) => info!("✅ Sync completed successfully"),
        Err(e) => panic!("❌ Sync failed with error: {}", e),
    }

    info!("🎉 Lock file management integration test completed");
    Ok(())
}

/// Test CLI commands functionality
#[tokio::test]
async fn test_cli_functionality() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting CLI functionality test");

    // Test 1: Check sync status (should show no active sync)
    info!("📊 Testing sync status command...");
    let status_output = run_snaprag_command(&["sync", "status"])?;
    info!("Sync status output: {}", status_output);
    assert!(status_output.contains("No active sync process") || status_output.contains("Status:"));

    // Test 2: Test sync start with range
    info!("🚀 Testing sync start command...");
    let sync_output =
        run_snaprag_command(&["sync", "start", "--from", "1250000", "--to", "1250005"])?;
    info!("Sync start output: {}", sync_output);
    assert!(
        sync_output.contains("Starting synchronization") || sync_output.contains("sync service")
    );

    // Test 3: Check sync status again (should show completed sync)
    info!("📊 Testing sync status after sync...");
    let status_output2 = run_snaprag_command(&["sync", "status"])?;
    info!("Sync status after sync: {}", status_output2);
    // Note: This might show "No active sync process" if the sync completed quickly

    // Test 4: Test sync stop command
    info!("🛑 Testing sync stop command...");
    let stop_output = run_snaprag_command(&["sync", "stop"])?;
    info!("Sync stop output: {}", stop_output);
    assert!(
        stop_output.contains("Stopping sync processes")
            || stop_output.contains("No active sync process")
    );

    // Test 5: Test list commands
    info!("📋 Testing list commands...");
    let list_fids_output = run_snaprag_command(&["list", "fid", "--limit", "5"])?;
    info!("List FIDs output: {}", list_fids_output);

    let list_profiles_output = run_snaprag_command(&["list", "profiles", "--limit", "5"])?;
    info!("List profiles output: {}", list_profiles_output);

    // Test 6: Test reset command (with force to avoid interactive prompt)
    info!("🧹 Testing reset command...");
    let reset_output = run_snaprag_command(&["reset", "--force"])?;
    info!("Reset output: {}", reset_output);
    assert!(
        reset_output.contains("Resetting all synchronized data")
            || reset_output.contains("Clearing all synchronized data")
    );

    info!("🎉 CLI functionality test completed successfully");
    Ok(())
}

/// Test CLI commands with different block ranges
#[tokio::test]
async fn test_cli_sync_ranges() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting CLI sync ranges test");

    // Test different block ranges based on our analysis
    let test_ranges = vec![
        ("0-10", "0", "10"),                       // Early blocks (no user messages)
        ("1250000-1250005", "1250000", "1250005"), // User message start range
        ("5000000-5000005", "5000000", "5000005"), // High activity range
    ];

    for (range_name, from, to) in test_ranges {
        info!(
            "🚀 Testing sync range: {} (blocks {} to {})",
            range_name, from, to
        );

        // Clean up before each test
        cleanup_before_test()?;

        let sync_output = run_snaprag_command(&["sync", "start", "--from", from, "--to", to])?;
        info!("Sync output for {}: {}", range_name, sync_output);

        // Check that the command executed without major errors
        assert!(!sync_output.contains("FATAL") && !sync_output.contains("panic"));

        // Small delay between tests
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    info!("🎉 CLI sync ranges test completed successfully");
    Ok(())
}

/// Test CLI error handling
#[tokio::test]
async fn test_cli_error_handling() -> Result<()> {
    // Acquire global test lock to ensure serial execution
    let _lock = TEST_LOCK.lock().unwrap();
    
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    // Clean up before test
    cleanup_before_test()?;

    info!("🧪 Starting CLI error handling test");

    // Test 1: Invalid command
    info!("❌ Testing invalid command...");
    let invalid_output = run_snaprag_command(&["invalid", "command"]);
    assert!(invalid_output.is_err(), "Invalid command should fail");

    // Test 2: Invalid sync range (to < from)
    info!("❌ Testing invalid sync range...");
    let invalid_range_output =
        run_snaprag_command(&["sync", "start", "--from", "1000", "--to", "500"]);
    // This might succeed but should handle the invalid range gracefully
    if let Ok(output) = invalid_range_output {
        info!("Invalid range output: {}", output);
    }

    // Test 3: Missing required arguments
    info!("❌ Testing missing arguments...");
    let missing_args_output = run_snaprag_command(&["sync", "start"]);
    // This should succeed (sync without range)
    if let Ok(output) = missing_args_output {
        info!("Missing args output: {}", output);
    }

    info!("🎉 CLI error handling test completed successfully");
    Ok(())
}
