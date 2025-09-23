use clap::{Parser, Subcommand};
use snaprag::config::AppConfig;
use snaprag::database::Database;
use snaprag::sync::service::SyncService;
use snaprag::Result;
use std::sync::Arc;
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
    /// List FIDs from the database
    List {
        /// The type of data to list
        #[arg(value_enum)]
        data_type: DataType,
        /// Maximum number of records to return
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },
    /// Clear all synchronized data from the database
    Clear {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Synchronization commands
    #[command(subcommand)]
    Sync(SyncCommands),
    /// Show current configuration
    Config,
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Run all sync (historical + real-time)
    All,
    /// Start synchronization
    Start,
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
        Commands::List { data_type, limit } => {
            handle_list_command(&db, data_type, limit).await?;
        }
        Commands::Clear { force } => {
            handle_clear_command(&db, force).await?;
        }
        Commands::Sync(sync_command) => {
            handle_sync_command(&db, sync_command).await?;
        }
        Commands::Config => {
            handle_config_command(&config).await?;
        }
    }

    Ok(())
}

async fn handle_list_command(db: &Database, data_type: DataType, limit: u32) -> Result<()> {
    match data_type {
        DataType::Fid => {
            println!("📋 Listing FIDs (limit: {})", limit);
            // Get FIDs from user profiles
            let profiles = db
                .list_user_profiles(snaprag::models::UserProfileQuery {
                    fid: None,
                    username: None,
                    display_name: None,
                    limit: Some(limit as i64),
                    offset: None,
                    start_timestamp: None,
                    end_timestamp: None,
                })
                .await?;
            println!("Found {} FIDs:", profiles.len());
            for profile in profiles {
                println!("  - FID: {}", profile.fid);
            }
        }
        DataType::Profiles => {
            println!("👤 Listing user profiles (limit: {})", limit);
            let profiles = db
                .list_user_profiles(snaprag::models::UserProfileQuery {
                    fid: None,
                    username: None,
                    display_name: None,
                    limit: Some(limit as i64),
                    offset: None,
                    start_timestamp: None,
                    end_timestamp: None,
                })
                .await?;
            println!("Found {} profiles:", profiles.len());
            for profile in profiles {
                println!(
                    "  - FID: {}, Username: {:?}, Display: {:?}",
                    profile.fid, profile.username, profile.display_name
                );
            }
        }
        DataType::Casts => {
            println!("💬 Listing casts (limit: {})", limit);
            // Note: This would need to be implemented in the database module
            println!("Cast listing not yet implemented");
        }
        DataType::Follows => {
            println!("👥 Listing follows (limit: {})", limit);
            // Note: This would need to be implemented in the database module
            println!("Follow listing not yet implemented");
        }
    }
    Ok(())
}

async fn handle_clear_command(db: &Database, force: bool) -> Result<()> {
    if !force {
        println!("⚠️  This will clear ALL synchronized data from the database!");
        println!("Are you sure you want to continue? (y/N)");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    println!("🧹 Clearing all synchronized data...");

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

    println!("✅ Database cleared successfully!");
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
        SyncCommands::Start => {
            println!("🚀 Starting synchronization...");
            let sync_service = SyncService::new(&config, db_arc).await?;
            // For now, just start the service which does historical sync by default
            sync_service.start().await?;
        }
        SyncCommands::Realtime => {
            println!("⚡ Starting real-time synchronization...");
            let sync_service = SyncService::new(&config, db_arc).await?;
            // For now, just start the service
            sync_service.start().await?;
        }
        SyncCommands::Status => {
            println!("📊 Sync Status:");
            // This would need to be implemented to show actual sync status
            println!("  - Historical sync: Not implemented");
            println!("  - Real-time sync: Not implemented");
            println!("  - Last sync time: Not implemented");
        }
        SyncCommands::Stop { force } => {
            println!("🛑 Stopping sync processes...");
            if force {
                println!("  - Force stopping (not implemented)");
            } else {
                println!("  - Graceful stop (not implemented)");
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
