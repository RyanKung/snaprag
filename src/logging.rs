//! Logging configuration for SnapRAG

use crate::Result;
use std::path::Path;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

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
        EnvFilter::new(&format!("{},snaprag={}", level, level))
    } else {
        // Fallback to environment variable or default
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,snaprag=debug"))
    };

    // Set up file appender for all logs
    let file_appender = tracing_appender::rolling::daily("logs", "snaprag.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up console appender with colors
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::io::stderr);

    // Set up file layer
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
    
    tracing::info!("Logging initialized with level: {} - console and file output enabled", level);
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
    let env_filter = EnvFilter::new(&format!("{},snaprag={}", level, level));

    // Set up file appender for all logs
    let file_appender = tracing_appender::rolling::daily("logs", "snaprag.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Set up console appender with colors
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::io::stderr);

    // Set up file layer
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

    tracing::info!("Logging initialized with level: {} - console and file output enabled", level);
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
