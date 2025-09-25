//! CLI command definitions and argument parsing

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Parser)]
#[command(name = "snaprag")]
#[command(about = "SnapRAG CLI tool for database queries and data synchronization")]
#[command(version)]
pub struct Cli {
    /// Enable verbose debug logging
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
pub enum SyncCommands {
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

#[derive(ValueEnum, Clone)]
pub enum DataType {
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
