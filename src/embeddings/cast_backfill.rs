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
    let batch_size = config
        .map(|c| c.embeddings_batch_size())
        .unwrap_or(100);
    let parallel_tasks = config
        .map(|c| c.embeddings_parallel_tasks())
        .unwrap_or(5);
    
    info!(
        "Using batch_size={}, parallel_tasks={} for embeddings generation",
        batch_size, parallel_tasks
    );
    let mut offset = 0;
    let mut processed = 0;

    while processed < process_limit {
        let current_batch_size = std::cmp::min(batch_size, process_limit - processed);

        // Get batch of casts without embeddings
        let casts = db.get_casts_without_embeddings(current_batch_size, offset).await?;

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

        // Process casts in parallel within the batch
        let results = stream::iter(casts)
            .map(|cast| {
                let db = Arc::clone(&db);
                let embedding_service = Arc::clone(&embedding_service);
                async move { process_single_cast_with_retry(cast, db, embedding_service, 3).await }
            })
            .buffered(parallel_tasks) // Configurable parallelism based on hardware
            .collect::<Vec<_>>()
            .await;

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
                    Ok(_) => {
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
    pub fn processed(&self) -> usize {
        self.success + self.skipped + self.failed
    }

    pub fn success_rate(&self) -> f64 {
        if self.processed() == 0 {
            0.0
        } else {
            self.success as f64 / self.processed() as f64
        }
    }
}
