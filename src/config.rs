use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
    #[serde(default = "default_slow_query_threshold")]
    pub slow_query_threshold_secs: f64,
}

const fn default_slow_query_threshold() -> f64 {
    1.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub backtrace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEndpoint {
    pub name: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub provider: String, // "openai", "ollama", or "local_gpu"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    pub dimension: usize,
    pub model: String,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_parallel_tasks")]
    pub parallel_tasks: usize,
    #[serde(default = "default_cpu_threads")]
    pub cpu_threads: usize,
    #[serde(default)]
    pub endpoints: Vec<EmbeddingEndpoint>,
}

const fn default_batch_size() -> usize {
    500 // Larger default for better performance
}

const fn default_parallel_tasks() -> usize {
    200 // High concurrency for Ollama performance
}

const fn default_cpu_threads() -> usize {
    0 // Auto-detect CPU cores
}

const fn default_enable_continuous_sync() -> bool {
    true // Enable continuous sync by default
}

const fn default_continuous_sync_interval() -> u64 {
    5 // Poll every 5 seconds by default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enable_vector_indexes: bool,
    pub vector_index_lists: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub snapchain_http_endpoint: String,
    pub snapchain_grpc_endpoint: String,
    pub enable_realtime_sync: bool,
    pub enable_historical_sync: bool,
    pub historical_sync_from_event_id: u64,
    pub batch_size: u32,
    pub sync_interval_ms: u64,
    pub shard_ids: Vec<u32>,
    /// Enable continuous sync monitoring after initial sync completion
    #[serde(default = "default_enable_continuous_sync")]
    pub enable_continuous_sync: bool,
    /// Continuous sync interval in seconds (how often to poll for new blocks)
    #[serde(default = "default_continuous_sync_interval")]
    pub continuous_sync_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub llm_endpoint: String,
    pub llm_key: String,
    #[serde(default = "default_llm_model")]
    pub llm_model: String,
}

fn default_llm_model() -> String {
    "gemma3:27b".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// Enable API caching
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,
    /// Profile cache TTL in seconds (default: 1 hour)
    #[serde(default = "default_profile_ttl")]
    pub profile_ttl_secs: u64,
    /// Social analysis cache TTL in seconds (default: 1 hour)
    #[serde(default = "default_social_ttl")]
    pub social_ttl_secs: u64,
    /// Maximum number of cache entries
    #[serde(default = "default_max_cache_entries")]
    pub max_entries: usize,
    /// Enable cache statistics
    #[serde(default = "default_cache_stats")]
    pub enable_stats: bool,
}

const fn default_cache_enabled() -> bool {
    true
}

const fn default_profile_ttl() -> u64 {
    3600 // 1 hour
}

const fn default_social_ttl() -> u64 {
    3600 // 1 hour
}

const fn default_max_cache_entries() -> usize {
    10000
}

const fn default_cache_stats() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Config {
    /// Address to receive payments (defaults to burn address)
    #[serde(default = "default_payment_address")]
    pub payment_address: String,
    /// Use testnet (base-sepolia) - x402.org/facilitator only supports testnet currently
    #[serde(default = "default_use_testnet")]
    pub use_testnet: bool,
    /// Enable payment by default
    #[serde(default)]
    pub enabled: bool,
}

const fn default_use_testnet() -> bool {
    true // x402.org/facilitator currently only supports testnet
}

fn default_payment_address() -> String {
    "0x0000000000000000000000000000000000000000".to_string()
}

impl Default for X402Config {
    fn default() -> Self {
        Self {
            payment_address: default_payment_address(),
            use_testnet: true, // x402.org/facilitator only supports testnet
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub embeddings: EmbeddingsConfig,
    pub performance: PerformanceConfig,
    pub sync: SyncConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub x402: X402Config,
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(crate::SnapRagError::Io)?;

        let config: Self = toml::from_str(&content).map_err(crate::SnapRagError::TomlParsing)?;

        Ok(config)
    }

    /// Load configuration from default config file path
    pub fn load() -> crate::Result<Self> {
        // Try to load from config.toml first, then fall back to config.example.toml
        if Path::new("config.toml").exists() {
            Self::from_file("config.toml")
        } else if Path::new("config.example.toml").exists() {
            println!(
                "Warning: Using config.example.toml. Please create config.toml for production use."
            );
            Self::from_file("config.example.toml")
        } else {
            Err(crate::SnapRagError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No config file found. Please create config.toml or config.example.toml",
            )))
        }
    }

    /// Get database URL
    #[must_use]
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Get max connections for database pool
    #[must_use]
    pub const fn max_connections(&self) -> u32 {
        self.database.max_connections
    }

    /// Get min connections for database pool
    #[must_use]
    pub const fn min_connections(&self) -> u32 {
        self.database.min_connections
    }

    /// Get connection timeout in seconds
    #[must_use]
    pub const fn connection_timeout(&self) -> u64 {
        self.database.connection_timeout
    }

    /// Get slow query threshold in seconds
    #[must_use]
    pub const fn slow_query_threshold_secs(&self) -> f64 {
        self.database.slow_query_threshold_secs
    }

    /// Get embedding dimension
    #[must_use]
    pub const fn embedding_dimension(&self) -> usize {
        self.embeddings.dimension
    }

    /// Get embedding model name
    #[must_use]
    pub fn embedding_model(&self) -> &str {
        &self.embeddings.model
    }

    /// Get embeddings batch size
    #[must_use]
    pub const fn embeddings_batch_size(&self) -> usize {
        self.embeddings.batch_size
    }

    /// Get embeddings parallel tasks
    #[must_use]
    pub const fn embeddings_parallel_tasks(&self) -> usize {
        self.embeddings.parallel_tasks
    }

    /// Get embeddings CPU threads
    #[must_use]
    pub const fn embeddings_cpu_threads(&self) -> usize {
        self.embeddings.cpu_threads
    }

    /// Get embedding endpoints
    #[must_use]
    pub const fn embedding_endpoints(&self) -> &Vec<EmbeddingEndpoint> {
        &self.embeddings.endpoints
    }

    /// Get embedding endpoint by name
    #[must_use]
    pub fn get_embedding_endpoint(&self, name: &str) -> Option<&EmbeddingEndpoint> {
        self.embeddings.endpoints.iter().find(|e| e.name == name)
    }

    /// Check if vector indexes are enabled
    #[must_use]
    pub const fn vector_indexes_enabled(&self) -> bool {
        self.performance.enable_vector_indexes
    }

    /// Get vector index lists count
    #[must_use]
    pub const fn vector_index_lists(&self) -> usize {
        self.performance.vector_index_lists
    }

    /// Get snapchain HTTP endpoint
    #[must_use]
    pub fn snapchain_http_endpoint(&self) -> &str {
        &self.sync.snapchain_http_endpoint
    }

    /// Get snapchain gRPC endpoint
    #[must_use]
    pub fn snapchain_grpc_endpoint(&self) -> &str {
        &self.sync.snapchain_grpc_endpoint
    }

    /// Check if real-time sync is enabled
    #[must_use]
    pub const fn realtime_sync_enabled(&self) -> bool {
        self.sync.enable_realtime_sync
    }

    /// Check if historical sync is enabled
    #[must_use]
    pub const fn historical_sync_enabled(&self) -> bool {
        self.sync.enable_historical_sync
    }

    /// Get historical sync start event ID
    #[must_use]
    pub const fn historical_sync_from_event_id(&self) -> u64 {
        self.sync.historical_sync_from_event_id
    }

    /// Get sync batch size
    #[must_use]
    pub const fn sync_batch_size(&self) -> u32 {
        self.sync.batch_size
    }

    /// Get sync interval in milliseconds
    #[must_use]
    pub const fn sync_interval_ms(&self) -> u64 {
        self.sync.sync_interval_ms
    }

    /// Check if continuous sync is enabled
    #[must_use]
    pub const fn continuous_sync_enabled(&self) -> bool {
        self.sync.enable_continuous_sync
    }

    /// Get continuous sync interval in seconds
    #[must_use]
    pub const fn continuous_sync_interval_secs(&self) -> u64 {
        self.sync.continuous_sync_interval_secs
    }

    /// Get shard IDs to sync
    #[must_use]
    pub const fn shard_ids(&self) -> &Vec<u32> {
        &self.sync.shard_ids
    }

    /// Get LLM endpoint
    #[must_use]
    pub fn llm_endpoint(&self) -> &str {
        &self.llm.llm_endpoint
    }

    /// Get LLM key
    #[must_use]
    pub fn llm_key(&self) -> &str {
        &self.llm.llm_key
    }

    /// Get LLM model
    #[must_use]
    pub fn llm_model(&self) -> &str {
        &self.llm.llm_model
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                url: "postgresql://username:password@your-db-host:5432/your-database".to_string(),
                max_connections: 20,
                min_connections: 5,
                connection_timeout: 30,
                slow_query_threshold_secs: 1.5,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                backtrace: true,
            },
            embeddings: EmbeddingsConfig {
                dimension: 1536,
                model: "text-embedding-ada-002".to_string(),
                batch_size: 100,
                parallel_tasks: 5,
                cpu_threads: 0, // Auto-detect
                endpoints: vec![],
            },
            performance: PerformanceConfig {
                enable_vector_indexes: true,
                vector_index_lists: 100,
            },
            sync: SyncConfig {
                snapchain_http_endpoint: "http://localhost:3383".to_string(),
                snapchain_grpc_endpoint: "http://localhost:3384".to_string(),
                enable_realtime_sync: true,
                enable_historical_sync: false,
                historical_sync_from_event_id: 0,
                batch_size: 100,
                sync_interval_ms: 1000,
                shard_ids: vec![0, 1, 2],
                enable_continuous_sync: true,
                continuous_sync_interval_secs: 5,
            },
            llm: LlmConfig {
                llm_endpoint: "http://localhost:11434".to_string(),
                llm_key: "ollama".to_string(),
                llm_model: "gemma3:27b".to_string(),
            },
            cache: CacheConfig {
                enabled: true,
                profile_ttl_secs: 3600, // 1 hour
                social_ttl_secs: 3600,  // 1 hour
                max_entries: 10000,
                enable_stats: true,
            },
            x402: X402Config::default(),
        }
    }
}
