//! Embedding generation handlers

use std::sync::Arc;

use crate::cli::output::print_info;
use crate::cli::output::print_success;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

pub async fn handle_cast_embeddings_backfill(
    config: &AppConfig,
    limit: Option<usize>,
    endpoint_name: Option<String>,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
    #[cfg(feature = "local-gpu")] multiprocess: bool,
    #[cfg(feature = "local-gpu")] workers: Option<usize>,
    #[cfg(feature = "local-gpu")] gpu_device: Option<usize>,
) -> Result<()> {
    use std::sync::Arc;

    use crate::database::Database;
    use crate::embeddings::backfill_cast_embeddings;
    #[cfg(feature = "local-gpu")]
    use crate::embeddings::backfill_cast_embeddings_multiprocess;
    use crate::embeddings::EmbeddingService;

    print_info("🚀 Starting cast embeddings backfill...");

    // Create services with optional endpoint override
    let database = Arc::new(Database::from_config(config).await?);

    let (embedding_service, endpoint_info) = {
        #[cfg(feature = "local-gpu")]
        if local_gpu {
            // Use local GPU
            print_info("🔧 Using local GPU for embedding generation...");
            let embedding_config = crate::embeddings::EmbeddingConfig {
                provider: crate::embeddings::EmbeddingProvider::LocalGPU,
                model: "BAAI/bge-small-en-v1.5".to_string(),
                dimension: config.embedding_dimension(),
                endpoint: "local-gpu".to_string(),
                api_key: None,
            };
            let service = Arc::new(EmbeddingService::from_config_async(embedding_config, gpu_device).await?);
            (
                service,
                "local-gpu (BAAI/bge-small-en-v1.5)".to_string(),
            )
        } else if let Some(ref ep_name) = endpoint_name {
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
        }
        #[cfg(not(feature = "local-gpu"))]
        if let Some(ref ep_name) = endpoint_name {
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
        }
    };

    // Skip counting for better performance - just start processing
    println!("\n📊 Starting cast embeddings backfill");
    println!(
        "   Processing: {} casts",
        limit.map_or("all".to_string(), |l| l.to_string())
    );
    println!("   Endpoint: {endpoint_info}");
    println!("   Batch size: {}", config.embeddings_batch_size());
    println!(
        "   Parallel tasks: {}\n",
        config.embeddings_parallel_tasks()
    );

    // Run backfill with config
    let stats = {
        #[cfg(feature = "local-gpu")]
        if multiprocess && local_gpu {
            print_info("🚀 Using multi-process parallel processing for maximum performance...");
            backfill_cast_embeddings_multiprocess(database, limit, Some(config), workers).await?
        } else {
            crate::embeddings::cast_backfill::backfill_cast_embeddings_with_config(
                database,
                embedding_service,
                limit,
                Some(config),
            )
            .await?
        }
        #[cfg(not(feature = "local-gpu"))]
        {
            crate::embeddings::cast_backfill::backfill_cast_embeddings_with_config(
                database,
                embedding_service,
                limit,
                Some(config),
            )
            .await?
        }
    };

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
    data_type: crate::cli::EmbeddingDataType,
    force: bool,
    batch_size: usize,
    limit: Option<usize>,
    endpoint: Option<String>,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
    #[cfg(feature = "local-gpu")] multiprocess: bool,
    #[cfg(feature = "local-gpu")] workers: Option<usize>,
    #[cfg(feature = "local-gpu")] gpu_device: Option<usize>,
) -> Result<()> {
    match data_type {
        crate::cli::EmbeddingDataType::User => {
            handle_user_embeddings_backfill(
                config,
                force,
                batch_size,
                #[cfg(feature = "local-gpu")]
                local_gpu,
            )
            .await
        }
        crate::cli::EmbeddingDataType::Cast => {
            handle_cast_embeddings_backfill(
                config,
                limit,
                endpoint,
                #[cfg(feature = "local-gpu")]
                local_gpu,
                #[cfg(feature = "local-gpu")]
                multiprocess,
                #[cfg(feature = "local-gpu")]
                workers,
                #[cfg(feature = "local-gpu")]
                gpu_device,
            )
            .await
        }
    }
}

async fn handle_user_embeddings_backfill(
    config: &AppConfig,
    force: bool,
    _batch_size: usize,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::backfill_embeddings;
    use crate::embeddings::EmbeddingService;

    println!("📊 User Embeddings Backfill");
    println!("============================\n");

    if !force {
        println!("⚠️  This will generate embeddings for all user profiles in the database.");
        println!("⚠️  This may take a long time and incur API costs.");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Initializing services...");
    let database = Arc::new(Database::from_config(config).await?);

    let embedding_service = {
        #[cfg(feature = "local-gpu")]
        if local_gpu {
            // Use local GPU
            print_info("🔧 Using local GPU for embedding generation...");
            let embedding_config = crate::embeddings::EmbeddingConfig {
                provider: crate::embeddings::EmbeddingProvider::LocalGPU,
                model: "BAAI/bge-small-en-v1.5".to_string(),
                dimension: config.embedding_dimension(),
                endpoint: "local-gpu".to_string(),
                api_key: None,
            };
            Arc::new(EmbeddingService::from_config_async(embedding_config, None).await?)
        } else {
            // Use default LLM endpoint
            Arc::new(EmbeddingService::new(config)?)
        }
        #[cfg(not(feature = "local-gpu"))]
        {
            // Use default LLM endpoint
            Arc::new(EmbeddingService::new(config)?)
        }
    };

    println!("🚀 Starting user embeddings backfill process...\n");
    let stats = backfill_embeddings(database, embedding_service).await?;

    println!("\n✅ User Embeddings Backfill Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Profiles: {}", stats.total_profiles);
    println!("Updated: {}", stats.updated);
    println!("Skipped: {}", stats.skipped);
    println!("Failed: {}", stats.failed);
    println!("Success Rate: {:.1}%", stats.success_rate());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Handle embeddings reset command
pub async fn handle_embeddings_reset(config: &AppConfig, force: bool) -> Result<()> {
    use crate::database::Database;

    println!("🗑️  Reset Embeddings");
    println!("===================\n");

    if !force {
        println!("⚠️  This will remove ALL embeddings from the database:");
        println!("   - Profile embeddings (profile_embedding, bio_embedding, interests_embedding)");
        println!("   - Cast embeddings (cast_embeddings table)");
        println!("   - This action cannot be undone!");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Connecting to database...");
    let database = Database::from_config(config).await?;

    println!("🗑️  Removing profile embeddings...");
    let profile_result = sqlx::query("DELETE FROM profile_embeddings")
        .execute(database.pool())
        .await?;
    println!(
        "   ✅ Removed {} profile embedding records",
        profile_result.rows_affected()
    );

    println!("🗑️  Removing cast embeddings...");
    let cast_result = sqlx::query("DELETE FROM cast_embeddings")
        .execute(database.pool())
        .await?;
    println!(
        "   ✅ Removed {} cast embedding records",
        cast_result.rows_affected()
    );

    println!("\n✅ All embeddings have been removed!");
    println!("   Run 'cargo run embeddings backfill --force' to regenerate them.");

    Ok(())
}

/// Handle embeddings generate command
pub async fn handle_embeddings_generate(config: &AppConfig, fid: i64, verbose: bool) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::EmbeddingService;

    println!("🔮 Generate Embeddings for FID: {fid}");
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
    let profile = profiles
        .into_iter()
        .next()
        .ok_or_else(|| crate::SnapRagError::Custom(format!("Profile not found for FID: {fid}")))?;

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
    println!("Text: {text}\n");

    println!("⏳ Initializing embedding service...");
    let embedding_service = EmbeddingService::new(config)?;

    println!("🔮 Generating embedding...");
    let start = std::time::Instant::now();
    let embedding = embedding_service.generate(&text).await?;
    let duration = start.elapsed();

    println!("✅ Generated embedding in {duration:?}");
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
    println!("  - Mean: {mean:.6}");
    println!("  - Std Dev: {std_dev:.6}");
    println!(
        "  - Min: {:.6}",
        embedding.iter().copied().fold(f32::INFINITY, f32::min)
    );
    println!(
        "  - Max: {:.6}",
        embedding.iter().copied().fold(f32::NEG_INFINITY, f32::max)
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
    let total: i64 = sqlx::query("SELECT COUNT(DISTINCT fid) as count FROM user_profile_changes")
        .fetch_one(database.pool())
        .await?
        .try_get("count")?;

    // Count profiles with embeddings
    let with_profile_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM profile_embeddings WHERE profile_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    let with_bio_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM profile_embeddings WHERE bio_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    let with_interests_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM profile_embeddings WHERE interests_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    let with_all_emb: i64 = sqlx::query(
        "SELECT COUNT(*) as count FROM profile_embeddings 
         WHERE profile_embedding IS NOT NULL 
           AND bio_embedding IS NOT NULL 
           AND interests_embedding IS NOT NULL",
    )
    .fetch_one(database.pool())
    .await?
    .try_get("count")?;

    println!("📈 Coverage:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total Profiles: {total}");
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
    Ok(())
}

/// Handle cast embeddings reset command
pub async fn handle_cast_embeddings_reset(config: &AppConfig, force: bool) -> Result<()> {
    use crate::database::Database;

    println!("🗑️  Reset Cast Embeddings");
    println!("=========================\n");

    if !force {
        println!("⚠️  This will remove ALL cast embeddings from the database:");
        println!("   - All entries in cast_embeddings table will be deleted");
        println!("   - This action cannot be undone!");
        println!("   - You can regenerate them with: cargo run --features local-gpu -- embeddings cast backfill --force");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Connecting to database...");
    let database = Database::from_config(config).await?;

    println!("🗑️  Removing cast embeddings...");
    let cast_result = sqlx::query("DELETE FROM cast_embeddings")
        .execute(database.pool())
        .await?;
    println!(
        "   ✅ Removed {} cast embedding records",
        cast_result.rows_affected()
    );

    println!("\n✅ All cast embeddings have been removed!");
    println!("   Run 'cargo run --features local-gpu -- embeddings cast backfill --force' to regenerate them.");

    Ok(())
}
