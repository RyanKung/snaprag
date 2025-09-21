use snaprag::AppConfig;
use snaprag::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Checking configuration...");

    match AppConfig::load() {
        Ok(config) => {
            println!("✅ Configuration loaded successfully!");
            println!("📋 Configuration details:");
            println!(
                "  Database URL: {}",
                mask_database_url(&config.database_url())
            );
            println!("  Max connections: {}", config.max_connections());
            println!("  Min connections: {}", config.min_connections());
            println!("  Connection timeout: {}s", config.connection_timeout());
            println!("  Embedding dimension: {}", config.embedding_dimension());
            println!("  Embedding model: {}", config.embedding_model());
            println!(
                "  Vector indexes enabled: {}",
                config.vector_indexes_enabled()
            );
            println!("  Vector index lists: {}", config.vector_index_lists());
            println!("  Snapchain endpoint: {}", config.snapchain_endpoint());
            println!(
                "  Real-time sync enabled: {}",
                config.realtime_sync_enabled()
            );
            println!(
                "  Historical sync enabled: {}",
                config.historical_sync_enabled()
            );
            println!("  Sync batch size: {}", config.sync_batch_size());
            println!("  Sync interval: {}ms", config.sync_interval_ms());
            println!("  Shard IDs: {:?}", config.shard_ids());

            // Configuration loaded successfully

            println!("\n🎉 Configuration check completed successfully!");
        }
        Err(e) => {
            println!("❌ Configuration error: {}", e);
            println!("\n💡 To fix this:");
            println!("  1. Copy config.example.toml to config.toml");
            println!("  2. Edit config.toml with your database connection details");
            println!("  3. Run this check again");
            return Err(e);
        }
    }

    Ok(())
}

/// Mask database URL for logging (hide password)
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            format!(
                "{}://{}@{}:{}",
                parsed.scheme(),
                parsed.username(),
                host,
                parsed.port().unwrap_or(5432)
            )
        } else {
            "***masked***".to_string()
        }
    } else {
        "***invalid***".to_string()
    }
}
