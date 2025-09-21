//! SnapRAG Build Script
//! 
//! This build script handles SQLx compilation by setting SQLX_OFFLINE=true
//! to avoid database connection issues during build.

use std::env;
use std::fs;

fn main() {
    // Check if we should use offline mode
    let use_offline = env::var("SQLX_OFFLINE").unwrap_or_else(|_| "false".to_string()) == "true";
    
    if use_offline {
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
        println!("cargo:warning=Using SQLX_OFFLINE mode - make sure .sqlx/ directory exists with prepared queries");
    } else {
        // Try to use database connection for live query validation
        println!("cargo:warning=Attempting live SQLx query validation (may fail if database is unavailable)");
    }
    
    // Check if DATABASE_URL is already set
    match env::var("DATABASE_URL") {
        Ok(_) => {
            println!("cargo:warning=Using provided DATABASE_URL for SQLx compilation");
            println!("cargo:rerun-if-env-changed=DATABASE_URL");
        }
        Err(_) => {
            // Try to read database URL from config.toml
            match read_database_url_from_config() {
                Ok(database_url) => {
                    println!("cargo:rustc-env=DATABASE_URL={}", database_url);
                    
                    // Also set it for the current process so sqlx can use it
                    env::set_var("DATABASE_URL", &database_url);
                    
                    println!("cargo:warning=Using database URL from config.toml for SQLx compilation");
                    println!("cargo:warning=Runtime database connection is configured via config.toml");
                }
                Err(e) => {
                    println!("cargo:warning=Failed to read config.toml: {}", e);
                    println!("cargo:warning=Please set DATABASE_URL environment variable or ensure config.toml exists");
                    
                    // Fallback to a generic URL for compilation (won't work for actual queries)
                    let fallback_url = "postgresql://user:pass@localhost/db";
                    println!("cargo:rustc-env=DATABASE_URL={}", fallback_url);
                    env::set_var("DATABASE_URL", fallback_url);
                }
            }
        }
    }
    
    // Tell Cargo to re-run this build script if config files change
    println!("cargo:rerun-if-changed=config.toml");
    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// Read database URL from config.toml file
fn read_database_url_from_config() -> Result<String, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string("config.toml")?;
    
    // Simple TOML parsing for the database.url field
    // This is a basic implementation - for production, consider using a proper TOML library
    for line in config_content.lines() {
        let line = line.trim();
        if line.starts_with("url = ") {
            // Extract the URL from the line
            let url = line
                .strip_prefix("url = ")
                .ok_or("Invalid URL format in config.toml")?
                .trim_matches('"');
            
            return Ok(url.to_string());
        }
    }
    
    Err("Database URL not found in config.toml".into())
}
