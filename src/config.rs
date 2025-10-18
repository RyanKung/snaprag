use std::path::Path;

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub backtrace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    pub dimension: usize,
    pub model: String,
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

fn default_use_testnet() -> bool {
    true  // x402.org/facilitator currently only supports testnet
}

fn default_payment_address() -> String {
    "0x0000000000000000000000000000000000000000".to_string()
}

impl Default for X402Config {
    fn default() -> Self {
        Self {
            payment_address: default_payment_address(),
            use_testnet: true,  // x402.org/facilitator only supports testnet
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
    pub x402: X402Config,
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| crate::SnapRagError::Io(e))?;

        let config: AppConfig =
            toml::from_str(&content).map_err(|e| crate::SnapRagError::TomlParsing(e))?;

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
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Get max connections for database pool
    pub fn max_connections(&self) -> u32 {
        self.database.max_connections
    }

    /// Get min connections for database pool
    pub fn min_connections(&self) -> u32 {
        self.database.min_connections
    }

    /// Get connection timeout in seconds
    pub fn connection_timeout(&self) -> u64 {
        self.database.connection_timeout
    }

    /// Get embedding dimension
    pub fn embedding_dimension(&self) -> usize {
        self.embeddings.dimension
    }

    /// Get embedding model name
    pub fn embedding_model(&self) -> &str {
        &self.embeddings.model
    }

    /// Check if vector indexes are enabled
    pub fn vector_indexes_enabled(&self) -> bool {
        self.performance.enable_vector_indexes
    }

    /// Get vector index lists count
    pub fn vector_index_lists(&self) -> usize {
        self.performance.vector_index_lists
    }

    /// Get snapchain HTTP endpoint
    pub fn snapchain_http_endpoint(&self) -> &str {
        &self.sync.snapchain_http_endpoint
    }

    /// Get snapchain gRPC endpoint
    pub fn snapchain_grpc_endpoint(&self) -> &str {
        &self.sync.snapchain_grpc_endpoint
    }

    /// Check if real-time sync is enabled
    pub fn realtime_sync_enabled(&self) -> bool {
        self.sync.enable_realtime_sync
    }

    /// Check if historical sync is enabled
    pub fn historical_sync_enabled(&self) -> bool {
        self.sync.enable_historical_sync
    }

    /// Get historical sync start event ID
    pub fn historical_sync_from_event_id(&self) -> u64 {
        self.sync.historical_sync_from_event_id
    }

    /// Get sync batch size
    pub fn sync_batch_size(&self) -> u32 {
        self.sync.batch_size
    }

    /// Get sync interval in milliseconds
    pub fn sync_interval_ms(&self) -> u64 {
        self.sync.sync_interval_ms
    }

    /// Get shard IDs to sync
    pub fn shard_ids(&self) -> &Vec<u32> {
        &self.sync.shard_ids
    }

    /// Get LLM endpoint
    pub fn llm_endpoint(&self) -> &str {
        &self.llm.llm_endpoint
    }

    /// Get LLM key
    pub fn llm_key(&self) -> &str {
        &self.llm.llm_key
    }

    /// Get LLM model
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
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                backtrace: true,
            },
            embeddings: EmbeddingsConfig {
                dimension: 1536,
                model: "text-embedding-ada-002".to_string(),
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
            },
            llm: LlmConfig {
                llm_endpoint: "http://localhost:11434".to_string(),
                llm_key: "ollama".to_string(),
                llm_model: "gemma3:27b".to_string(),
            },
            x402: X402Config::default(),
        }
    }
}
