//! Logging configuration for SnapRAG

use std::path::Path;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::{
    self,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Registry;

use crate::Result;

/// Initialize logging system with file output
pub fn init_logging() -> Result<()> {
    init_logging_with_config(None)
}

/// Initialize logging with configuration
pub fn init_logging_with_config(config: Option<&crate::config::AppConfig>) -> Result<()> {
    // Create logs directory if it doesn't exist
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(logs_dir)?;
    }

    // Set up environment filter - use config if available, otherwise default
    let env_filter = if let Some(config) = config {
        let level = &config.logging.level;
        // 🚀 Always suppress sqlx SQL queries and third-party library noise
        EnvFilter::new(&format!(
            "warn,snaprag={},sqlx=warn,h2=warn,tonic=warn,hyper=warn,tower=warn",
            level
        ))
    } else {
        // Fallback to environment variable or default to info level
        // Only show warnings from third-party libraries, info from snaprag
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("warn,snaprag=info,sqlx=warn"))
    };

    // Set up file appender for all logs
    let file_appender = tracing_appender::rolling::daily("logs", "snaprag.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up console appender - simpler format for cleaner output
    let console_layer = fmt::layer()
        .with_target(false) // Don't show target for cleaner console output
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false) // Don't show file paths in console
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .with_writer(std::io::stderr);

    // Set up file layer - keep detailed info in log files
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(non_blocking)
        .with_ansi(false); // No colors in file

    // Initialize the registry
    Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    let level = if let Some(config) = config {
        &config.logging.level
    } else {
        "info"
    };

    tracing::info!(
        "Logging initialized with level: {} - console and file output enabled",
        level
    );
    tracing::info!("Log files will be saved to: logs/snaprag.log.YYYY-MM-DD");

    // Store the guard to prevent it from being dropped
    std::mem::forget(_guard);

    Ok(())
}

/// Initialize logging with custom log level
pub fn init_logging_with_level(level: &str) -> Result<()> {
    // Create logs directory if it doesn't exist
    let logs_dir = Path::new("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(logs_dir)?;
    }

    // Set up environment filter with custom level
    // Even in debug mode, keep third-party libraries at warn level to reduce noise
    let env_filter = if level == "debug" || level == "trace" {
        // 🚀 In debug/trace mode, still suppress SQL queries unless explicitly needed
        EnvFilter::new(&format!(
            "warn,snaprag={},sqlx=warn,h2=warn,tonic=warn,hyper=warn,tower=warn",
            level
        ))
    } else {
        // 🚀 For info/warn/error levels, suppress all third-party noise
        EnvFilter::new(&format!(
            "warn,snaprag={},sqlx=warn,h2=warn,tonic=warn,hyper=warn,tower=warn",
            level
        ))
    };

    // Set up file appender for all logs
    let file_appender = tracing_appender::rolling::daily("logs", "snaprag.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up console appender - show details in verbose/debug mode
    let console_layer = fmt::layer()
        .with_target(true) // Show target in debug mode
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .with_writer(std::io::stderr);

    // Set up file layer - keep detailed info in log files
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(non_blocking)
        .with_ansi(false); // No colors in file

    // Initialize the registry
    Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        "Logging initialized with level: {} - console and file output enabled",
        level
    );
    tracing::info!("Log files will be saved to: logs/snaprag.log.YYYY-MM-DD");

    // Store the guard to prevent it from being dropped
    std::mem::forget(_guard);

    Ok(())
}

/// Initialize simple logging for testing
pub fn init_simple_logging() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Simple logging initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        // This test just ensures the logging functions don't panic
        // In a real test environment, we'd need to be more careful about
        // multiple initializations
        let _ = init_simple_logging();
    }
}
