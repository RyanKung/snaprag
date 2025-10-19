//! CLI command handlers module
//!
//! This module is organized by functional domains:
//! - init: Database initialization and reset
//! - data: Data querying (list, search, activity)
//! - cast: Cast operations (search, recent, thread)
//! - rag: RAG queries
//! - embeddings: Embedding generation and backfill
//! - fetch: Lazy loading (on-demand fetching)
//! - sync: Synchronization commands
//! - serve: API server
//! - info: Information display (stats, dashboard, config)

pub mod cast;
pub mod data;
pub mod embeddings;
pub mod fetch;
pub mod info;
pub mod init;
pub mod rag;
pub mod serve;
pub mod sync;

// Re-export all public handlers
pub use cast::*;
pub use data::*;
pub use embeddings::*;
pub use fetch::*;
pub use info::*;
pub use init::*;
pub use rag::*;
pub use serve::*;
pub use sync::*;
