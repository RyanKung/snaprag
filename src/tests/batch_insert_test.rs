//! Batch insert integration tests
//! Tests that verify the actual batch insert logic with correct parameter binding

use crate::database::Database;
use crate::models::ShardBlockInfo;
use crate::sync::shard_processor::flush_batched_data;
use crate::sync::shard_processor::BatchedData;
use crate::Result;

/// Test helper to create a test database
async fn setup_test_db() -> Database {
    // 🛡️ CRITICAL: Force use of test configuration
    let config_path =
        std::env::var("SNAPRAG_CONFIG").unwrap_or_else(|_| "config.test.toml".to_string());
    let config = crate::config::AppConfig::from_file(&config_path).expect("Failed to load config");

    // 🛡️ CRITICAL: Verify we're using local database
    let db_url = config.database_url();
    if !db_url.contains("localhost") && !db_url.contains("127.0.0.1") && !db_url.contains("::1") {
        panic!(
            "❌ SAFETY CHECK FAILED: Test database must be localhost!\n\
             Current URL: {}\n\
             Set SNAPRAG_CONFIG=config.test.toml to use test database",
            db_url
        );
    }

    Database::from_config(&config)
        .await
        .expect("Failed to connect to test database")
}

fn test_shard_info() -> ShardBlockInfo {
    ShardBlockInfo {
        shard_id: 1,
        block_height: 1000,
        transaction_fid: 99,
        timestamp: 1_698_765_432,
    }
}

fn test_message_hash(seed: u32) -> Vec<u8> {
    let mut hash = vec![0u8; 32];
    hash[0..4].copy_from_slice(&seed.to_be_bytes());
    hash
}

#[tokio::test]
#[ignore = "Requires database access - production database should not be modified"]
async fn test_batch_insert_reactions_parameter_binding() {
    let db = setup_test_db().await;
    let shard_info = test_shard_info();

    println!("🧪 Testing reactions batch insert parameter binding...");

    // Create multiple reactions to test batch insert
    let mut batched = BatchedData::new();

    for i in 0..5 {
        let hash = test_message_hash(9000 + i);
        batched.reactions.push((
            99 + i64::from(i),            // fid
            vec![0xAA + i as u8; 32],     // target_cast_hash (BYTEA)
            Some(100 + i64::from(i)),     // target_fid (Option<i64>)
            1,                            // reaction_type (i16)
            "add".to_string(),            // event_type (String)
            1_698_765_432 + i64::from(i), // timestamp (i64)
            hash.clone(),                 // message_hash (Vec<u8>)
            shard_info.clone(),           // shard_block_info
        ));
    }

    // This should succeed if parameter binding is correct
    let result = flush_batched_data(&db, batched).await;

    assert!(
        result.is_ok(),
        "Batch insert should succeed with correct parameter binding: {:?}",
        result.err()
    );

    // Verify all reactions were inserted
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM reactions WHERE fid >= 99 AND fid <= 103")
            .fetch_one(db.pool())
            .await
            .expect("Failed to count reactions");

    assert_eq!(count, 5, "All 5 reactions should be inserted");

    // Verify parameter values are correct (not shifted)
    let first_reaction: (i64, Vec<u8>, Option<i64>, i16, String) = sqlx::query_as(
        "SELECT fid, target_cast_hash, target_fid, reaction_type, event_type 
         FROM reactions 
         WHERE fid = 99 
         ORDER BY timestamp DESC 
         LIMIT 1",
    )
    .fetch_one(db.pool())
    .await
    .expect("Failed to query reaction");

    assert_eq!(first_reaction.0, 99, "FID should be 99");
    assert_eq!(
        first_reaction.1[0], 0xAA,
        "target_cast_hash should start with 0xAA"
    );
    assert_eq!(first_reaction.2, Some(100), "target_fid should be 100");
    assert_eq!(first_reaction.3, 1, "reaction_type should be 1");
    assert_eq!(first_reaction.4, "add", "event_type should be 'add'");

    // Cleanup
    sqlx::query("DELETE FROM reactions WHERE fid >= 99 AND fid <= 103")
        .execute(db.pool())
        .await
        .ok();

    println!("✅ Reactions batch insert parameter binding test passed");
}

#[tokio::test]
#[ignore = "Requires database access - production database should not be modified"]
async fn test_batch_insert_links_parameter_binding() {
    let db = setup_test_db().await;
    let shard_info = test_shard_info();

    println!("🧪 Testing links batch insert parameter binding...");

    let mut batched = BatchedData::new();
    batched.fids_to_ensure.insert(99);

    for i in 0..5 {
        let hash = test_message_hash(8000 + i);
        batched.links.push((
            99,                           // fid
            200 + i64::from(i),           // target_fid
            "follow".to_string(),         // link_type
            "add".to_string(),            // event_type
            1_698_765_432 + i64::from(i), // timestamp
            hash.clone(),                 // message_hash
            shard_info.clone(),           // shard_block_info
        ));
    }

    let result = flush_batched_data(&db, batched).await;

    assert!(
        result.is_ok(),
        "Batch insert should succeed: {:?}",
        result.err()
    );

    // Verify all links were inserted
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM links WHERE fid = 99 AND target_fid >= 200 AND target_fid <= 204",
    )
    .fetch_one(db.pool())
    .await
    .expect("Failed to count links");

    assert_eq!(count, 5, "All 5 links should be inserted");

    // Verify parameter values
    let first_link: (i64, i64, String, String) = sqlx::query_as(
        "SELECT fid, target_fid, link_type, event_type 
         FROM links 
         WHERE fid = 99 AND target_fid = 200",
    )
    .fetch_one(db.pool())
    .await
    .expect("Failed to query link");

    assert_eq!(first_link.0, 99, "FID should be 99");
    assert_eq!(first_link.1, 200, "target_fid should be 200");
    assert_eq!(first_link.2, "follow", "link_type should be 'follow'");
    assert_eq!(first_link.3, "add", "event_type should be 'add'");

    // Cleanup
    sqlx::query("DELETE FROM links WHERE fid = 99 AND target_fid >= 200 AND target_fid <= 204")
        .execute(db.pool())
        .await
        .ok();

    println!("✅ Links batch insert parameter binding test passed");
}

#[tokio::test]
#[ignore = "Requires database access - production database should not be modified"]
async fn test_batch_insert_verifications_parameter_binding() {
    let db = setup_test_db().await;
    let shard_info = test_shard_info();

    println!("🧪 Testing verifications batch insert parameter binding...");

    let mut batched = BatchedData::new();
    batched.fids_to_ensure.insert(99);

    for i in 0..5 {
        let hash = test_message_hash(7000 + i);
        batched.verifications.push((
            99,                           // fid
            vec![0xBB + i as u8; 20],     // address (BYTEA, 20 bytes)
            Some(vec![0xCC; 65]),         // claim_signature
            Some(vec![0xDD; 32]),         // block_hash
            Some(0),                      // verification_type
            Some(1),                      // chain_id
            "add".to_string(),            // event_type
            1_698_765_432 + i64::from(i), // timestamp
            hash.clone(),                 // message_hash
            shard_info.clone(),           // shard_block_info
        ));
    }

    let result = flush_batched_data(&db, batched).await;

    assert!(
        result.is_ok(),
        "Batch insert should succeed: {:?}",
        result.err()
    );

    // Verify all verifications were inserted
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM verifications WHERE fid = 99")
        .fetch_one(db.pool())
        .await
        .expect("Failed to count verifications");

    assert_eq!(count, 5, "All 5 verifications should be inserted");

    // Verify parameter values are correct (address should be BYTEA, not bigint)
    let first_verification: (i64, Vec<u8>, Option<i16>, String) = sqlx::query_as(
        "SELECT fid, address, verification_type, event_type 
         FROM verifications 
         WHERE fid = 99 
         ORDER BY timestamp DESC 
         LIMIT 1",
    )
    .fetch_one(db.pool())
    .await
    .expect("Failed to query verification");

    assert_eq!(first_verification.0, 99, "FID should be 99");
    assert_eq!(first_verification.1.len(), 20, "address should be 20 bytes");
    assert_eq!(
        first_verification.2,
        Some(0),
        "verification_type should be 0"
    );
    assert_eq!(first_verification.3, "add", "event_type should be 'add'");

    // Cleanup
    sqlx::query("DELETE FROM verifications WHERE fid = 99")
        .execute(db.pool())
        .await
        .ok();

    println!("✅ Verifications batch insert parameter binding test passed");
}

#[tokio::test]
#[ignore = "Requires database access - production database should not be modified"]
async fn test_large_batch_insert() {
    let db = setup_test_db().await;
    let shard_info = test_shard_info();

    println!("🧪 Testing large batch insert (100 records)...");

    let mut batched = BatchedData::new();
    batched.fids_to_ensure.insert(99);

    // Insert 100 reactions to test chunking logic
    for i in 0..100 {
        let hash = test_message_hash(6000 + i);
        batched.reactions.push((
            99,
            vec![0xAA; 32],
            Some(100),
            1,
            "add".to_string(),
            1_698_765_432 + i64::from(i),
            hash,
            shard_info.clone(),
        ));
    }

    let result = flush_batched_data(&db, batched).await;

    assert!(
        result.is_ok(),
        "Large batch insert should succeed: {:?}",
        result.err()
    );

    // Verify count
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reactions WHERE fid = 99 AND timestamp >= 1698765432 AND timestamp < 1698765532"
    )
    .fetch_one(db.pool())
    .await
    .expect("Failed to count");

    assert_eq!(count, 100, "All 100 reactions should be inserted");

    // Cleanup
    sqlx::query("DELETE FROM reactions WHERE fid = 99 AND timestamp >= 1698765432 AND timestamp < 1698765532")
        .execute(db.pool())
        .await
        .ok();

    println!("✅ Large batch insert test passed");
}
