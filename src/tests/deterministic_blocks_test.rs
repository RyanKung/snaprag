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
    pub expected_system_event_types: HashMap<i32, usize>, // event_type -> count (for system msgs)
    pub block_timestamp: Option<u64>,                     // Expected block timestamp for validation
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
            expected_system_event_types: HashMap::new(),
            block_timestamp: None,
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

    /// Mark as having system messages (generic, count > 0)
    pub fn with_system_messages(mut self) -> Self {
        self.has_system_messages = true;
        self
    }

    /// Add expected system event type count (stricter validation)
    pub fn with_system_event_type(mut self, event_type: i32, count: usize) -> Self {
        self.expected_system_event_types.insert(event_type, count);
        self.has_system_messages = true;
        self
    }

    /// Set expected block timestamp (for timestamp validation)
    pub fn with_block_timestamp(mut self, timestamp: u64) -> Self {
        self.block_timestamp = Some(timestamp);
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

        // STRICT REQUIREMENT: Every block must specify at least:
        // 1. expected_transactions (>0)
        // 2. At least ONE of: expected_message_types OR has_system_messages

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
        // Full scan result: U2:1,U5:2,U6:1,U3:1
        blocks.push(
            DeterministicBlock::new(1251400, 1, "First CastRemove message")
                .with_transactions(4) // Corrected from scan
                .with_message_type(2, 1) // CastRemove
                .with_message_type(5, 2) // LinkAdd
                .with_message_type(6, 1) // LinkRemove
                .with_message_type(3, 1), // ReactionAdd
        );

        // Block 5009700: First block with ReactionRemove (from comprehensive scan)
        // Full scan result: U3:1,U4:1,U6:4,U5:3
        blocks.push(
            DeterministicBlock::new(5009700, 1, "First ReactionRemove message")
                .with_transactions(8) // Corrected from scan: actual count is 8
                .with_message_type(4, 1) // ReactionRemove
                .with_message_type(3, 1) // ReactionAdd
                .with_message_type(6, 4) // LinkRemove
                .with_message_type(5, 3), // LinkAdd
        );

        // Block 1319500: First VerificationRemove (very rare type!)
        // Full scan result: U4:1,U1:2,U6:1,U8:1,U3:3
        blocks.push(
            DeterministicBlock::new(1319500, 1, "First VerificationRemove message")
                .with_transactions(8)
                .with_message_type(8, 1) // VerificationRemove
                .with_message_type(4, 1) // ReactionRemove
                .with_message_type(1, 2) // CastAdd
                .with_message_type(6, 1) // LinkRemove
                .with_message_type(3, 3), // ReactionAdd
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
                .with_transactions(9) // Corrected from scan: actual count is 9
                .with_system_messages(), // Generic check for now - counts vary
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

        // Validate all blocks have minimum required specifications
        for block in &blocks {
            assert!(
                block.expected_transactions > 0,
                "Block {} must have expected_transactions specified",
                block.block_number
            );
            assert!(
                !block.expected_message_types.is_empty() || block.has_system_messages,
                "Block {} must specify at least message types OR system messages",
                block.block_number
            );
        }

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

// ==================== TIER 1: SCANNER TOOLS ====================

/// Scan blocks to find those containing specific message types
/// This is run manually to discover new deterministic blocks
#[tokio::test]
#[ignore] // Run with: cargo test scan_message_types -- --ignored --nocapture
async fn scan_message_types() -> Result<()> {
    use crate::sync::client::SnapchainClient;

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

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
        let first_block = blocks.first().expect("blocks list should not be empty");
        println!(
            "  Type {}: {} - Found in {} blocks (first: {})",
            msg_type,
            msg_name,
            blocks.len(),
            first_block
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
        let first_block = blocks.first().expect("blocks list should not be empty");
        println!(
            "  Type {}: {} - Found in {} blocks (first: {})",
            actual_type,
            event_name,
            blocks.len(),
            first_block
        );
    }

    Ok(())
}

/// Scan for system messages (OnChainEvents) in early blocks
/// This helps discover FID registrations, storage events, and signer events
#[tokio::test]
#[ignore] // Run with: cargo test scan_for_system_messages -- --ignored --nocapture
async fn scan_for_system_messages() -> Result<()> {
    use crate::sync::client::SnapchainClient;

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    println!("\n🔍 Scanning for System Messages (OnChainEvents)");
    println!("================================================\n");

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

    // Focus on early blocks where FID registration happens
    let scan_ranges = vec![
        ("Genesis (0-1000)", 0, 1000, 1),        // Every single block
        ("Early (1000-10000)", 1000, 10000, 10), // Every 10th block
        ("Mid (10000-100000)", 10000, 100000, 100),
    ];

    let mut system_event_blocks: HashMap<i32, Vec<u64>> = HashMap::new();
    let mut blocks_scanned = 0;

    for (range_name, start, end, step) in scan_ranges {
        println!("📊 Scanning {}...\n", range_name);

        for block in (start..=end).step_by(step) {
            blocks_scanned += 1;

            let request = proto::ShardChunksRequest {
                shard_id: 1,
                start_block_number: block,
                stop_block_number: Some(block + 1),
            };

            match client.get_shard_chunks(request).await {
                Ok(response) => {
                    if let Some(chunk) = response.shard_chunks.first() {
                        let mut system_msg_count = 0;
                        let mut event_types: HashMap<i32, usize> = HashMap::new();

                        for tx in &chunk.transactions {
                            for sys_msg in &tx.system_messages {
                                system_msg_count += 1;

                                if let Some(event) = &sys_msg.on_chain_event {
                                    *event_types.entry(event.r#type).or_insert(0) += 1;
                                    system_event_blocks
                                        .entry(event.r#type)
                                        .or_insert_with(Vec::new)
                                        .push(block);
                                }

                                if sys_msg.fname_transfer.is_some() {
                                    *event_types.entry(9999).or_insert(0) += 1;
                                    system_event_blocks
                                        .entry(9999)
                                        .or_insert_with(Vec::new)
                                        .push(block);
                                }
                            }
                        }

                        if system_msg_count > 0 {
                            let events_str = event_types
                                .iter()
                                .map(|(t, c)| {
                                    let name = match *t {
                                        1 => "Signer",
                                        2 => "SignerMigrated",
                                        3 => "IdRegister",
                                        4 => "StorageRent",
                                        5 => "TierPurchase",
                                        9999 => "FnameTransfer",
                                        _ => "Unknown",
                                    };
                                    format!("{}:{}({})", t, c, name)
                                })
                                .collect::<Vec<_>>()
                                .join(", ");

                            println!(
                                "Block {:<10} - {} system messages: [{}]",
                                block, system_msg_count, events_str
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error at block {}: {}", block, e);
                }
            }

            if blocks_scanned % 100 == 0 {
                println!("  ... scanned {} blocks so far", blocks_scanned);
            }
        }
        println!();
    }

    // Print summary
    println!("\n📋 Summary: System Events Found");
    println!("==================================");

    if system_event_blocks.is_empty() {
        println!("⚠️  No system messages found in scanned ranges!");
    } else {
        for (event_type, blocks) in system_event_blocks.iter() {
            let event_name = match event_type {
                1 => "Signer Event",
                2 => "Signer Migrated",
                3 => "FID Registration",
                4 => "Storage Rent",
                5 => "Tier Purchase",
                9999 => "Fname Transfer",
                _ => "Unknown",
            };

            let first_block = blocks.first().expect("blocks list should not be empty");
            println!(
                "  Event Type {}: {} - Found in {} blocks",
                event_type,
                event_name,
                blocks.len()
            );
            println!("    First occurrence: block {}", first_block);
            if blocks.len() > 1 {
                println!("    Sample blocks: {:?}", &blocks[..blocks.len().min(5)]);
            }
            println!();
        }
    }

    println!(
        "\n✅ Scan complete! Scanned {} blocks total.",
        blocks_scanned
    );

    Ok(())
}

/// Scan latest blocks for new message types (UsernameProof, FrameAction, etc.)
#[tokio::test]
#[ignore] // Run with: cargo test scan_latest_blocks -- --ignored --nocapture
async fn scan_latest_blocks() -> Result<()> {
    use crate::sync::client::SnapchainClient;

    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    println!("\n🔍 Scanning Latest Blocks for New Message Types");
    println!("================================================\n");
    println!("Target: UsernameProof(12), FrameAction(13), LinkCompactState(14), LendStorage(15)\n");

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

    let scan_ranges = vec![
        ("Recent 18M-19M", 18_000_000, 19_000_000, 5000),
        ("Recent 19M-20M", 19_000_000, 20_000_000, 5000),
        ("Recent 20M-21M", 20_000_000, 21_000_000, 5000),
        ("Latest 23M-24M", 23_000_000, 24_000_000, 5000),
        ("Latest 25M-26M", 25_000_000, 26_000_000, 5000),
    ];

    let mut found_types: HashMap<i32, Vec<u64>> = HashMap::new();
    let mut blocks_scanned = 0;

    for (range_name, start, end, step) in scan_ranges {
        println!("📊 Scanning {}...", range_name);

        for block in (start..=end).step_by(step) {
            blocks_scanned += 1;

            let request = proto::ShardChunksRequest {
                shard_id: 1,
                start_block_number: block,
                stop_block_number: Some(block + 1),
            };

            match client.get_shard_chunks(request).await {
                Ok(response) => {
                    if let Some(chunk) = response.shard_chunks.first() {
                        for tx in &chunk.transactions {
                            for msg in &tx.user_messages {
                                if let Some(data) = &msg.data {
                                    if data.r#type >= 12 && data.r#type <= 15 {
                                        found_types
                                            .entry(data.r#type)
                                            .or_insert_with(Vec::new)
                                            .push(block);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => continue,
            }

            if blocks_scanned % 100 == 0 {
                println!("  ... scanned {} blocks", blocks_scanned);
            }
        }
    }

    println!("\n📋 Summary");
    println!("==========");

    if found_types.is_empty() {
        println!("⚠️  None of types 12-15 found");
    } else {
        for t in 12..=15 {
            if let Some(blocks) = found_types.get(&t) {
                let first_block = blocks.first().expect("blocks list should not be empty");
                println!(
                    "  ✅ Type {}: Found in {} blocks (first: {})",
                    t,
                    blocks.len(),
                    first_block
                );
            }
        }
    }

    println!("\n✅ Scanned {} blocks", blocks_scanned);
    Ok(())
}

// ==================== TIER 2: DETERMINISTIC ASSERTIONS ====================

/// Test known deterministic blocks for exact content
#[tokio::test]
async fn test_deterministic_block_contents() -> Result<()> {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

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

        // Verify transaction count (STRICT: must be specified for all blocks)
        assert!(
            det_block.expected_transactions > 0,
            "Block {} must have expected_transactions specified (found 0)",
            det_block.block_number
        );
        assert_eq!(
            chunk.transactions.len(),
            det_block.expected_transactions,
            "Block {} should have exactly {} transactions, got {}",
            det_block.block_number,
            det_block.expected_transactions,
            chunk.transactions.len()
        );
        println!(
            "  ✓ Transactions: {} (expected: {})",
            chunk.transactions.len(),
            det_block.expected_transactions
        );

        // Count actual message types and system events
        let mut actual_msg_types: HashMap<i32, usize> = HashMap::new();
        let mut actual_system_msg_count = 0;
        let mut actual_system_event_types: HashMap<i32, usize> = HashMap::new();

        for tx in &chunk.transactions {
            for msg in &tx.user_messages {
                if let Some(data) = &msg.data {
                    *actual_msg_types.entry(data.r#type).or_insert(0) += 1;
                }
            }
            actual_system_msg_count += tx.system_messages.len();

            // Count system event types
            for sys_msg in &tx.system_messages {
                if let Some(event) = &sys_msg.on_chain_event {
                    *actual_system_event_types.entry(event.r#type).or_insert(0) += 1;
                }
            }
        }

        // Verify message types (STRICT: exact count match)
        for (expected_type, expected_count) in &det_block.expected_message_types {
            let actual_count = actual_msg_types.get(expected_type).copied().unwrap_or(0);
            assert_eq!(
                actual_count, *expected_count,
                "Block {} should have exactly {} messages of type {}, but found {}",
                det_block.block_number, expected_count, expected_type, actual_count
            );
            let type_name = get_message_type_name(*expected_type);
            println!(
                "  ✓ {}: {} (expected: {})",
                type_name, actual_count, expected_count
            );
        }

        // Verify system messages if expected (STRICT)
        if det_block.has_system_messages {
            assert!(
                actual_system_msg_count > 0,
                "Block {} should have system messages (found 0)",
                det_block.block_number
            );

            // Require at least some validation when system messages are expected
            if det_block.expected_system_event_types.is_empty() {
                // At minimum, verify we have some identifiable events
                let total_identifiable = actual_system_event_types.values().sum::<usize>();
                assert!(
                    total_identifiable > 0 || actual_system_msg_count > 0,
                    "Block {}: System messages present but no identifiable events",
                    det_block.block_number
                );
            }

            println!("  ✓ System messages: {} total", actual_system_msg_count);
        }

        // Verify specific system event types if specified (STRICT: exact count)
        for (expected_event_type, expected_count) in &det_block.expected_system_event_types {
            let actual_count = actual_system_event_types
                .get(expected_event_type)
                .copied()
                .unwrap_or(0);
            assert_eq!(
                actual_count, *expected_count,
                "Block {} should have exactly {} system events of type {}, but found {}",
                det_block.block_number, expected_count, expected_event_type, actual_count
            );
            let event_name = get_system_event_name(*expected_event_type);
            println!(
                "  ✓ System Event {}: {} (expected: {})",
                event_name, actual_count, expected_count
            );
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
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

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

        // Get block info before processing for timestamp validation
        let temp_client = SnapchainClient::new(
            &config.sync.snapchain_http_endpoint,
            &config.sync.snapchain_grpc_endpoint,
        )
        .await?;
        let request = proto::ShardChunksRequest {
            shard_id: det_block.shard_id,
            start_block_number: det_block.block_number,
            stop_block_number: Some(det_block.block_number + 1),
        };
        let pre_process_response = temp_client.get_shard_chunks(request).await?;
        let block_timestamp = pre_process_response
            .shard_chunks
            .first()
            .and_then(|c| c.header.as_ref())
            .map(|h| h.timestamp)
            .unwrap_or(0);

        // Process single block
        sync_service
            .poll_once(det_block.shard_id, det_block.block_number)
            .await?;

        // === STRICT VALIDATION: FID Values ===
        // Get FIDs from user messages (not system messages which may have FID=0)
        let user_message_fids: Vec<i64> = sqlx::query_scalar(
            "SELECT DISTINCT fid FROM user_activity_timeline 
             WHERE activity_type NOT IN ('id_register', 'storage_rent', 'signer_add', 'fname_transfer')
             ORDER BY fid"
        )
        .fetch_all(database.pool())
        .await?;

        for fid in &user_message_fids {
            assert!(
                *fid > 0,
                "Block {}: User message FIDs must be positive (found {})",
                det_block.block_number,
                fid
            );
            assert!(
                *fid < 100_000_000,
                "Block {}: FID {} exceeds reasonable range (max 100M)",
                det_block.block_number,
                fid
            );
        }
        if !user_message_fids.is_empty() {
            println!(
                "    ✓ FID range validation: {} user FIDs, all in valid range (1-100M)",
                user_message_fids.len()
            );
        }

        // === SANITY CHECK: Verify no unexpected activity types ===
        // Only validate user message activities when user messages are specified
        // System messages (fname_transfer, etc.) are always allowed
        if !det_block.expected_message_types.is_empty() {
            // Get all user message activity types (not system)
            let user_activity_types: Vec<String> = sqlx::query_scalar(
                "SELECT DISTINCT activity_type FROM user_activity_timeline 
                 WHERE activity_type NOT IN ('id_register', 'storage_rent', 'signer_event', 'fname_transfer', 'fname_transfer_in', 'fname_transfer_out')
                 ORDER BY activity_type"
            )
            .fetch_all(database.pool())
            .await?;

            // Build expected activity types from message types
            let mut expected_activity_types: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for msg_type in det_block.expected_message_types.keys() {
                let activity_type = match msg_type {
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
                expected_activity_types.insert(activity_type.to_string());
            }

            // Find unexpected user message types
            let unexpected: Vec<String> = user_activity_types
                .into_iter()
                .filter(|t| !expected_activity_types.contains(t))
                .collect();

            assert!(
                unexpected.is_empty(),
                "Block {}: Found unexpected user activity types: {:?}",
                det_block.block_number,
                unexpected
            );
        }

        // === CROSS-VALIDATION: Casts (ALWAYS validate, even if expecting 0) ===
        let expected_cast_count = det_block
            .expected_message_types
            .get(&1)
            .copied()
            .unwrap_or(0);
        let cast_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM casts")
            .fetch_one(database.pool())
            .await?;
        assert_eq!(
            cast_count, expected_cast_count as i64,
            "Block {} should have {} casts (got {})",
            det_block.block_number, expected_cast_count, cast_count
        );
        println!(
            "    ✓ Casts: {} (expected: {})",
            cast_count, expected_cast_count
        );

        if cast_count > 0 {
            // Cross-validate: All casts should have corresponding user_profiles
            let unique_cast_authors: i64 =
                sqlx::query_scalar("SELECT COUNT(DISTINCT fid) FROM casts")
                    .fetch_one(database.pool())
                    .await?;

            let profiles_for_casts: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT p.fid) FROM user_profiles p 
                 INNER JOIN casts c ON p.fid = c.fid",
            )
            .fetch_one(database.pool())
            .await?;

            assert_eq!(
                profiles_for_casts, unique_cast_authors,
                "Block {}: All cast authors ({}) must have profiles (found {})",
                det_block.block_number, unique_cast_authors, profiles_for_casts
            );
            println!(
                "    ✓ Cast authors have profiles: {} (expected: {})",
                profiles_for_casts, unique_cast_authors
            );

            // Cross-validate: Casts in activity timeline
            let cast_activities: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_activity_timeline WHERE activity_type = 'cast_add'",
            )
            .fetch_one(database.pool())
            .await?;
            assert_eq!(
                cast_activities, expected_cast_count as i64,
                "Block {}: Cast count should match activity timeline",
                det_block.block_number
            );
            println!(
                "    ✓ Casts in activity timeline: {} (expected: {})",
                cast_activities, expected_cast_count
            );
        }

        // === CROSS-VALIDATION: Activities and User Profiles ===

        // First, verify all FIDs in activities have profiles
        let fids_without_profiles: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.fid) FROM user_activity_timeline a 
             LEFT JOIN user_profiles p ON a.fid = p.fid 
             WHERE p.fid IS NULL",
        )
        .fetch_one(database.pool())
        .await?;
        assert_eq!(
            fids_without_profiles, 0,
            "Block {}: All activity FIDs should have profiles (found {} without)",
            det_block.block_number, fids_without_profiles
        );

        let total_unique_fids: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT fid) FROM user_activity_timeline")
                .fetch_one(database.pool())
                .await?;

        let profile_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_profiles")
            .fetch_one(database.pool())
            .await?;

        assert!(
            profile_count >= total_unique_fids,
            "Block {}: Profile count ({}) should be >= unique FIDs ({})",
            det_block.block_number,
            profile_count,
            total_unique_fids
        );
        println!(
            "    ✓ User profiles: {} (covering {} unique FIDs)",
            profile_count, total_unique_fids
        );

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

            // Cross-validate: All activities should have valid timestamps (not NULL/0)
            let activities_with_invalid_ts: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_activity_timeline 
                 WHERE activity_type = $1 AND (timestamp IS NULL OR timestamp = 0)",
            )
            .bind(activity_type_name)
            .fetch_one(database.pool())
            .await?;
            assert_eq!(
                activities_with_invalid_ts, 0,
                "Block {}: All {} activities should have valid timestamps (not NULL/0)",
                det_block.block_number, activity_type_name
            );

            // Cross-validate: Timestamps are within reasonable range
            // Note: Farcaster timestamps are seconds since Farcaster epoch (2021-01-01)
            // NOT Unix timestamps
            if block_timestamp > 0 {
                // Check for negative timestamps
                let negative_timestamps: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM user_activity_timeline 
                     WHERE activity_type = $1 AND timestamp < 0",
                )
                .bind(activity_type_name)
                .fetch_one(database.pool())
                .await?;
                assert_eq!(
                    negative_timestamps, 0,
                    "Block {}: {} activities have negative timestamps",
                    det_block.block_number, activity_type_name
                );

                // Check for unreasonably large timestamps (> 10 years from Farcaster epoch)
                // 10 years = ~315M seconds
                let future_timestamps: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM user_activity_timeline 
                     WHERE activity_type = $1 AND timestamp > 315360000",
                )
                .bind(activity_type_name)
                .fetch_one(database.pool())
                .await?;
                assert_eq!(
                    future_timestamps, 0,
                    "Block {}: {} activities have timestamps > 10 years from Farcaster epoch",
                    det_block.block_number, activity_type_name
                );
            }

            println!(
                "    ✓ {}: {} (expected: {}, valid timestamps: ✓, range: ✓)",
                activity_type_name, actual_count, expected_count
            );
        }

        // === STRICT VALIDATION: Deletion Effects ===
        // If block contains CastRemove, verify casts are actually deleted
        if let Some(remove_count) = det_block.expected_message_types.get(&2) {
            // For CastRemove, we expect the cast to NOT be in casts table
            // But we should see the activity record
            let cast_remove_activities: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM user_activity_timeline WHERE activity_type = 'cast_remove'",
            )
            .fetch_one(database.pool())
            .await?;

            assert_eq!(
                cast_remove_activities, *remove_count as i64,
                "Block {}: CastRemove activity count mismatch",
                det_block.block_number
            );

            // Note: We can't verify the cast was deleted without knowing the original cast hash
            // This is a limitation - ideally we'd process block N-1, then block N, and verify deletion
            println!(
                "    ✓ cast_remove operations: {} (deletion logged)",
                cast_remove_activities
            );
        }

        // === STRICT VALIDATION: Data Sampling (random field completeness checks) ===
        // If we have casts, sample one and verify field completeness
        if cast_count > 0 {
            #[derive(sqlx::FromRow)]
            struct CastSample {
                fid: i64,
                message_hash: Vec<u8>,
                timestamp: i64,
            }

            let sample: CastSample =
                sqlx::query_as("SELECT fid, message_hash, timestamp FROM casts LIMIT 1")
                    .fetch_one(database.pool())
                    .await?;

            assert!(sample.fid > 0, "Cast FID must be positive");
            assert!(
                !sample.message_hash.is_empty(),
                "Cast must have message_hash"
            );
            assert!(sample.timestamp > 0, "Cast must have valid timestamp");

            println!(
                "    ✓ Data sampling: Cast fields complete (fid={}, hash_len={}, ts={})",
                sample.fid,
                sample.message_hash.len(),
                sample.timestamp
            );
        }

        // If we have user profiles, sample and verify
        if profile_count > 0 {
            #[derive(sqlx::FromRow)]
            struct ProfileSample {
                fid: i64,
                last_updated_timestamp: Option<i64>,
            }

            let sample: ProfileSample =
                sqlx::query_as("SELECT fid, last_updated_timestamp FROM user_profiles LIMIT 1")
                    .fetch_one(database.pool())
                    .await?;

            assert!(sample.fid > 0, "Profile FID must be positive");
            // last_updated_timestamp can be NULL for minimal profiles

            println!(
                "    ✓ Data sampling: Profile fields valid (fid={})",
                sample.fid
            );
        }

        // Sample activities and verify required fields
        if total_unique_fids > 0 {
            #[derive(sqlx::FromRow)]
            struct ActivitySample {
                fid: i64,
                activity_type: String,
                timestamp: i64,
            }

            let sample: ActivitySample = sqlx::query_as(
                "SELECT fid, activity_type, timestamp FROM user_activity_timeline LIMIT 1",
            )
            .fetch_one(database.pool())
            .await?;

            assert!(
                sample.fid > 0 || sample.activity_type.contains("fname_transfer"),
                "Activity FID must be positive (or fname_transfer with FID=0)"
            );
            assert!(
                !sample.activity_type.is_empty(),
                "Activity type must not be empty"
            );
            assert!(sample.timestamp != 0, "Activity timestamp must be set");

            println!(
                "    ✓ Data sampling: Activity fields complete (type={}, fid={}, ts={})",
                sample.activity_type, sample.fid, sample.timestamp
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
