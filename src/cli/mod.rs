//! CLI module for `SnapRAG` binary
//!
//! This module contains all CLI-related functionality including:
//! - Command line argument parsing
//! - Command handlers (organized by domain in handlers/ subdirectory)
//! - Output formatting
//! - Interactive prompts

pub mod commands;
pub mod handlers;
pub mod output;

pub use commands::*;
pub use handlers::*;
pub use output::*;
