//! CLI command definitions and argument parsing

use clap::Parser;
use clap::Subcommand;
use clap::ValueEnum;

#[derive(Parser)]
#[command(name = "snaprag")]
#[command(about = "SnapRAG CLI tool for database queries and data synchronization")]
#[command(version)]
pub struct Cli {
    /// Enable verbose debug logging (default: info level)
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize database schema and indexes
    Init {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Skip creating indexes (faster initialization, add indexes later)
        #[arg(long)]
        skip_indexes: bool,
    },
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
    /// Query user activity timeline by FID
    Activity {
        /// Farcaster ID to query
        fid: i64,
        /// Maximum number of activities to return
        #[arg(short, long, default_value = "50")]
        limit: i64,
        /// Skip first N activities
        #[arg(short, long, default_value = "0")]
        offset: i64,
        /// Filter by activity type (cast_add, reaction_add, link_add, etc.)
        #[arg(short = 't', long)]
        activity_type: Option<String>,
        /// Show detailed JSON data
        #[arg(short, long)]
        detailed: bool,
    },
    /// Cast commands
    #[command(subcommand)]
    Cast(CastCommands),
    /// RAG (Retrieval-Augmented Generation) commands
    #[command(subcommand)]
    Rag(RagCommands),
    /// Embeddings generation commands
    #[command(subcommand)]
    Embeddings(EmbeddingsCommands),
    /// Serve API commands
    #[command(subcommand)]
    Serve(ServeCommands),
    /// Fetch user data on-demand (lazy loading)
    #[command(subcommand)]
    Fetch(FetchCommands),
}

#[derive(Subcommand)]
pub enum FetchCommands {
    /// Fetch single user profile and optionally their casts
    User {
        /// FID to fetch
        fid: u64,
        /// Also fetch user's casts
        #[arg(long)]
        with_casts: bool,
        /// Maximum number of casts to fetch
        #[arg(long, default_value = "1000")]
        max_casts: usize,
        /// Generate embeddings for fetched casts
        #[arg(long)]
        generate_embeddings: bool,
        /// Embedding endpoint to use (from config)
        #[arg(long)]
        embedding_endpoint: Option<String>,
    },
    /// Batch fetch multiple users
    Users {
        /// Comma-separated FIDs (e.g., "99,100,101")
        fids: String,
        /// Also fetch their casts
        #[arg(long)]
        with_casts: bool,
        /// Generate embeddings for fetched casts
        #[arg(long)]
        generate_embeddings: bool,
        /// Embedding endpoint to use
        #[arg(long)]
        embedding_endpoint: Option<String>,
    },
    /// Preload popular users (top N by activity)
    Popular {
        /// Number of popular users to preload
        #[arg(short, long, default_value = "100")]
        limit: usize,
        /// Also fetch their casts
        #[arg(long)]
        with_casts: bool,
        /// Generate embeddings for fetched casts
        #[arg(long)]
        generate_embeddings: bool,
        /// Embedding endpoint to use
        #[arg(long)]
        embedding_endpoint: Option<String>,
    },
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
        /// Shard IDs to sync (comma-separated, e.g., "1,2")
        #[arg(long)]
        shard: Option<String>,
        /// Batch size for fetching blocks (default: from config)
        #[arg(long)]
        batch: Option<u32>,
        /// Sync interval in milliseconds (default: from config)
        #[arg(long)]
        interval: Option<u64>,
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

#[derive(Subcommand)]
pub enum CastCommands {
    /// Search casts by semantic similarity
    Search {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Minimum similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        threshold: f32,
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },
    /// Get recent casts by FID
    Recent {
        /// FID to query
        fid: i64,
        /// Maximum number of casts
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Show cast thread (conversation)
    Thread {
        /// Cast hash (hex)
        hash: String,
        /// Maximum depth
        #[arg(short, long, default_value = "10")]
        depth: usize,
    },
}

#[derive(Subcommand)]
pub enum RagCommands {
    /// Execute a RAG query (profiles)
    Query {
        /// The question to ask
        query: String,
        /// Maximum number of profiles to retrieve
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Retrieval method (semantic, keyword, hybrid, auto)
        #[arg(short, long, default_value = "auto")]
        method: String,
        /// LLM temperature (0.0 - 1.0)
        #[arg(long, default_value = "0.7")]
        temperature: f32,
        /// Maximum tokens for response
        #[arg(long, default_value = "2000")]
        max_tokens: usize,
        /// Show detailed sources
        #[arg(short, long)]
        verbose: bool,
    },
    /// RAG query on cast content
    QueryCasts {
        /// The question to ask
        query: String,
        /// Maximum number of casts to retrieve
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Minimum similarity threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        threshold: f32,
        /// LLM temperature (0.0 - 1.0)
        #[arg(long, default_value = "0.7")]
        temperature: f32,
        /// Maximum tokens for response
        #[arg(long, default_value = "2000")]
        max_tokens: usize,
        /// Show detailed sources
        #[arg(short, long)]
        verbose: bool,
    },
    /// Search profiles without LLM generation
    Search {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Search method (semantic, keyword, hybrid, auto)
        #[arg(short, long, default_value = "auto")]
        method: String,
    },
}

#[derive(Subcommand)]
pub enum EmbeddingsCommands {
    /// Backfill embeddings for all profiles
    Backfill {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Process in batches of N profiles
        #[arg(short, long, default_value = "50")]
        batch_size: usize,
    },
    /// Generate embeddings for cast content
    BackfillCasts {
        /// Maximum number of casts to process
        #[arg(short, long)]
        limit: Option<usize>,
        /// Embedding endpoint to use (from config.toml endpoints list)
        #[arg(short, long)]
        endpoint: Option<String>,
    },
    /// Generate embeddings for a specific profile
    Generate {
        /// FID of the profile
        #[arg(long)]
        fid: i64,
        /// Show generated embedding details
        #[arg(short, long)]
        verbose: bool,
    },
    /// Test embedding generation
    Test {
        /// Text to generate embedding for
        text: String,
    },
    /// Show embedding statistics
    Stats,
}

#[derive(Subcommand)]
pub enum ServeCommands {
    /// Start API server (RESTful + MCP)
    Api {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port to bind to
        #[arg(short, long, default_value = "3000")]
        port: u16,
        /// Enable CORS
        #[arg(long)]
        cors: bool,
        /// Enable x402 payment
        #[cfg(feature = "payment")]
        #[arg(long)]
        payment: bool,
        /// Address to receive payments (defaults to 0x0 - burn address)
        #[cfg(feature = "payment")]
        #[arg(long, default_value = "0x0000000000000000000000000000000000000000")]
        payment_address: Option<String>,
        /// Use testnet (base-sepolia) or mainnet (base). If not specified, uses config default.
        #[cfg(feature = "payment")]
        #[arg(long)]
        testnet: Option<bool>,
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
