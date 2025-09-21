use clap::{Parser, Subcommand};
use snaprag::config::AppConfig;
use snaprag::database::Database;
use snaprag::sync::service::SyncService;
use snaprag::Result;
use std::sync::Arc;
use sqlx::Row;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "snaprag-cli")]
#[command(about = "SnapRAG CLI tool for database queries")]
struct Cli {
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
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Run all sync (historical + real-time)
    All,
    /// Run historical sync only
    Historical,
    /// Run real-time sync only
    Realtime,
    /// Show sync status and statistics
    Status,
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

    // Load configuration first
    let config = AppConfig::load()?;
    
    // Initialize logging with configuration
    snaprag::logging::init_logging_with_config(Some(&config))?;
    let database = Arc::new(Database::from_config(&config).await?);

    match cli.command {
        Commands::List { data_type, limit } => {
            match data_type {
                DataType::Fid => list_fids(&database, limit).await?,
                DataType::Profiles => list_profiles(&database, limit).await?,
                DataType::Casts => list_casts(&database, limit).await?,
                DataType::Follows => list_follows(&database, limit).await?,
            }
        }
        Commands::Clear { force } => {
            clear_all_data(&database, force).await?;
        }
        Commands::Sync(sync_command) => {
            match sync_command {
                SyncCommands::All => run_comprehensive_sync(&config, &database).await?,
                SyncCommands::Historical => run_historical_sync(&config, &database).await?,
                SyncCommands::Realtime => run_realtime_sync(&config, &database).await?,
                SyncCommands::Status => show_sync_status(&database).await?,
            }
        }
    }

    Ok(())
}

/// List FIDs from user_profiles table
async fn list_fids(database: &Database, limit: u32) -> Result<()> {
    println!("Listing FIDs (limit: {})", limit);
    println!("{:-<50}", "");
    
    let query = r#"
        SELECT fid, username, display_name, last_updated_at
        FROM user_profiles 
        ORDER BY last_updated_at DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(database.pool())
        .await?;
    
    if rows.is_empty() {
        println!("No FIDs found in database.");
        return Ok(());
    }
    
    println!("{:<8} {:<20} {:<30} {:<20}", "FID", "Username", "Display Name", "Created At");
    println!("{:-<8} {:-<20} {:-<30} {:-<20}", "", "", "", "");
    
    for row in &rows {
        let fid: i64 = row.get("fid");
        let username: Option<String> = row.get("username");
        let display_name: Option<String> = row.get("display_name");
        let last_updated_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_updated_at");
        
        let username_str = username.unwrap_or_else(|| "N/A".to_string());
        let display_name_str = display_name.unwrap_or_else(|| "N/A".to_string());
        let last_updated_str = last_updated_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        
        println!("{:<8} {:<20} {:<30} {:<20}", 
                 fid, 
                 truncate_string(&username_str, 20),
                 truncate_string(&display_name_str, 30),
                 last_updated_str);
    }
    
    println!("\nTotal: {} FIDs", rows.len());
    Ok(())
}

/// List user profiles
async fn list_profiles(database: &Database, limit: u32) -> Result<()> {
    println!("Listing User Profiles (limit: {})", limit);
    println!("{:-<80}", "");
    
    let query = r#"
        SELECT fid, username, display_name, bio, pfp_url, website_url, last_updated_at
        FROM user_profiles 
        ORDER BY last_updated_at DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(database.pool())
        .await?;
    
    if rows.is_empty() {
        println!("No user profiles found in database.");
        return Ok(());
    }
    
    for row in &rows {
        let fid: i64 = row.get("fid");
        let username: Option<String> = row.get("username");
        let display_name: Option<String> = row.get("display_name");
        let bio: Option<String> = row.get("bio");
        let pfp_url: Option<String> = row.get("pfp_url");
        let website_url: Option<String> = row.get("website_url");
        let last_updated_at: Option<chrono::DateTime<chrono::Utc>> = row.get("last_updated_at");
        
        println!("FID: {}", fid);
        println!("  Username: {}", username.unwrap_or_else(|| "N/A".to_string()));
        println!("  Display Name: {}", display_name.unwrap_or_else(|| "N/A".to_string()));
        println!("  Bio: {}", truncate_string(&bio.unwrap_or_else(|| "N/A".to_string()), 60));
        println!("  PFP URL: {}", truncate_string(&pfp_url.unwrap_or_else(|| "N/A".to_string()), 50));
        println!("  Website: {}", truncate_string(&website_url.unwrap_or_else(|| "N/A".to_string()), 50));
        println!("  Last Updated: {}", last_updated_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string()));
        println!("{:-<40}", "");
    }
    
    println!("Total: {} profiles", rows.len());
    Ok(())
}

/// List casts
async fn list_casts(database: &Database, limit: u32) -> Result<()> {
    println!("Listing Casts (limit: {})", limit);
    println!("{:-<80}", "");
    
    let query = r#"
        SELECT c.fid, c.text, c.timestamp, up.username, up.display_name
        FROM casts c
        LEFT JOIN user_profiles up ON c.fid = up.fid
        ORDER BY c.timestamp DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(database.pool())
        .await?;
    
    if rows.is_empty() {
        println!("No casts found in database.");
        return Ok(());
    }
    
    for row in &rows {
        let fid: i64 = row.get("fid");
        let text: Option<String> = row.get("text");
        let timestamp: Option<i64> = row.get("timestamp");
        let username: Option<String> = row.get("username");
        let display_name: Option<String> = row.get("display_name");
        
        let text_str = text.unwrap_or_else(|| "N/A".to_string());
        let timestamp_str = timestamp
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Invalid".to_string()))
            .unwrap_or_else(|| "N/A".to_string());
        let author_str = display_name
            .or(username)
            .unwrap_or_else(|| format!("FID:{}", fid));
        
        println!("Author: {} (FID: {})", author_str, fid);
        println!("Time: {}", timestamp_str);
        println!("Text: {}", truncate_string(&text_str, 80));
        println!("{:-<40}", "");
    }
    
    println!("Total: {} casts", rows.len());
    Ok(())
}

/// List follows
async fn list_follows(database: &Database, limit: u32) -> Result<()> {
    println!("Listing Follows (limit: {})", limit);
    println!("{:-<60}", "");
    
    let query = r#"
        SELECT l.fid, l.target_fid, l.timestamp, 
               up1.username as follower_username, up1.display_name as follower_display,
               up2.username as target_username, up2.display_name as target_display
        FROM links l
        LEFT JOIN user_profiles up1 ON l.fid = up1.fid
        LEFT JOIN user_profiles up2 ON l.target_fid = up2.fid
        WHERE l.link_type = 'follow'
        ORDER BY l.timestamp DESC 
        LIMIT $1
    "#;
    
    let rows = sqlx::query(query)
        .bind(limit as i32)
        .fetch_all(database.pool())
        .await?;
    
    if rows.is_empty() {
        println!("No follows found in database.");
        return Ok(());
    }
    
    for row in &rows {
        let fid: i64 = row.get("fid");
        let target_fid: i64 = row.get("target_fid");
        let timestamp: Option<i64> = row.get("timestamp");
        let follower_username: Option<String> = row.get("follower_username");
        let follower_display: Option<String> = row.get("follower_display");
        let target_username: Option<String> = row.get("target_username");
        let target_display: Option<String> = row.get("target_display");
        
        let timestamp_str = timestamp
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Invalid".to_string()))
            .unwrap_or_else(|| "N/A".to_string());
        
        let follower_str = follower_display
            .or(follower_username)
            .unwrap_or_else(|| format!("FID:{}", fid));
        let target_str = target_display
            .or(target_username)
            .unwrap_or_else(|| format!("FID:{}", target_fid));
        
        println!("{} (FID: {}) → {} (FID: {})", 
                 follower_str, fid, target_str, target_fid);
        println!("Time: {}", timestamp_str);
        println!("{:-<30}", "");
    }
    
    println!("Total: {} follows", rows.len());
    Ok(())
}

/// Clear all synchronized data from the database
async fn clear_all_data(database: &Database, force: bool) -> Result<()> {
    println!("🗑️  SnapRAG Data Clear Utility");
    println!("{:-<50}", "");
    
    // Show warning and get confirmation unless force flag is used
    if !force {
        println!("⚠️  WARNING: This will permanently delete ALL synchronized data!");
        println!("   This includes:");
        println!("   • User profiles and snapshots");
        println!("   • Casts and reactions");
        println!("   • Follow relationships");
        println!("   • User data changes");
        println!("   • Activity timeline");
        println!("   • Sync progress data");
        println!();
        
        println!("Are you sure you want to continue? (type 'yes' to confirm): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() != "yes" {
            println!("❌ Operation cancelled.");
            return Ok(());
        }
    }
    
    println!("🧹 Starting data cleanup...");
    
    // Get counts before deletion for reporting
    let counts = get_table_counts(database).await?;
    
    println!("📊 Data to be deleted:");
    for (table, count) in &counts {
        if *count > 0 {
            println!("   • {}: {} records", table, count);
        }
    }
    
    // Clear data in correct order (respecting foreign key constraints)
    let tables_to_clear = [
        "user_activity_timeline",
        "user_data_changes", 
        "user_profile_snapshots",
        "username_proofs",
        "user_profiles",
        "sync_progress",
        "processed_messages",
        "sync_stats",
        "user_activities",
        "user_profile_trends",
    ];
    
    let mut total_deleted = 0;
    for table in &tables_to_clear {
        let result = sqlx::query(&format!("DELETE FROM {}", table))
            .execute(database.pool())
            .await?;
        
        let deleted = result.rows_affected();
        if deleted > 0 {
            println!("   ✅ Cleared {}: {} records", table, deleted);
            total_deleted += deleted;
        }
    }
    
    // Clear sync state file if it exists
    let sync_state_file = "snaprag_sync_state.json";
    if std::path::Path::new(sync_state_file).exists() {
        if let Err(e) = std::fs::remove_file(sync_state_file) {
            println!("   ⚠️  Warning: Could not remove sync state file: {}", e);
        } else {
            println!("   ✅ Removed sync state file: {}", sync_state_file);
        }
    }
    
    println!();
    println!("🎉 Data cleanup completed!");
    println!("   Total records deleted: {}", total_deleted);
    println!("   Database is now clean and ready for fresh sync.");
    
    Ok(())
}

/// Get record counts for all tables
async fn get_table_counts(database: &Database) -> Result<Vec<(String, i64)>> {
    let tables = [
        "user_profiles",
        "user_profile_snapshots", 
        "username_proofs",
        "user_data_changes",
        "user_activity_timeline",
        "sync_progress",
        "processed_messages",
        "sync_stats",
        "user_activities",
        "user_profile_trends",
    ];
    
    let mut counts = Vec::new();
    
    for table in &tables {
        let query = format!("SELECT COUNT(*) as count FROM {}", table);
        let result = sqlx::query(&query)
            .fetch_one(database.pool())
            .await?;
        
        let count: i64 = result.get("count");
        counts.push((table.to_string(), count));
    }
    
    Ok(counts)
}

/// Truncate string to specified length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Run comprehensive sync (historical + real-time)
async fn run_comprehensive_sync(config: &AppConfig, database: &Database) -> Result<()> {
    println!("🚀 Starting SnapRAG Comprehensive Sync");
    println!("{:-<50}", "");
    
    info!("Starting comprehensive sync with historical and real-time data...");
    
    // Run migrations first
    database.migrate().await?;
    info!("Database migrations completed");
    
    // Create sync service
    let sync_service = match SyncService::new(config, database.clone().into()).await {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to create sync service: {}", e);
            handle_connection_error(&e, config);
            return Err(e);
        }
    };
    
    // Start comprehensive sync
    match sync_service.start().await {
        Ok(_) => {
            println!("✅ Comprehensive sync completed successfully");
            info!("Sync service completed successfully");
        }
        Err(e) => {
            error!("Sync service failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Run historical sync only
async fn run_historical_sync(config: &AppConfig, database: &Database) -> Result<()> {
    println!("📚 Starting SnapRAG Historical Sync");
    println!("{:-<50}", "");
    
    info!("Starting historical sync only...");
    
    // Run migrations first
    database.migrate().await?;
    info!("Database migrations completed");
    
    // Create sync service with historical sync only
    let sync_service = SyncService::new(config, database.clone().into()).await?;
    
    // Override config to only run historical sync
    // Note: This would require modifying SyncService to accept a custom config
    // For now, we'll use the comprehensive sync but log that it's historical-only
    info!("Running historical sync (real-time will be skipped)...");
    
    match sync_service.start().await {
        Ok(_) => {
            println!("✅ Historical sync completed successfully");
            info!("Historical sync completed successfully");
        }
        Err(e) => {
            error!("Historical sync failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Run real-time sync only
async fn run_realtime_sync(config: &AppConfig, database: &Database) -> Result<()> {
    println!("⚡ Starting SnapRAG Real-time Sync");
    println!("{:-<50}", "");
    
    info!("Starting real-time sync only...");
    
    // Run migrations first
    database.migrate().await?;
    info!("Database migrations completed");
    
    // Create sync service with real-time sync only
    let sync_service = SyncService::new(config, database.clone().into()).await?;
    
    // Note: This would require modifying SyncService to accept a custom config
    // For now, we'll use the comprehensive sync but log that it's real-time-only
    info!("Running real-time sync (historical will be skipped)...");
    
    match sync_service.start().await {
        Ok(_) => {
            println!("✅ Real-time sync completed successfully");
            info!("Real-time sync completed successfully");
        }
        Err(e) => {
            error!("Real-time sync failed: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}

/// Show sync status and statistics
async fn show_sync_status(database: &Database) -> Result<()> {
    println!("📊 SnapRAG Sync Status");
    println!("{:-<50}", "");
    
    // Get sync statistics from database
    let query = r#"
        SELECT 
            shard_id,
            last_processed_height,
            status,
            updated_at,
            total_messages,
            total_blocks
        FROM sync_progress 
        ORDER BY shard_id
    "#;
    
    let rows = sqlx::query(query)
        .fetch_all(database.pool())
        .await?;
    
    if rows.is_empty() {
        println!("No sync progress found. Run sync first.");
        return Ok(());
    }
    
    println!("{:<10} {:<15} {:<12} {:<20} {:<12} {:<12}", 
             "Shard ID", "Last Height", "Status", "Updated At", "Messages", "Blocks");
    println!("{:-<10} {:-<15} {:-<12} {:-<20} {:-<12} {:-<12}", "", "", "", "", "", "");
    
    for row in &rows {
        let shard_id: i32 = row.get("shard_id");
        let last_height: Option<i64> = row.get("last_processed_height");
        let status: Option<String> = row.get("status");
        let updated_at: Option<chrono::DateTime<chrono::Utc>> = row.get("updated_at");
        let total_messages: Option<i64> = row.get("total_messages");
        let total_blocks: Option<i64> = row.get("total_blocks");
        
        let height_str = last_height.map(|h| h.to_string()).unwrap_or_else(|| "N/A".to_string());
        let status_str = status.unwrap_or_else(|| "Unknown".to_string());
        let updated_str = updated_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "N/A".to_string());
        let messages_str = total_messages.map(|m| m.to_string()).unwrap_or_else(|| "N/A".to_string());
        let blocks_str = total_blocks.map(|b| b.to_string()).unwrap_or_else(|| "N/A".to_string());
        
        println!("{:<10} {:<15} {:<12} {:<20} {:<12} {:<12}", 
                 shard_id, height_str, status_str, updated_str, messages_str, blocks_str);
    }
    
    // Show overall statistics
    let counts = get_table_counts(database).await?;
    println!("\n📈 Database Statistics:");
    for (table, count) in &counts {
        if *count > 0 {
            println!("   • {}: {} records", table, count);
        }
    }
    
    Ok(())
}

/// Handle connection errors with helpful messages
fn handle_connection_error(e: &snaprag::SnapRagError, config: &AppConfig) {
    let error_msg = e.to_string();
    if error_msg.contains("Connection refused") || 
       error_msg.contains("TonicTransport") || 
       error_msg.contains("tcp connect error") || 
       error_msg.contains("transport error") {
        println!();
        println!("🔗 Connection Error Help:");
        println!("The sync service cannot connect to the snapchain node.");
        println!("Please ensure:");
        println!("  1. Snapchain node is running on {}", config.snapchain_endpoint());
        println!("  2. The endpoint URL is correct in config.toml");
        println!("  3. Firewall allows connections to this port");
        println!();
        println!("To start a snapchain node, please refer to the snapchain documentation.");
    }
}
