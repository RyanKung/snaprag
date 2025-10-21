//! Database index and autovacuum management handlers
//!
//! This module provides handlers for managing database indexes and autovacuum settings
//! during bulk synchronization operations. Disabling non-essential indexes and autovacuum
//! can significantly speed up bulk inserts (30-70% faster).

use crate::cli::commands::IndexCommands;
use crate::errors::Result;
use crate::SnapRag;
use std::io::{self, Write};

/// Handle index management commands
pub async fn handle_index_command(snaprag: &SnapRag, command: &IndexCommands) -> Result<()> {
    match command {
        IndexCommands::Unset { force } => handle_index_unset(snaprag, *force).await,
        IndexCommands::Set { force } => handle_index_set(snaprag, *force).await,
        IndexCommands::Status => handle_index_status(snaprag).await,
    }
}

/// Disable non-essential indexes and autovacuum for bulk operations
async fn handle_index_unset(snaprag: &SnapRag, force: bool) -> Result<()> {
    tracing::info!("Preparing to disable non-essential indexes and autovacuum...");

    // Show what will be done
    println!("\n⚠️  This will:");
    println!("  1. Drop non-essential indexes (idx_casts_fid, idx_user_profiles_username, etc.)");
    println!("  2. Disable autovacuum on all main tables");
    println!("  3. Speed up bulk inserts by 30-70%");
    println!("\n⚠️  You MUST run 'snaprag index set' after bulk sync completes!");
    println!("     Without indexes, queries will be VERY slow.\n");

    if !force {
        print!("Continue? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Aborted");
            return Ok(());
        }
    }

    let db = snaprag.database.pool();

    println!("\n🔨 Dropping non-essential indexes...");

    // Drop non-essential indexes (keep primary keys and unique constraints)
    let indexes_to_drop = vec![
        "idx_casts_fid",
        "idx_casts_timestamp",
        "idx_user_profiles_username",
        "idx_user_profiles_display_name",
        "idx_links_source_fid",
        "idx_links_target_fid",
        "idx_links_timestamp",
        "idx_reactions_fid",
        "idx_reactions_target_cast_hash",
        "idx_reactions_timestamp",
        "idx_verifications_fid",
        "idx_verifications_timestamp",
        "idx_user_data_fid",
        "idx_user_data_type",
    ];

    for index_name in &indexes_to_drop {
        match sqlx::query(&format!("DROP INDEX IF EXISTS {} CASCADE", index_name))
            .execute(db)
            .await
        {
            Ok(_) => println!("  ✅ Dropped: {}", index_name),
            Err(e) => println!("  ⚠️  Failed to drop {}: {}", index_name, e),
        }
    }

    println!("\n🛑 Disabling autovacuum...");

    // Disable autovacuum on all main tables
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "user_profiles",
        "user_data",
    ];

    for table in &tables {
        match sqlx::query(&format!(
            "ALTER TABLE {} SET (autovacuum_enabled = false)",
            table
        ))
        .execute(db)
        .await
        {
            Ok(_) => println!("  ✅ Disabled autovacuum: {}", table),
            Err(e) => println!("  ⚠️  Failed for {}: {}", table, e),
        }
    }

    println!("\n✅ Done! Bulk sync mode enabled.");
    println!("   Speed boost: +30-70% for inserts");
    println!("\n⚠️  Remember to run 'snaprag index set' after sync completes!");

    Ok(())
}

/// Re-enable indexes and autovacuum after bulk operations
async fn handle_index_set(snaprag: &SnapRag, force: bool) -> Result<()> {
    tracing::info!("Preparing to re-enable indexes and autovacuum...");

    println!("\n✅ This will:");
    println!("  1. Recreate all non-essential indexes (CONCURRENTLY, won't block writes)");
    println!("  2. Re-enable autovacuum on all tables");
    println!("  3. Run VACUUM ANALYZE to optimize query performance");
    println!("\n⏱️  This may take 30-60 minutes for large datasets.\n");

    if !force {
        print!("Continue? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("❌ Aborted");
            return Ok(());
        }
    }

    let db = snaprag.database.pool();

    println!("\n🔨 Recreating indexes (CONCURRENTLY)...");

    // Recreate indexes with CONCURRENTLY (won't block writes)
    let indexes_to_create = vec![
        ("idx_casts_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_casts_fid ON casts(fid)"),
        ("idx_casts_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_casts_timestamp ON casts(timestamp DESC)"),
        ("idx_user_profiles_username", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_username ON user_profiles(username)"),
        ("idx_user_profiles_display_name", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_display_name ON user_profiles(display_name)"),
        ("idx_links_source_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_source_fid ON links(source_fid)"),
        ("idx_links_target_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_target_fid ON links(target_fid)"),
        ("idx_links_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_timestamp ON links(timestamp DESC)"),
        ("idx_reactions_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_fid ON reactions(fid)"),
        ("idx_reactions_target_cast_hash", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_target_cast_hash ON reactions(target_cast_hash)"),
        ("idx_reactions_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_timestamp ON reactions(timestamp DESC)"),
        ("idx_verifications_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_fid ON verifications(fid)"),
        ("idx_verifications_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_timestamp ON verifications(timestamp DESC)"),
        ("idx_user_data_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_data_fid ON user_data(fid)"),
        ("idx_user_data_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_data_type ON user_data(data_type)"),
    ];

    for (name, sql) in &indexes_to_create {
        print!("  🔨 Creating {}... ", name);
        io::stdout().flush()?;
        match sqlx::query(sql).execute(db).await {
            Ok(_) => println!("✅"),
            Err(e) => println!("⚠️  Failed: {}", e),
        }
    }

    println!("\n🔄 Re-enabling autovacuum...");

    // Re-enable autovacuum on all main tables
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "user_profiles",
        "user_data",
    ];

    for table in &tables {
        match sqlx::query(&format!(
            "ALTER TABLE {} SET (autovacuum_enabled = true)",
            table
        ))
        .execute(db)
        .await
        {
            Ok(_) => println!("  ✅ Enabled autovacuum: {}", table),
            Err(e) => println!("  ⚠️  Failed for {}: {}", table, e),
        }
    }

    println!("\n🧹 Running VACUUM ANALYZE (this may take a while)...");

    for table in &tables {
        print!("  🧹 Analyzing {}... ", table);
        io::stdout().flush()?;
        match sqlx::query(&format!("VACUUM ANALYZE {}", table))
            .execute(db)
            .await
        {
            Ok(_) => println!("✅"),
            Err(e) => println!("⚠️  Failed: {}", e),
        }
    }

    println!("\n✅ Done! Normal operation mode restored.");
    println!("   All indexes recreated");
    println!("   Autovacuum re-enabled");
    println!("   Query performance optimized");

    Ok(())
}

/// Show current status of indexes and autovacuum
async fn handle_index_status(snaprag: &SnapRag) -> Result<()> {
    let db = snaprag.database.pool();

    println!("\n📊 Database Index & Autovacuum Status\n");

    // Check which indexes exist
    println!("🔍 Non-Essential Indexes:");
    let indexes = vec![
        "idx_casts_fid",
        "idx_casts_timestamp",
        "idx_user_profiles_username",
        "idx_user_profiles_display_name",
        "idx_links_source_fid",
        "idx_links_target_fid",
        "idx_links_timestamp",
        "idx_reactions_fid",
        "idx_reactions_target_cast_hash",
        "idx_reactions_timestamp",
        "idx_verifications_fid",
        "idx_verifications_timestamp",
        "idx_user_data_fid",
        "idx_user_data_type",
    ];

    let mut existing_count = 0;
    for index_name in &indexes {
        let result: Option<(bool,)> = sqlx::query_as(
            "SELECT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = $1)",
        )
        .bind(index_name)
        .fetch_optional(db)
        .await?;

        if let Some((exists,)) = result {
            if exists {
                println!("  ✅ {}", index_name);
                existing_count += 1;
            } else {
                println!("  ❌ {} (missing)", index_name);
            }
        }
    }

    println!(
        "\n  Status: {}/{} indexes present",
        existing_count,
        indexes.len()
    );

    // Check autovacuum status
    println!("\n🛑 Autovacuum Status:");
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "user_profiles",
        "user_data",
    ];

    let mut enabled_count = 0;
    for table in &tables {
        let result: Option<(Option<Vec<String>>,)> = sqlx::query_as(
            "SELECT reloptions FROM pg_class WHERE relname = $1",
        )
        .bind(table)
        .fetch_optional(db)
        .await?;

        let is_enabled = if let Some((Some(options),)) = result {
            !options
                .iter()
                .any(|opt| opt.contains("autovacuum_enabled=false"))
        } else {
            true // Default is enabled if no explicit setting
        };

        if is_enabled {
            println!("  ✅ {} (enabled)", table);
            enabled_count += 1;
        } else {
            println!("  ❌ {} (disabled)", table);
        }
    }

    println!(
        "\n  Status: {}/{} tables have autovacuum enabled",
        enabled_count,
        tables.len()
    );

    // Determine current mode
    println!("\n🎯 Current Mode:");
    if existing_count == indexes.len() && enabled_count == tables.len() {
        println!("  ✅ NORMAL OPERATION MODE");
        println!("     - All indexes present");
        println!("     - Autovacuum enabled");
        println!("     - Query performance: FAST");
        println!("     - Insert performance: NORMAL");
    } else if existing_count == 0 && enabled_count == 0 {
        println!("  🚀 BULK SYNC MODE (Turbo)");
        println!("     - Indexes dropped");
        println!("     - Autovacuum disabled");
        println!("     - Query performance: SLOW");
        println!("     - Insert performance: FAST (+30-70%)");
        println!("\n  ⚠️  Run 'snaprag index set' after sync completes!");
    } else {
        println!("  ⚠️  MIXED/INCONSISTENT STATE");
        println!("     - Some indexes missing: {}/{}", indexes.len() - existing_count, indexes.len());
        println!("     - Autovacuum disabled on: {}/{}", tables.len() - enabled_count, tables.len());
        println!("\n  💡 Recommendation:");
        println!("     - Run 'snaprag index unset' before bulk sync");
        println!("     - Run 'snaprag index set' after bulk sync");
    }

    Ok(())
}

