//! Backfill embeddings for cast content

use std::sync::Arc;

use tracing::info;
use tracing::warn;

use super::generator::EmbeddingService;
use crate::database::Database;
use crate::errors::Result;

/// Backfill embeddings for all casts
pub async fn backfill_cast_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
    limit: Option<usize>,
) -> Result<CastBackfillStats> {
    info!("Starting cast embeddings backfill");

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

    // Process in batches
    const BATCH_SIZE: usize = 100;
    let mut offset = 0;
    let mut processed = 0;

    while processed < process_limit {
        let batch_size = std::cmp::min(BATCH_SIZE, process_limit - processed);

        // Get batch of casts without embeddings
        let casts = db.get_casts_without_embeddings(batch_size, offset).await?;

        if casts.is_empty() {
            break;
        }

        info!(
            "Processing batch: {}-{}/{} casts",
            processed + 1,
            processed + casts.len(),
            process_limit
        );

        for cast in casts {
            // Skip casts without text
            if cast.text.is_none() || cast.text.as_ref().unwrap().trim().is_empty() {
                stats.skipped += 1;
                continue;
            }

            let text = cast.text.as_ref().unwrap();

            // Generate embedding
            match embedding_service.generate(text).await {
                Ok(embedding) => {
                    // Store embedding in database
                    match db
                        .store_cast_embedding(&cast.message_hash, cast.fid, text, &embedding)
                        .await
                    {
                        Ok(_) => {
                            stats.success += 1;
                            if stats.success % 10 == 0 {
                                info!("✓ Generated {} cast embeddings", stats.success);
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to store embedding for cast {}: {}",
                                hex::encode(&cast.message_hash),
                                e
                            );
                            stats.failed += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to generate embedding for cast {}: {}",
                        hex::encode(&cast.message_hash),
                        e
                    );
                    stats.failed += 1;
                }
            }
        }

        processed += batch_size;
        offset += batch_size;

        // Small delay to avoid overwhelming the embedding service
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!(
        "Cast embeddings backfill complete: {} success, {} skipped, {} failed",
        stats.success, stats.skipped, stats.failed
    );

    Ok(stats)
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
