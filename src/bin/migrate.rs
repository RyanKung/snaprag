use snaprag::database::Database;
use snaprag::AppConfig;
use snaprag::Result;
use sqlx::PgPool;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration from config.toml
    let config = AppConfig::load()?;
    
    println!("🔄 Running database migrations...");
    println!("📋 Database URL: {}", mask_database_url(&config.database_url()));

    // Create database connection pool with config
    let pool = PgPool::connect(&config.database_url()).await?;
        
    let db = Database::new(pool);

    // Initialize database schema
    db.init_schema().await?;
    println!("✅ Database migrations completed successfully!");
    
    Ok(())
}

/// Mask database URL for logging (hide password)
fn mask_database_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            format!("{}://{}@{}:{}", 
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
