//! Embedding generation handlers

use std::sync::Arc;

use crate::cli::output::*;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

pub async fn handle_cast_embeddings_backfill(
    config: &AppConfig,
    limit: Option<usize>,
    endpoint_name: Option<String>,
) -> Result<()> {
    use std::sync::Arc;

    use crate::database::Database;
    use crate::embeddings::backfill_cast_embeddings;
    use crate::embeddings::EmbeddingService;

    print_info("🚀 Starting cast embeddings generation...");

    // Create services with optional endpoint override
    let database = Arc::new(Database::from_config(config).await?);

    let (embedding_service, endpoint_info) = if let Some(ref ep_name) = endpoint_name {
        // Use specified endpoint from config
        let endpoint_config = config.get_embedding_endpoint(ep_name).ok_or_else(|| {
            crate::SnapRagError::Custom(format!(
                "Endpoint '{}' not found in config. Available endpoints: {:?}",
                ep_name,
                config
                    .embedding_endpoints()
                    .iter()
                    .map(|e| &e.name)
                    .collect::<Vec<_>>()
            ))
        })?;

        let embedding_config =
            crate::embeddings::EmbeddingConfig::from_endpoint(config, endpoint_config);
        let service = Arc::new(EmbeddingService::from_config(embedding_config)?);

        (
            service,
            format!("{} ({})", endpoint_config.name, endpoint_config.endpoint),
        )
    } else {
        // Use default LLM endpoint
        let service = Arc::new(EmbeddingService::new(config)?);
        (service, format!("default ({})", config.llm_endpoint()))
    };

    // Check how many casts need embeddings
    let count = database.count_casts_without_embeddings().await?;

    if count == 0 {
        print_success("✅ All casts already have embeddings!");
        return Ok(());
    }

    let process_count = limit.unwrap_or(count as usize);
    println!("\n📊 Found {} casts without embeddings", count);
    println!("   Processing: {} casts", process_count);
    println!("   Endpoint: {}", endpoint_info);
    println!("   Batch size: {}", config.embeddings_batch_size());
    println!(
        "   Parallel tasks: {}\n",
        config.embeddings_parallel_tasks()
    );

    // Run backfill with config
    let stats = crate::embeddings::cast_backfill::backfill_cast_embeddings_with_config(
        database,
        embedding_service,
        limit,
        Some(config),
    )
    .await?;

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
