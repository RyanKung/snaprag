//! Embeddings generation module
//!
//! This module provides functionality for generating text embeddings using various providers:
//! - `OpenAI` (text-embedding-ada-002, text-embedding-3-small, etc.)
//! - Ollama (local models)
//! - Custom endpoints
//!
//! # Examples
//!
//! ```rust,no_run
//! use snaprag::embeddings::EmbeddingService;
//! use snaprag::config::AppConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = AppConfig::load()?;
//!     let service = EmbeddingService::new(&config)?;
//!     
//!     let embedding = service.generate("Hello, world!").await?;
//!     println!("Generated embedding with {} dimensions", embedding.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod backfill;
pub mod cast_backfill;
pub mod client;
pub mod generator;

pub use backfill::backfill_embeddings;
pub use cast_backfill::backfill_cast_embeddings;
pub use cast_backfill::CastBackfillStats;
pub use client::EmbeddingClient;
pub use client::EmbeddingProvider;
pub use generator::EmbeddingService;

use crate::errors::Result;

/// Default embedding dimension for `OpenAI` text-embedding-ada-002
pub const DEFAULT_EMBEDDING_DIM: usize = 1536;

/// Maximum batch size for embedding generation
pub const MAX_BATCH_SIZE: usize = 100;

/// Configuration for embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: String,
    pub dimension: usize,
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl EmbeddingConfig {
    /// Create from app config using default LLM endpoint
    #[must_use] 
    pub fn from_app_config(config: &crate::config::AppConfig) -> Self {
        // Determine provider based on llm_key or endpoint
        // Priority: llm_key > endpoint domain
        let provider = if config.llm_key() == "ollama" {
            EmbeddingProvider::Ollama
        } else if config.llm_endpoint().contains("api.openai.com") {
            EmbeddingProvider::OpenAI
        } else if config.llm_endpoint().contains("localhost")
            || !config.llm_endpoint().contains("openai")
        {
            // Local or non-OpenAI endpoint, assume Ollama
            EmbeddingProvider::Ollama
        } else {
            // Default to OpenAI if endpoint looks like OpenAI
            EmbeddingProvider::OpenAI
        };

        Self {
            provider,
            model: config.embedding_model().to_string(),
            dimension: config.embedding_dimension(),
            endpoint: config.llm_endpoint().to_string(),
            api_key: if provider == EmbeddingProvider::OpenAI {
                Some(config.llm_key().to_string())
            } else {
                None
            },
        }
    }

    /// Create from a specific endpoint configuration
    #[must_use] 
    pub fn from_endpoint(
        config: &crate::config::AppConfig,
        endpoint_config: &crate::config::EmbeddingEndpoint,
    ) -> Self {
        let provider = match endpoint_config.provider.to_lowercase().as_str() {
            "openai" => EmbeddingProvider::OpenAI,
            "ollama" => EmbeddingProvider::Ollama,
            _ => {
                // Auto-detect based on endpoint
                if endpoint_config.endpoint.contains("api.openai.com") {
                    EmbeddingProvider::OpenAI
                } else {
                    EmbeddingProvider::Ollama
                }
            }
        };

        Self {
            provider,
            model: endpoint_config.model.clone(),
            dimension: config.embedding_dimension(),
            endpoint: endpoint_config.endpoint.clone(),
            api_key: endpoint_config.api_key.clone(),
        }
    }
}
