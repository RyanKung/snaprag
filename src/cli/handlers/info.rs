//! Information display handlers (stats, dashboard, config)

use crate::cli::output::*;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;
use std::sync::Arc;

pub async fn handle_stats_command(
    snaprag: &SnapRag,
    detailed: bool,
    export: Option<String>,
) -> Result<()> {
    let stats = snaprag.get_statistics().await?;
    print_statistics(&stats, detailed);

    if let Some(export_path) = export {
        let json = serde_json::to_string_pretty(&stats)?;
        std::fs::write(&export_path, json)?;
        print_success(&format!("Statistics exported to: {}", export_path));
    }

    Ok(())
}

/// Handle dashboard command (FAST version with minimal queries)
pub async fn handle_dashboard_command(snaprag: &SnapRag) -> Result<()> {
    print_info("📊 SnapRAG Dashboard (Fast Mode)");
    println!();

    // Use faster queries with limited data
    let pool = snaprag.database().pool();

    // For small tables (user_profiles), use exact COUNT for accuracy
    // For large tables (casts, activities), use PostgreSQL statistics for speed
    let total_profiles: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(pool)
        .await?;

    // Use pg_class.reltuples for large tables (instant vs minutes for COUNT)
    let large_table_stats: Vec<(String, i64)> = sqlx::query_as(
        "SELECT relname, reltuples::bigint FROM pg_class 
         WHERE relname IN ('casts', 'user_activity_timeline')"
    )
    .fetch_all(pool)
    .await?;

    let mut total_casts = 0i64;
    let mut total_activities = 0i64;

    for (table, count) in large_table_stats {
        match table.as_str() {
            "casts" => total_casts = count,
            "user_activity_timeline" => total_activities = count,
            _ => {}
        }
    }

    // Get profiles with username (fast partial index count)
    let profiles_with_username: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_profiles WHERE username IS NOT NULL AND username != ''"
    )
    .fetch_one(pool)
    .await?;

    // Get embeddings count
    let profiles_with_embeddings: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_profiles WHERE profile_embedding IS NOT NULL"
    )
    .fetch_one(pool)
    .await?;

    let casts_with_embeddings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cast_embeddings")
        .fetch_one(pool)
        .await?;

    // Latest activity (fast with index)
    let latest_timestamp: Option<i64> = sqlx::query_scalar(
        "SELECT timestamp FROM user_activity_timeline ORDER BY timestamp DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    // Display results
    println!("📈 Database Statistics:");
    println!("  Total Profiles: {} (exact)", format_number(total_profiles));
    println!("  └─ With Username: {} ({:.1}%)", 
        format_number(profiles_with_username),
        (profiles_with_username as f64 / total_profiles.max(1) as f64 * 100.0)
    );
    println!("  Casts: ~{} (estimated)", format_number(total_casts));
    println!("  Activities: ~{} (estimated)", format_number(total_activities));
    println!();

    println!("🔮 Embeddings:");
    println!("  Profiles: {} ({:.1}%)", 
        format_number(profiles_with_embeddings),
        (profiles_with_embeddings as f64 / total_profiles as f64 * 100.0)
    );
    println!("  Casts: {} ({:.1}%)", 
        format_number(casts_with_embeddings),
        (casts_with_embeddings as f64 / total_casts.max(1) as f64 * 100.0)
    );
    println!();

    if let Some(ts) = latest_timestamp {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
            println!("⏰ Latest Activity: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
            println!();
        }
    }

    // Sync status - read directly from database (real-time)
    let sync_info: Vec<(i32, i64, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT shard_id, last_processed_height, status, updated_at 
         FROM sync_progress 
         ORDER BY shard_id"
    )
    .fetch_all(pool)
    .await?;
    
    if !sync_info.is_empty() {
        println!("🔄 Sync Status (Real-time from DB):");
        
        let mut total_height = 0i64;
        for (shard_id, height, status, updated_at) in &sync_info {
            total_height += height;
            
            // Calculate time since last update
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(*updated_at);
            let time_ago = if duration.num_seconds() < 60 {
                format!("{}s ago", duration.num_seconds())
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else {
                format!("{}h ago", duration.num_hours())
            };
            
            println!("  Shard {}: {} ({}) - {}", 
                shard_id, 
                format_number(*height), 
                status,
                time_ago
            );
        }
        
        // Estimate total messages (activities)
        let avg_height = total_height / sync_info.len() as i64;
        println!("  Avg Height: {}", format_number(avg_height));
        println!();
    }

    print_info("💡 Tip: Use 'snaprag stats' for detailed statistics");
    
    Ok(())
}

/// Handle config command
pub async fn handle_config_command(config: &AppConfig) -> Result<()> {
    print_config(config);
    Ok(())
}

/// Print sync status
pub(crate) async fn print_sync_status(snaprag: &SnapRag) -> Result<()> {
    print_info("Sync Status:");

    match snaprag.get_sync_status()? {
        Some(lock) => {
            println!("  - Status: {}", lock.status);
            println!("  - PID: {}", lock.pid);
            println!(
                "  - Start time: {}",
                chrono::DateTime::from_timestamp(lock.start_time as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "  - Last update: {}",
                chrono::DateTime::from_timestamp(lock.last_update as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
            );

            // Display per-shard progress
            if !lock.progress.shard_progress.is_empty() {
                println!("  - Shards:");
                let mut shards: Vec<_> = lock.progress.shard_progress.iter().collect();
                shards.sort_by_key(|(shard_id, _)| *shard_id);

                for (shard_id, shard_progress) in shards {
                    println!(
                        "    • Shard {}: Block {} ({} blocks, {} msgs)",
                        shard_id,
                        shard_progress.current_block,
                        shard_progress.blocks_processed,
                        shard_progress.messages_processed
                    );
                }
            }
            println!(
                "  - Total blocks processed: {}",
                lock.progress.total_blocks_processed
            );
            println!(
                "  - Total messages processed: {}",
                lock.progress.total_messages_processed
            );

            if let Some(range) = &lock.progress.sync_range {
                println!(
                    "  - Sync range: {} to {}",
                    range.from_block,
                    range
                        .to_block
                        .map(|b| b.to_string())
                        .unwrap_or("latest".to_string())
                );
            }

            if let Some(error) = &lock.error_message {
                println!("  - Error: {}", error);
            }

            // Show latest synced message timestamp
            match get_latest_message_time(snaprag).await {
                Ok(time_info) => {
                    println!("  - Latest message time: {}", time_info);
                }
                Err(e) => {
                    tracing::debug!("Failed to get latest message time: {}", e);
                }
            }
        }
        None => {
            println!("  - No active sync process");
        }
    }
    Ok(())
}

/// Get the timestamp of the latest synced message (fast version)
async fn get_latest_message_time(snaprag: &SnapRag) -> Result<String> {
    const FARCASTER_EPOCH: i64 = 1609459200;

    // Use LIMIT 1 with ORDER BY DESC - uses index efficiently
    let latest_timestamp = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT timestamp FROM user_activity_timeline ORDER BY timestamp DESC LIMIT 1"
    )
    .fetch_one(snaprag.database().pool())
    .await?;

    if let Some(ts) = latest_timestamp {
        let actual_timestamp = FARCASTER_EPOCH + ts;
        let datetime = chrono::DateTime::from_timestamp(actual_timestamp, 0)
            .ok_or_else(|| crate::SnapRagError::Custom("Invalid timestamp".to_string()))?;

        Ok(format!(
            "{} (Farcaster ts: {})",
            datetime.format("%Y-%m-%d %H:%M:%S UTC"),
            ts
        ))
    } else {
        Ok("No messages synced yet".to_string())
    }
}

/// Format large numbers with commas
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let mut count = 0;
    
    for c in s.chars().rev() {
        if count == 3 {
            result.push(',');
            count = 0;
        }
        result.push(c);
        count += 1;
    }
    
    result.chars().rev().collect()
}
