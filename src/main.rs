use std::sync::Arc;

use clap::Parser;
use clap::Subcommand;
use snaprag::config::AppConfig;
use snaprag::database::Database;
use snaprag::sync::service::SyncService;
use snaprag::Result;
use tracing::info;

#[derive(Parser)]
#[command(name = "snaprag")]
#[command(about = "SnapRAG CLI tool for database queries and data synchronization")]
#[command(version)]
struct Cli {
    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List data from the database
    List {
        /// The type of data to list
        #[arg(value_enum)]
        data_type: DataType,
        /// Maximum number of records to return
        #[arg(short, long, default_value = "100")]
        limit: u32,
        /// Search term for filtering
        #[arg(short, long)]
        search: Option<String>,
        /// Sort by field
        #[arg(long)]
        sort_by: Option<String>,
        /// Sort order (asc/desc)
        #[arg(long, default_value = "desc")]
        sort_order: String,
        /// Filter by FID range (min-max)
        #[arg(long)]
        fid_range: Option<String>,
        /// Filter by username
        #[arg(long)]
        username: Option<String>,
        /// Filter by display name
        #[arg(long)]
        display_name: Option<String>,
        /// Filter by bio content
        #[arg(long)]
        bio: Option<String>,
        /// Filter by location
        #[arg(long)]
        location: Option<String>,
        /// Filter by Twitter username
        #[arg(long)]
        twitter: Option<String>,
        /// Filter by GitHub username
        #[arg(long)]
        github: Option<String>,
        /// Show only profiles with username
        #[arg(long)]
        has_username: bool,
        /// Show only profiles with display name
        #[arg(long)]
        has_display_name: bool,
        /// Show only profiles with bio
        #[arg(long)]
        has_bio: bool,
    },
    /// Reset all synchronized data from the database and remove lock files
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Synchronization commands
    #[command(subcommand)]
    Sync(SyncCommands),
    /// Show statistics and analytics
    Stats {
        /// Show detailed statistics
        #[arg(short, long)]
        detailed: bool,
        /// Export statistics to JSON
        #[arg(short, long)]
        export: Option<String>,
    },
    /// Search profiles with advanced filters
    Search {
        /// Search term
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: u32,
        /// Search in specific fields (username,display_name,bio,all)
        #[arg(long, default_value = "all")]
        fields: String,
    },
    /// Show dashboard with key metrics
    Dashboard,
    /// Show current configuration
    Config,
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Run all sync (historical + real-time)
    All,
    /// Start synchronization
    Start {
        /// Start block number (default: 0)
        #[arg(long)]
        from: Option<u64>,
        /// End block number (default: latest)
        #[arg(long)]
        to: Option<u64>,
    },
    /// Test single block synchronization
    Test {
        /// Shard ID to test
        #[arg(long, default_value = "1")]
        shard: u32,
        /// Block number to test
        #[arg(long)]
        block: u64,
    },
    /// Run real-time sync only
    Realtime,
    /// Show sync status and statistics
    Status,
    /// Stop all running sync processes
    Stop {
        /// Force kill processes without graceful shutdown
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum DataType {
    /// List FIDs (user IDs)
    Fid,
    /// List user profiles
    Profiles,
    /// List casts
    Casts,
    /// List follows
    Follows,
    /// List user data
    UserData,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        snaprag::logging::init_logging_with_level("debug")?;
    } else {
        snaprag::logging::init_logging()?;
    }

    // Load configuration
    let config = AppConfig::load()?;
    info!("Configuration loaded successfully");

    // Create database connection
    let pool = sqlx::PgPool::connect(&config.database_url()).await?;
    let db = Database::new(pool);

    // Initialize database schema
    db.init_schema().await?;
    info!("Database schema initialized");

    // Execute the requested command
    match cli.command {
        Commands::List {
            data_type,
            limit,
            search,
            sort_by,
            sort_order,
            fid_range,
            username,
            display_name,
            bio,
            location,
            twitter,
            github,
            has_username,
            has_display_name,
            has_bio,
        } => {
            handle_list_command(
                &db,
                data_type,
                limit,
                search,
                sort_by,
                sort_order,
                fid_range,
                username,
                display_name,
                bio,
                location,
                twitter,
                github,
                has_username,
                has_display_name,
                has_bio,
            )
            .await?;
        }
        Commands::Reset { force } => {
            handle_clear_command(&db, force).await?;
        }
        Commands::Sync(sync_command) => {
            handle_sync_command(&db, sync_command).await?;
        }
        Commands::Stats { detailed, export } => {
            handle_stats_command(&db, detailed, export).await?;
        }
        Commands::Search {
            query,
            limit,
            fields,
        } => {
            handle_search_command(&db, query, limit, fields).await?;
        }
        Commands::Dashboard => {
            handle_dashboard_command(&db).await?;
        }
        Commands::Config => {
            handle_config_command(&config).await?;
        }
    }

    Ok(())
}

async fn handle_list_command(
    db: &Database,
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
            println!("📋 Listing FIDs (limit: {})", limit);

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
                Some("fid") => Some(snaprag::models::FidSortBy::Fid),
                Some("username") => Some(snaprag::models::FidSortBy::Username),
                Some("last_updated") => Some(snaprag::models::FidSortBy::LastUpdated),
                Some("created_at") => Some(snaprag::models::FidSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(snaprag::models::SortOrder::Asc),
                "desc" => Some(snaprag::models::SortOrder::Desc),
                _ => Some(snaprag::models::SortOrder::Asc),
            };

            // Build FID query
            let fid_query = snaprag::models::FidQuery {
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

            let profiles = db.list_fids(fid_query).await?;
            println!("Found {} FIDs:", profiles.len());
            for profile in profiles {
                println!(
                    "  - FID: {} | Username: {} | Display: {}",
                    profile.fid,
                    profile.username.as_deref().unwrap_or("N/A"),
                    profile.display_name.as_deref().unwrap_or("N/A")
                );
            }
        }
        DataType::Profiles => {
            println!("👤 Listing user profiles (limit: {})", limit);

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(snaprag::models::ProfileSortBy::Fid),
                Some("username") => Some(snaprag::models::ProfileSortBy::Username),
                Some("display_name") => Some(snaprag::models::ProfileSortBy::DisplayName),
                Some("last_updated") => Some(snaprag::models::ProfileSortBy::LastUpdated),
                Some("created_at") => Some(snaprag::models::ProfileSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(snaprag::models::SortOrder::Asc),
                "desc" => Some(snaprag::models::SortOrder::Desc),
                _ => Some(snaprag::models::SortOrder::Desc),
            };

            // Build profile query
            let profile_query = snaprag::models::UserProfileQuery {
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

            let profiles = db.list_user_profiles(profile_query).await?;
            println!("Found {} profiles:", profiles.len());
            for profile in profiles {
                println!(
                    "  - FID: {}, Username: {:?}, Display: {:?}, Bio: {:?}",
                    profile.fid, profile.username, profile.display_name, profile.bio
                );
            }
        }
        DataType::Casts => {
            println!("💬 Listing casts (limit: {})", limit);

            // Build cast query
            let cast_query = snaprag::models::CastQuery {
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
                sort_by: Some(snaprag::models::CastSortBy::Timestamp),
                sort_order: Some(snaprag::models::SortOrder::Desc),
            };

            let casts = db.list_casts(cast_query).await?;
            println!("Found {} casts:", casts.len());
            for cast in casts {
                let text_preview = cast
                    .text
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(100)
                    .collect::<String>();
                let text_display = if text_preview.len() >= 100 {
                    format!("{}...", text_preview)
                } else {
                    text_preview
                };
                println!(
                    "  - FID: {} | Text: {} | Timestamp: {}",
                    cast.fid, text_display, cast.timestamp
                );
            }
        }
        DataType::Follows => {
            println!("👥 Listing follows (limit: {})", limit);

            // Build link query for follows
            let link_query = snaprag::models::LinkQuery {
                fid: None,
                target_fid: None,
                link_type: Some("follow".to_string()),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(snaprag::models::LinkSortBy::Timestamp),
                sort_order: Some(snaprag::models::SortOrder::Desc),
            };

            let links = db.list_links(link_query).await?;
            println!("Found {} follow relationships:", links.len());
            for link in links {
                println!(
                    "  - FID: {} -> Target: {} | Type: {} | Timestamp: {}",
                    link.fid, link.target_fid, link.link_type, link.timestamp
                );
            }
        }
        DataType::UserData => {
            println!("📊 Listing user data (limit: {})", limit);

            // Build user data query
            let user_data_query = snaprag::models::UserDataQuery {
                fid: None,
                data_type: None, // TODO: Parse data_type from search if needed
                value_search: search.clone(),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(snaprag::models::UserDataSortBy::Timestamp),
                sort_order: Some(snaprag::models::SortOrder::Desc),
            };

            let user_data = db.list_user_data(user_data_query).await?;
            println!("Found {} user data records:", user_data.len());
            for data in user_data {
                println!(
                    "  - FID: {} | Type: {} | Value: {} | Timestamp: {}",
                    data.fid, data.data_type, data.value, data.timestamp
                );
            }
        }
    }
    Ok(())
}

async fn handle_clear_command(db: &Database, force: bool) -> Result<()> {
    if !force {
        println!(
            "⚠️  This will reset ALL synchronized data from the database and remove lock files!"
        );
        println!("Are you sure you want to continue? (y/N)");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    println!("🔄 Resetting all synchronized data and lock files...");

    // Remove lock file if it exists
    if std::path::Path::new("snaprag.lock").exists() {
        std::fs::remove_file("snaprag.lock")?;
        println!("  - Removed snaprag.lock file");
    } else {
        println!("  - No lock file found");
    }

    // Clear user profiles
    let deleted_profiles = sqlx::query("DELETE FROM user_profiles")
        .execute(db.pool())
        .await?;
    println!(
        "  - Deleted {} user profiles",
        deleted_profiles.rows_affected()
    );

    // Clear username proofs
    let deleted_proofs = sqlx::query("DELETE FROM username_proofs")
        .execute(db.pool())
        .await?;
    println!(
        "  - Deleted {} username proofs",
        deleted_proofs.rows_affected()
    );

    // Clear user activities
    let deleted_activities = sqlx::query("DELETE FROM user_activities")
        .execute(db.pool())
        .await?;
    println!(
        "  - Deleted {} user activities",
        deleted_activities.rows_affected()
    );

    // Clear user data changes
    let deleted_changes = sqlx::query("DELETE FROM user_data_changes")
        .execute(db.pool())
        .await?;
    println!(
        "  - Deleted {} user data changes",
        deleted_changes.rows_affected()
    );

    // Clear casts
    let deleted_casts = sqlx::query("DELETE FROM casts").execute(db.pool()).await?;
    println!("  - Deleted {} casts", deleted_casts.rows_affected());

    // Clear links
    let deleted_links = sqlx::query("DELETE FROM links").execute(db.pool()).await?;
    println!("  - Deleted {} links", deleted_links.rows_affected());

    // Clear user_data
    let deleted_user_data = sqlx::query("DELETE FROM user_data")
        .execute(db.pool())
        .await?;
    println!(
        "  - Deleted {} user data records",
        deleted_user_data.rows_affected()
    );

    println!("✅ Database and lock files reset successfully!");
    Ok(())
}

async fn handle_sync_command(db: &Database, sync_command: SyncCommands) -> Result<()> {
    // Load configuration for sync service
    let config = AppConfig::load()?;
    let db_arc = Arc::new(db.clone());

    match sync_command {
        SyncCommands::All => {
            println!("🔄 Starting full synchronization (historical + real-time)...");
            let sync_service = SyncService::new(&config, db_arc).await?;
            sync_service.start().await?;
        }
        SyncCommands::Start { from, to } => {
            let from_block = from.unwrap_or(0);
            let to_block = to.unwrap_or(u64::MAX);

            if let Some(to_val) = to {
                println!(
                    "🚀 Starting synchronization from block {} to block {}...",
                    from_block, to_val
                );
            } else {
                println!(
                    "🚀 Starting synchronization from block {} to latest...",
                    from_block
                );
            }

            let sync_service = SyncService::new(&config, db_arc).await?;
            sync_service.start_with_range(from_block, to_block).await?;
        }
        SyncCommands::Test { shard, block } => {
            println!(
                "🧪 Testing single block synchronization for shard {} block {}...",
                shard, block
            );

            let sync_service = SyncService::new(&config, db_arc).await?;
            match sync_service.poll_once(shard, block).await {
                Ok(()) => {
                    println!("✅ Single block test completed successfully!");
                }
                Err(e) => {
                    println!("❌ Single block test failed: {}", e);
                    return Err(e);
                }
            }
        }
        SyncCommands::Realtime => {
            println!("⚡ Starting real-time synchronization...");
            let sync_service = SyncService::new(&config, db_arc).await?;
            // For now, just start the service
            sync_service.start().await?;
        }
        SyncCommands::Status => {
            println!("📊 Sync Status:");
            let sync_service = SyncService::new(&config, db_arc).await?;

            match sync_service.get_sync_status()? {
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
        }
        SyncCommands::Stop { force } => {
            println!("🛑 Stopping sync processes...");
            let sync_service = SyncService::new(&config, db_arc).await?;
            sync_service.stop(force).await?;

            if force {
                println!("  - Force stopped successfully");
            } else {
                println!("  - Gracefully stopped successfully");
            }
        }
    }
    Ok(())
}

async fn handle_config_command(config: &AppConfig) -> Result<()> {
    println!("📋 SnapRAG Configuration:");
    println!();

    println!("🗄️  Database:");
    println!("  URL: {}", mask_database_url(config.database_url()));
    println!("  Max connections: {}", config.max_connections());
    println!("  Min connections: {}", config.min_connections());
    println!("  Connection timeout: {}s", config.connection_timeout());
    println!();

    println!("📝 Logging:");
    println!("  Level: {}", config.logging.level);
    println!("  Backtrace: {}", config.logging.backtrace);
    println!();

    println!("🧠 Embeddings:");
    println!("  Dimension: {}", config.embedding_dimension());
    println!("  Model: {}", config.embedding_model());
    println!();

    println!("⚡ Performance:");
    println!("  Vector indexes: {}", config.vector_indexes_enabled());
    println!("  Vector index lists: {}", config.vector_index_lists());
    println!();

    println!("🔄 Sync:");
    println!("  HTTP endpoint: {}", config.snapchain_http_endpoint());
    println!("  gRPC endpoint: {}", config.snapchain_grpc_endpoint());
    println!("  Real-time sync: {}", config.realtime_sync_enabled());
    println!("  Historical sync: {}", config.historical_sync_enabled());
    println!(
        "  Historical sync from event ID: {}",
        config.historical_sync_from_event_id()
    );
    println!("  Batch size: {}", config.sync_batch_size());
    println!("  Sync interval: {}ms", config.sync_interval_ms());
    println!("  Shard IDs: {:?}", config.shard_ids());

    Ok(())
}

/// Mask database URL for logging (hide password)
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            format!(
                "{}://{}@{}:{}",
                parsed.scheme(),
                parsed.username(),
                host,
                parsed.port().unwrap_or(5432)
            )
        } else {
            "***masked***".to_string()
        }
    } else {
        "***invalid***".to_string()
    }
}

async fn handle_stats_command(db: &Database, detailed: bool, export: Option<String>) -> Result<()> {
    println!("📊 SnapRAG Statistics");
    println!("===================");

    let stats_query = snaprag::models::StatisticsQuery {
        start_date: None,
        end_date: None,
        group_by: None,
    };

    let stats = db.get_statistics(stats_query).await?;

    println!();
    println!("📈 Overview:");
    println!("  Total FIDs: {}", stats.total_fids);
    println!("  Total Profiles: {}", stats.total_profiles);

    println!();
    println!("👤 Profile Completeness:");
    println!(
        "  With Username: {} ({:.1}%)",
        stats.profiles_with_username,
        if stats.total_profiles > 0 {
            (stats.profiles_with_username as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Display Name: {} ({:.1}%)",
        stats.profiles_with_display_name,
        if stats.total_profiles > 0 {
            (stats.profiles_with_display_name as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Bio: {} ({:.1}%)",
        stats.profiles_with_bio,
        if stats.total_profiles > 0 {
            (stats.profiles_with_bio as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  With Profile Picture: {} ({:.1}%)",
        stats.profiles_with_pfp,
        if stats.total_profiles > 0 {
            (stats.profiles_with_pfp as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );

    if detailed {
        println!();
        println!("🔗 Social Links:");
        println!(
            "  With Website: {} ({:.1}%)",
            stats.profiles_with_website,
            if stats.total_profiles > 0 {
                (stats.profiles_with_website as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Twitter: {} ({:.1}%)",
            stats.profiles_with_twitter,
            if stats.total_profiles > 0 {
                (stats.profiles_with_twitter as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With GitHub: {} ({:.1}%)",
            stats.profiles_with_github,
            if stats.total_profiles > 0 {
                (stats.profiles_with_github as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Ethereum Address: {} ({:.1}%)",
            stats.profiles_with_ethereum_address,
            if stats.total_profiles > 0 {
                (stats.profiles_with_ethereum_address as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );
        println!(
            "  With Solana Address: {} ({:.1}%)",
            stats.profiles_with_solana_address,
            if stats.total_profiles > 0 {
                (stats.profiles_with_solana_address as f64 / stats.total_profiles as f64) * 100.0
            } else {
                0.0
            }
        );

        println!();
        println!("🆕 Recent Registrations:");
        for reg in &stats.recent_registrations {
            println!(
                "  - FID: {} | Username: {} | Display: {} | Created: {}",
                reg.fid,
                reg.username.as_deref().unwrap_or("N/A"),
                reg.display_name.as_deref().unwrap_or("N/A"),
                reg.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }
    }

    if let Some(export_path) = export {
        let json = serde_json::to_string_pretty(&stats)?;
        std::fs::write(&export_path, json)?;
        println!();
        println!("📁 Statistics exported to: {}", export_path);
    }

    Ok(())
}

async fn handle_search_command(
    db: &Database,
    query: String,
    limit: u32,
    fields: String,
) -> Result<()> {
    println!("🔍 Searching profiles for: \"{}\"", query);
    println!("Fields: {}", fields);
    println!();

    // Build search query based on fields
    let search_query = snaprag::models::UserProfileQuery {
        fid: None,
        username: if fields == "all" || fields == "username" {
            Some(query.clone())
        } else {
            None
        },
        display_name: if fields == "all" || fields == "display_name" {
            Some(query.clone())
        } else {
            None
        },
        bio: if fields == "all" || fields == "bio" {
            Some(query.clone())
        } else {
            None
        },
        location: None,
        twitter_username: None,
        github_username: None,
        limit: Some(limit as i64),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
        sort_by: Some(snaprag::models::ProfileSortBy::LastUpdated),
        sort_order: Some(snaprag::models::SortOrder::Desc),
        search_term: if fields == "all" { Some(query) } else { None },
    };

    let profiles = db.list_user_profiles(search_query).await?;
    println!("Found {} profiles:", profiles.len());

    for profile in profiles {
        println!();
        println!("  🆔 FID: {}", profile.fid);
        if let Some(username) = &profile.username {
            println!("  👤 Username: {}", username);
        }
        if let Some(display_name) = &profile.display_name {
            println!("  📝 Display Name: {}", display_name);
        }
        if let Some(bio) = &profile.bio {
            println!("  📄 Bio: {}", bio);
        }
        if let Some(location) = &profile.location {
            println!("  📍 Location: {}", location);
        }
        if let Some(twitter) = &profile.twitter_username {
            println!("  🐦 Twitter: @{}", twitter);
        }
        if let Some(github) = &profile.github_username {
            println!("  🐙 GitHub: @{}", github);
        }
        println!(
            "  🕒 Last Updated: {}",
            profile.last_updated_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    Ok(())
}

async fn handle_dashboard_command(db: &Database) -> Result<()> {
    println!("📊 SnapRAG Dashboard");
    println!("===================");

    let stats_query = snaprag::models::StatisticsQuery {
        start_date: None,
        end_date: None,
        group_by: None,
    };

    let stats = db.get_statistics(stats_query).await?;

    println!();
    println!("🎯 Key Metrics:");
    println!("  Total Users: {}", stats.total_fids);
    println!(
        "  Complete Profiles: {} ({:.1}%)",
        stats.profiles_with_username,
        if stats.total_profiles > 0 {
            (stats.profiles_with_username as f64 / stats.total_profiles as f64) * 100.0
        } else {
            0.0
        }
    );

    println!();
    println!("📈 Profile Health:");
    println!("  ✅ With Username: {}", stats.profiles_with_username);
    println!(
        "  ✅ With Display Name: {}",
        stats.profiles_with_display_name
    );
    println!("  ✅ With Bio: {}", stats.profiles_with_bio);
    println!("  ✅ With Profile Picture: {}", stats.profiles_with_pfp);

    println!();
    println!("🔗 Social Presence:");
    println!("  🌐 With Website: {}", stats.profiles_with_website);
    println!("  🐦 With Twitter: {}", stats.profiles_with_twitter);
    println!("  🐙 With GitHub: {}", stats.profiles_with_github);
    println!(
        "  💰 With Ethereum: {}",
        stats.profiles_with_ethereum_address
    );
    println!("  💰 With Solana: {}", stats.profiles_with_solana_address);

    println!();
    println!("🆕 Recent Activity:");
    for (i, reg) in stats.recent_registrations.iter().take(5).enumerate() {
        println!(
            "  {}. FID: {} | @{} | {}",
            i + 1,
            reg.fid,
            reg.username.as_deref().unwrap_or("N/A"),
            reg.display_name.as_deref().unwrap_or("N/A")
        );
    }

    Ok(())
}
