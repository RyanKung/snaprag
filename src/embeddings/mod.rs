//! Embeddings generation module
//!
//! This module provides functionality for generating text embeddings using various providers:
//! - OpenAI (text-embedding-ada-002, text-embedding-3-small, etc.)
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
pub mod client;
pub mod generator;

pub use backfill::backfill_embeddings;
pub use client::EmbeddingClient;
pub use client::EmbeddingProvider;
pub use generator::EmbeddingService;

use crate::errors::Result;

/// Default embedding dimension for OpenAI text-embedding-ada-002
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
    pub fn from_app_config(config: &crate::config::AppConfig) -> Self {
        // Check if using OpenAI or local Ollama
        let provider = if config.embedding_model().contains("ada")
            || config.embedding_model().contains("text-embedding")
        {
            EmbeddingProvider::OpenAI
        } else {
            EmbeddingProvider::Ollama
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
}
