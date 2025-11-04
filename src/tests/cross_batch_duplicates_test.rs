//! Test to verify if cross-batch duplicate `message_hash` actually exists
//!
//! This test fetches real data from Snapchain to check if:
//! 1. Different batches contain the same `message_hash`
//! 2. Same FID appears in multiple batches with different messages

use std::collections::HashMap;
use std::collections::HashSet;

use crate::sync::client::SnapchainClient;
use crate::Result;

#[tokio::test]
#[ignore] // Run manually: cargo test --test cross_batch_duplicates_test -- --ignored
async fn test_cross_batch_message_hash_duplicates() -> Result<()> {
    // Initialize client
    let config = crate::tests::load_test_config()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

    // Test parameters
    let shard_id = 1; // Test shard 1
    let test_start_block = 100_000; // Start from a block with activity
    let batch_size = 500;
    let num_batches = 3; // Test 3 consecutive batches

    println!("\n🔍 Testing for cross-batch duplicate message_hash");
    println!(
        "Shard: {shard_id}, Starting block: {test_start_block}, Batch size: {batch_size}, Batches: {num_batches}"
    );
    println!("{}", "=".repeat(80));

    let mut all_message_hashes: HashMap<Vec<u8>, (u32, u64)> = HashMap::new(); // hash -> (batch_num, block_num)
    let mut all_fids: HashMap<i64, Vec<(u32, Vec<u8>)>> = HashMap::new(); // fid -> [(batch_num, message_hash)]
    let mut total_messages = 0;
    let mut cross_batch_duplicates = 0;

    for batch_num in 0..num_batches {
        let start = test_start_block + (u64::from(batch_num) * batch_size as u64);
        let end = start + batch_size as u64 - 1;

        println!("\n📦 Batch {}: blocks {}-{}", batch_num + 1, start, end);

        let request = crate::sync::client::proto::ShardChunksRequest {
            shard_id,
            start_block_number: start,
            stop_block_number: Some(end),
        };

        let response = client.get_shard_chunks(request).await?;
        let chunks = response.shard_chunks;

        let mut batch_message_hashes = HashSet::new();
        let mut batch_messages = 0;

        // Process each chunk
        for chunk in &chunks {
            for tx in &chunk.transactions {
                for msg in &tx.user_messages {
                    batch_messages += 1;
                    total_messages += 1;

                    let message_hash = msg.hash.clone();
                    let block_num = chunk
                        .header
                        .as_ref()
                        .and_then(|h| h.height.as_ref())
                        .map_or(0, |h| h.block_number);

                    // Extract FID from message data
                    if let Some(msg_data) = &msg.data {
                        if let Some(body_value) = &msg_data.body {
                            if let Ok(body) =
                                serde_json::from_value::<serde_json::Value>(body_value.clone())
                            {
                                if let Some(fid) =
                                    body.get("fid").and_then(serde_json::Value::as_i64)
                                {
                                    all_fids
                                        .entry(fid)
                                        .or_default()
                                        .push((batch_num, message_hash.clone()));
                                }
                            }
                        }
                    }

                    // Check if this message_hash was seen in a previous batch
                    if let Some((prev_batch, prev_block)) = all_message_hashes.get(&message_hash) {
                        cross_batch_duplicates += 1;
                        println!("   ⚠️  DUPLICATE FOUND:");
                        println!("      Message hash: {}", hex::encode(&message_hash[..8]));
                        println!(
                            "      First seen: Batch {} (block {})",
                            prev_batch + 1,
                            prev_block
                        );
                        println!(
                            "      Seen again: Batch {} (block {})",
                            batch_num + 1,
                            block_num
                        );
                    } else {
                        all_message_hashes.insert(message_hash.clone(), (batch_num, block_num));
                    }

                    batch_message_hashes.insert(message_hash);
                }
            }
        }

        println!("   Messages in batch: {batch_messages}");
        println!(
            "   Unique message_hash in batch: {}",
            batch_message_hashes.len()
        );
        println!(
            "   Duplicates within batch: {}",
            batch_messages - batch_message_hashes.len()
        );
    }

    println!("\n{}", "=".repeat(80));
    println!("📊 RESULTS:");
    println!("   Total messages processed: {total_messages}");
    println!(
        "   Unique message_hash across all batches: {}",
        all_message_hashes.len()
    );
    println!("   Cross-batch duplicates: {cross_batch_duplicates}");
    println!(
        "   Duplicate rate: {:.2}%",
        (f64::from(cross_batch_duplicates) / f64::from(total_messages)) * 100.0
    );

    // Analyze FID distribution
    let mut fids_in_multiple_batches = 0;
    let mut max_batches_per_fid = 0;

    for (fid, appearances) in &all_fids {
        let unique_batches: HashSet<u32> = appearances.iter().map(|(b, _)| *b).collect();
        if unique_batches.len() > 1 {
            fids_in_multiple_batches += 1;
        }
        max_batches_per_fid = max_batches_per_fid.max(unique_batches.len());

        if unique_batches.len() == num_batches as usize {
            println!(
                "   FID {} appears in ALL {} batches ({} messages)",
                fid,
                num_batches,
                appearances.len()
            );
        }
    }

    println!("\n📈 FID Analysis:");
    println!("   Total unique FIDs: {}", all_fids.len());
    println!("   FIDs appearing in multiple batches: {fids_in_multiple_batches}");
    println!("   Max batches for single FID: {max_batches_per_fid}");

    // Assertions
    println!("\n✅ Test Assertions:");

    if cross_batch_duplicates > 0 {
        println!("   ⚠️  Cross-batch duplicates EXIST!");
        println!("   → This explains the lock contention issue");
        println!("   → Zero-lock mode (no UNIQUE constraint) is necessary");
    } else {
        println!("   ✅ NO cross-batch duplicates found");
        println!("   → Lock contention must be from other sources");
        panic!("Expected to find cross-batch duplicates based on observed behavior");
    }

    assert!(
        cross_batch_duplicates == 0,
        "Found {cross_batch_duplicates} cross-batch duplicate message_hash! This is the root cause of lock contention."
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_same_fid_across_batches() -> Result<()> {
    // Simpler test: just check if same FID appears in consecutive batches
    let config = crate::tests::load_test_config()?;
    let client = SnapchainClient::new(
        &config.sync.snapchain_http_endpoint,
        &config.sync.snapchain_grpc_endpoint,
    )
    .await?;

    let shard_id = 1;
    let start_block = 100_000;

    // Fetch batch 1
    let req1 = crate::sync::client::proto::ShardChunksRequest {
        shard_id,
        start_block_number: start_block,
        stop_block_number: Some(start_block + 499),
    };
    let resp1 = client.get_shard_chunks(req1).await?;

    // Fetch batch 2
    let req2 = crate::sync::client::proto::ShardChunksRequest {
        shard_id,
        start_block_number: start_block + 500,
        stop_block_number: Some(start_block + 999),
    };
    let resp2 = client.get_shard_chunks(req2).await?;

    // Extract FIDs from both batches
    let mut fids_batch1 = HashSet::new();
    let mut fids_batch2 = HashSet::new();

    for chunk in &resp1.shard_chunks {
        for tx in &chunk.transactions {
            for msg in &tx.user_messages {
                if let Some(msg_data) = &msg.data {
                    if let Some(body_value) = &msg_data.body {
                        if let Ok(body) =
                            serde_json::from_value::<serde_json::Value>(body_value.clone())
                        {
                            if let Some(fid) = body.get("fid").and_then(serde_json::Value::as_i64) {
                                fids_batch1.insert(fid);
                            }
                        }
                    }
                }
            }
        }
    }

    for chunk in &resp2.shard_chunks {
        for tx in &chunk.transactions {
            for msg in &tx.user_messages {
                if let Some(msg_data) = &msg.data {
                    if let Some(body_value) = &msg_data.body {
                        if let Ok(body) =
                            serde_json::from_value::<serde_json::Value>(body_value.clone())
                        {
                            if let Some(fid) = body.get("fid").and_then(serde_json::Value::as_i64) {
                                fids_batch2.insert(fid);
                            }
                        }
                    }
                }
            }
        }
    }

    let common_fids: HashSet<_> = fids_batch1.intersection(&fids_batch2).collect();

    println!("\n📊 FID Distribution Analysis:");
    println!(
        "   Batch 1 (blocks {}-{}): {} unique FIDs",
        start_block,
        start_block + 499,
        fids_batch1.len()
    );
    println!(
        "   Batch 2 (blocks {}-{}): {} unique FIDs",
        start_block + 500,
        start_block + 999,
        fids_batch2.len()
    );
    println!("   FIDs appearing in BOTH batches: {}", common_fids.len());
    println!(
        "   Overlap rate: {:.2}%",
        (common_fids.len() as f64 / fids_batch1.len() as f64) * 100.0
    );

    if !common_fids.is_empty() {
        println!("\n   Example FIDs in both batches:");
        for (idx, fid) in common_fids.iter().take(5).enumerate() {
            println!("   {}. FID {}", idx + 1, fid);
        }
    }

    // This is the root cause of lock contention!
    assert!(
        !common_fids.is_empty(),
        "Expected to find FIDs appearing in multiple batches"
    );

    Ok(())
}
