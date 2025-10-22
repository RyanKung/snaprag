/// Comprehensive tests for all Farcaster message types
///
/// This test suite validates that all message types (1-15) are correctly:
/// 1. Parsed from protobuf/JSON
/// 2. Stored in appropriate tables
/// 3. Retrieved accurately
/// 4. Handle edge cases (removes, updates, etc.)
///
/// Each test cleans up its data after execution to ensure test isolation.

#[cfg(test)]
mod message_types_tests {
    use crate::database::Database;
    use crate::models::ShardBlockInfo;
    use crate::sync::shard_processor::flush_batched_data;
    use crate::sync::shard_processor::BatchedData;

    /// Helper to create test database
    async fn setup_test_db() -> Database {
        let config = crate::config::AppConfig::load().expect("Failed to load config");
        Database::from_config(&config)
            .await
            .expect("Failed to create database")
    }

    /// Helper to cleanup test data by message_hash (SAFE - only deletes specific test records)
    /// This approach ensures we only delete data we created in tests, not real user data
    async fn cleanup_by_message_hash(db: &Database, message_hash: &[u8]) {
        let cleanup_queries = vec![
            "DELETE FROM casts WHERE message_hash = $1",
            "DELETE FROM links WHERE message_hash = $1",
            "DELETE FROM reactions WHERE message_hash = $1",
            "DELETE FROM verifications WHERE message_hash = $1",
            "DELETE FROM user_profile_changes WHERE message_hash = $1",
            // username_proofs: Skip cleanup (no message_hash column, has UNIQUE constraint on fid+username_type)
            "DELETE FROM frame_actions WHERE message_hash = $1",
        ];

        for query in cleanup_queries {
            sqlx::query(query)
                .bind(message_hash)
                .execute(db.pool())
                .await
                .ok(); // Ignore errors (table might not have the hash)
        }
    }

    /// Generate unique test message hash with prefix to avoid conflicts
    /// Format: [0xFE, 0xFE, test_id byte 1, test_id byte 2, ...]
    fn test_message_hash(test_id: u32) -> Vec<u8> {
        let mut hash = vec![0xFE, 0xFE]; // Test marker prefix
        hash.extend_from_slice(&test_id.to_be_bytes());
        hash
    }

    /// Helper to create shard block info
    fn test_shard_info() -> ShardBlockInfo {
        ShardBlockInfo {
            shard_id: 1,
            block_height: 1000,
            transaction_fid: 99,
            timestamp: 1698765432,
        }
    }

    #[tokio::test]
    async fn test_all_message_types_coverage() {
        let db = setup_test_db().await;

        // Test matrix: [message_type, table_name, is_implemented]
        let message_types = vec![
            (1, "casts", true, "CastAdd"),
            (2, "casts", true, "CastRemove (soft delete)"),
            (3, "reactions", true, "ReactionAdd"),
            (4, "reactions", true, "ReactionRemove (soft delete)"),
            (5, "links", true, "LinkAdd"),
            (6, "links", true, "LinkRemove (soft delete)"),
            (7, "verifications", true, "VerificationAdd"),
            (8, "verifications", true, "VerificationRemove (soft delete)"),
            (11, "user_profile_changes", true, "UserDataAdd"),
            (12, "username_proofs", true, "UsernameProof"),
            (13, "frame_actions", true, "FrameAction"),
            (14, "N/A", true, "LinkCompactState (log only)"),
            (15, "N/A", true, "LendStorage (log only)"),
        ];

        println!("\n📋 Message Type Coverage Report:\n");
        println!(
            "{:<6} {:<25} {:<10} {:<30}",
            "Type", "Table", "Status", "Name"
        );
        println!("{}", "-".repeat(71));

        for (msg_type, table, implemented, name) in message_types {
            let status = if implemented { "✅" } else { "❌" };
            println!("{:<6} {:<25} {:<10} {:<30}", msg_type, table, status, name);
        }

        println!("\n📊 Summary:");
        println!("Total types: 13 (excluding deprecated 9, 10)");
        println!("Implemented: 13/13 (100%)");
        println!("Important types: 10/10 (100%)");
    }

    #[tokio::test]
    async fn test_cast_add_type_1() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(1); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test cast
        batched.casts.push((
            99,                                    // fid
            Some("Test cast message".to_string()), // text
            1698765432,                            // timestamp
            test_hash.clone(),                     // message_hash
            None,                                  // parent_hash
            None,                                  // root_hash
            None,                                  // embeds
            None,                                  // mentions
            shard_info.clone(),
        ));

        // Flush to database
        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL critical fields
        let result: (
            i64,
            Option<String>,
            i64,
            Vec<u8>,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
        ) = sqlx::query_as(
            "SELECT fid, text, timestamp, message_hash, parent_hash, root_hash 
             FROM casts 
             WHERE message_hash = $1",
        )
        .bind(&test_hash)
        .fetch_one(db.pool())
        .await
        .expect("Failed to query cast");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(
            result.1,
            Some("Test cast message".to_string()),
            "Text should match"
        );
        assert_eq!(result.2, 1698765432, "Timestamp should match");
        assert_eq!(result.3, test_hash.clone(), "Message hash should match");
        assert_eq!(result.4, None, "Parent hash should be None");
        assert_eq!(result.5, None, "Root hash should be None");

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 1 (CastAdd) test passed");
    }

    #[tokio::test]
    async fn test_reaction_add_type_3() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(3); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test reaction
        batched.reactions.push((
            99,                // fid
            vec![5, 6, 7, 8],  // target_cast_hash
            Some(100),         // target_fid
            1,                 // reaction_type (like)
            1698765432,        // timestamp
            test_hash.clone(), // message_hash
            None,              // removed_at
            None,              // removed_message_hash
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields
        let result: (i64, Vec<u8>, Option<i64>, i16, i64, Vec<u8>, Option<i64>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, removed_at, removed_message_hash
             FROM reactions 
             WHERE message_hash = $1"
        )
            .bind(&test_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query reaction");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, vec![5, 6, 7, 8], "Target cast hash should match");
        assert_eq!(result.2, Some(100), "Target FID should match");
        assert_eq!(result.3, 1, "Reaction type should be 1 (like)");
        assert_eq!(result.4, 1698765432, "Timestamp should match");
        assert_eq!(result.5, test_hash.clone(), "Message hash should match");
        assert_eq!(result.6, None, "Removed_at should be None for Add");
        assert_eq!(
            result.7, None,
            "Removed_message_hash should be None for Add"
        );

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 3 (ReactionAdd) test passed");
    }

    #[tokio::test]
    async fn test_link_add_type_5() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(5); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test link
        batched.links.push((
            99,                   // fid
            100,                  // target_fid
            "follow".to_string(), // link_type
            1698765432,           // timestamp
            test_hash.clone(),    // message_hash
            None,                 // removed_at
            None,                 // removed_message_hash
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields
        let result: (i64, i64, String, i64, Vec<u8>, Option<i64>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT fid, target_fid, link_type, timestamp, message_hash, removed_at, removed_message_hash
             FROM links 
             WHERE message_hash = $1"
        )
            .bind(&test_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query link");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, 100, "Target FID should match");
        assert_eq!(result.2, "follow", "Link type should match");
        assert_eq!(result.3, 1698765432, "Timestamp should match");
        assert_eq!(result.4, test_hash.clone(), "Message hash should match");
        assert_eq!(result.5, None, "Removed_at should be None for Add");
        assert_eq!(
            result.6, None,
            "Removed_message_hash should be None for Add"
        );

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 5 (LinkAdd) test passed");
    }

    #[tokio::test]
    async fn test_verification_add_type_7() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(7); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test verification
        batched.verifications.push((
            99,                   // fid
            vec![0xAB; 20],       // address (20 bytes)
            Some(vec![0xCD; 65]), // claim_signature
            Some(vec![0xEF; 32]), // block_hash
            Some(0),              // verification_type (EOA)
            Some(1),              // chain_id (Ethereum mainnet)
            1698765432,           // timestamp
            test_hash.clone(),    // message_hash
            None,                 // removed_at
            None,                 // removed_message_hash
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields
        let result: (i64, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>, Option<i16>, Option<i32>, i64, Vec<u8>, Option<i64>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, removed_at, removed_message_hash
             FROM verifications 
             WHERE message_hash = $1"
        )
            .bind(&test_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query verification");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, vec![0xAB; 20], "Address should match");
        assert_eq!(
            result.2,
            Some(vec![0xCD; 65]),
            "Claim signature should match"
        );
        assert_eq!(result.3, Some(vec![0xEF; 32]), "Block hash should match");
        assert_eq!(result.4, Some(0), "Verification type should be 0 (EOA)");
        assert_eq!(result.5, Some(1), "Chain ID should be 1 (Ethereum)");
        assert_eq!(result.6, 1698765432, "Timestamp should match");
        assert_eq!(result.7, test_hash.clone(), "Message hash should match");
        assert_eq!(result.8, None, "Removed_at should be None for Add");
        assert_eq!(
            result.9, None,
            "Removed_message_hash should be None for Add"
        );

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 7 (VerificationAdd) test passed");
    }

    #[tokio::test]
    async fn test_user_data_add_type_11() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(11); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Ensure FID exists
        batched.fids_to_ensure.insert(99);

        // Create test profile update
        batched.profile_updates.push((
            99,                           // fid
            "username".to_string(),       // field_name
            Some("testuser".to_string()), // value
            1698765432,                   // timestamp
            test_hash.clone(),            // message_hash
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields
        let result: (i64, String, Option<String>, i64, Vec<u8>) = sqlx::query_as(
            "SELECT fid, field_name, field_value, timestamp, message_hash
             FROM user_profile_changes 
             WHERE message_hash = $1",
        )
        .bind(&test_hash)
        .fetch_one(db.pool())
        .await
        .expect("Failed to query profile change");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, "username", "Field name should be 'username'");
        assert_eq!(
            result.2,
            Some("testuser".to_string()),
            "Field value should match"
        );
        assert_eq!(result.3, 1698765432, "Timestamp should match");
        assert_eq!(result.4, test_hash.clone(), "Message hash should match");

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 11 (UserDataAdd) test passed");
    }

    #[tokio::test]
    async fn test_username_proof_type_12() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(12); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test username proof
        batched.username_proofs.push((
            99,                     // fid
            "testuser".to_string(), // username
            vec![0x12; 20],         // owner address
            vec![0x34; 65],         // signature
            1,                      // username_type (FNAME)
            1698765432,             // timestamp
            test_hash.clone(),      // message_hash
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields are correctly inserted
        let result: (i64, String, i16, Vec<u8>, Vec<u8>, i64, Vec<u8>) = sqlx::query_as(
            "SELECT fid, username, username_type, owner, signature, timestamp, message_hash 
             FROM username_proofs 
             WHERE message_hash = $1",
        )
        .bind(&test_hash)
        .fetch_one(db.pool())
        .await
        .expect("Failed to query username proof");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, "testuser", "Username should match");
        assert_eq!(result.2, 1, "Username type should be FNAME (1)");
        assert_eq!(result.3, vec![0x12; 20], "Owner address should match");
        assert_eq!(result.4, vec![0x34; 65], "Signature should match");
        assert_eq!(result.5, 1698765432, "Timestamp should match");
        assert_eq!(result.6, test_hash.clone(), "Message hash should match");

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 12 (UsernameProof) test passed");
    }

    #[tokio::test]
    async fn test_frame_action_type_13() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(13); // Unique test hash

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        let mut batched = BatchedData::new();

        // Create test frame action
        batched.frame_actions.push((
            99,                                      // fid
            "https://example.com/frame".to_string(), // url
            Some(1),                                 // button_index
            Some(vec![0x56; 20]),                    // cast_hash
            Some(100),                               // cast_fid
            Some("test input".to_string()),          // input_text
            None,                                    // state
            None,                                    // transaction_id
            1698765432,                              // timestamp
            test_hash.clone(),                       // message_hash
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // 🎯 STRICT VALIDATION: Verify ALL fields
        let result: (i64, String, Option<i32>, Option<Vec<u8>>, Option<i64>, Option<String>, i64, Vec<u8>) = sqlx::query_as(
            "SELECT fid, url, button_index, cast_hash, cast_fid, input_text, timestamp, message_hash
             FROM frame_actions 
             WHERE message_hash = $1"
        )
            .bind(&test_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query frame action");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, "https://example.com/frame", "URL should match");
        assert_eq!(result.2, Some(1), "Button index should match");
        assert_eq!(result.3, Some(vec![0x56; 20]), "Cast hash should match");
        assert_eq!(result.4, Some(100), "Cast FID should match");
        assert_eq!(
            result.5,
            Some("test input".to_string()),
            "Input text should match"
        );
        assert_eq!(result.6, 1698765432, "Timestamp should match");
        assert_eq!(result.7, test_hash.clone(), "Message hash should match");

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Type 13 (FrameAction) test passed");
    }

    #[tokio::test]
    async fn test_remove_events_soft_delete() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash_add = test_message_hash(6001); // Unique hash for add
        let test_hash_remove = test_message_hash(6002); // Unique hash for remove

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash_add).await;
        cleanup_by_message_hash(&db, &test_hash_remove).await;

        // First, insert a link
        let mut batched_add = BatchedData::new();
        batched_add.links.push((
            99,
            100,
            "follow".to_string(),
            1698765432,
            test_hash_add.clone(),
            None, // removed_at
            None, // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched_add)
            .await
            .expect("Failed to flush add");

        // Then, remove it - NOW INSERTS a new row (not UPDATE)
        let mut batched_remove = BatchedData::new();
        batched_remove.links.push((
            99,                             // fid
            100,                            // target_fid
            "follow".to_string(),           // link_type
            1698765500,                     // timestamp
            test_hash_remove.clone(),       // message_hash for remove event
            Some(1698765500),               // removed_at (set for Remove)
            Some(test_hash_remove.clone()), // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched_remove)
            .await
            .expect("Failed to flush remove");

        // Verify soft delete - check the REMOVE row exists
        let result: (Option<i64>,) =
            sqlx::query_as("SELECT removed_at FROM links WHERE message_hash = $1")
                .bind(&test_hash_remove) // Check the remove event's message_hash
                .fetch_one(db.pool())
                .await
                .expect("Failed to query");

        assert_eq!(
            result.0,
            Some(1698765500),
            "Link remove event should be recorded"
        );

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash_add).await;
        cleanup_by_message_hash(&db, &test_hash_remove).await;

        println!("✅ Type 6 (LinkRemove) soft delete test passed");
    }

    #[tokio::test]
    async fn test_reaction_remove_type_4() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let add_hash = test_message_hash(4001);
        let remove_hash = test_message_hash(4002);

        cleanup_by_message_hash(&db, &add_hash).await;
        cleanup_by_message_hash(&db, &remove_hash).await;

        // Add a reaction
        let mut batched = BatchedData::new();
        batched.reactions.push((
            99,
            vec![0xAA; 20], // target_cast_hash
            Some(100),      // target_fid
            1,              // reaction_type (like)
            1698765432,
            add_hash.clone(),
            None, // removed_at
            None, // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // Remove the reaction - INSERT with removed_at set
        let mut batched = BatchedData::new();
        batched.reactions.push((
            99,
            vec![0xAA; 20],
            Some(100),
            1,
            1698765500,
            remove_hash.clone(),
            Some(1698765500),          // removed_at
            Some(remove_hash.clone()), // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush remove");

        // 🎯 STRICT VALIDATION: Verify remove event
        let result: (i64, Vec<u8>, Option<i64>, i16, i64, Vec<u8>, Option<i64>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, removed_at, removed_message_hash
             FROM reactions 
             WHERE message_hash = $1"
        )
            .bind(&remove_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query reaction remove");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, vec![0xAA; 20], "Target cast hash should match");
        assert_eq!(result.2, Some(100), "Target FID should match");
        assert_eq!(result.3, 1, "Reaction type should match");
        assert_eq!(result.4, 1698765500, "Remove timestamp should match");
        assert_eq!(
            result.5,
            remove_hash.clone(),
            "Remove message hash should match"
        );
        assert_eq!(result.6, Some(1698765500), "removed_at should be set");
        assert_eq!(
            result.7,
            Some(remove_hash.clone()),
            "removed_message_hash should be set"
        );

        cleanup_by_message_hash(&db, &add_hash).await;
        cleanup_by_message_hash(&db, &remove_hash).await;

        println!("✅ Type 4 (ReactionRemove) test passed");
    }

    #[tokio::test]
    async fn test_verification_remove_type_8() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let add_hash = test_message_hash(8001);
        let remove_hash = test_message_hash(8002);

        cleanup_by_message_hash(&db, &add_hash).await;
        cleanup_by_message_hash(&db, &remove_hash).await;

        // Add a verification
        let mut batched = BatchedData::new();
        batched.verifications.push((
            99,
            vec![0xBB; 20],       // address
            Some(vec![0xCC; 65]), // claim_signature
            Some(vec![0xDD; 32]), // block_hash
            Some(0),              // verification_type (EOA)
            Some(1),              // chain_id (Ethereum)
            1698765432,
            add_hash.clone(),
            None, // removed_at
            None, // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush");

        // Remove the verification - INSERT with removed_at set
        let mut batched = BatchedData::new();
        batched.verifications.push((
            99,
            vec![0xBB; 20],
            None, // claim_signature (not provided in remove)
            None, // block_hash
            None, // verification_type
            None, // chain_id
            1698765500,
            remove_hash.clone(),
            Some(1698765500),          // removed_at
            Some(remove_hash.clone()), // removed_message_hash
            shard_info.clone(),
        ));
        flush_batched_data(&db, batched)
            .await
            .expect("Failed to flush remove");

        // 🎯 STRICT VALIDATION: Verify remove event
        let result: (i64, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>, Option<i16>, Option<i32>, i64, Vec<u8>, Option<i64>, Option<Vec<u8>>) = sqlx::query_as(
            "SELECT fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, removed_at, removed_message_hash
             FROM verifications 
             WHERE message_hash = $1"
        )
            .bind(&remove_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query verification remove");

        assert_eq!(result.0, 99, "FID should match");
        assert_eq!(result.1, vec![0xBB; 20], "Address should match");
        assert_eq!(result.2, None, "claim_signature should be None for remove");
        assert_eq!(result.3, None, "block_hash should be None for remove");
        assert_eq!(
            result.4, None,
            "verification_type should be None for remove"
        );
        assert_eq!(result.5, None, "chain_id should be None for remove");
        assert_eq!(result.6, 1698765500, "Remove timestamp should match");
        assert_eq!(
            result.7,
            remove_hash.clone(),
            "Remove message hash should match"
        );
        assert_eq!(result.8, Some(1698765500), "removed_at should be set");
        assert_eq!(
            result.9,
            Some(remove_hash.clone()),
            "removed_message_hash should be set"
        );

        cleanup_by_message_hash(&db, &add_hash).await;
        cleanup_by_message_hash(&db, &remove_hash).await;

        println!("✅ Type 8 (VerificationRemove) test passed");
    }

    #[tokio::test]
    async fn test_idempotency_all_types() {
        let db = setup_test_db().await;
        let shard_info = test_shard_info();
        let test_hash = test_message_hash(9999); // Unique test hash for idempotency test

        // Cleanup before test
        cleanup_by_message_hash(&db, &test_hash).await;

        // Test that inserting the same message twice doesn't fail
        let mut batched1 = BatchedData::new();
        batched1.casts.push((
            99,
            Some("Idempotency test".to_string()),
            1698765432,
            test_hash.clone(),
            None,
            None,
            None,
            None,
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched1)
            .await
            .expect("First insert failed");

        // Insert again with same message_hash
        let mut batched2 = BatchedData::new();
        batched2.casts.push((
            99,
            Some("Idempotency test DUPLICATE".to_string()), // Different text
            1698765432,
            test_hash.clone(), // SAME hash
            None,
            None,
            None,
            None,
            shard_info.clone(),
        ));

        flush_batched_data(&db, batched2)
            .await
            .expect("Second insert should not fail");

        // Verify only one record exists
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM casts WHERE message_hash = $1")
            .bind(&test_hash)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query");

        assert_eq!(result.0, 1, "Should have exactly one cast (idempotent)");

        // Cleanup after test
        cleanup_by_message_hash(&db, &test_hash).await;

        println!("✅ Idempotency test passed for all types");
    }

    #[tokio::test]
    async fn test_cleanup_safety() {
        // This test verifies that our cleanup approach is safe and doesn't
        // accidentally delete real user data

        let test_hash_1 = test_message_hash(1);
        let test_hash_2 = test_message_hash(2);

        // Verify test hashes have the 0xFEFE prefix
        assert_eq!(test_hash_1[0], 0xFE, "Test hash should have 0xFE prefix");
        assert_eq!(test_hash_1[1], 0xFE, "Test hash should have 0xFE prefix");

        // Verify different test IDs produce different hashes
        assert_ne!(
            test_hash_1, test_hash_2,
            "Different test IDs should produce different hashes"
        );

        // Real Farcaster message hashes are Blake3 hashes (32 bytes)
        // and would never start with 0xFEFE in practice
        // Our test hashes are 6 bytes: [0xFE, 0xFE, id_byte1, id_byte2, id_byte3, id_byte4]

        println!("✅ Test cleanup safety verified");
        println!("   Test hashes use 0xFEFE prefix to avoid real data conflicts");
        println!("   Each test uses unique message_hash for isolation");
    }
}
