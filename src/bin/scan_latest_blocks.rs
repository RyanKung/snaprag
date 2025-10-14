//! Specialized scanner for latest blocks to find new message types
//! Searches for: UsernameProof(12), FrameAction(13), LinkCompactState(14), LendStorage(15)

use std::collections::HashMap;

use snaprag::config::AppConfig;
use snaprag::sync::client::proto;
use snaprag::sync::client::SnapchainClient;
use snaprag::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("\n🔍 Scanning Latest Blocks for New Message Types");
    println!("================================================\n");
    println!(
        "Target types: UsernameProof(12), FrameAction(13), LinkCompactState(14), LendStorage(15)\n"
    );

    let config = AppConfig::load()?;
    let client = SnapchainClient::new(&config.sync.snapchain_grpc_endpoint).await?;

    // Scan very recent blocks where new features are likely to appear
    let scan_ranges = vec![
        ("Recent 1 (18M-19M)", 18_000_000, 19_000_000, 5000),
        ("Recent 2 (19M-20M)", 19_000_000, 20_000_000, 5000),
        ("Recent 3 (20M-21M)", 20_000_000, 21_000_000, 5000),
        ("Recent 4 (21M-22M)", 21_000_000, 22_000_000, 5000),
        ("Recent 5 (22M-23M)", 22_000_000, 23_000_000, 5000),
        ("Latest 1 (23M-24M)", 23_000_000, 24_000_000, 5000),
        ("Latest 2 (24M-25M)", 24_000_000, 25_000_000, 5000),
        ("Latest 3 (25M-26M)", 25_000_000, 26_000_000, 5000),
        ("Cutting edge (26M-27M)", 26_000_000, 27_000_000, 5000),
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
                        let mut type_counts: HashMap<i32, usize> = HashMap::new();

                        for tx in &chunk.transactions {
                            for msg in &tx.user_messages {
                                if let Some(data) = &msg.data {
                                    let msg_type = data.r#type;

                                    // Only track types we're looking for
                                    if msg_type >= 12 && msg_type <= 15 {
                                        *type_counts.entry(msg_type).or_insert(0) += 1;
                                        found_types
                                            .entry(msg_type)
                                            .or_insert_with(Vec::new)
                                            .push(block);
                                    }
                                }
                            }
                        }

                        if !type_counts.is_empty() {
                            let types_str = type_counts
                                .iter()
                                .map(|(t, c)| {
                                    let name = match *t {
                                        12 => "UsernameProof",
                                        13 => "FrameAction",
                                        14 => "LinkCompactState",
                                        15 => "LendStorage",
                                        _ => "Unknown",
                                    };
                                    format!("Type{}({}):{}x", t, name, c)
                                })
                                .collect::<Vec<_>>()
                                .join(", ");

                            println!("✨ Block {:<10} - [{}]", block, types_str);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Error at block {}: {}", block, e);
                    // Continue scanning
                }
            }

            if blocks_scanned % 100 == 0 {
                println!("  ... scanned {} blocks so far", blocks_scanned);
            }
        }
        println!();
    }

    // Print summary
    println!("\n📋 Summary: New Message Types Found");
    println!("====================================");

    if found_types.is_empty() {
        println!("⚠️  None of the target types (12-15) found in scanned ranges!");
        println!("    Possibilities:");
        println!("    - These features may not be active yet");
        println!("    - May need to scan even newer blocks");
        println!("    - Or check different shards");
    } else {
        for msg_type in 12..=15 {
            let type_name = match msg_type {
                12 => "UsernameProof",
                13 => "FrameAction",
                14 => "LinkCompactState",
                15 => "LendStorage",
                _ => "Unknown",
            };

            if let Some(blocks) = found_types.get(&msg_type) {
                println!(
                    "  ✅ Type {}: {} - Found in {} blocks",
                    msg_type,
                    type_name,
                    blocks.len()
                );
                println!(
                    "     First occurrence: block {}",
                    blocks.first().unwrap_or(&0)
                );
                if blocks.len() > 1 {
                    println!("     Sample blocks: {:?}", &blocks[..blocks.len().min(5)]);
                }
                println!();
            } else {
                println!("  ❌ Type {}: {} - Not found", msg_type, type_name);
            }
        }
    }

    println!(
        "\n✅ Scan complete! Scanned {} blocks total.",
        blocks_scanned
    );

    Ok(())
}
