//! Embedding generation handlers

#![allow(clippy::cast_precision_loss)] // Acceptable for progress displays and statistics

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
            let service =
                Arc::new(EmbeddingService::from_config_async(embedding_config, gpu_device).await?);
            (service, "local-gpu (BAAI/bge-small-en-v1.5)".to_string())
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
            backfill_cast_embeddings_multiprocess(database, limit, Some(config), gpu_device).await?
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

    println!("🗑️  Removing multi-vector cast embeddings...");
    let chunks_result = sqlx::query("DELETE FROM cast_embedding_chunks")
        .execute(database.pool())
        .await?;
    println!(
        "   ✅ Removed {} cast embedding chunk records",
        chunks_result.rows_affected()
    );

    let aggregated_result = sqlx::query("DELETE FROM cast_embedding_aggregated")
        .execute(database.pool())
        .await?;
    println!(
        "   ✅ Removed {} cast embedding aggregated records",
        aggregated_result.rows_affected()
    );

    println!("\n✅ All cast embeddings have been removed!");
    println!("   Run 'cargo run --features local-gpu -- embeddings cast backfill --force' to regenerate them.");

    Ok(())
}

/// Handle embeddings test cast command
pub async fn handle_embeddings_test_cast(
    config: &AppConfig,
    message_hash: String,
    endpoint: Option<String>,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
    #[cfg(feature = "local-gpu")] gpu_device: Option<usize>,
) -> Result<()> {
    use hex;

    use crate::database::Database;
    use crate::embeddings::EmbeddingService;

    println!("🧪 Test Embedding Generation for Cast");
    println!("=====================================\n");

    // Parse message hash
    let message_hash_bytes = hex::decode(&message_hash)
        .map_err(|e| crate::SnapRagError::Custom(format!("Invalid message hash: {e}")))?;

    println!("📋 Message Hash: {message_hash}");
    println!("⏳ Initializing services...");

    // Initialize database
    let database = Arc::new(Database::from_config(config).await?);

    // Initialize embedding service
    let embedding_config = if let Some(endpoint_name) = endpoint {
        // Use specific endpoint from config
        let endpoint_config = config
            .embedding_endpoints()
            .iter()
            .find(|ep| ep.name == endpoint_name)
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!(
                    "Embedding endpoint '{endpoint_name}' not found in config"
                ))
            })?;
        crate::embeddings::EmbeddingConfig::from_endpoint(config, endpoint_config)
    } else {
        // Use default configuration
        crate::embeddings::EmbeddingConfig::from_app_config(config)
    };

    #[cfg(feature = "local-gpu")]
    let embedding_service = if local_gpu {
        Arc::new(EmbeddingService::from_config_async(embedding_config, gpu_device).await?)
    } else {
        Arc::new(EmbeddingService::from_config(embedding_config)?)
    };

    #[cfg(not(feature = "local-gpu"))]
    let embedding_service = Arc::new(EmbeddingService::from_config(embedding_config)?);

    // Fetch cast from database
    println!("🔍 Fetching cast from database...");
    let cast = database
        .get_cast_by_hash(message_hash_bytes)
        .await?
        .ok_or_else(|| {
            crate::SnapRagError::Custom(format!(
                "Cast with hash {message_hash} not found in database"
            ))
        })?;

    println!("✅ Found cast:");
    println!("   FID: {}", cast.fid);
    println!("   Text: {}", cast.text.as_deref().unwrap_or("(no text)"));
    println!("   Timestamp: {}", cast.timestamp);

    // Check if text exists
    let Some(text) = &cast.text else {
        println!("❌ Cast has no text content - cannot generate embedding");
        return Ok(());
    };

    if text.trim().is_empty() {
        println!("❌ Cast text is empty - cannot generate embedding");
        return Ok(());
    }

    // Test text preprocessing
    println!("\n🔧 Testing text preprocessing...");
    match crate::embeddings::preprocess_text_for_embedding(text) {
        Ok(processed_text) => {
            println!("✅ Text preprocessing successful");
            println!("   Original length: {} chars", text.len());
            println!("   Processed length: {} chars", processed_text.len());
            if text.as_str() != processed_text {
                println!("   Original: {text:?}");
                println!("   Processed: {processed_text:?}");
            }
        }
        Err(e) => {
            println!("❌ Text preprocessing failed: {e}");
            return Ok(());
        }
    }

    // Generate embedding
    println!("\n🔮 Generating embedding...");
    match embedding_service.generate(text).await {
        Ok(embedding) => {
            println!("✅ Embedding generation successful!");
            println!("   Dimension: {}", embedding.len());
            println!(
                "   First 5 values: {:?}",
                &embedding[..5.min(embedding.len())]
            );
            println!(
                "   Last 5 values: {:?}",
                &embedding[embedding.len().saturating_sub(5)..]
            );

            // Calculate some basic stats
            let min_val = embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_val = embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let mean_val = embedding.iter().sum::<f32>() / embedding.len() as f32;

            println!("   Stats:");
            println!("     Min: {min_val:.6}");
            println!("     Max: {max_val:.6}");
            println!("     Mean: {mean_val:.6}");

            println!("\n🎉 Test completed successfully!");
            println!("   Note: Embedding was NOT stored in database (test mode)");
        }
        Err(e) => {
            println!("❌ Embedding generation failed: {e}");
            println!("   This is the error you're seeing in the logs");
        }
    }

    Ok(())
}

/// Handle cast embeddings backfill with optional multi-vector support
///
/// # Panics
/// Panics if casts retrieved from database have None for text field (should never happen for valid casts)
pub async fn handle_cast_embeddings_backfill_multivector(
    config: &AppConfig,
    force: bool,
    limit: Option<usize>,
    endpoint: Option<String>,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
    #[cfg(feature = "local-gpu")] gpu_device: Option<usize>,
    enable_multi_vector: bool,
    strategy: &str,
    aggregation: &str,
    min_length: usize,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::AggregationStrategy;
    use crate::embeddings::ChunkStrategy;
    use crate::embeddings::EmbeddingService;
    use crate::embeddings::MultiVectorEmbeddingService;

    if enable_multi_vector {
        println!("🚀 Cast Embeddings Backfill with Multi-Vector Support");
        println!("====================================================\n");
    } else {
        println!("🚀 Cast Embeddings Backfill (Single Vector Mode)");
        println!("===============================================\n");
    }

    if !force {
        if enable_multi_vector {
            println!("⚠️  This will generate embeddings for casts using multi-vector approach:");
            println!("   - Multi-vector enabled: YES");
            println!("   - Chunking strategy: {strategy}");
            println!("   - Aggregation strategy: {aggregation}");
            println!("   - Multi-vector threshold: {min_length} characters");
        } else {
            println!("⚠️  This will generate embeddings for casts using single-vector approach:");
            println!("   - Multi-vector enabled: NO (use --enable-multi-vector to enable)");
            println!("   - All texts will be processed as single vectors");
        }
        println!("   - This may take a while for large datasets!");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Connecting to database...");
    let database = Database::from_config(config).await?;

    println!("🔧 Initializing embedding service...");
    let embedding_service = EmbeddingService::new(config)?;

    // Only initialize multi-vector service if enabled
    let multi_vector_service = if enable_multi_vector {
        Some(MultiVectorEmbeddingService::new(
            EmbeddingService::new(config)?,
            1500, // default chunk size
            parse_chunk_strategy(strategy)?,
            parse_aggregation_strategy(aggregation)?,
        ))
    } else {
        None
    };

    if enable_multi_vector {
        println!("📊 Starting multi-vector backfill...");
        println!("   Strategy: {strategy}");
        println!("   Aggregation: {aggregation}");
        println!("   Min length for multi-vector: {min_length} chars");
    } else {
        println!("📊 Starting single-vector backfill...");
        println!("   All texts will be processed as single vectors");
    }
    println!(
        "   Limit: {}",
        limit.map_or("all".to_string(), |l| l.to_string())
    );

    // Get casts that need embeddings
    let casts = database
        .get_casts_without_embeddings(
            limit.unwrap_or(1000), // Default limit
            0,                     // Start from beginning
        )
        .await?;
    println!("   Found {} casts to process", casts.len());

    let mut success = 0;
    let skipped = 0; // Not used in current implementation
    let mut failed = 0;
    let mut single_vector = 0;
    let mut multi_vector = 0;

    for (idx, cast) in casts.iter().enumerate() {
        let text = cast.text.as_ref().unwrap();
        let hash_str = hex::encode(&cast.message_hash);

        println!(
            "Processing {}/{}: {} chars",
            idx + 1,
            casts.len(),
            text.len()
        );

        // Decide whether to use single or multi-vector approach
        if !enable_multi_vector || text.len() < min_length {
            // Use single vector for all texts (if multi-vector disabled) or short texts
            match embedding_service.generate(text).await {
                Ok(embedding) => {
                    match database
                        .store_cast_embedding(&cast.message_hash, cast.fid, text, &embedding)
                        .await
                    {
                        Ok(()) => {
                            println!("   ✅ Single vector: {hash_str}");
                            success += 1;
                            single_vector += 1;
                        }
                        Err(e) => {
                            println!("   ❌ Failed to store single vector: {hash_str} - {e}");
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to generate single vector: {hash_str} - {e}");
                    failed += 1;
                }
            }
        } else {
            // Use multi-vector for long texts (only if multi-vector is enabled)
            if let Some(ref multi_vector_service) = multi_vector_service {
                match multi_vector_service
                    .generate_cast_embeddings(
                        cast.message_hash.clone(),
                        cast.fid,
                        text,
                        None, // Use default strategy
                        None, // Use default aggregation
                    )
                    .await
                {
                    Ok(result) => {
                        // Store chunked embeddings
                        let chunks: Vec<(usize, String, Vec<f32>, String)> = result
                            .chunks
                            .iter()
                            .map(|(metadata, embedding)| {
                                (
                                    metadata.chunk_index,
                                    metadata.chunk_text.clone(),
                                    embedding.clone(),
                                    format!("{:?}", metadata.chunk_strategy),
                                )
                            })
                            .collect();

                        match database
                            .store_cast_embedding_chunks(&cast.message_hash, cast.fid, &chunks)
                            .await
                        {
                            Ok(()) => {
                                // Store aggregated embedding
                                if let Some(aggregated_embedding) = result.aggregated_embedding {
                                    match database
                                        .store_cast_embedding_aggregated(
                                            &cast.message_hash,
                                            cast.fid,
                                            text,
                                            &aggregated_embedding,
                                            &format!("{:?}", result.aggregation_strategy),
                                            result.chunks.len(),
                                            text.len(),
                                        )
                                        .await
                                    {
                                        Ok(()) => {
                                            println!(
                                                "   ✅ Multi-vector ({} chunks): {}",
                                                result.chunks.len(),
                                                hash_str
                                            );
                                            success += 1;
                                            multi_vector += 1;
                                        }
                                        Err(e) => {
                                            println!("   ⚠️  Chunks stored but aggregation failed: {hash_str} - {e}");
                                            success += 1;
                                            multi_vector += 1;
                                        }
                                    }
                                } else {
                                    println!("   ✅ Multi-vector chunks only: {hash_str}");
                                    success += 1;
                                    multi_vector += 1;
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Failed to store chunks: {hash_str} - {e}");
                                failed += 1;
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to generate multi-vector: {hash_str} - {e}");
                        failed += 1;
                    }
                }
            } else {
                // This should never happen since we check enable_multi_vector above
                println!("   ❌ Multi-vector service not available: {hash_str}");
                failed += 1;
            }
        }
    }

    println!("\n✅ Multi-Vector Backfill Complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total processed: {}", casts.len());
    println!("Success: {success}");
    println!("Failed: {failed}");
    println!("Single vector: {single_vector}");
    println!("Multi-vector: {multi_vector}");
    println!(
        "Success rate: {:.1}%",
        (success as f32 / casts.len() as f32) * 100.0
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Handle cast embeddings migration command
pub async fn handle_cast_embeddings_migrate(
    config: &AppConfig,
    force: bool,
    min_length: usize,
    strategy: &str,
    keep_original: bool,
    batch_size: usize,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::AggregationStrategy;
    use crate::embeddings::ChunkStrategy;
    use crate::embeddings::EmbeddingService;
    use crate::embeddings::MigrationOptions;
    use crate::embeddings::MultiVectorEmbeddingService;

    println!("🔄 Migrate Cast Embeddings to Multi-Vector Format");
    println!("================================================\n");

    if !force {
        println!("⚠️  This will migrate existing embeddings to multi-vector format:");
        println!("   - Only texts longer than {min_length} characters will be migrated");
        println!("   - Chunking strategy: {strategy}");
        println!("   - Keep original embeddings: {keep_original}");
        println!("   - This process may take a while for large datasets!");
        println!("\nUse --force to confirm and proceed.");
        return Ok(());
    }

    println!("⏳ Connecting to database...");
    let database = Database::from_config(config).await?;

    println!("🔧 Initializing embedding service...");
    let embedding_service = EmbeddingService::new(config)?;
    let multi_vector_service = MultiVectorEmbeddingService::new(
        embedding_service,
        1500, // default chunk size
        parse_chunk_strategy(strategy)?,
        AggregationStrategy::WeightedMean,
    );

    println!("📊 Analyzing existing embeddings...");
    let analysis = crate::embeddings::analyze_existing_embeddings(&database).await?;
    println!("   Total embeddings: {}", analysis.total_embeddings);
    println!("   Long texts (>1000 chars): {}", analysis.long_text_count);
    println!(
        "   Very long texts (>2000 chars): {}",
        analysis.very_long_text_count
    );
    println!(
        "   Average text length: {} chars",
        analysis.average_text_length
    );
    println!(
        "   Migration recommended: {}",
        analysis.migration_recommended
    );
    println!(
        "   Estimated time: {:.1} minutes",
        analysis.estimated_migration_time_minutes
    );

    let options = MigrationOptions {
        min_text_length: min_length,
        chunk_strategy: parse_chunk_strategy(strategy)?,
        aggregation_strategy: AggregationStrategy::WeightedMean,
        keep_original,
        batch_size,
    };

    println!("\n🚀 Starting migration...");
    let stats =
        crate::embeddings::migrate_existing_embeddings(&database, &multi_vector_service, options)
            .await?;

    println!("\n✅ Migration completed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total processed: {}", stats.total_embeddings);
    println!("Migrated: {}", stats.migrated_count);
    println!("Skipped: {}", stats.skipped_count);
    println!("Failed: {}", stats.failed_count);
    println!("Success rate: {:.1}%", stats.success_rate());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Handle cast embeddings analysis command
pub async fn handle_cast_embeddings_analyze(config: &AppConfig) -> Result<()> {
    use crate::database::Database;

    println!("📊 Analyze Cast Embeddings for Migration");
    println!("=======================================\n");

    println!("⏳ Connecting to database...");
    let database = Database::from_config(config).await?;

    println!("🔍 Analyzing existing embeddings...");
    let analysis = crate::embeddings::analyze_existing_embeddings(&database).await?;

    println!("📈 Analysis Results:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total embeddings: {}", analysis.total_embeddings);
    println!("Long texts (>1000 chars): {}", analysis.long_text_count);
    println!(
        "Very long texts (>2000 chars): {}",
        analysis.very_long_text_count
    );
    println!("Short texts (<500 chars): {}", analysis.short_text_count);
    println!(
        "Average text length: {} chars",
        analysis.average_text_length
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    println!("\n💡 Migration Recommendation:");
    if analysis.migration_recommended {
        println!("✅ Migration is RECOMMENDED");
        println!(
            "   - {}% of texts are long enough to benefit from chunking",
            (analysis.long_text_count as f32 / analysis.total_embeddings as f32) * 100.0
        );
        println!(
            "   - Estimated migration time: {:.1} minutes",
            analysis.estimated_migration_time_minutes
        );
        println!("\n🚀 To migrate, run:");
        println!("   cargo run -- embeddings cast migrate --force");
    } else {
        println!("ℹ️  Migration is NOT necessary");
        println!("   - Most texts are short and don't need chunking");
        println!("   - Current single-vector approach is sufficient");
    }

    Ok(())
}

/// Parse chunking strategy from string
fn parse_chunk_strategy(strategy: &str) -> Result<crate::embeddings::ChunkStrategy> {
    match strategy.to_lowercase().as_str() {
        "single" => Ok(crate::embeddings::ChunkStrategy::Single),
        "paragraph" => Ok(crate::embeddings::ChunkStrategy::Paragraph),
        "sentence" => Ok(crate::embeddings::ChunkStrategy::Sentence),
        "importance" => Ok(crate::embeddings::ChunkStrategy::Importance),
        "sliding_window" => Ok(crate::embeddings::ChunkStrategy::SlidingWindow),
        _ => Err(crate::SnapRagError::Custom(format!(
            "Invalid chunking strategy: {strategy}"
        ))),
    }
}

/// Parse aggregation strategy from string
fn parse_aggregation_strategy(strategy: &str) -> Result<crate::embeddings::AggregationStrategy> {
    match strategy.to_lowercase().as_str() {
        "first_chunk" => Ok(crate::embeddings::AggregationStrategy::FirstChunk),
        "mean" => Ok(crate::embeddings::AggregationStrategy::Mean),
        "weighted_mean" => Ok(crate::embeddings::AggregationStrategy::WeightedMean),
        "max" => Ok(crate::embeddings::AggregationStrategy::Max),
        "concatenate" => Ok(crate::embeddings::AggregationStrategy::Concatenate),
        _ => Err(crate::SnapRagError::Custom(format!(
            "Invalid aggregation strategy: {strategy}"
        ))),
    }
}
