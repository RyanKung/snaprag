//! Deterministic block testing system
//!
//! This test module provides a three-tier testing approach:
//! 1. Scanner Tool - Finds blocks containing specific message types
//! 2. Deterministic Assertions - Validates exact block contents
//! 3. Processing Verification - Validates database results after processing

use std::collections::HashMap;

use crate::config::AppConfig;
use crate::database::Database;
use crate::sync::client::proto;
use crate::sync::client::SnapchainClient;
use crate::sync::service::SyncService;
use crate::Result;

/// Known deterministic blocks with specific message types
/// These are discovered through scanning and validated through testing
#[derive(Debug, Clone)]
pub struct DeterministicBlock {
    pub block_number: u64,
    pub shard_id: u32,
    pub description: String,
    pub expected_transactions: usize,
    pub expected_message_types: HashMap<i32, usize>, // message_type -> count
    pub has_system_messages: bool,
}

impl DeterministicBlock {
    /// Create a new deterministic block definition
    pub fn new(block_number: u64, shard_id: u32, description: impl Into<String>) -> Self {
        Self {
            block_number,
            shard_id,
            description: description.into(),
            expected_transactions: 0,
            expected_message_types: HashMap::new(),
            has_system_messages: false,
        }
    }

    /// Add expected message type count
    pub fn with_message_type(mut self, message_type: i32, count: usize) -> Self {
        self.expected_message_types.insert(message_type, count);
        self
    }

    /// Set expected transaction count
    pub fn with_transactions(mut self, count: usize) -> Self {
        self.expected_transactions = count;
        self
    }

    /// Mark as having system messages
    pub fn with_system_messages(mut self) -> Self {
        self.has_system_messages = true;
        self
    }
}

/// Registry of known deterministic blocks for testing
pub struct DeterministicBlockRegistry {
    blocks: Vec<DeterministicBlock>,
}

impl DeterministicBlockRegistry {
    /// Create registry with known blocks
    /// Based on scan results from scan_message_types test
    pub fn new() -> Self {
        let mut blocks = vec![];

        // Block 1250000: First block with user messages
        // Discovered via scanning - contains multiple message types
        blocks.push(
            DeterministicBlock::new(1250000, 1, "First user messages block")
                .with_transactions(7)
                .with_message_type(1, 2) // CastAdd
                .with_message_type(3, 3) // ReactionAdd
                .with_message_type(5, 2) // LinkAdd
                .with_message_type(6, 2), // LinkRemove
        );

        // Block 1250300: Multiple CastAdd messages
        blocks.push(
            DeterministicBlock::new(1250300, 1, "Block with multiple casts")
                .with_transactions(8)
                .with_message_type(1, 5) // CastAdd
                .with_message_type(3, 3) // ReactionAdd
                .with_message_type(5, 1), // LinkAdd
        );

        // Block 1250500: Contains VerificationAdd
        blocks.push(
            DeterministicBlock::new(1250500, 1, "First VerificationAdd message")
                .with_transactions(5)
                .with_message_type(7, 1) // VerificationAdd
                .with_message_type(3, 3) // ReactionAdd
                .with_message_type(5, 2), // LinkAdd
        );

        // Block 1250800: First block with UserDataAdd
        blocks.push(
            DeterministicBlock::new(1250800, 1, "First UserDataAdd message")
                .with_transactions(6)
                .with_message_type(11, 1) // UserDataAdd
                .with_message_type(1, 1) // CastAdd
                .with_message_type(3, 3) // ReactionAdd
                .with_message_type(5, 1), // LinkAdd
        );

        // Block 1251400: First block with CastRemove
        blocks.push(
            DeterministicBlock::new(1251400, 1, "First CastRemove message").with_message_type(2, 1), // CastRemove
        );

        // Block 5009700: First block with ReactionRemove (from comprehensive scan)
        blocks.push(
            DeterministicBlock::new(5009700, 1, "First ReactionRemove message")
                .with_message_type(4, 1), // ReactionRemove
        );

        // Block 1319500: First VerificationRemove (very rare type!)
        blocks.push(
            DeterministicBlock::new(1319500, 1, "First VerificationRemove message")
                .with_transactions(8)
                .with_message_type(8, 1) // VerificationRemove
                .with_message_type(3, 3) // ReactionAdd
                .with_message_type(1, 2) // CastAdd
                .with_message_type(4, 1) // ReactionRemove
                .with_message_type(6, 1), // LinkRemove
        );

        // ========== SYSTEM MESSAGES (ON-CHAIN EVENTS) ==========

        // Block 1: Fname Transfers (very first block!)
        blocks.push(
            DeterministicBlock::new(1, 1, "Early Fname Transfers")
                .with_transactions(1000)
                .with_system_messages(),
        );

        // Block 10: First Storage Rent event
        blocks.push(
            DeterministicBlock::new(10, 1, "First Storage Rent")
                .with_transactions(1000)
                .with_system_messages(),
        );

        // Block 32900: First FID Registration and Signer events
        blocks.push(
            DeterministicBlock::new(32900, 1, "FID Registration and Signer events")
                .with_system_messages(),
        );

        // Note: Still missing very new/rare message types:
        // - Type 12: UsernameProof (not found in blocks 0-27M)
        // - Type 13: FrameAction (not found in blocks 0-27M)
        // - Type 14: LinkCompactState (not found in blocks 0-27M)
        // - Type 15: LendStorage (not found in blocks 0-27M)
        //
        // These may not be active yet or require specific conditions.
        // Types 9-10 (SignerAdd/Remove) are deprecated.
        //
        // Tools to continue scanning:
        // 1. cargo test scan_message_types -- --ignored --nocapture
        // 2. cargo run --bin scan_for_system_messages
        // 3. cargo run --bin scan_latest_blocks
        //
        // **Current Coverage:**
        // User Messages: 9/13 types (69% of active types)
        //   ✅ CastAdd(1), CastRemove(2), ReactionAdd(3), ReactionRemove(4)
        //   ✅ LinkAdd(5), LinkRemove(6), VerificationAdd(7), VerificationRemove(8)
        //   ✅ UserDataAdd(11)
        //   ⏳ UsernameProof(12), FrameAction(13), LinkCompactState(14), LendStorage(15)
        //   🚫 SignerAdd(9), SignerRemove(10) - Deprecated
        //
        // System Events: 3/3 common types (100%)
        //   ✅ Signer(1), FID Register(3), Storage Rent(4)
        //   ✅ Fname Transfers
        //
        // Total: 10 deterministic blocks covering 13 message/event types
        // Scanned ranges: Blocks 0 to 27,000,000

        Self { blocks }
    }

    /// Get all blocks
    pub fn blocks(&self) -> &[DeterministicBlock] {
        &self.blocks
    }

    /// Find blocks with specific message type
    pub fn find_by_message_type(&self, message_type: i32) -> Vec<&DeterministicBlock> {
        self.blocks
            .iter()
            .filter(|b| b.expected_message_types.contains_key(&message_type))
            .collect()
    }

    /// Find blocks with system messages
    pub fn find_with_system_messages(&self) -> Vec<&DeterministicBlock> {
        self.blocks
            .iter()
            .filter(|b| b.has_system_messages)
            .collect()
    }
}

// ==================== TIER 1: SCANNER TOOL ====================

/// Scan blocks to find those containing specific message types
/// This is run manually to discover new deterministic blocks
#[tokio::test]
#[ignore] // Run with: cargo test scan_message_types -- --ignored --nocapture
async fn scan_message_types() -> Result<()> {
    use crate::sync::client::SnapchainClient;

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(&config.sync.snapchain_grpc_endpoint).await?;

    println!("\n🔍 Scanning for Message Type Distribution");
    println!("==========================================\n");

    // Define scan ranges to find all message types
    // EXTENDED: Comprehensive scan including very large ranges for rare types
    let scan_ranges = vec![
        ("Genesis blocks (0-100)", 0, 100, 1), // Every block - system msgs
        ("Early blocks (100-1000)", 100, 1000, 10), // Frequent scan
        ("Early-mid (1000-10000)", 1000, 10000, 100), // Moderate scan
        ("Mid blocks (10000-100000)", 10000, 100000, 1000), // Sparse scan
        (
            "User activity start (1250000-1260000)",
            1250000,
            1260000,
            50,
        ), // Dense scan
        (
            "Extended user activity (1260000-1500000)",
            1260000,
            1500000,
            500,
        ), // Wide scan for Type 8
        ("Later activity (5000000-5020000)", 5000000, 5020000, 100), // ReactionRemove found here
        ("Extended later (5020000-6000000)", 5020000, 6000000, 1000), // More Type 8 search
        ("Very late (10000000-10020000)", 10000000, 10020000, 100),
        ("Recent blocks (15000000-15020000)", 15000000, 15020000, 100), // Frame, UsernameProof
        ("Latest blocks (20000000-20020000)", 20000000, 20020000, 100), // Very new features
        ("Cutting edge (25000000-25010000)", 25000000, 25010000, 100),  // Latest features
    ];

    let mut found_blocks: HashMap<i32, Vec<u64>> = HashMap::new();

    for (range_name, start, end, step) in scan_ranges {
        println!("📊 Scanning {}...", range_name);
        println!(
            "{:<12} {:<8} {:<8} {:<40}",
            "Block", "Txns", "Msgs", "Message Types"
        );
        println!("{}", "=".repeat(75));

        for block in (start..=end).step_by(step) {
            let request = proto::ShardChunksRequest {
                shard_id: 1,
                start_block_number: block,
                stop_block_number: Some(block + 1),
            };

            match client.get_shard_chunks(request).await {
                Ok(response) => {
                    if let Some(chunk) = response.shard_chunks.first() {
                        let tx_count = chunk.transactions.len();
                        let mut total_user_msgs = 0;
                        let mut total_system_msgs = 0;
                        let mut user_msg_types: HashMap<i32, usize> = HashMap::new();
                        let mut system_event_types: HashMap<i32, usize> = HashMap::new();

                        for tx in &chunk.transactions {
                            total_user_msgs += tx.user_messages.len();
                            total_system_msgs += tx.system_messages.len();

                            for msg in &tx.user_messages {
                                if let Some(data) = &msg.data {
                                    *user_msg_types.entry(data.r#type).or_insert(0) += 1;
                                    found_blocks
                                        .entry(data.r#type)
                                        .or_insert_with(Vec::new)
                                        .push(block);
                                }
                            }

                            for sys_msg in &tx.system_messages {
                                if let Some(event) = &sys_msg.on_chain_event {
                                    *system_event_types.entry(event.r#type).or_insert(0) += 1;
                                    found_blocks
                                        .entry(1000 + event.r#type)
                                        .or_insert_with(Vec::new)
                                        .push(block);
                                }
                            }
                        }

                        if total_user_msgs > 0 || total_system_msgs > 0 {
                            let user_types_str = if user_msg_types.is_empty() {
                                "none".to_string()
                            } else {
                                user_msg_types
                                    .iter()
                                    .map(|(t, c)| format!("U{}:{}", t, c))
                                    .collect::<Vec<_>>()
                                    .join(",")
                            };

                            let system_types_str = if system_event_types.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    " SYS:[{}]",
                                    system_event_types
                                        .iter()
                                        .map(|(t, c)| format!("{}:{}", t, c))
                                        .collect::<Vec<_>>()
                                        .join(",")
                                )
                            };

                            println!(
                                "{:<12} {:<8} {:<8} {}{}",
                                block,
                                tx_count,
                                total_user_msgs + total_system_msgs,
                                user_types_str,
                                system_types_str
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("Error at block {}: {}", block, e);
                }
            }
        }
        println!();
    }

    // Print summary
    println!("\n📋 Summary: Message Types Found");
    println!("================================");
    println!("User Message Types:");
    for (msg_type, blocks) in found_blocks.iter().filter(|(k, _)| **k < 100) {
        let msg_name = match msg_type {
            1 => "CastAdd",
            2 => "CastRemove",
            3 => "ReactionAdd",
            4 => "ReactionRemove",
            5 => "LinkAdd",
            6 => "LinkRemove",
            7 => "VerificationAdd",
            8 => "VerificationRemove",
            11 => "UserDataAdd",
            _ => "Unknown",
        };
        println!(
            "  Type {}: {} - Found in {} blocks (first: {})",
            msg_type,
            msg_name,
            blocks.len(),
            blocks.first().unwrap_or(&0)
        );
    }

    println!("\nSystem Event Types:");
    for (event_type, blocks) in found_blocks.iter().filter(|(k, _)| **k >= 1000) {
        let actual_type = event_type - 1000;
        let event_name = match actual_type {
            1 => "Signer",
            2 => "SignerMigrated",
            3 => "IdRegister",
            4 => "StorageRent",
            5 => "TierPurchase",
            _ => "Unknown",
        };
        println!(
            "  Type {}: {} - Found in {} blocks (first: {})",
            actual_type,
            event_name,
            blocks.len(),
            blocks.first().unwrap_or(&0)
        );
    }

    Ok(())
}

// ==================== TIER 2: DETERMINISTIC ASSERTIONS ====================

/// Test known deterministic blocks for exact content
#[tokio::test]
async fn test_deterministic_block_contents() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(&config.sync.snapchain_grpc_endpoint).await?;

    let registry = DeterministicBlockRegistry::new();

    println!("\n✅ Testing Deterministic Block Contents");
    println!("======================================\n");

    for det_block in registry.blocks() {
        println!(
            "Testing Block {}: {}",
            det_block.block_number, det_block.description
        );

        let request = proto::ShardChunksRequest {
            shard_id: det_block.shard_id,
            start_block_number: det_block.block_number,
            stop_block_number: Some(det_block.block_number + 1),
        };

        let response = client.get_shard_chunks(request).await?;
        assert_eq!(
            response.shard_chunks.len(),
            1,
            "Block {} should return exactly 1 chunk",
            det_block.block_number
        );

        let chunk = &response.shard_chunks[0];

        // Verify transaction count if specified
        if det_block.expected_transactions > 0 {
            assert_eq!(
                chunk.transactions.len(),
                det_block.expected_transactions,
                "Block {} should have exactly {} transactions",
                det_block.block_number,
                det_block.expected_transactions
            );
            println!("  ✓ Transactions: {}", chunk.transactions.len());
        }

        // Count actual message types
        let mut actual_msg_types: HashMap<i32, usize> = HashMap::new();
        let mut actual_system_msg_count = 0;

        for tx in &chunk.transactions {
            for msg in &tx.user_messages {
                if let Some(data) = &msg.data {
                    *actual_msg_types.entry(data.r#type).or_insert(0) += 1;
                }
            }
            actual_system_msg_count += tx.system_messages.len();
        }

        // Verify message types
        for (expected_type, expected_count) in &det_block.expected_message_types {
            let actual_count = actual_msg_types.get(expected_type).copied().unwrap_or(0);
            assert_eq!(
                actual_count, *expected_count,
                "Block {} should have exactly {} messages of type {}, but found {}",
                det_block.block_number, expected_count, expected_type, actual_count
            );
            let type_name = get_message_type_name(*expected_type);
            println!("  ✓ {}: {}", type_name, actual_count);
        }

        // Verify system messages if expected
        if det_block.has_system_messages {
            assert!(
                actual_system_msg_count > 0,
                "Block {} should have system messages",
                det_block.block_number
            );
            println!("  ✓ System messages: {}", actual_system_msg_count);
        }

        println!("  ✅ Block {} validated\n", det_block.block_number);
    }

    println!("✅ All deterministic blocks validated successfully!\n");

    Ok(())
}

// ==================== TIER 3: PROCESSING VERIFICATION ====================

/// Test that deterministic blocks are processed correctly and produce expected database state
#[tokio::test]
async fn test_deterministic_block_processing() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    println!("\n🧪 Testing Deterministic Block Processing");
    println!("=========================================\n");

    // Use internal helper which properly cleans between blocks
    process_and_verify_internal().await?;

    println!("✅ All deterministic blocks processed and verified successfully!\n");

    Ok(())
}

// ==================== TIER 4: COMPREHENSIVE INTEGRATION TEST ====================

/// Comprehensive test using all three tiers
/// Note: Run tier 1 (scan) manually first, then run tier 2 and 3 as unit tests
#[tokio::test]
async fn test_comprehensive_deterministic() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    println!("\n🧪 Comprehensive Deterministic Block Test");
    println!("==========================================\n");

    println!("ℹ️  This test validates that all deterministic blocks are processed correctly.");
    println!(
        "ℹ️  To discover new blocks, run: cargo test scan_message_types -- --ignored --nocapture\n"
    );

    // Tier 1: Validate block contents
    println!("1️⃣  Validating deterministic block contents...");
    validate_block_contents_internal().await?;
    println!("   ✅ Content validation passed\n");

    // Tier 2: Process and verify
    println!("2️⃣  Processing blocks and verifying database state...");
    process_and_verify_internal().await?;
    println!("   ✅ Processing verification passed\n");

    println!("🎉 All comprehensive tests passed!\n");

    Ok(())
}

/// Internal helper for block content validation
async fn validate_block_contents_internal() -> Result<()> {
    let config = AppConfig::load()?;
    let client = SnapchainClient::new(&config.sync.snapchain_grpc_endpoint).await?;

    let registry = DeterministicBlockRegistry::new();

    for det_block in registry.blocks() {
        let request = proto::ShardChunksRequest {
            shard_id: det_block.shard_id,
            start_block_number: det_block.block_number,
            stop_block_number: Some(det_block.block_number + 1),
        };

        let response = client.get_shard_chunks(request).await?;
        assert_eq!(response.shard_chunks.len(), 1);

        let chunk = &response.shard_chunks[0];

        // Count message types
        let mut actual_msg_types: HashMap<i32, usize> = HashMap::new();
        for tx in &chunk.transactions {
            for msg in &tx.user_messages {
                if let Some(data) = &msg.data {
                    *actual_msg_types.entry(data.r#type).or_insert(0) += 1;
                }
            }
        }

        // Verify expected types
        for (expected_type, expected_count) in &det_block.expected_message_types {
            let actual_count = actual_msg_types.get(expected_type).copied().unwrap_or(0);
            assert_eq!(actual_count, *expected_count);
        }
    }

    Ok(())
}

/// Internal helper for processing verification
async fn process_and_verify_internal() -> Result<()> {
    let config = AppConfig::load()?;
    let database = std::sync::Arc::new(Database::from_config(&config).await?);
    let registry = DeterministicBlockRegistry::new();
    let sync_service = SyncService::new(&config, database.clone()).await?;

    for det_block in registry.blocks() {
        println!(
            "  🧹 Cleaning database for block {}...",
            det_block.block_number
        );

        // Clean database before processing each block
        sqlx::query("TRUNCATE user_profiles, casts, user_activity_timeline CASCADE")
            .execute(database.pool())
            .await?;

        // Verify database is empty
        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_activity_timeline")
            .fetch_one(database.pool())
            .await?;
        assert_eq!(
            count_before, 0,
            "Database should be empty before processing"
        );

        println!("  🔄 Processing block {}...", det_block.block_number);

        // Process single block
        sync_service
            .poll_once(det_block.shard_id, det_block.block_number)
            .await?;

        // Verify casts (should match single block)
        if let Some(cast_add_count) = det_block.expected_message_types.get(&1) {
            let cast_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casts")
                .fetch_one(database.pool())
                .await?;
            assert_eq!(
                cast_count, *cast_add_count as i64,
                "Block {} should create {} casts (got {})",
                det_block.block_number, cast_add_count, cast_count
            );
            println!("    ✓ Casts: {} (expected: {})", cast_count, cast_add_count);
        }

        // Verify each expected message type created corresponding activities
        for (msg_type, expected_count) in &det_block.expected_message_types {
            let activity_type_name = match msg_type {
                1 => "cast_add",
                2 => "cast_remove",
                3 => "reaction_add",
                4 => "reaction_remove",
                5 => "link_add",
                6 => "link_remove",
                7 => "verification_add",
                8 => "verification_remove",
                11 => "user_data_add",
                _ => continue,
            };

            let actual_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_activity_timeline WHERE activity_type = $1",
            )
            .bind(activity_type_name)
            .fetch_one(database.pool())
            .await?;

            assert_eq!(
                actual_count, *expected_count as i64,
                "Block {} should produce {} {} activities (got {})",
                det_block.block_number, expected_count, activity_type_name, actual_count
            );
            println!(
                "    ✓ {}: {} (expected: {})",
                activity_type_name, actual_count, expected_count
            );
        }

        println!("  ✅ Block {} verified\n", det_block.block_number);
    }

    Ok(())
}

// ==================== HELPER FUNCTIONS ====================

fn get_message_type_name(msg_type: i32) -> &'static str {
    match msg_type {
        1 => "CastAdd",
        2 => "CastRemove",
        3 => "ReactionAdd",
        4 => "ReactionRemove",
        5 => "LinkAdd",
        6 => "LinkRemove",
        7 => "VerificationAdd",
        8 => "VerificationRemove",
        11 => "UserDataAdd",
        14 => "LinkBody",
        15 => "UsernameProof",
        16 => "FrameAction",
        _ => "Unknown",
    }
}

fn get_system_event_name(event_type: i32) -> &'static str {
    match event_type {
        1 => "Signer",
        2 => "SignerMigrated",
        3 => "IdRegister",
        4 => "StorageRent",
        5 => "TierPurchase",
        _ => "Unknown",
    }
}
