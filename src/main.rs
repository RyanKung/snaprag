use clap::Parser;
use snaprag::cli::Cli;
use snaprag::cli::Commands;
use snaprag::cli::SyncCommands;
use snaprag::AppConfig;
use snaprag::Result;
use snaprag::SnapRag;
use tracing::info;

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

    // Create SnapRAG instance
    let snaprag = SnapRag::new(&config).await?;

    // Initialize database schema
    snaprag.init_database().await?;
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
            snaprag::cli::handle_list_command(
                &snaprag,
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
            snaprag::cli::handle_reset_command(&snaprag, force).await?;
        }
        Commands::Sync(sync_command) => {
            snaprag::cli::handle_sync_command(snaprag, sync_command).await?;
        }
        Commands::Stats { detailed, export } => {
            snaprag::cli::handle_stats_command(&snaprag, detailed, export).await?;
        }
        Commands::Search {
            query,
            limit,
            fields,
        } => {
            snaprag::cli::handle_search_command(&snaprag, query, limit, fields).await?;
        }
        Commands::Dashboard => {
            snaprag::cli::handle_dashboard_command(&snaprag).await?;
        }
        Commands::Config => {
            snaprag::cli::handle_config_command(&config).await?;
        }
    }

    Ok(())
}
