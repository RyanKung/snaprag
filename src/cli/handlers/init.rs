//! Database initialization and reset handlers

use std::io::stdin;
use std::io::Read;

use crate::cli::output::print_info;
use crate::cli::output::print_prompt;
use crate::cli::output::print_success;
use crate::cli::output::print_warning;
use crate::Result;
use crate::SnapRag;

/// Handle database initialization command
pub async fn handle_init_command(snaprag: &SnapRag, force: bool, skip_indexes: bool) -> Result<()> {
    if !force {
        print_warning("This will initialize the database schema and create indexes.");
        print_warning("This operation is safe - it uses CREATE IF NOT EXISTS.");
        println!("\nUse --force to proceed.");
        return Ok(());
    }

    print_info("🗄️  Initializing SnapRAG database...");
    println!();

    // Run complete initialization SQL
    print_info("📋 Running complete initialization script...");
    match run_complete_init(snaprag) {
        Ok(()) => {
            print_success("✅ All tables created");
            print_success("✅ Vector columns configured");
            print_success("✅ Triggers and functions created");
        }
        Err(e) => {
            if e.to_string().contains("vector") || e.to_string().contains("extension") {
                print_warning(&format!("⚠️  Could not enable pgvector extension: {e}"));
                print_warning("Please run on the database server (192.168.1.192):");
                println!(
                    "  sudo -u postgres psql -d snaprag -c 'CREATE EXTENSION IF NOT EXISTS vector;'"
                );
                println!();
                println!("Then run: snaprag init --force");
                return Err(e);
            }
            return Err(e);
        }
    }

    // Schema migrations are now consolidated in 000_complete_init.sql
    // No need to run additional migrations (they would overwrite with old schema)
    print_info("✅ Schema fully initialized from 000_complete_init.sql");

    if skip_indexes {
        print_info("⏭️  Skipping index creation (--skip-indexes)");
    } else {
        print_info("📊 Creating performance-optimized indexes...");
        create_optimized_indexes(snaprag).await?;
        print_success("✅ Indexes created");
    }

    println!();
    print_success("🎉 Database initialization complete!");
    println!();

    if skip_indexes {
        print_info("ℹ️  To create indexes later, run:");
        println!("   snaprag init --force");
    }

    print_info("ℹ️  To start syncing data, run:");
    println!("   snaprag sync start");

    Ok(())
}

/// Handle database reset command
pub async fn handle_reset_command(snaprag: &SnapRag, force: bool) -> Result<()> {
    if !force {
        print_warning("This will DROP ALL TABLES from the database and remove lock files!");
        print_warning("This is a complete reset - all data will be lost!");
        print_prompt("Are you sure you want to continue? (y/N)");

        let mut input = String::new();
        stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            print_info("Operation cancelled.");
            return Ok(());
        }
    }

    print_info("🔥 Resetting database - DROPPING ALL TABLES...");

    // Remove lock file if it exists
    if std::path::Path::new("snaprag.lock").exists() {
        std::fs::remove_file("snaprag.lock")?;
        print_success("Removed snaprag.lock file");
    } else {
        print_info("No lock file found");
    }

    let pool = snaprag.database().pool();

    // 🚀 DROP all views and tables (complete reset)
    // Drop views first
    for view_name in ["user_profiles", "user_profiles_with_embeddings"] {
        sqlx::query(&format!("DROP VIEW IF EXISTS {view_name} CASCADE"))
            .execute(pool)
            .await
            .ok();
    }

    // Then drop tables
    let tables = [
        "cast_embeddings",      // Drop first due to FK constraint
        "user_profile_changes", // Event-sourcing table
        "profile_embeddings",   // Embeddings table
        "user_profile_snapshots",
        "user_profile_trends",
        "user_data",
        "user_data_changes",
        "casts",
        "links",
        "reactions",
        "verifications",
        "onchain_events", // System messages
        "username_proofs",
        "frame_actions",
        "user_activities",
        "processed_messages",
        "sync_progress",
        "sync_stats",
    ];

    for table in &tables {
        match sqlx::query(&format!("DROP TABLE IF EXISTS {table} CASCADE"))
            .execute(snaprag.database().pool())
            .await
        {
            Ok(_) => print_success(&format!("Dropped table: {table}")),
            Err(e) => print_warning(&format!("Could not drop {table}: {e}")),
        }
    }

    // Drop the trigger function
    if let Ok(_) =
        sqlx::query("DROP FUNCTION IF EXISTS update_cast_embeddings_updated_at() CASCADE")
            .execute(snaprag.database().pool())
            .await
    {
        print_success("Dropped trigger function")
    }

    println!();
    print_success("✅ Database completely reset!");
    print_info("ℹ️  To reinitialize, run:");
    println!("   snaprag init --force");

    Ok(())
}

/// Run complete database initialization from migration file
fn run_complete_init(snaprag: &SnapRag) -> Result<()> {
    // Write SQL file to temp location
    let init_sql = include_str!("../../../migrations/000_complete_init.sql");
    let temp_sql_path = "/tmp/snaprag_init.sql";
    std::fs::write(temp_sql_path, init_sql)?;

    // Get database URL from environment or use default parsing
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        // Fallback: read from config.toml
        std::fs::read_to_string("config.toml")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.contains("url =") && line.contains("postgresql"))
                    .and_then(|line| line.split('"').nth(1))
                    .map(String::from)
            })
            .unwrap_or_else(|| {
                "postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag".to_string()
            })
    });

    //Parse DATABASE_URL to get components
    // Format: postgresql://user:password@host:port/database
    // or postgres://user:password@host:port/database
    let url_without_scheme = db_url
        .strip_prefix("postgresql://")
        .or_else(|| db_url.strip_prefix("postgres://"))
        .ok_or_else(|| crate::SnapRagError::Custom("Invalid database URL scheme".to_string()))?;

    // Split by @ to get user_pass and host_db
    let (user_pass, host_db) = url_without_scheme
        .split_once('@')
        .ok_or_else(|| crate::SnapRagError::Custom("Invalid database URL format".to_string()))?;

    // Extract user and password
    let (user, password) = user_pass.split_once(':').unwrap_or((user_pass, ""));

    // Split host_db by / to get host_port and database
    let (host_port, database) = host_db.split_once('/').unwrap_or((host_db, "snaprag"));

    // Extract host and port
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, "5432"));

    // Use psql to execute the SQL file (most reliable for complex SQL)
    tracing::debug!(
        "Executing SQL via psql: {}@{}:{}/{}",
        user,
        host,
        port,
        database
    );

    let output = std::process::Command::new("psql")
        .env("PGPASSWORD", password)
        .arg("-h")
        .arg(host)
        .arg("-p")
        .arg(port)
        .arg("-U")
        .arg(user)
        .arg("-d")
        .arg(database)
        .arg("-f")
        .arg(temp_sql_path)
        .arg("-v")
        .arg("ON_ERROR_STOP=0") // Continue on errors
        .output()?;

    // Clean up temp file (keep for debugging)
    // std::fs::remove_file(temp_sql_path).ok();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Log full output for debugging
    if !stderr.is_empty() {
        tracing::debug!("psql stderr: {}", stderr);
    }
    if !stdout.is_empty() {
        tracing::debug!("psql stdout: {}", stdout);
    }

    if !output.status.success() {
        if stderr.contains("extension") && stderr.contains("permission") {
            return Err(crate::SnapRagError::Custom(format!(
                "Extension error: {stderr}"
            )));
        }
        return Err(crate::SnapRagError::Custom(format!(
            "psql failed with exit code {:?}: {}",
            output.status.code(),
            stderr
        )));
    }

    tracing::info!("✅ Successfully executed initialization SQL");
    Ok(())
}

/// Run schema migrations
/// NOTE: All schema is now in 000_complete_init.sql, no additional migrations needed
fn run_schema_migrations(_snaprag: &SnapRag) -> Result<()> {
    // No additional migrations - everything is in 000_complete_init.sql
    let migrations: Vec<(&str, &str)> = vec![];

    // Get database connection info
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        std::fs::read_to_string("config.toml")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.contains("url =") && line.contains("postgresql"))
                    .and_then(|line| line.split('"').nth(1))
                    .map(String::from)
            })
            .unwrap_or_else(|| {
                "postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag".to_string()
            })
    });

    let url_without_scheme = db_url
        .strip_prefix("postgresql://")
        .or_else(|| db_url.strip_prefix("postgres://"))
        .ok_or_else(|| crate::SnapRagError::Custom("Invalid database URL".to_string()))?;

    let (user_pass, host_db) = url_without_scheme
        .split_once('@')
        .ok_or_else(|| crate::SnapRagError::Custom("Invalid database URL".to_string()))?;

    let (user, password) = user_pass.split_once(':').unwrap_or((user_pass, ""));
    let (host_port, database) = host_db.split_once('/').unwrap_or((host_db, "snaprag"));
    let (host, port) = host_port.split_once(':').unwrap_or((host_port, "5432"));

    // Run each migration
    for (name, sql) in migrations {
        let temp_path = format!("/tmp/snaprag_{name}");
        std::fs::write(&temp_path, sql)?;

        tracing::info!("Running migration: {}", name);

        let output = std::process::Command::new("psql")
            .env("PGPASSWORD", password)
            .arg("-h")
            .arg(host)
            .arg("-p")
            .arg(port)
            .arg("-U")
            .arg(user)
            .arg("-d")
            .arg(database)
            .arg("-f")
            .arg(&temp_path)
            .arg("-v")
            .arg("ON_ERROR_STOP=0")
            .output()?;

        std::fs::remove_file(&temp_path).ok();

        if output.status.success() {
            tracing::info!("Migration {} completed", name);
        } else {
            tracing::warn!(
                "Migration {} had warnings: {}",
                name,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    Ok(())
}

/// Create performance-optimized indexes
async fn create_optimized_indexes(snaprag: &SnapRag) -> Result<()> {
    let pool = snaprag.database().pool();

    // Only create essential indexes for write-heavy workload

    // Note: user_activity_timeline table removed for performance

    // 1. casts (essential for message lookups)
    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_casts_fid 
         ON casts(fid)",
    )
    .execute(pool)
    .await
    .ok();

    // 3. sync tracking
    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_processed_shard_height 
         ON processed_messages(shard_id, block_height DESC)",
    )
    .execute(pool)
    .await
    .ok();

    // 4. Update statistics
    sqlx::query("ANALYZE casts").execute(pool).await.ok();
    sqlx::query("ANALYZE user_profile_changes")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE profile_embeddings")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE links").execute(pool).await.ok();
    sqlx::query("ANALYZE reactions").execute(pool).await.ok();
    sqlx::query("ANALYZE verifications")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE onchain_events")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE username_proofs")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE frame_actions")
        .execute(pool)
        .await
        .ok();

    Ok(())
}
