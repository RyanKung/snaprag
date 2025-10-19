//! Synchronization handlers

use crate::cli::output::*;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;
use std::sync::Arc;

use crate::cli::commands::SyncCommands;
use super::info::print_sync_status;

pub async fn handle_sync_command(mut snaprag: SnapRag, sync_command: SyncCommands) -> Result<()> {
    // For commands that need database access, verify schema is initialized
    match &sync_command {
        SyncCommands::Stop { .. } | SyncCommands::Status => {
            // These commands don't require full schema
        }
        _ => {
            // TODO: Re-enable after first init
            // All other sync commands require initialized database
            // snaprag.database().verify_schema_or_error().await?;
        }
    }

    match sync_command {
        SyncCommands::All => {
            print_info("Starting full synchronization (historical + real-time)...");
            snaprag.start_sync().await?;
        }
        SyncCommands::Start {
            from,
            to,
            shard,
            batch,
            interval,
        } => {
            let from_block = from.unwrap_or(0);
            let to_block = to.unwrap_or(u64::MAX);

            // Parse shard IDs if provided
            let shard_ids = if let Some(shard_str) = shard {
                shard_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u32>().ok())
                    .collect::<Vec<_>>()
            } else {
                vec![] // Use default from config
            };

            // Apply command-line overrides to config
            if batch.is_some() || interval.is_some() || !shard_ids.is_empty() {
                snaprag.override_sync_config(shard_ids.clone(), batch, interval)?;
            }

            if let Some(to_val) = to {
                print_info(&format!(
                    "Starting synchronization from block {} to block {}{}{}...",
                    from_block,
                    to_val,
                    if let Some(b) = batch {
                        format!(" (batch: {})", b)
                    } else {
                        String::new()
                    },
                    if !shard_ids.is_empty() {
                        format!(" (shards: {:?})", shard_ids)
                    } else {
                        String::new()
                    }
                ));
            } else {
                print_info(&format!(
                    "Starting synchronization from block {} to latest{}{}...",
                    from_block,
                    if let Some(b) = batch {
                        format!(" (batch: {})", b)
                    } else {
                        String::new()
                    },
                    if !shard_ids.is_empty() {
                        format!(" (shards: {:?})", shard_ids)
                    } else {
                        String::new()
                    }
                ));
            }

            snaprag.start_sync_with_range(from_block, to_block).await?;
        }
        SyncCommands::Test { shard, block } => {
            print_info(&format!(
                "Testing single block synchronization for shard {} block {}...",
                shard, block
            ));

            // For test command, we need to create a sync service directly
            let sync_service =
                crate::sync::service::SyncService::new(&snaprag.config, snaprag.database().clone())
                    .await?;

            match sync_service.poll_once(shard, block).await {
                Ok(stats) => {
                    print_success(&format!(
                        "Single block test completed successfully! Blocks processed: {}, messages: {}",
                        stats.blocks_processed(),
                        stats.messages_processed()
                    ));
                }
                Err(e) => {
                    print_error(&format!("Single block test failed: {}", e));
                    return Err(e);
                }
            }
        }
        SyncCommands::Realtime => {
            print_info("Starting real-time synchronization...");
            snaprag.start_sync().await?;
        }
        SyncCommands::Status => {
            print_sync_status(&snaprag).await?;
        }
        SyncCommands::Stop { force } => {
            print_info("Stopping sync processes...");
            snaprag.stop_sync(force).await?;

            if force {
                print_success("Force stopped successfully");
            } else {
                print_success("Gracefully stopped successfully");
            }
        }
    }
    Ok(())
}

