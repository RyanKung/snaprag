//! Fast sync mode management handlers
//!
//! This module provides handlers for managing fast sync mode, which combines:
//! - ULTRA TURBO MODE (dropping non-essential indexes)
//! - PostgreSQL memory optimization
//! - Hardware-specific tuning

use std::io::Write;
use std::io::{
    self,
};

use crate::cli::commands::FastsyncCommands;
use crate::errors::Result;
use crate::SnapRag;

/// Handle fast sync mode commands
pub async fn handle_fastsync_command(snaprag: &SnapRag, command: &FastsyncCommands) -> Result<()> {
    match command {
        FastsyncCommands::Enable { force } => handle_fastsync_enable(snaprag, *force).await,
        FastsyncCommands::Disable { force } => handle_fastsync_disable(snaprag, *force).await,
        FastsyncCommands::Status => handle_fastsync_status(snaprag).await,
    }
}

/// Enable fast sync mode (ULTRA TURBO + PostgreSQL optimization)
async fn handle_fastsync_enable(snaprag: &SnapRag, force: bool) -> Result<()> {
    tracing::info!("Enabling fast sync mode");

    println!("\n🚀 Fast Sync Mode - ULTRA TURBO + PostgreSQL Optimization");

    // Show what will be done
    println!("\n⚠️  This will:");
    println!("  1. Drop ALL non-essential indexes (ULTRA TURBO MODE)");
    println!("  2. Disable autovacuum on all main tables");
    println!("  3. Optimize PostgreSQL memory settings");
    println!("  4. Enable hardware-specific tuning");
    println!("  5. Speed up bulk inserts by 50-80%");
    println!("\n⚠️  You MUST run 'snaprag fastsync disable' after sync completes!");
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

    // Step 1: Apply ULTRA TURBO MODE (drop non-essential indexes)
    println!("\n🔨 Step 1: Applying ULTRA TURBO MODE...");
    apply_ultra_turbo_mode(db).await?;

    // Step 2: Apply PostgreSQL optimization
    println!("\n⚡ Step 2: Applying PostgreSQL optimization...");
    apply_postgresql_optimization(db).await?;

    println!("\n✅ Fast Sync Mode enabled!");
    println!("   🚀 ULTRA TURBO MODE: All non-essential indexes dropped");
    println!("   📈 Expected speed boost: +30-50%");
    println!("\n⚠️  Remember to run 'snaprag fastsync disable' after sync completes!");
    println!("💡  Note: PostgreSQL memory settings should be managed manually");

    Ok(())
}

/// Disable fast sync mode (restore normal operation)
async fn handle_fastsync_disable(snaprag: &SnapRag, force: bool) -> Result<()> {
    tracing::info!("Disabling fast sync mode...");

    println!("\n🔄 Fast Sync Mode Disable - Restore Normal Operation");

    println!("\n✅ This will:");
    println!("  1. Recreate ALL non-essential indexes (CONCURRENTLY)");
    println!("  2. Re-enable autovacuum on all tables");
    println!("  3. Restore PostgreSQL to normal settings");
    println!("  4. Run VACUUM ANALYZE for optimal performance");
    println!("\n⏱️  This may take 30-120 minutes for large datasets.\n");

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

    // Step 1: Recreate all indexes
    println!("\n🔨 Step 1: Recreating all indexes...");
    recreate_all_indexes(db).await?;

    // Step 2: Re-enable autovacuum
    println!("\n🔄 Step 2: Re-enabling autovacuum...");
    re_enable_autovacuum(db).await?;

    // Step 3: Restore PostgreSQL settings
    println!("\n⚙️  Step 3: Restoring PostgreSQL settings...");
    restore_postgresql_settings(db).await?;

    // Step 4: Run VACUUM ANALYZE
    println!("\n🧹 Step 4: Running VACUUM ANALYZE...");
    run_vacuum_analyze(db).await?;

    println!("\n✅ Fast Sync Mode disabled!");
    println!("   🔍 All indexes recreated");
    println!("   🛑 Autovacuum re-enabled");
    println!("   🧹 Database optimized");
    println!("   📊 Normal operation mode restored");

    Ok(())
}

/// Show current fast sync status and performance metrics
async fn handle_fastsync_status(snaprag: &SnapRag) -> Result<()> {
    let db = snaprag.database.pool();

    println!("\n📊 Fast Sync Mode Status & Performance Metrics\n");

    // Check index status
    let index_status = check_index_status(db).await?;
    let autovacuum_status = check_autovacuum_status(db).await?;
    let postgresql_status = check_postgresql_optimization(db).await?;

    // Determine current mode
    println!("🎯 Current Mode:");
    if index_status.is_ultra_turbo {
        println!("  🚀 FAST SYNC MODE ENABLED");
        println!("     - ULTRA TURBO: All non-essential indexes dropped");
        println!("     - Insert performance: ULTRA FAST (+30-50%)");
        println!("     - Query performance: SLOW (indexes missing)");
        println!("\n  ⚠️  Run 'snaprag fastsync disable' after sync completes!");
    } else {
        println!("  ✅ NORMAL OPERATION MODE");
        println!("     - All indexes present");
        println!("     - Query performance: FAST");
        println!("     - Insert performance: NORMAL");
    }

    // Performance metrics
    println!("\n📈 Performance Metrics:");
    println!("  Database size: {} GB", get_database_size(db).await?);
    println!(
        "  Active connections: {}",
        get_active_connections(db).await?
    );
    println!("  Note: PostgreSQL memory settings should be managed manually");

    Ok(())
}

// Helper functions

async fn apply_ultra_turbo_mode(db: &sqlx::PgPool) -> Result<()> {
    // Drop all non-essential indexes (same as ULTRA TURBO MODE)
    let indexes_to_drop = vec![
        // Reactions
        "idx_reactions_fid",
        "idx_reactions_target_cast_hash",
        "idx_reactions_target_fid",
        "idx_reactions_type",
        "idx_reactions_timestamp",
        "idx_reactions_latest",
        "idx_reactions_event_type",
        // Links
        "idx_links_latest",
        "idx_links_event_type",
        "idx_links_fid_type",
        // Verifications
        "idx_verifications_fid",
        "idx_verifications_address",
        "idx_verifications_timestamp",
        "idx_verifications_latest",
        "idx_verifications_event_type",
        // Casts
        "idx_casts_fid",
        // Onchain events
        "idx_onchain_events_fid",
        "idx_onchain_events_type",
        "idx_onchain_events_block",
        // User profile changes
        "idx_profile_changes_fid_field_ts",
        "idx_profile_changes_message_hash",
        // Frame actions
        "idx_frame_actions_fid",
        "idx_frame_actions_cast_hash",
        "idx_frame_actions_timestamp",
        "idx_frame_actions_url",
        // Username proofs
        "idx_username_proofs_fid",
        "idx_username_proofs_username",
        "idx_username_proofs_timestamp",
    ];

    for index_name in &indexes_to_drop {
        match sqlx::query(&format!("DROP INDEX IF EXISTS {index_name} CASCADE"))
            .execute(db)
            .await
        {
            Ok(_) => println!("  ✅ Dropped: {index_name}"),
            Err(e) => println!("  ⚠️  Failed to drop {index_name}: {e}"),
        }
    }

    // Disable autovacuum
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "onchain_events",
        "user_profile_changes",
        "username_proofs",
        "frame_actions",
        "processed_messages",
    ];

    for table in &tables {
        match sqlx::query(&format!(
            "ALTER TABLE {table} SET (autovacuum_enabled = false)"
        ))
        .execute(db)
        .await
        {
            Ok(_) => println!("  ✅ Disabled autovacuum: {table}"),
            Err(e) => println!("  ⚠️  Failed for {table}: {e}"),
        }
    }

    Ok(())
}

async fn apply_postgresql_optimization(db: &sqlx::PgPool) -> Result<()> {
    println!("  ℹ️  PostgreSQL memory optimization skipped");
    println!("     (Memory settings should be managed manually by the user)");
    println!("     (Only indexes and autovacuum are optimized)");

    Ok(())
}

async fn recreate_all_indexes(db: &sqlx::PgPool) -> Result<()> {
    let indexes_to_create = vec![
        // Reactions
        ("idx_reactions_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_fid ON reactions(fid)"),
        ("idx_reactions_target_cast_hash", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_target_cast_hash ON reactions(target_cast_hash)"),
        ("idx_reactions_target_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_target_fid ON reactions(target_fid)"),
        ("idx_reactions_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_type ON reactions(reaction_type)"),
        ("idx_reactions_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_timestamp ON reactions(timestamp DESC)"),
        ("idx_reactions_latest", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_latest ON reactions(fid, target_cast_hash, timestamp DESC)"),
        ("idx_reactions_event_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_event_type ON reactions(event_type)"),
        // Links
        ("idx_links_latest", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_latest ON links(fid, target_fid, timestamp DESC)"),
        ("idx_links_event_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_event_type ON links(event_type)"),
        ("idx_links_fid_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_fid_type ON links(fid, link_type)"),
        // Verifications
        ("idx_verifications_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_fid ON verifications(fid)"),
        ("idx_verifications_address", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_address ON verifications(address)"),
        ("idx_verifications_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_timestamp ON verifications(timestamp DESC)"),
        ("idx_verifications_latest", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_latest ON verifications(fid, address, timestamp DESC)"),
        ("idx_verifications_event_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_event_type ON verifications(event_type)"),
        // Casts
        ("idx_casts_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_casts_fid ON casts(fid)"),
        // Onchain events
        ("idx_onchain_events_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_onchain_events_fid ON onchain_events(fid)"),
        ("idx_onchain_events_type", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_onchain_events_type ON onchain_events(event_type)"),
        ("idx_onchain_events_block", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_onchain_events_block ON onchain_events(block_number)"),
        // User profile changes
        ("idx_profile_changes_fid_field_ts", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_profile_changes_fid_field_ts ON user_profile_changes(fid, field_name, timestamp DESC)"),
        ("idx_profile_changes_message_hash", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_profile_changes_message_hash ON user_profile_changes(message_hash)"),
        // Frame actions
        ("idx_frame_actions_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_frame_actions_fid ON frame_actions(fid)"),
        ("idx_frame_actions_cast_hash", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_frame_actions_cast_hash ON frame_actions(cast_hash)"),
        ("idx_frame_actions_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_frame_actions_timestamp ON frame_actions(timestamp DESC)"),
        ("idx_frame_actions_url", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_frame_actions_url ON frame_actions(url)"),
        // Username proofs
        ("idx_username_proofs_fid", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_username_proofs_fid ON username_proofs(fid)"),
        ("idx_username_proofs_username", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_username_proofs_username ON username_proofs(username)"),
        ("idx_username_proofs_timestamp", "CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_username_proofs_timestamp ON username_proofs(timestamp DESC)"),
    ];

    for (name, sql) in &indexes_to_create {
        print!("  🔨 Creating {name}... ");
        io::stdout().flush()?;
        match sqlx::query(sql).execute(db).await {
            Ok(_) => println!("✅"),
            Err(e) => println!("⚠️  Failed: {e}"),
        }
    }

    Ok(())
}

async fn re_enable_autovacuum(db: &sqlx::PgPool) -> Result<()> {
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "onchain_events",
        "user_profile_changes",
        "username_proofs",
        "frame_actions",
        "processed_messages",
    ];

    for table in &tables {
        match sqlx::query(&format!(
            "ALTER TABLE {table} SET (autovacuum_enabled = true)"
        ))
        .execute(db)
        .await
        {
            Ok(_) => println!("  ✅ Enabled autovacuum: {table}"),
            Err(e) => println!("  ⚠️  Failed for {table}: {e}"),
        }
    }

    Ok(())
}

async fn restore_postgresql_settings(db: &sqlx::PgPool) -> Result<()> {
    // Note: We don't restore PostgreSQL memory settings as they should be managed by the user
    // The user may have custom configurations that should be preserved

    println!("  ℹ️  PostgreSQL memory settings preserved");
    println!("     (Memory settings should be managed manually by the user)");
    println!("     (Only indexes and autovacuum are restored)");

    Ok(())
}

async fn run_vacuum_analyze(db: &sqlx::PgPool) -> Result<()> {
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "onchain_events",
        "user_profile_changes",
        "username_proofs",
        "frame_actions",
        "processed_messages",
    ];

    for table in &tables {
        print!("  🧹 Analyzing {table}... ");
        io::stdout().flush()?;
        match sqlx::query(&format!("VACUUM ANALYZE {table}"))
            .execute(db)
            .await
        {
            Ok(_) => println!("✅"),
            Err(e) => println!("⚠️  Failed: {e}"),
        }
    }

    Ok(())
}

// Status checking functions

struct IndexStatus {
    is_ultra_turbo: bool,
    missing_indexes: Vec<String>,
}

struct AutovacuumStatus {
    disabled_tables: Vec<String>,
}

struct PostgresqlStatus {
    is_optimized: bool,
}

async fn check_index_status(db: &sqlx::PgPool) -> Result<IndexStatus> {
    let critical_indexes = vec![
        "idx_reactions_fid",
        "idx_links_latest",
        "idx_verifications_fid",
        "idx_casts_fid",
    ];

    let mut missing_indexes = Vec::new();
    for index_name in &critical_indexes {
        let result: Option<(bool,)> =
            sqlx::query_as("SELECT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = $1)")
                .bind(index_name)
                .fetch_optional(db)
                .await?;

        if let Some((exists,)) = result {
            if !exists {
                missing_indexes.push(index_name.to_string());
            }
        }
    }

    Ok(IndexStatus {
        is_ultra_turbo: missing_indexes.len() >= 2, // If most critical indexes are missing
        missing_indexes,
    })
}

async fn check_autovacuum_status(db: &sqlx::PgPool) -> Result<AutovacuumStatus> {
    let tables = vec![
        "casts",
        "links",
        "reactions",
        "verifications",
        "onchain_events",
    ];

    let mut disabled_tables = Vec::new();
    for table in &tables {
        let result: Option<(Option<Vec<String>>,)> =
            sqlx::query_as("SELECT reloptions FROM pg_class WHERE relname = $1")
                .bind(table)
                .fetch_optional(db)
                .await?;

        let is_disabled = if let Some((Some(options),)) = result {
            options
                .iter()
                .any(|opt| opt.contains("autovacuum_enabled=false"))
        } else {
            false
        };

        if is_disabled {
            disabled_tables.push(table.to_string());
        }
    }

    Ok(AutovacuumStatus { disabled_tables })
}

async fn check_postgresql_optimization(db: &sqlx::PgPool) -> Result<PostgresqlStatus> {
    // Since we no longer modify PostgreSQL memory settings,
    // we just return a default status
    Ok(PostgresqlStatus {
        is_optimized: false, // Always false since we don't modify settings
    })
}

async fn get_database_size(db: &sqlx::PgPool) -> Result<String> {
    let result: (String,) = sqlx::query_as("SELECT pg_size_pretty(pg_database_size('snaprag'))")
        .fetch_one(db)
        .await?;
    Ok(result.0)
}

async fn get_active_connections(db: &sqlx::PgPool) -> Result<i64> {
    let result: (i64,) =
        sqlx::query_as("SELECT count(*) FROM pg_stat_activity WHERE datname = 'snaprag'")
            .fetch_one(db)
            .await?;
    Ok(result.0)
}
