//! CLI module for SnapRAG binary
//!
//! This module contains all CLI-related functionality including:
//! - Command line argument parsing
//! - Command handlers
//! - Output formatting
//! - Interactive prompts

pub mod commands;
pub mod handlers;
pub mod output;

pub use commands::*;
pub use handlers::handle_activity_command;
pub use handlers::handle_cast_embeddings_backfill;
pub use handlers::handle_cast_recent;
pub use handlers::handle_cast_search;
pub use handlers::handle_cast_thread;
pub use handlers::handle_config_command;
pub use handlers::handle_dashboard_command;
pub use handlers::handle_embeddings_backfill;
pub use handlers::handle_embeddings_generate;
pub use handlers::handle_embeddings_stats;
pub use handlers::handle_embeddings_test;
pub use handlers::handle_list_command;
pub use handlers::handle_rag_query;
pub use handlers::handle_rag_query_casts;
pub use handlers::handle_rag_search;
pub use handlers::handle_reset_command;
pub use handlers::handle_search_command;
pub use handlers::handle_stats_command;
pub use handlers::handle_sync_command;
pub use output::*;
