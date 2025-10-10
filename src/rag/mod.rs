//! RAG (Retrieval-Augmented Generation) module
//!
//! This module provides end-to-end RAG functionality for querying Farcaster data:
//! - Semantic retrieval using vector embeddings
//! - Result ranking and reranking
//! - Context assembly from retrieved documents
//! - LLM-based answer generation
//!
//! # Examples
//!
//! ```rust,no_run
//! use snaprag::rag::RagService;
//! use snaprag::config::AppConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = AppConfig::load()?;
//!     let service = RagService::new(&config).await?;
//!     
//!     let response = service.query("Find developers interested in AI").await?;
//!     println!("Answer: {}", response.answer);
//!     println!("Sources: {} profiles", response.sources.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod context;
pub mod pipeline;
pub mod retriever;

pub use context::ContextAssembler;
pub use pipeline::RagQuery;
pub use pipeline::RagResponse;
pub use pipeline::RagService;
pub use pipeline::RetrievalMethod;
pub use retriever::Retriever;

use crate::errors::Result;
use crate::models::UserProfile;

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub profile: UserProfile,
    pub score: f32,
    pub match_type: MatchType,
}

/// Type of match for the search result
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// Vector similarity match
    Semantic,
    /// Text keyword match
    Keyword,
    /// Combined semantic and keyword match
    Hybrid,
}
