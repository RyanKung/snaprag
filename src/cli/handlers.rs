//! CLI command handlers
//!
//! This module contains all the command handlers for the SnapRAG CLI

use std::sync::Arc;

use crate::cli::commands::Commands;
use crate::cli::commands::DataType;
use crate::cli::commands::EmbeddingsCommands;
use crate::cli::commands::RagCommands;
use crate::cli::commands::SyncCommands;
use crate::cli::output::*;
use crate::database::Database;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

/// Handle list command
pub async fn handle_list_command(
    snaprag: &SnapRag,
    data_type: DataType,
    limit: u32,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: String,
    fid_range: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    bio: Option<String>,
    location: Option<String>,
    twitter: Option<String>,
    github: Option<String>,
    has_username: bool,
    has_display_name: bool,
    has_bio: bool,
) -> Result<()> {
    match data_type {
        DataType::Fid => {
            print_list_header("FIDs", limit);

            // Parse FID range if provided
            let (min_fid, max_fid) = if let Some(range) = fid_range {
                if let Some((min, max)) = range.split_once('-') {
                    (
                        Some(min.parse::<i64>().unwrap_or(0)),
                        Some(max.parse::<i64>().unwrap_or(i64::MAX)),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::FidSortBy::Fid),
                Some("username") => Some(crate::models::FidSortBy::Username),
                Some("last_updated") => Some(crate::models::FidSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::FidSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Asc),
            };

            // Build FID query
            let fid_query = crate::models::FidQuery {
                fid: None,
                min_fid,
                max_fid,
                has_username: if has_username { Some(true) } else { None },
                has_display_name: if has_display_name { Some(true) } else { None },
                has_bio: if has_bio { Some(true) } else { None },
                limit: Some(limit as i64),
                offset: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_fids(fid_query).await?;
            print_fid_list(&profiles);
        }
        DataType::Profiles => {
            print_list_header("User Profiles", limit);

            // Parse sort options
            let sort_by = match sort_by.as_deref() {
                Some("fid") => Some(crate::models::ProfileSortBy::Fid),
                Some("username") => Some(crate::models::ProfileSortBy::Username),
                Some("display_name") => Some(crate::models::ProfileSortBy::DisplayName),
                Some("last_updated") => Some(crate::models::ProfileSortBy::LastUpdated),
                Some("created_at") => Some(crate::models::ProfileSortBy::CreatedAt),
                _ => None,
            };

            let sort_order = match sort_order.as_str() {
                "asc" => Some(crate::models::SortOrder::Asc),
                "desc" => Some(crate::models::SortOrder::Desc),
                _ => Some(crate::models::SortOrder::Desc),
            };

            // Build profile query
            let profile_query = crate::models::UserProfileQuery {
                fid: None,
                username,
                display_name,
                bio,
                location,
                twitter_username: twitter,
                github_username: github,
                limit: Some(limit as i64),
                offset: None,
                start_timestamp: None,
                end_timestamp: None,
                sort_by,
                sort_order,
                search_term: search,
            };

            let profiles = snaprag.database().list_user_profiles(profile_query).await?;
            print_profile_list(&profiles);
        }
        DataType::Casts => {
            print_list_header("Casts", limit);

            // Build cast query
            let cast_query = crate::models::CastQuery {
                fid: None,
                text_search: search,
                parent_hash: None,
                root_hash: None,
                has_mentions: None,
                has_embeds: None,
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::CastSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let casts = snaprag.database().list_casts(cast_query).await?;
            print_cast_list(&casts);
        }
        DataType::Follows => {
            print_list_header("Follows", limit);

            // Build link query for follows
            let link_query = crate::models::LinkQuery {
                fid: None,
                target_fid: None,
                link_type: Some("follow".to_string()),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::LinkSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let links = snaprag.database().list_links(link_query).await?;
            print_link_list(&links);
        }
        DataType::UserData => {
            print_list_header("User Data", limit);

            // Build user data query
            let user_data_query = crate::models::UserDataQuery {
                fid: None,
                data_type: None,
                value_search: search.clone(),
                start_timestamp: None,
                end_timestamp: None,
                limit: Some(limit as i64),
                offset: None,
                sort_by: Some(crate::models::UserDataSortBy::Timestamp),
                sort_order: Some(crate::models::SortOrder::Desc),
            };

            let user_data = snaprag.database().list_user_data(user_data_query).await?;
            print_user_data_list(&user_data);
        }
    }
    Ok(())
}

/// Handle reset command
pub async fn handle_reset_command(snaprag: &SnapRag, force: bool) -> Result<()> {
    if !force {
        print_warning(
            "This will reset ALL synchronized data from the database and remove lock files!",
        );
        print_prompt("Are you sure you want to continue? (y/N)");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().to_lowercase().starts_with('y') {
            print_info("Operation cancelled.");
            return Ok(());
        }
    }

    print_info("Resetting all synchronized data and lock files...");

    // Remove lock file if it exists
    if std::path::Path::new("snaprag.lock").exists() {
        std::fs::remove_file("snaprag.lock")?;
        print_success("Removed snaprag.lock file");
    } else {
        print_info("No lock file found");
    }

    // Clear all tables including sync progress
    let tables = [
        "user_profiles",
        "username_proofs",
        "user_activities",
        "user_activity_timeline",
        "user_data_changes",
        "cast_embeddings",  // Only cast_embeddings exists; profile embeddings are in user_profiles table
        "casts",
        "reactions",
        "verifications",
        "links",
        "user_data",
        "sync_progress", // ⭐ Clear sync progress so next sync starts from 0
        "sync_stats",    // ⭐ Clear sync statistics
    ];

    for table in &tables {
        let deleted = sqlx::query(&format!("DELETE FROM {}", table))
            .execute(snaprag.database().pool())
            .await?;
        print_success(&format!(
            "Deleted {} {} records",
            deleted.rows_affected(),
            table
        ));
    }

    print_success("Database and lock files reset successfully!");
    Ok(())
}

/// Handle activity command
pub async fn handle_activity_command(
    snaprag: &SnapRag,
    fid: i64,
    limit: i64,
    offset: i64,
    activity_type: Option<String>,
    detailed: bool,
) -> Result<()> {
    print_info(&format!("🔍 Querying activity timeline for FID {}", fid));

    // Check if profile exists
    let profile = snaprag.database().get_user_profile(fid).await?;
    if profile.is_none() {
        print_error(&format!("❌ Profile not found for FID {}", fid));
        return Ok(());
    }

    let profile = profile.unwrap();

    // Get registration activity
    let registration = snaprag
        .database()
        .get_user_activity_timeline(
            fid,
            Some("id_register".to_string()),
            None,
            None,
            Some(1),
            Some(0),
        )
        .await?;

    println!("\n👤 Profile Information:");
    if let Some(username) = &profile.username {
        println!("  Username: @{}", username);
    }
    if let Some(display_name) = &profile.display_name {
        println!("  Display Name: {}", display_name);
    }
    println!("  FID: {}", fid);

    // Show registration time if available
    if let Some(reg) = registration.first() {
        if reg.timestamp > 0 {
            // Timestamp is already Unix timestamp, no need to add Farcaster epoch
            if let Some(dt) = chrono::DateTime::from_timestamp(reg.timestamp, 0) {
                println!("  🆕 Registered: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
            }
        }

        // Show registration block if available
        if let Some(data) = &reg.activity_data {
            if let Some(block) = data.get("block_number") {
                println!("  📦 Registration Block: {}", block);
            }
        }
    }

    println!();

    // Get activities
    let activities = snaprag
        .database()
        .get_user_activity_timeline(
            fid,
            activity_type.clone(),
            None, // start_timestamp
            None, // end_timestamp
            Some(limit),
            Some(offset),
        )
        .await?;

    if activities.is_empty() {
        print_warning("No activities found for this user");
        return Ok(());
    }

    // Group activities by type for summary
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for activity in &activities {
        *type_counts
            .entry(activity.activity_type.clone())
            .or_insert(0) += 1;
    }

    // Print summary
    println!("📊 Activity Summary ({} total):", activities.len());
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (activity_type, count) in sorted_types {
        let icon = match activity_type.as_str() {
            "cast_add" => "✍️",
            "cast_remove" => "🗑️",
            "reaction_add" => "❤️",
            "reaction_remove" => "💔",
            "link_add" => "👥",
            "link_remove" => "👋",
            "verification_add" => "✅",
            "verification_remove" => "❌",
            "user_data_add" => "📝",
            "id_register" => "🆕",
            "storage_rent" => "💰",
            "signer_event" => "🔑",
            _ => "📌",
        };
        println!("  {} {}: {}", icon, activity_type, count);
    }
    println!();

    // Print activity timeline
    println!("📅 Activity Timeline:");
    println!("{}", "─".repeat(100));

    for (idx, activity) in activities.iter().enumerate() {
        let icon = match activity.activity_type.as_str() {
            "cast_add" => "✍️",
            "cast_remove" => "🗑️",
            "reaction_add" => "❤️",
            "reaction_remove" => "💔",
            "link_add" => "👥",
            "link_remove" => "👋",
            "verification_add" => "✅",
            "verification_remove" => "❌",
            "user_data_add" => "📝",
            "id_register" => "🆕",
            "storage_rent" => "💰",
            "signer_event" => "🔑",
            _ => "📌",
        };

        // Format timestamp (already Unix timestamp)
        let timestamp_str = if activity.timestamp > 0 {
            chrono::DateTime::from_timestamp(activity.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| activity.timestamp.to_string())
        } else {
            "N/A".to_string()
        };

        println!(
            "{:3}. {} {} | {}",
            offset + idx as i64 + 1,
            icon,
            activity.activity_type,
            timestamp_str
        );

        if detailed {
            if let Some(data) = &activity.activity_data {
                println!(
                    "     Data: {}",
                    serde_json::to_string_pretty(data).unwrap_or_default()
                );
            }
            if let Some(hash) = &activity.message_hash {
                println!("     Hash: {}", hex::encode(hash));
            }
            println!();
        }
    }

    println!("{}", "─".repeat(100));
    println!(
        "\n💡 Tip: Use --limit and --offset for pagination, --activity-type to filter, --detailed for full data"
    );

    Ok(())
}

/// Handle cast search command
pub async fn handle_cast_search(
    snaprag: &SnapRag,
    query: String,
    limit: usize,
    threshold: f32,
    detailed: bool,
) -> Result<()> {
    use crate::embeddings::EmbeddingService;

    print_info(&format!("🔍 Searching casts: \"{}\"", query));

    // Check if we have any cast embeddings
    let embed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cast_embeddings")
        .fetch_one(snaprag.database().pool())
        .await?;

    if embed_count == 0 {
        print_warning("⚠️  No cast embeddings found. Please run:");
        println!("   snaprag embeddings backfill-casts");
        return Ok(());
    }

    // Generate query embedding (create new service instance)
    let config = AppConfig::load()?;
    let embedding_service = EmbeddingService::new(&config)?;
    let query_embedding = embedding_service.generate(&query).await?;

    // Search casts
    let results = snaprag
        .database()
        .semantic_search_casts(query_embedding, limit as i64, Some(threshold))
        .await?;

    if results.is_empty() {
        print_warning(&format!(
            "No casts found matching '{}' (threshold: {:.2})",
            query, threshold
        ));
        return Ok(());
    }

    println!("\n📝 Found {} matching casts:\n", results.len());
    println!("{}", "─".repeat(100));

    for (idx, result) in results.iter().enumerate() {
        // Get author profile
        let author = snaprag.database().get_user_profile(result.fid).await?;
        let author_display = if let Some(profile) = author {
            if let Some(username) = profile.username {
                format!("@{}", username)
            } else if let Some(display_name) = profile.display_name {
                display_name
            } else {
                format!("FID {}", result.fid)
            }
        } else {
            format!("FID {}", result.fid)
        };

        // Format timestamp
        let timestamp_str = chrono::DateTime::from_timestamp(result.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        println!(
            "{}. {} | {} | Similarity: {:.2}%",
            idx + 1,
            author_display,
            timestamp_str,
            result.similarity * 100.0
        );

        // Show cast text (truncate if needed)
        let display_text = if result.text.len() > 200 && !detailed {
            format!("{}...", &result.text[..200])
        } else {
            result.text.clone()
        };
        println!("   {}", display_text);

        if detailed {
            println!("   Hash: {}", hex::encode(&result.message_hash));
            if result.parent_hash.is_some() {
                println!(
                    "   (Reply to: {})",
                    hex::encode(result.parent_hash.as_ref().unwrap())
                );
            }
        }
        println!();
    }

    println!("{}", "─".repeat(100));
    println!("💡 Tip: Use --threshold to adjust sensitivity, --detailed for full info");

    Ok(())
}

/// Handle cast recent command
pub async fn handle_cast_recent(snaprag: &SnapRag, fid: i64, limit: usize) -> Result<()> {
    print_info(&format!("📝 Recent casts by FID {}", fid));

    // Get profile
    let profile = snaprag.database().get_user_profile(fid).await?;
    if profile.is_none() {
        print_error(&format!("❌ Profile not found for FID {}", fid));
        return Ok(());
    }

    let profile = profile.unwrap();
    println!("\n👤 Author:");
    if let Some(username) = &profile.username {
        println!("  @{}", username);
    } else if let Some(display_name) = &profile.display_name {
        println!("  {}", display_name);
    } else {
        println!("  FID {}", fid);
    }
    println!();

    // Get casts
    let casts = snaprag
        .database()
        .get_casts_by_fid(fid, Some(limit as i64), Some(0))
        .await?;

    if casts.is_empty() {
        print_warning("No casts found for this user");
        return Ok(());
    }

    println!("📅 Recent Casts ({} total):", casts.len());
    println!("{}", "─".repeat(100));

    for (idx, cast) in casts.iter().enumerate() {
        let timestamp_str = chrono::DateTime::from_timestamp(cast.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        println!("{}. {}", idx + 1, timestamp_str);
        if let Some(text) = &cast.text {
            println!("   {}", text);
        } else {
            println!("   (No text content)");
        }

        if cast.parent_hash.is_some() {
            println!("   ↳ Reply");
        }
        println!();
    }

    println!("{}", "─".repeat(100));

    Ok(())
}

/// Handle cast thread command
pub async fn handle_cast_thread(snaprag: &SnapRag, hash: String, depth: usize) -> Result<()> {
    print_info(&format!("🧵 Loading cast thread for {}...", &hash[..12]));

    let message_hash = hex::decode(&hash)
        .map_err(|_| crate::SnapRagError::Custom("Invalid hash format".to_string()))?;

    // Get the full thread
    let thread = snaprag
        .database()
        .get_cast_thread(message_hash, depth)
        .await?;

    if thread.root.is_none() {
        print_error(&format!("❌ Cast not found: {}", hash));
        return Ok(());
    }

    let root_cast = thread.root.as_ref().unwrap();

    println!("\n{}", "═".repeat(100));

    // Show parent chain if any
    if !thread.parents.is_empty() {
        println!("⬆️  Parent Context ({} levels):\n", thread.parents.len());

        for (idx, parent) in thread.parents.iter().enumerate() {
            let indent = "  ".repeat(idx);
            let author = snaprag.database().get_user_profile(parent.fid).await?;
            let author_name = if let Some(p) = author {
                p.username
                    .or(p.display_name)
                    .unwrap_or_else(|| format!("FID {}", parent.fid))
            } else {
                format!("FID {}", parent.fid)
            };

            println!("{}📝 {}", indent, author_name);
            if let Some(text) = &parent.text {
                let display_text = if text.len() > 100 {
                    format!("{}...", &text[..100])
                } else {
                    text.clone()
                };
                println!("{}   {}", indent, display_text);
            }
            println!("{}   ↓", indent);
        }
    }

    // Show the target cast
    let indent = "  ".repeat(thread.parents.len());
    let author = snaprag.database().get_user_profile(root_cast.fid).await?;
    let author_name = if let Some(p) = author {
        p.username
            .or(p.display_name)
            .unwrap_or_else(|| format!("FID {}", root_cast.fid))
    } else {
        format!("FID {}", root_cast.fid)
    };

    let timestamp_str = chrono::DateTime::from_timestamp(root_cast.timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    println!("\n{}🎯 {} | {}", indent, author_name, timestamp_str);
    if let Some(text) = &root_cast.text {
        println!("{}   {}", indent, text);
    }
    println!("{}   Hash: {}", indent, &hash[..16]);

    // Show replies if any
    if !thread.children.is_empty() {
        println!("\n⬇️  Replies ({}):\n", thread.children.len());

        for (idx, reply) in thread.children.iter().enumerate() {
            let author = snaprag.database().get_user_profile(reply.fid).await?;
            let author_name = if let Some(p) = author {
                p.username
                    .or(p.display_name)
                    .unwrap_or_else(|| format!("FID {}", reply.fid))
            } else {
                format!("FID {}", reply.fid)
            };

            println!("{}. ↳ {}", idx + 1, author_name);
            if let Some(text) = &reply.text {
                let display_text = if text.len() > 100 {
                    format!("{}...", &text[..100])
                } else {
                    text.clone()
                };
                println!("      {}", display_text);
            }
            println!();
        }
    }

    println!("{}", "═".repeat(100));
    println!(
        "\n📊 Thread Summary: {} parent(s), 1 target, {} reply/replies",
        thread.parents.len(),
        thread.children.len()
    );

    Ok(())
}

/// Handle RAG query on casts
pub async fn handle_rag_query_casts(
    snaprag: &SnapRag,
    query: String,
    limit: usize,
    threshold: f32,
    temperature: f32,
    max_tokens: usize,
    verbose: bool,
) -> Result<()> {
    use std::sync::Arc;

    use crate::embeddings::EmbeddingService;
    use crate::llm::LlmService;
    use crate::rag::CastContextAssembler;
    use crate::rag::CastRetriever;

    print_info(&format!("🤖 RAG Query on Casts: \"{}\"", query));

    // Check if we have embeddings
    let embed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cast_embeddings")
        .fetch_one(snaprag.database().pool())
        .await?;

    if embed_count == 0 {
        print_warning("⚠️  No cast embeddings found. Run: snaprag embeddings backfill-casts");
        return Ok(());
    }

    // Step 1: Retrieve relevant casts using CastRetriever
    println!("\n🔍 Step 1: Retrieving relevant casts...");
    let config = AppConfig::load()?;
    let embedding_service = Arc::new(EmbeddingService::new(&config)?);
    let database = Arc::new(Database::from_config(&config).await?);
    let cast_retriever = CastRetriever::new(database, embedding_service);

    let results = cast_retriever
        .semantic_search(&query, limit, Some(threshold))
        .await?;

    if results.is_empty() {
        print_warning("No relevant casts found");
        return Ok(());
    }

    println!("   ✓ Found {} relevant casts", results.len());

    // Step 2: Assemble context using CastContextAssembler
    println!("🔧 Step 2: Assembling context...");
    let context_assembler = CastContextAssembler::default();
    let context = context_assembler
        .assemble_with_authors(&results, snaprag.database())
        .await?;

    if verbose {
        println!("   Context length: {} chars", context.len());
    }

    // Step 3: Generate answer with LLM using enhanced prompts
    println!("💭 Step 3: Generating answer...");
    let llm_service = LlmService::new(&config)?;

    // Use specialized cast RAG prompt
    let prompt = crate::rag::build_cast_rag_prompt(&query, &context);

    let answer = llm_service
        .generate_with_params(&prompt, temperature, max_tokens)
        .await?;

    // Print results
    println!("\n{}", "═".repeat(100));
    println!("📝 Answer:\n");
    println!("{}", answer.trim());
    println!("\n{}", "═".repeat(100));

    if verbose {
        println!("\n📚 Sources ({} casts):", results.len());
        for (idx, result) in results.iter().enumerate() {
            println!(
                "  {}. FID {} | Similarity: {:.2}% | \"{}...\"",
                idx + 1,
                result.fid,
                result.similarity * 100.0,
                result.text.chars().take(50).collect::<String>()
            );
        }
    } else {
        println!("\n💡 Use --verbose to see source casts");
    }

    Ok(())
}

/// Handle cast embeddings backfill command
pub async fn handle_cast_embeddings_backfill(
    config: &AppConfig,
    limit: Option<usize>,
) -> Result<()> {
    use std::sync::Arc;

    use crate::database::Database;
    use crate::embeddings::backfill_cast_embeddings;
    use crate::embeddings::EmbeddingService;

    print_info("🚀 Starting cast embeddings generation...");

    // Create services
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);

    // Check how many casts need embeddings
    let count = database.count_casts_without_embeddings().await?;

    if count == 0 {
        print_success("✅ All casts already have embeddings!");
        return Ok(());
    }

    let process_count = limit.unwrap_or(count as usize);
    println!("\n📊 Found {} casts without embeddings", count);
    println!("   Processing: {} casts\n", process_count);

    // Run backfill
    let stats = backfill_cast_embeddings(database, embedding_service, limit).await?;

    // Print results
    println!("\n📈 Cast Embeddings Generation Complete:");
    println!("   ✅ Success: {}", stats.success);
    println!("   ⏭️  Skipped: {} (empty text)", stats.skipped);
    if stats.failed > 0 {
        println!("   ❌ Failed: {}", stats.failed);
    }
    println!("   📊 Success Rate: {:.1}%", stats.success_rate() * 100.0);

    print_success(&format!(
        "✅ Generated embeddings for {} casts!",
        stats.success
    ));

    Ok(())
}

/// Handle sync command
pub async fn handle_sync_command(mut snaprag: SnapRag, sync_command: SyncCommands) -> Result<()> {
    match sync_command {
        SyncCommands::All => {
            print_info("Starting full synchronization (historical + real-time)...");
            snaprag.start_sync().await?;
        }
        SyncCommands::Start {
            from,
            to,
            shard,
            batch,
            interval,
        } => {
            let from_block = from.unwrap_or(0);
            let to_block = to.unwrap_or(u64::MAX);

            // Parse shard IDs if provided
            let shard_ids = if let Some(shard_str) = shard {
                shard_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u32>().ok())
                    .collect::<Vec<_>>()
            } else {
                vec![] // Use default from config
            };

            // Apply command-line overrides to config
            if batch.is_some() || interval.is_some() || !shard_ids.is_empty() {
                snaprag.override_sync_config(shard_ids.clone(), batch, interval)?;
            }

            if let Some(to_val) = to {
                print_info(&format!(
                    "Starting synchronization from block {} to block {}{}{}...",
                    from_block,
                    to_val,
                    if let Some(b) = batch {
                        format!(" (batch: {})", b)
                    } else {
                        String::new()
                    },
                    if !shard_ids.is_empty() {
                        format!(" (shards: {:?})", shard_ids)
                    } else {
                        String::new()
                    }
                ));
            } else {
                print_info(&format!(
                    "Starting synchronization from block {} to latest{}{}...",
                    from_block,
                    if let Some(b) = batch {
                        format!(" (batch: {})", b)
                    } else {
                        String::new()
                    },
                    if !shard_ids.is_empty() {
                        format!(" (shards: {:?})", shard_ids)
                    } else {
                        String::new()
                    }
                ));
            }

            snaprag.start_sync_with_range(from_block, to_block).await?;
        }
        SyncCommands::Test { shard, block } => {
            print_info(&format!(
                "Testing single block synchronization for shard {} block {}...",
                shard, block
            ));

            // For test command, we need to create a sync service directly
            let sync_service =
                crate::sync::service::SyncService::new(&snaprag.config, snaprag.database().clone())
                    .await?;

            match sync_service.poll_once(shard, block).await {
                Ok(stats) => {
                    print_success(&format!(
                        "Single block test completed successfully! Blocks processed: {}, messages: {}",
                        stats.blocks_processed(),
                        stats.messages_processed()
                    ));
                }
                Err(e) => {
                    print_error(&format!("Single block test failed: {}", e));
                    return Err(e);
                }
            }
        }
        SyncCommands::Realtime => {
            print_info("Starting real-time synchronization...");
            snaprag.start_sync().await?;
        }
        SyncCommands::Status => {
            print_sync_status(&snaprag)?;
        }
        SyncCommands::Stop { force } => {
            print_info("Stopping sync processes...");
            snaprag.stop_sync(force).await?;

            if force {
                print_success("Force stopped successfully");
            } else {
                print_success("Gracefully stopped successfully");
            }
        }
    }
    Ok(())
}

/// Handle stats command
pub async fn handle_stats_command(
    snaprag: &SnapRag,
    detailed: bool,
    export: Option<String>,
) -> Result<()> {
    let stats = snaprag.get_statistics().await?;
    print_statistics(&stats, detailed);

    if let Some(export_path) = export {
        let json = serde_json::to_string_pretty(&stats)?;
        std::fs::write(&export_path, json)?;
        print_success(&format!("Statistics exported to: {}", export_path));
    }

    Ok(())
}

/// Handle search command
pub async fn handle_search_command(
    snaprag: &SnapRag,
    query: String,
    limit: u32,
    fields: String,
) -> Result<()> {
    print_search_header(&query, &fields);

    let profiles = snaprag.search_profiles(&query).await?;
    print_search_results(&profiles, limit as usize);

    Ok(())
}

/// Handle dashboard command
pub async fn handle_dashboard_command(snaprag: &SnapRag) -> Result<()> {
    let stats = snaprag.get_statistics().await?;
    print_dashboard(&stats);
    Ok(())
}

/// Handle config command
pub async fn handle_config_command(config: &AppConfig) -> Result<()> {
    print_config(config);
    Ok(())
}

/// Print sync status
fn print_sync_status(snaprag: &SnapRag) -> Result<()> {
    print_info("Sync Status:");

    match snaprag.get_sync_status()? {
        Some(lock) => {
            println!("  - Status: {}", lock.status);
            println!("  - PID: {}", lock.pid);
            println!(
                "  - Start time: {}",
                chrono::DateTime::from_timestamp(lock.start_time as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "  - Last update: {}",
                chrono::DateTime::from_timestamp(lock.last_update as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
            );

            if let Some(shard) = lock.progress.current_shard {
                println!("  - Current shard: {}", shard);
            }
            if let Some(block) = lock.progress.current_block {
                println!("  - Current block: {}", block);
            }
            println!(
                "  - Total blocks processed: {}",
                lock.progress.total_blocks_processed
            );
            println!(
                "  - Total messages processed: {}",
                lock.progress.total_messages_processed
            );

            if let Some(range) = &lock.progress.sync_range {
                println!(
                    "  - Sync range: {} to {}",
                    range.from_block,
                    range
                        .to_block
                        .map(|b| b.to_string())
                        .unwrap_or("latest".to_string())
                );
            }

            if let Some(error) = &lock.error_message {
                println!("  - Error: {}", error);
            }
        }
        None => {
            println!("  - No active sync process");
        }
    }
    Ok(())
}

/// Handle RAG query command
pub async fn handle_rag_query(
    config: &AppConfig,
    query: String,
    limit: usize,
    method: String,
    temperature: f32,
    max_tokens: usize,
    verbose: bool,
) -> Result<()> {
    use crate::rag::RagQuery;
    use crate::rag::RagService;
    use crate::rag::RetrievalMethod;

    println!("🤖 SnapRAG Query");
    println!("================\n");
    println!("Question: {}\n", query);

    // Parse retrieval method
    let retrieval_method = match method.as_str() {
        "semantic" => RetrievalMethod::Semantic,
        "keyword" => RetrievalMethod::Keyword,
        "hybrid" => RetrievalMethod::Hybrid,
        _ => RetrievalMethod::Auto,
    };

    println!("⏳ Initializing RAG service...");
    let rag_service = RagService::new(config).await?;

    println!("🔍 Retrieving relevant profiles...");
    let rag_query = RagQuery {
        question: query.clone(),
        retrieval_limit: limit,
        retrieval_method,
        temperature,
        max_tokens,
    };

    let response = rag_service.query_with_options(rag_query).await?;

    println!("\n📝 Answer:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{}", response.answer);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📚 Sources ({} profiles):", response.sources.len());
    for (idx, source) in response.sources.iter().enumerate().take(10) {
        let username = source.profile.username.as_deref().unwrap_or("unknown");
        let display_name = source.profile.display_name.as_deref().unwrap_or("No name");

        println!(
            "  {}. @{} ({}) - FID: {}, Score: {:.3}, Match: {:?}",
            idx + 1,
            username,
            display_name,
            source.profile.fid,
            source.score,
            source.match_type
        );

        if verbose {
            if let Some(bio) = &source.profile.bio {
                let bio_preview = if bio.len() > 100 {
                    format!("{}...", &bio[..100])
                } else {
                    bio.clone()
                };
                println!("     Bio: {}", bio_preview);
            }
        }
    }

    if response.sources.len() > 10 {
        println!("  ... and {} more", response.sources.len() - 10);
    }

    Ok(())
}

/// Handle RAG search command
pub async fn handle_rag_search(
    config: &AppConfig,
    query: String,
    limit: usize,
    method: String,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::EmbeddingService;
    use crate::rag::Retriever;

    println!("🔍 SnapRAG Search");
    println!("=================\n");
    println!("Query: {}\n", query);

    println!("⏳ Initializing search...");
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);
    let retriever = Retriever::new(database, embedding_service);

    println!("🔎 Searching profiles...");
    let results = match method.as_str() {
        "semantic" => retriever.semantic_search(&query, limit, None).await?,
        "keyword" => retriever.keyword_search(&query, limit).await?,
        "hybrid" => retriever.hybrid_search(&query, limit).await?,
        _ => retriever.auto_search(&query, limit).await?,
    };

    println!("\n✅ Found {} profiles:\n", results.len());

    for (idx, result) in results.iter().enumerate() {
        let username = result.profile.username.as_deref().unwrap_or("unknown");
        let display_name = result.profile.display_name.as_deref().unwrap_or("No name");

        println!(
            "{}. @{} ({}) - FID: {}",
            idx + 1,
            username,
            display_name,
            result.profile.fid
        );
        println!(
            "   Score: {:.3} | Match Type: {:?}",
            result.score, result.match_type
        );

        if let Some(bio) = &result.profile.bio {
            let bio_preview = if bio.len() > 150 {
                format!("{}...", &bio[..150])
            } else {
                bio.clone()
            };
            println!("   Bio: {}", bio_preview);
        }

        if let Some(location) = &result.profile.location {
            println!("   Location: {}", location);
        }

        println!();
    }

    Ok(())
}

/// Handle embeddings backfill command
pub async fn handle_embeddings_backfill(
    config: &AppConfig,
    force: bool,
    _batch_size: usize,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::backfill_embeddings;
    use crate::embeddings::EmbeddingService;

    println!("📊 Embeddings Backfill");
    println!("======================\n");

    if !force {
        println!("⚠️  This will generate embeddings for all profiles in the database.");
        println!("⚠️  This may take a long time and incur API costs.");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Initializing services...");
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);

    println!("🚀 Starting backfill process...\n");
    let stats = backfill_embeddings(database, embedding_service).await?;

    println!("\n✅ Backfill Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Profiles: {}", stats.total_profiles);
    println!("Updated: {}", stats.updated);
    println!("Skipped: {}", stats.skipped);
    println!("Failed: {}", stats.failed);
    println!("Success Rate: {:.1}%", stats.success_rate());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Handle embeddings generate command
pub async fn handle_embeddings_generate(config: &AppConfig, fid: i64, verbose: bool) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::EmbeddingService;

    println!("🔮 Generate Embeddings for FID: {}", fid);
    println!("====================================\n");

    println!("⏳ Initializing services...");
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);

    println!("📊 Fetching profile...");
    let profile_query = crate::models::UserProfileQuery {
        fid: Some(fid),
        username: None,
        display_name: None,
        bio: None,
        location: None,
        twitter_username: None,
        github_username: None,
        limit: Some(1),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
        sort_by: None,
        sort_order: None,
        search_term: None,
    };

    let profiles = database.list_user_profiles(profile_query).await?;
    let profile = profiles.into_iter().next().ok_or_else(|| {
        crate::SnapRagError::Custom(format!("Profile not found for FID: {}", fid))
    })?;

    println!(
        "✅ Found profile: @{}",
        profile.username.as_deref().unwrap_or("unknown")
    );
    println!("\n🔮 Generating embeddings...");

    // Generate embeddings
    let profile_embedding = embedding_service
        .generate_profile_embedding(
            profile.username.as_deref(),
            profile.display_name.as_deref(),
            profile.bio.as_deref(),
            profile.location.as_deref(),
        )
        .await?;

    let bio_embedding = embedding_service
        .generate_bio_embedding(profile.bio.as_deref())
        .await?;

    let interests_embedding = embedding_service
        .generate_interests_embedding(
            profile.bio.as_deref(),
            profile.twitter_username.as_deref(),
            profile.github_username.as_deref(),
        )
        .await?;

    println!("✅ Generated embeddings:");
    println!("  - Profile: {} dimensions", profile_embedding.len());
    println!("  - Bio: {} dimensions", bio_embedding.len());
    println!("  - Interests: {} dimensions", interests_embedding.len());

    if verbose {
        println!("\n📊 Sample values (first 10 dimensions):");
        println!(
            "  Profile: {:?}",
            &profile_embedding[..10.min(profile_embedding.len())]
        );
        println!("  Bio: {:?}", &bio_embedding[..10.min(bio_embedding.len())]);
        println!(
            "  Interests: {:?}",
            &interests_embedding[..10.min(interests_embedding.len())]
        );
    }

    println!("\n💾 Saving to database...");
    database
        .update_profile_embeddings(
            fid,
            Some(profile_embedding),
            Some(bio_embedding),
            Some(interests_embedding),
        )
        .await?;

    println!("✅ Embeddings saved successfully!");

    Ok(())
}

/// Handle embeddings test command
pub async fn handle_embeddings_test(config: &AppConfig, text: String) -> Result<()> {
    use crate::embeddings::EmbeddingService;

    println!("🧪 Test Embedding Generation");
    println!("============================\n");
    println!("Text: {}\n", text);

    println!("⏳ Initializing embedding service...");
    let embedding_service = EmbeddingService::new(config)?;

    println!("🔮 Generating embedding...");
    let start = std::time::Instant::now();
    let embedding = embedding_service.generate(&text).await?;
    let duration = start.elapsed();

    println!("✅ Generated embedding in {:?}", duration);
    println!("\n📊 Embedding Details:");
    println!("  - Dimension: {}", embedding.len());
    println!("  - Model: {}", embedding_service.model());
    println!("  - Provider: {:?}", embedding_service.provider());
    println!("\n📈 Sample values (first 20 dimensions):");
    println!("  {:?}", &embedding[..20.min(embedding.len())]);

    // Calculate basic statistics
    let sum: f32 = embedding.iter().sum();
    let mean = sum / embedding.len() as f32;
    let variance: f32 =
        embedding.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / embedding.len() as f32;
    let std_dev = variance.sqrt();

    println!("\n📊 Statistics:");
    println!("  - Mean: {:.6}", mean);
    println!("  - Std Dev: {:.6}", std_dev);
    println!(
        "  - Min: {:.6}",
        embedding.iter().cloned().fold(f32::INFINITY, f32::min)
    );
    println!(
        "  - Max: {:.6}",
        embedding.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    );

    Ok(())
}

/// Handle embeddings stats command
pub async fn handle_embeddings_stats(config: &AppConfig) -> Result<()> {
    use sqlx::Row;

    use crate::database::Database;

    println!("📊 Embeddings Statistics");
    println!("========================\n");

    let database = Database::from_config(config).await?;

    println!("⏳ Querying database...\n");

    // Count total profiles
    let total: i64 = sqlx::query("SELECT COUNT(*) as count FROM user_profiles")
        .fetch_one(database.pool())
        .await?
        .try_get("count")?;

    // Count profiles with embeddings
    let with_profile_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM user_profiles WHERE profile_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    let with_bio_emb: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM user_profiles WHERE bio_embedding IS NOT NULL")
            .fetch_one(database.pool())
            .await?
            .try_get("count")?;

    let with_interests_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM user_profiles WHERE interests_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    let with_all_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM user_profiles 
         WHERE profile_embedding IS NOT NULL 
           AND bio_embedding IS NOT NULL 
           AND interests_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    println!("📈 Coverage:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Profiles: {}", total);
    println!(
        "With Profile Embedding: {} ({:.1}%)",
        with_profile_emb,
        (with_profile_emb as f64 / total as f64) * 100.0
    );
    println!(
        "With Bio Embedding: {} ({:.1}%)",
        with_bio_emb,
        (with_bio_emb as f64 / total as f64) * 100.0
    );
    println!(
        "With Interests Embedding: {} ({:.1}%)",
        with_interests_emb,
        (with_interests_emb as f64 / total as f64) * 100.0
    );
    println!(
        "With All Embeddings: {} ({:.1}%)",
        with_all_emb,
        (with_all_emb as f64 / total as f64) * 100.0
    );

    let missing = total - with_all_emb;
    if missing > 0 {
        println!("\n⚠️  {} profiles need embeddings", missing);
        println!("   Run: cargo run embeddings backfill --force");
    } else {
        println!("\n✅ All profiles have embeddings!");
    }

    Ok(())
}
