use serde::{Deserialize, Serialize};
use std::path::Path;

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
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
    pub embeddings: EmbeddingsConfig,
    pub performance: PerformanceConfig,
}

impl AppConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::SnapRagError::Io(e))?;
        
        let config: AppConfig = toml::from_str(&content)
            .map_err(|e| crate::SnapRagError::TomlParsing(e))?;
        
        Ok(config)
    }

    /// Load configuration from default config file path
    pub fn load() -> crate::Result<Self> {
        // Try to load from config.toml first, then fall back to config.example.toml
        if Path::new("config.toml").exists() {
            Self::from_file("config.toml")
        } else if Path::new("config.example.toml").exists() {
            println!("Warning: Using config.example.toml. Please create config.toml for production use.");
            Self::from_file("config.example.toml")
        } else {
            Err(crate::SnapRagError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No config file found. Please create config.toml or config.example.toml"
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
        }
    }
}
