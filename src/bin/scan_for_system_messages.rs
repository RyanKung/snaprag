//! Specialized scanner to find blocks with system messages
//! This helps discover FID registrations, storage events, and signer events

use std::collections::HashMap;

use snaprag::config::AppConfig;
use snaprag::sync::client::proto;
use snaprag::sync::client::SnapchainClient;
use snaprag::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n🔍 Scanning for System Messages (OnChainEvents)");
    println!("================================================\n");

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(&config.sync.snapchain_grpc_endpoint).await?;

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
                                    *event_types.entry(9999).or_insert(0) += 1; // Special code for fname
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

            // Progress indicator
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
        println!("    Try scanning:");
        println!("    - Earlier blocks (if available)");
        println!("    - Different shard IDs");
        println!("    - Or check if system messages are in the protobuf conversion");
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

            println!(
                "  Event Type {}: {} - Found in {} blocks",
                event_type,
                event_name,
                blocks.len()
            );
            println!(
                "    First occurrence: block {}",
                blocks.first().unwrap_or(&0)
            );
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
