//! Backfill embeddings for cast content

use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use futures::stream::StreamExt;
use futures::stream::{
    self,
};
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::generator::EmbeddingService;
use crate::database::Database;
use crate::errors::Result;

/// Backfill embeddings for all casts with parallel processing
pub async fn backfill_cast_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    limit: Option<usize>,
) -> Result<CastBackfillStats> {
    backfill_cast_embeddings_with_config(db, embedding_service, limit, None).await
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

    info!(
        "Using batch_size={}, parallel_tasks={} for embeddings generation",
        batch_size, parallel_tasks
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

        info!(
            "Processing batch: {}-{}/{} casts (rate: {:.1} casts/sec)",
            batch_start,
            batch_end,
            process_limit,
            stats.success as f64 / start_time.elapsed().as_secs_f64()
        );

        // Process casts with separated GPU computation and DB insertion concurrency
        let results = process_casts_with_separated_concurrency(casts, &db, &embedding_service, parallel_tasks).await;

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

        // Report progress every 100 successful embeddings
        if stats.success > 0 && stats.success % 100 == 0 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let rate = stats.success as f64 / elapsed;
            let eta_secs = ((process_limit - stats.success) as f64 / rate) as u64;
            info!(
                "✓ Progress: {}/{} success ({:.1}%), {:.1} casts/sec, ETA: {}s",
                stats.success,
                process_limit,
                stats.success as f64 / process_limit as f64 * 100.0,
                rate,
                eta_secs
            );
        }
    }

    let elapsed = start_time.elapsed();
    info!(
        "Cast embeddings backfill complete in {:.1}s: {} success, {} skipped, {} failed (rate: {:.1} casts/sec)",
        elapsed.as_secs_f64(),
        stats.success,
        stats.skipped,
        stats.failed,
        stats.success as f64 / elapsed.as_secs_f64()
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
) -> Vec<ProcessResult> {
    use futures::stream::StreamExt;
    
    // Step 0: CPU parallel preprocessing (utilize 56 virtual cores)
    let cpu_concurrency = std::cmp::min(56, casts.len()); // Use all available CPU cores
    let preprocessed_casts: Vec<(crate::models::Cast, bool)> = 
        stream::iter(casts)
            .map(|cast| {
                async move {
                    // Simplified preprocessing with debugging
                    let text_len = cast.text.as_ref().map(|t| t.len()).unwrap_or(0);
                    let text_preview = cast.text.as_ref()
                        .map(|t| t.chars().take(50).collect::<String>())
                        .unwrap_or_else(|| "None".to_string());
                    
                    info!("Cast {}: text_len={}, preview='{}'", 
                          hex::encode(&cast.message_hash), text_len, text_preview);
                    
                    let is_valid = cast.text.is_some() && 
                                  !cast.text.as_ref().unwrap().trim().is_empty();
                    
                    if is_valid {
                        let text = cast.text.as_ref().unwrap();
                        // Minimal processing - just trim and basic cleanup
                        let processed_text = text.trim().to_string();
                        
                        if !processed_text.is_empty() {
                            let mut processed_cast = cast;
                            processed_cast.text = Some(processed_text);
                            (processed_cast, true)
                        } else {
                            (cast, false)
                        }
                    } else {
                        (cast, false)
                    }
                }
            })
            .buffered(cpu_concurrency) // High CPU concurrency for preprocessing
            .collect()
            .await;
    
    // Filter out invalid casts and collect stats
    let total_casts = preprocessed_casts.len();
    let valid_casts: Vec<crate::models::Cast> = preprocessed_casts
        .into_iter()
        .filter_map(|(cast, is_valid)| if is_valid { Some(cast) } else { None })
        .collect();
    
    info!("Preprocessed {} casts, {} valid for GPU processing", 
          total_casts, valid_casts.len());
    
    // Step 1: Generate embeddings with high GPU concurrency
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
