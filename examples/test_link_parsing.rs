use snaprag::sync::client::SnapchainClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🔍 Testing Link Message Parsing");
    println!("================================\n");

    // Connect to Snapchain
    let client = SnapchainClient::new(
        "http://192.168.1.192:3381",
        "http://192.168.1.192:3383",
    )
    .await?;

    println!("✅ Connected to Snapchain\n");

    // Get info to find a good block range
    let info = client.get_info().await?;
    println!("📊 Snapchain Info:");
    for shard in &info.shard_infos {
        println!("  Shard {}: max_height = {}", shard.shard_id, shard.max_height);
    }
    println!();

    // Test multiple block ranges to find links
    let test_ranges = vec![
        (1, 100000, 100100),      // Early blocks
        (1, 1000000, 1000100),    // Mid blocks
        (1, 5000000, 5000100),    // Later blocks
        (1, 10000000, 10000100),  // Recent blocks
        (2, 100000, 100100),      // Shard 2 early
        (2, 10000000, 10000100),  // Shard 2 recent
    ];

    for (shard_id, start, end) in test_ranges {
        println!("🔍 Testing shard {} blocks {}-{}", shard_id, start, end);
        
        let request = snaprag::sync::client::proto::ShardChunksRequest {
            shard_id,
            start_block_number: start,
            stop_block_number: Some(end),
        };

        match client.get_shard_chunks(request).await {
            Ok(response) => {
                let mut link_count = 0;
                let mut reaction_count = 0;
                let mut cast_count = 0;
                let mut verification_count = 0;
                let mut total_messages = 0;

                for chunk in &response.shard_chunks {
                    for tx in &chunk.transactions {
                        for msg in &tx.user_messages {
                            if let Some(data) = &msg.data {
                                total_messages += 1;
                                
                                match data.r#type {
                                    1 => cast_count += 1,
                                    3 => reaction_count += 1,
                                    5 => {
                                        link_count += 1;
                                        // Check if body has link_body
                                        if let Some(body) = &data.body {
                                            if let Some(link_body) = body.get("link_body") {
                                                println!("  ✅ Found LinkAdd with link_body: FID {}", data.fid);
                                                println!("     link_body: {:?}", link_body);
                                            } else {
                                                println!("  ❌ Found LinkAdd but NO link_body! FID {}", data.fid);
                                                println!("     body keys: {:?}", body.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                                                println!("     body value: {:?}", body);
                                            }
                                        } else {
                                            println!("  ❌ Found LinkAdd but body is None! FID {}", data.fid);
                                        }
                                    }
                                    7 => verification_count += 1,
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                println!("  📊 Results: {} total messages", total_messages);
                println!("     Casts: {}", cast_count);
                println!("     Links: {}", link_count);
                println!("     Reactions: {}", reaction_count);
                println!("     Verifications: {}", verification_count);
                
                if link_count > 0 {
                    println!("\n  🎯 Found {} LinkAdd messages in this range!", link_count);
                    println!("     This range is good for testing!\n");
                    break;
                }
                println!();
            }
            Err(e) => {
                println!("  ❌ Error: {}\n", e);
            }
        }
    }

    Ok(())
}

