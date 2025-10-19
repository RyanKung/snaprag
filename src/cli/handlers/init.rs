//! Database initialization and reset handlers

use crate::cli::output::{print_info, print_prompt, print_success, print_warning};
use crate::Result;
use crate::SnapRag;
use std::io::{stdin, Read};

/// Handle database initialization command
pub async fn handle_init_command(
    snaprag: &SnapRag,
    force: bool,
    skip_indexes: bool,
) -> Result<()> {
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
    match run_complete_init(snaprag).await {
        Ok(_) => {
            print_success("✅ All tables created");
            print_success("✅ Vector columns configured");
            print_success("✅ Triggers and functions created");
        }
        Err(e) => {
            if e.to_string().contains("vector") || e.to_string().contains("extension") {
                print_warning(&format!("⚠️  Could not enable pgvector extension: {}", e));
                print_warning("Please run on the database server (192.168.1.192):");
                println!(
                    "  sudo -u postgres psql -d snaprag -c 'CREATE EXTENSION IF NOT EXISTS vector;'"
                );
                println!();
                println!("Then run: snaprag init --force");
                return Err(e);
            } else {
                return Err(e);
            }
        }
    }

    if !skip_indexes {
        print_info("📊 Creating performance-optimized indexes...");
        create_optimized_indexes(snaprag).await?;
        print_success("✅ Indexes created");
    } else {
        print_info("⏭️  Skipping index creation (--skip-indexes)");
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

    // 🚀 DROP all tables (complete reset)
    let tables = [
        "cast_embeddings", // Drop first due to FK constraint
        "user_activity_timeline",
        "user_profiles",
        "user_profile_snapshots",
        "user_profile_trends",
        "user_data",
        "user_data_changes",
        "casts",
        "links",
        "username_proofs",
        "user_activities",
        "processed_messages",
        "sync_progress",
        "sync_stats",
    ];

    for table in &tables {
        match sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table))
            .execute(snaprag.database().pool())
            .await
        {
            Ok(_) => print_success(&format!("Dropped table: {}", table)),
            Err(e) => print_warning(&format!("Could not drop {}: {}", table, e)),
        }
    }

    // Drop the trigger function
    match sqlx::query("DROP FUNCTION IF EXISTS update_cast_embeddings_updated_at() CASCADE")
        .execute(snaprag.database().pool())
        .await
    {
        Ok(_) => print_success("Dropped trigger function"),
        Err(_) => {}
    }

    println!();
    print_success("✅ Database completely reset!");
    print_info("ℹ️  To reinitialize, run:");
    println!("   snaprag init --force");

    Ok(())
}

/// Run complete database initialization from migration file
async fn run_complete_init(snaprag: &SnapRag) -> Result<()> {
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

    // Clean up temp file
    std::fs::remove_file(temp_sql_path).ok();

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
                "Extension error: {}",
                stderr
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

/// Create performance-optimized indexes
async fn create_optimized_indexes(snaprag: &SnapRag) -> Result<()> {
    let pool = snaprag.database().pool();

    // Only create essential indexes for write-heavy workload

    // 1. user_activity_timeline (most critical)
    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_activity_fid_register 
         ON user_activity_timeline(fid, activity_type) 
         WHERE activity_type = 'id_register'",
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query(
        "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_activity_timeline_fid_timestamp 
         ON user_activity_timeline(fid, timestamp DESC)",
    )
    .execute(pool)
    .await
    .ok();

    // 2. casts (essential for message lookups)
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
    sqlx::query("ANALYZE user_activity_timeline")
        .execute(pool)
        .await
        .ok();
    sqlx::query("ANALYZE casts").execute(pool).await.ok();
    sqlx::query("ANALYZE user_profiles")
        .execute(pool)
        .await
        .ok();

    Ok(())
}

