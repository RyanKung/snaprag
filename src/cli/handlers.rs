//! CLI command handlers
//!
//! This module contains all the command handlers for the SnapRAG CLI

use std::sync::Arc;

use crate::cli::commands::Commands;
use crate::cli::commands::DataType;
use crate::cli::commands::SyncCommands;
use crate::cli::output::*;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

/// Handle list command
pub async fn handle_list_command(
    snaprag: &SnapRag,
    data_type: DataType,
    limit: u32,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: String,
    fid_range: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    bio: Option<String>,
    location: Option<String>,
    twitter: Option<String>,
    github: Option<String>,
    has_username: bool,
    has_display_name: bool,
    has_bio: bool,
) -> Result<()> {
    match data_type {
        DataType::Fid => {
            print_list_header("FIDs", limit);

            // Parse FID range if provided
            let (min_fid, max_fid) = if let Some(range) = fid_range {
                if let Some((min, max)) = range.split_once('-') {
                    (
                        Some(min.parse::<i64>().unwrap_or(0)),
                        Some(max.parse::<i64>().unwrap_or(i64::MAX)),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::FidSortBy::Fid),
                Some("username") => Some(crate::models::FidSortBy::Username),
                Some("last_updated") => Some(crate::models::FidSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::FidSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Asc),
            };

            // Build FID query
            let fid_query = crate::models::FidQuery {
                fid: None,
                min_fid,
                max_fid,
                has_username: if has_username { Some(true) } else { None },
                has_display_name: if has_display_name { Some(true) } else { None },
                has_bio: if has_bio { Some(true) } else { None },
                limit: Some(limit as i64),
                offset: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_fids(fid_query).await?;
            print_fid_list(&profiles);
        }
        DataType::Profiles => {
            print_list_header("User Profiles", limit);

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::ProfileSortBy::Fid),
                Some("username") => Some(crate::models::ProfileSortBy::Username),
                Some("display_name") => Some(crate::models::ProfileSortBy::DisplayName),
                Some("last_updated") => Some(crate::models::ProfileSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::ProfileSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Desc),
            };

            // Build profile query
            let profile_query = crate::models::UserProfileQuery {
                fid: None,
                username,
                display_name,
                bio,
                location,
                twitter_username: twitter,
                github_username: github,
                limit: Some(limit as i64),
                offset: None,
                start_timestamp: None,
                end_timestamp: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_user_profiles(profile_query).await?;
            print_profile_list(&profiles);
        }
        DataType::Casts => {
            print_list_header("Casts", limit);

            // Build cast query
            let cast_query = crate::models::CastQuery {
                fid: None,
                text_search: search,
                parent_hash: None,
                root_hash: None,
                has_mentions: None,
                has_embeds: None,
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::CastSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let casts = snaprag.database().list_casts(cast_query).await?;
            print_cast_list(&casts);
        }
        DataType::Follows => {
            print_list_header("Follows", limit);

            // Build link query for follows
            let link_query = crate::models::LinkQuery {
                fid: None,
                target_fid: None,
                link_type: Some("follow".to_string()),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::LinkSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let links = snaprag.database().list_links(link_query).await?;
            print_link_list(&links);
        }
        DataType::UserData => {
            print_list_header("User Data", limit);

            // Build user data query
            let user_data_query = crate::models::UserDataQuery {
                fid: None,
                data_type: None,
                value_search: search.clone(),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::UserDataSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let user_data = snaprag.database().list_user_data(user_data_query).await?;
            print_user_data_list(&user_data);
        }
    }
    Ok(())
}

/// Handle reset command
pub async fn handle_reset_command(snaprag: &SnapRag, force: bool) -> Result<()> {
    if !force {
        print_warning(
            "This will reset ALL synchronized data from the database and remove lock files!",
        );
        print_prompt("Are you sure you want to continue? (y/N)");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            print_info("Operation cancelled.");
            return Ok(());
        }
    }

    print_info("Resetting all synchronized data and lock files...");

    // Remove lock file if it exists
    if std::path::Path::new("snaprag.lock").exists() {
        std::fs::remove_file("snaprag.lock")?;
        print_success("Removed snaprag.lock file");
    } else {
        print_info("No lock file found");
    }

    // Clear all tables
    let tables = [
        "user_profiles",
        "username_proofs",
        "user_activities",
        "user_data_changes",
        "casts",
        "links",
        "user_data",
    ];

    for table in &tables {
        let deleted = sqlx::query(&format!("DELETE FROM {}", table))
            .execute(snaprag.database().pool())
            .await?;
        print_success(&format!(
            "Deleted {} {} records",
            deleted.rows_affected(),
            table
        ));
    }

    print_success("Database and lock files reset successfully!");
    Ok(())
}

/// Handle sync command
pub async fn handle_sync_command(mut snaprag: SnapRag, sync_command: SyncCommands) -> Result<()> {
    match sync_command {
        SyncCommands::All => {
            print_info("Starting full synchronization (historical + real-time)...");
            snaprag.start_sync().await?;
        }
        SyncCommands::Start { from, to } => {
            let from_block = from.unwrap_or(0);
            let to_block = to.unwrap_or(u64::MAX);

            if let Some(to_val) = to {
                print_info(&format!(
                    "Starting synchronization from block {} to block {}...",
                    from_block, to_val
                ));
            } else {
                print_info(&format!(
                    "Starting synchronization from block {} to latest...",
                    from_block
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
            print_sync_status(&snaprag)?;
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

/// Handle stats command
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

/// Handle search command
pub async fn handle_search_command(
    snaprag: &SnapRag,
    query: String,
    limit: u32,
    fields: String,
) -> Result<()> {
    print_search_header(&query, &fields);

    let profiles = snaprag.search_profiles(&query).await?;
    print_search_results(&profiles, limit as usize);

    Ok(())
}

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
fn print_sync_status(snaprag: &SnapRag) -> Result<()> {
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

            if let Some(shard) = lock.progress.current_shard {
                println!("  - Current shard: {}", shard);
            }
            if let Some(block) = lock.progress.current_block {
                println!("  - Current block: {}", block);
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
        }
        None => {
            println!("  - No active sync process");
        }
    }
    Ok(())
}
