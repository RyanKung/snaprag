//! Backfill embeddings for cast content

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use futures::stream::StreamExt;
use futures::stream::{
    self,
};
use rayon::prelude::*;  // CPU parallel processing
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::generator::EmbeddingService;
use crate::database::Database;
use crate::errors::Result;

#[cfg(feature = "local-gpu")]
use super::multiprocess::{MultiProcessEmbeddingGenerator, MultiProcessConfig};

/// Backfill embeddings for all casts with parallel processing
pub async fn backfill_cast_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    limit: Option<usize>,
) -> Result<CastBackfillStats> {
    backfill_cast_embeddings_with_config(db, embedding_service, limit, None).await
}

/// Detect available GPU devices (simplified version to avoid Metal issues)
#[cfg(feature = "local-gpu")]
fn detect_available_gpus() -> Vec<usize> {
    use candle_core::Device;
    
    let mut available_gpus = Vec::new();
    
    // Check CUDA devices (Linux/Windows)
    #[cfg(not(target_os = "macos"))]
    {
        for i in 0..8 { // Check up to 8 CUDA devices
            if Device::cuda_if_available(i).is_ok() {
                available_gpus.push(i);
            }
        }
    }
    
    // For macOS, use a simpler approach - just try Metal device 0
    #[cfg(target_os = "macos")]
    {
        // Try Metal device 0 first
        match Device::new_metal(0) {
            Ok(_) => {
                available_gpus.push(0);
                tracing::info!("Detected Metal GPU device 0");
            }
            Err(e) => {
                tracing::debug!("Metal device 0 not available: {}", e);
            }
        }
    }
    
    // If no GPUs found, return empty vector (will fall back to CPU)
    if available_gpus.is_empty() {
        tracing::warn!("No GPU devices detected, will use CPU");
    } else {
        tracing::info!("Detected {} GPU devices: {:?}", available_gpus.len(), available_gpus);
    }
    
    available_gpus
}

/// Backfill embeddings using multi-process parallel processing for maximum performance
#[cfg(feature = "local-gpu")]
pub async fn backfill_cast_embeddings_multiprocess(
    db: Arc<Database>,
    limit: Option<usize>,
    config: Option<&crate::config::AppConfig>,
    gpu_device: Option<usize>,
) -> Result<CastBackfillStats> {
    info!("Starting multi-process cast embeddings backfill");
    let start_time = Instant::now();

    // Get count of casts needing embeddings
    let total_count = db.count_casts_without_embeddings().await?;
    info!("Found {} casts without embeddings", total_count);

    if total_count == 0 {
        info!("No casts need embeddings");
        return Ok(CastBackfillStats::default());
    }

    let process_limit = limit.unwrap_or(total_count as usize);

    // Detect available GPUs and configure multi-process settings
    let available_gpus = detect_available_gpus();
    
    // Determine GPU devices to use
    let gpu_devices = if let Some(specified_gpu) = gpu_device {
        // User specified a specific GPU device
        if available_gpus.contains(&specified_gpu) {
            vec![specified_gpu]
        } else {
            tracing::warn!("Specified GPU device {} not available, using detected GPUs: {:?}", specified_gpu, available_gpus);
            available_gpus.clone()
        }
    } else {
        // Use all available GPUs
        available_gpus.clone()
    };
    
    // Calculate optimal number of worker processes based on GPU count
    let worker_processes = if gpu_devices.is_empty() {
        // No GPUs available, use CPU-based calculation
        config.map_or(2, |c| {
            let cores = num_cpus::get();
            std::cmp::min(cores / 4, 4) // Conservative for CPU-only
        })
    } else {
        // Use 2 workers per GPU for optimal performance
        gpu_devices.len() * 2
    };
    
    let multiprocess_config = MultiProcessConfig {
        worker_processes,
        batch_size_per_worker: config.map_or(50, |c| c.embeddings_batch_size() / 4),
        gpu_devices,
        worker_startup_timeout_secs: 30,
        max_retries: 3,
    };

    info!(
        "Using {} worker processes with batch size {} per worker",
        multiprocess_config.worker_processes,
        multiprocess_config.batch_size_per_worker
    );
    info!("GPU devices: {:?}", multiprocess_config.gpu_devices);

    // Create multi-process generator
    let mut generator = MultiProcessEmbeddingGenerator::new(multiprocess_config.clone());
    
    // Start workers
    generator.start_workers().await?;

    let mut stats = CastBackfillStats::default();
    stats.total_casts = total_count as usize;
    let mut offset = 0;
    let mut processed = 0;

    while processed < process_limit {
        let current_batch_size = std::cmp::min(
            multiprocess_config.worker_processes * multiprocess_config.batch_size_per_worker,
            process_limit - processed,
        );

        // Get batch of casts without embeddings
        let casts = db
            .get_casts_without_embeddings(current_batch_size, offset)
            .await?;

        if casts.is_empty() {
            break;
        }

        let batch_start = processed + 1;
        let batch_end = processed + casts.len();
        
        info!(
            "Processing batch: {}-{}/{} casts using multi-process",
            batch_start,
            batch_end,
            process_limit
        );

        // Process casts using multi-process
        let multiprocess_stats = generator.process_casts(casts, Arc::clone(&db)).await?;
        
        // Aggregate results
        stats.success += multiprocess_stats.success;
        stats.failed += multiprocess_stats.failed;

        processed += current_batch_size;
        offset += current_batch_size;

        // Report progress
        let total_processed = stats.success + stats.failed;
        if total_processed > 0 && total_processed % 100 == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = total_processed as f64 / elapsed;
            let success_rate = if total_processed > 0 {
                stats.success as f64 / total_processed as f64 * 100.0
            } else {
                0.0
            };
            let eta_secs = if rate > 0.0 {
                ((process_limit - total_processed) as f64 / rate) as u64
            } else {
                0
            };
            info!(
                "✓ Multi-process progress: {}/{} processed ({:.1}%), {} success ({:.1}%), {:.1} casts/sec, ETA: {}s",
                total_processed,
                process_limit,
                total_processed as f64 / process_limit as f64 * 100.0,
                stats.success,
                success_rate,
                rate,
                eta_secs
            );
        }
    }

    // Shutdown workers
    generator.shutdown().await?;

    let elapsed = start_time.elapsed();
    let total_processed = stats.success + stats.failed;
    let success_rate = if total_processed > 0 {
        stats.success as f64 / total_processed as f64 * 100.0
    } else {
        0.0
    };
    let overall_rate = total_processed as f64 / elapsed.as_secs_f64();
    
    info!(
        "Multi-process cast embeddings backfill complete in {:.1}s: {} processed ({} success, {} failed) - {:.1}% success rate, {:.1} casts/sec",
        elapsed.as_secs_f64(),
        total_processed,
        stats.success,
        stats.failed,
        success_rate,
        overall_rate
    );

    Ok(stats)
}

/// Backfill embeddings with custom batch configuration
pub async fn backfill_cast_embeddings_with_config(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    limit: Option<usize>,
    config: Option<&crate::config::AppConfig>,
) -> Result<CastBackfillStats> {
    info!("Starting cast embeddings backfill with parallel processing");
    let start_time = Instant::now();

    let mut stats = CastBackfillStats::default();

    // Get count of casts needing embeddings
    let total_count = db.count_casts_without_embeddings().await?;
    info!("Found {} casts without embeddings", total_count);

    if total_count == 0 {
        info!("No casts need embeddings");
        return Ok(stats);
    }

    stats.total_casts = total_count as usize;
    let process_limit = limit.unwrap_or(total_count as usize);

    // Process in batches with parallelism - configurable for different hardware
    let batch_size = config.map_or(100, super::super::config::AppConfig::embeddings_batch_size);
    let parallel_tasks = config.map_or(
        5,
        super::super::config::AppConfig::embeddings_parallel_tasks,
    );
    let cpu_threads = config.map_or(
        Some(56), // Default to 56 threads
        |c| {
            let threads = c.embeddings_cpu_threads();
            if threads == 0 {
                None // Auto-detect
            } else {
                Some(threads)
            }
        },
    );

    info!(
        "Using batch_size={}, parallel_tasks={}, cpu_threads={:?} for embeddings generation",
        batch_size, parallel_tasks, cpu_threads
    );
    let mut offset = 0;
    let mut processed = 0;

    while processed < process_limit {
        let current_batch_size = std::cmp::min(batch_size, process_limit - processed);

        // Get batch of casts without embeddings
        let casts = db
            .get_casts_without_embeddings(current_batch_size, offset)
            .await?;

        if casts.is_empty() {
            break;
        }

        let batch_start = processed + 1;
        let batch_end = processed + casts.len();
        
        // Calculate rate based on total processed casts (success + skipped + failed)
        let total_processed = stats.success + stats.skipped + stats.failed;
        let elapsed_secs = start_time.elapsed().as_secs_f64();
        let current_rate = if elapsed_secs > 0.0 && total_processed > 0 {
            total_processed as f64 / elapsed_secs
        } else {
            0.0
        };

        info!(
            "Processing batch: {}-{}/{} casts (rate: {:.1} casts/sec, processed: {})",
            batch_start,
            batch_end,
            process_limit,
            current_rate,
            total_processed
        );

        // Process casts with separated GPU computation and DB insertion concurrency
        let results = process_casts_with_separated_concurrency(casts, &db, &embedding_service, parallel_tasks, cpu_threads).await;

        // Aggregate results
        for result in results {
            match result {
                ProcessResult::Success => stats.success += 1,
                ProcessResult::Skipped => stats.skipped += 1,
                ProcessResult::Failed => stats.failed += 1,
            }
        }

        processed += current_batch_size;
        offset += current_batch_size;

        // Report progress every 50 processed casts (success + skipped + failed)
        let total_processed = stats.success + stats.skipped + stats.failed;
        if total_processed > 0 && total_processed % 50 == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = total_processed as f64 / elapsed;
            let success_rate = if total_processed > 0 {
                stats.success as f64 / total_processed as f64 * 100.0
            } else {
                0.0
            };
            let eta_secs = if rate > 0.0 {
                ((process_limit - total_processed) as f64 / rate) as u64
            } else {
                0
            };
            info!(
                "✓ Progress: {}/{} processed ({:.1}%), {} success ({:.1}%), {:.1} casts/sec, ETA: {}s",
                total_processed,
                process_limit,
                total_processed as f64 / process_limit as f64 * 100.0,
                stats.success,
                success_rate,
                rate,
                eta_secs
            );
        }
    }

    let elapsed = start_time.elapsed();
    let total_processed = stats.success + stats.skipped + stats.failed;
    let success_rate = if total_processed > 0 {
        stats.success as f64 / total_processed as f64 * 100.0
    } else {
        0.0
    };
    let overall_rate = total_processed as f64 / elapsed.as_secs_f64();
    
    info!(
        "Cast embeddings backfill complete in {:.1}s: {} processed ({} success, {} skipped, {} failed) - {:.1}% success rate, {:.1} casts/sec",
        elapsed.as_secs_f64(),
        total_processed,
        stats.success,
        stats.skipped,
        stats.failed,
        success_rate,
        overall_rate
    );

    Ok(stats)
}

/// Result of processing a single cast
enum ProcessResult {
    Success,
    Skipped,
    Failed,
}

/// Process a single cast with retry logic
async fn process_single_cast_with_retry(
    cast: crate::models::Cast,
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    max_retries: usize,
) -> ProcessResult {
    // Skip casts without text
    if cast.text.is_none() || cast.text.as_ref().unwrap().trim().is_empty() {
        debug!(
            "Skipping cast {} (no text)",
            hex::encode(&cast.message_hash)
        );
        return ProcessResult::Skipped;
    }

    let text = cast.text.as_ref().unwrap();
    let hash_str = hex::encode(&cast.message_hash);

    // Retry logic for embedding generation and storage
    for attempt in 1..=max_retries {
        match embedding_service.generate(text).await {
            Ok(embedding) => {
                // Store embedding in database
                match db
                    .store_cast_embedding(&cast.message_hash, cast.fid, text, &embedding)
                    .await
                {
                    Ok(()) => {
                        debug!("✓ Generated embedding for cast {}", hash_str);
                        return ProcessResult::Success;
                    }
                    Err(e) => {
                        warn!(
                            "Attempt {}/{}: Failed to store embedding for cast {}: {}",
                            attempt, max_retries, hash_str, e
                        );
                        if attempt < max_retries {
                            tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Attempt {}/{}: Failed to generate embedding for cast {}: {}",
                    attempt, max_retries, hash_str, e
                );
                if attempt < max_retries {
                    tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                    continue;
                }
            }
        }
    }

    ProcessResult::Failed
}

/// Statistics for cast embeddings backfill
#[derive(Debug, Default, Clone)]
pub struct CastBackfillStats {
    pub total_casts: usize,
    pub success: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl CastBackfillStats {
    #[must_use]
    pub const fn processed(&self) -> usize {
        self.success + self.skipped + self.failed
    }

    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.processed() == 0 {
            0.0
        } else {
            self.success as f64 / self.processed() as f64
        }
    }
}

/// Process casts with separated GPU computation and DB insertion concurrency
async fn process_casts_with_separated_concurrency(
    casts: Vec<crate::models::Cast>,
    db: &Arc<Database>,
    embedding_service: &Arc<EmbeddingService>,
    gpu_concurrency: usize,
    cpu_threads: Option<usize>,
) -> Vec<ProcessResult> {
    use futures::stream::StreamExt;
    
    // Step 0: CPU parallel preprocessing using rayon for true CPU parallelism
    let cpu_concurrency = std::cmp::min(56, casts.len()); // Use all available CPU cores
    
    // Configure rayon thread pool if specified
    if let Some(threads) = cpu_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap_or_else(|_| {
                info!("Using default rayon thread pool (already configured)");
            });
        info!("Configured rayon to use {} CPU threads", threads);
    }
    
    info!("Starting CPU parallel preprocessing with {} cores for {} casts", cpu_concurrency, casts.len());
    
    // Use rayon for true CPU parallel processing
    let preprocessed_casts: Vec<(crate::models::Cast, bool)> = casts
        .into_par_iter()  // Parallel iteration using rayon
        .map(|cast| {
            // Simplified preprocessing with debugging
            let text_len = cast.text.as_ref().map(|t| t.len()).unwrap_or(0);
            let text_preview = cast.text.as_ref()
                .map(|t| t.chars().take(50).collect::<String>())
                .unwrap_or_else(|| "None".to_string());
            
            debug!("Cast {}: text_len={}, preview='{}'", 
                   hex::encode(&cast.message_hash), text_len, text_preview);
            
            let is_valid = cast.text.is_some() && 
                          !cast.text.as_ref().unwrap().trim().is_empty();
            
            if is_valid {
                let text = cast.text.as_ref().unwrap();
                // Use comprehensive text preprocessing
                match crate::embeddings::preprocess_text_for_embedding(text) {
                    Ok(processed_text) => {
                        let mut processed_cast = cast;
                        processed_cast.text = Some(processed_text);
                        (processed_cast, true)
                    }
                    Err(e) => {
                        debug!("Failed to preprocess cast {}: {}", 
                               hex::encode(&cast.message_hash), e);
                        (cast, false)
                    }
                }
            } else {
                (cast, false)
            }
        })
        .collect();
    
    // Filter out invalid casts and collect stats
    let total_casts = preprocessed_casts.len();
    let valid_casts: Vec<crate::models::Cast> = preprocessed_casts
        .into_iter()
        .filter_map(|(cast, is_valid)| if is_valid { Some(cast) } else { None })
        .collect();
    
    info!("Preprocessed {} casts, {} valid for GPU processing", 
          total_casts, valid_casts.len());
    
    // Step 1: Generate embeddings with high GPU concurrency
    info!("Starting GPU embedding generation for {} casts with {} GPU concurrency", 
          valid_casts.len(), gpu_concurrency);
    
    let embedding_results: Vec<(crate::models::Cast, crate::errors::Result<Vec<f32>>)> = 
        stream::iter(valid_casts)
            .map(|cast| {
                let embedding_service = Arc::clone(embedding_service);
                async move {
                    let result = embedding_service.generate(cast.text.as_ref().unwrap()).await;
                    (cast, result)
                }
            })
            .buffered(gpu_concurrency) // High concurrency for GPU computation
            .collect()
            .await;
    
    info!("Completed GPU embedding generation, starting database storage");
    
    // Step 2: Store embeddings with lower DB concurrency
    let db_concurrency = std::cmp::min(50, gpu_concurrency / 4); // Much lower DB concurrency
    let results: Vec<ProcessResult> = stream::iter(embedding_results)
        .map(|(cast, embedding_result)| {
            let db = Arc::clone(db);
            async move {
                match embedding_result {
                    Ok(embedding) => {
                        // Store embedding in database with retry logic
                        let hash_str = hex::encode(&cast.message_hash);
                        for attempt in 1..=3 {
                            match db.store_cast_embedding(&cast.message_hash, cast.fid, cast.text.as_ref().unwrap(), &embedding).await {
                                Ok(()) => {
                                    debug!("✓ Generated embedding for cast {}", hash_str);
                                    return ProcessResult::Success;
                                }
                                Err(e) => {
                                    warn!("Attempt {}/3: Failed to store embedding for cast {}: {}", attempt, hash_str, e);
                                    if attempt < 3 {
                                        tokio::time::sleep(Duration::from_millis(100 * attempt as u64)).await;
                                        continue;
                                    }
                                }
                            }
                        }
                        ProcessResult::Failed
                    }
                    Err(_) => ProcessResult::Skipped,
                }
            }
        })
        .buffered(db_concurrency) // Lower concurrency for DB operations
        .collect()
        .await;
    
    results
}
