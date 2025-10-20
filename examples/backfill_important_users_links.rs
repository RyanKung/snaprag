//! Backfill links for important/active users from Snapchain
//!
//! Usage: cargo run --example backfill_important_users_links

use std::sync::Arc;

use snaprag::sync::client::SnapchainClient;
use snaprag::AppConfig;
use snaprag::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    snaprag::logging::init_logging()?;

    // Load config
    let config = AppConfig::load()?;
    let database = Arc::new(Database::from_config(&config).await?);
    let client = Arc::new(SnapchainClient::from_config(&config).await?);

    println!("🔗 Starting links backfill for important users...\n");

    // Get top users by cast count (most active users)
    println!("📊 Finding most active users...");
    let top_users: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT fid, COUNT(*) as cast_count 
         FROM casts 
         GROUP BY fid 
         ORDER BY cast_count DESC 
         LIMIT 1000",
    )
    .fetch_all(database.pool())
    .await?;

    println!("   Found {} active users to backfill\n", top_users.len());

    let mut total_inserted = 0;
    let mut total_skipped = 0;
    let mut total_errors = 0;

    for (idx, (fid, cast_count)) in top_users.iter().enumerate() {
        if idx % 10 == 0 {
            println!("Progress: {}/{} users processed...", idx, top_users.len());
        }

        // Fetch links from Snapchain
        match client.get_links_by_fid(*fid as u64, Some(1000)).await {
            Ok(messages) => {
                for message in &messages {
                    if let Some(data) = &message.data {
                        if let Some(body) = &data.body {
                            if let Some(link_body) = body.get("link_body") {
                                let target_fid = link_body
                                    .get("target_fid")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);

                                if target_fid > 0 {
                                    let link_type = link_body
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("follow");

                                    let result = sqlx::query(
                                        "INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash)
                                         VALUES ($1, $2, $3, $4, $5)
                                         ON CONFLICT (message_hash) DO NOTHING"
                                    )
                                    .bind(fid)
                                    .bind(target_fid)
                                    .bind(link_type)
                                    .bind(data.timestamp as i64)
                                    .bind(&message.hash)
                                    .execute(database.pool())
                                    .await;

                                    match result {
                                        Ok(r) if r.rows_affected() > 0 => total_inserted += 1,
                                        Ok(_) => total_skipped += 1,
                                        Err(_) => total_errors += 1,
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("  ⚠️  Failed to fetch links for FID {}: {}", fid, e);
                total_errors += 1;
            }
        }

        // Small delay to avoid overwhelming the API
        if idx % 10 == 0 && idx > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    println!("\n✅ Backfill complete!");
    println!("   Inserted: {}", total_inserted);
    println!("   Skipped (duplicates): {}", total_skipped);
    println!("   Errors: {}", total_errors);

    // Show final count
    let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM links")
        .fetch_one(database.pool())
        .await?;

    println!("\n📊 Total links in database: {}", final_count);

    Ok(())
}
