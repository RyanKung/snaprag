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
    // TODO: Re-enable after first init
    // snaprag.database().verify_schema_or_error().await?;
    let stats = snaprag.get_statistics().await?;
    print_statistics(&stats, detailed);

    if let Some(export_path) = export {
        let json = serde_json::to_string_pretty(&stats)?;
        std::fs::write(&export_path, json)?;
        print_success(&format!("Statistics exported to: {}", export_path));
    }

    Ok(())
}

/// Handle search command
/// Handle dashboard command
pub async fn handle_dashboard_command(snaprag: &SnapRag) -> Result<()> {
    let stats = snaprag.get_statistics().await?;
    print_dashboard(&stats);
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

/// Get the timestamp of the latest synced message
async fn get_latest_message_time(snaprag: &SnapRag) -> Result<String> {
    // Farcaster epoch: 2021-01-01 00:00:00 UTC
    const FARCASTER_EPOCH: i64 = 1609459200;

    let latest_timestamp =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(timestamp) FROM user_activity_timeline")
            .fetch_one(snaprag.database().pool())
            .await?;

    if let Some(ts) = latest_timestamp {
        // Convert Farcaster timestamp to actual time
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

