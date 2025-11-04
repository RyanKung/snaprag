//! Migration utilities for converting existing embeddings to multi-vector format
//!
//! This module provides tools to migrate existing single-vector embeddings
//! to the new multi-vector format while maintaining backward compatibility.

use tracing::error;
use tracing::info;
use tracing::warn;

use crate::database::Database;
use crate::embeddings::AggregationStrategy;
use crate::embeddings::ChunkStrategy;
use crate::embeddings::MultiVectorEmbeddingService;
use crate::errors::Result;
use crate::errors::SnapRagError;

/// Migration statistics
#[derive(Debug, Clone)]
pub struct MigrationStats {
    pub total_embeddings: usize,
    pub migrated_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub long_text_count: usize,
    pub short_text_count: usize,
}

impl MigrationStats {
    #[must_use]
    pub fn success_rate(&self) -> f32 {
        if self.total_embeddings == 0 {
            0.0
        } else {
            (self.migrated_count as f32 / self.total_embeddings as f32) * 100.0
        }
    }
}

/// Migration options
#[derive(Debug, Clone)]
pub struct MigrationOptions {
    /// Only migrate embeddings for texts longer than this threshold
    pub min_text_length: usize,
    /// Chunking strategy to use for long texts
    pub chunk_strategy: ChunkStrategy,
    /// Aggregation strategy for multi-vector embeddings
    pub aggregation_strategy: AggregationStrategy,
    /// Whether to keep original embeddings in `cast_embeddings` table
    pub keep_original: bool,
    /// Batch size for processing
    pub batch_size: usize,
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            min_text_length: 1000, // Only migrate texts longer than 1000 chars
            chunk_strategy: ChunkStrategy::Importance,
            aggregation_strategy: AggregationStrategy::WeightedMean,
            keep_original: true, // Keep original embeddings for backward compatibility
            batch_size: 100,
        }
    }
}

/// Migrate existing embeddings to multi-vector format
pub async fn migrate_existing_embeddings(
    database: &Database,
    embedding_service: &MultiVectorEmbeddingService,
    options: MigrationOptions,
) -> Result<MigrationStats> {
    info!("Starting migration of existing embeddings to multi-vector format");
    info!("Options: {:?}", options);

    // Get total count
    let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cast_embeddings")
        .fetch_one(database.pool())
        .await?;

    info!("Found {} existing embeddings to process", total_count);

    let mut stats = MigrationStats {
        total_embeddings: total_count as usize,
        migrated_count: 0,
        skipped_count: 0,
        failed_count: 0,
        long_text_count: 0,
        short_text_count: 0,
    };

    // Process in batches
    let mut offset = 0;
    while offset < total_count {
        let batch_size = options.batch_size as i64;
        let limit = batch_size.min(total_count - offset);

        // Get batch of embeddings
        let embeddings = sqlx::query_as::<_, (Vec<u8>, i64, String)>(
            "SELECT message_hash, fid, text FROM cast_embeddings ORDER BY created_at LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(database.pool())
        .await?;

        for (message_hash, fid, text) in embeddings {
            match migrate_single_embedding(
                &message_hash,
                fid,
                &text,
                database,
                embedding_service,
                &options,
            )
            .await
            {
                Ok(MigrationResult::Migrated) => {
                    stats.migrated_count += 1;
                    if text.len() > options.min_text_length {
                        stats.long_text_count += 1;
                    } else {
                        stats.short_text_count += 1;
                    }
                }
                Ok(MigrationResult::Skipped) => {
                    stats.skipped_count += 1;
                }
                Err(e) => {
                    stats.failed_count += 1;
                    error!(
                        "Failed to migrate embedding for {}: {}",
                        hex::encode(&message_hash),
                        e
                    );
                }
            }
        }

        offset += limit;
        info!("Processed {}/{} embeddings", offset, total_count);
    }

    info!("Migration completed: {:?}", stats);
    Ok(stats)
}

/// Result of migrating a single embedding
#[derive(Debug)]
enum MigrationResult {
    Migrated,
    Skipped,
}

/// Migrate a single embedding
async fn migrate_single_embedding(
    message_hash: &[u8],
    fid: i64,
    text: &str,
    database: &Database,
    embedding_service: &MultiVectorEmbeddingService,
    options: &MigrationOptions,
) -> Result<MigrationResult> {
    // Skip if text is too short
    if text.len() < options.min_text_length {
        return Ok(MigrationResult::Skipped);
    }

    // Check if already migrated
    let existing_chunks: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cast_embedding_chunks WHERE message_hash = $1")
            .bind(message_hash)
            .fetch_one(database.pool())
            .await?;

    if existing_chunks > 0 {
        return Ok(MigrationResult::Skipped);
    }

    // Generate multi-vector embeddings
    let result = embedding_service
        .generate_cast_embeddings(
            message_hash.to_vec(),
            fid,
            text,
            Some(options.chunk_strategy.clone()),
            Some(options.aggregation_strategy.clone()),
        )
        .await?;

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

    database
        .store_cast_embedding_chunks(message_hash, fid, &chunks)
        .await?;

    // Store aggregated embedding if available
    if let Some(aggregated_embedding) = result.aggregated_embedding {
        database
            .store_cast_embedding_aggregated(
                message_hash,
                fid,
                text,
                &aggregated_embedding,
                &format!("{:?}", result.aggregation_strategy),
                result.chunks.len(),
                text.len(),
            )
            .await?;
    }

    Ok(MigrationResult::Migrated)
}

/// Analyze existing embeddings to determine migration strategy
pub async fn analyze_existing_embeddings(database: &Database) -> Result<MigrationAnalysis> {
    info!("Analyzing existing embeddings for migration planning");

    // Get statistics about text lengths
    let stats = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r"
        SELECT 
            COUNT(*) as total,
            COUNT(CASE WHEN length(text) > 1000 THEN 1 END) as long_texts,
            COUNT(CASE WHEN length(text) > 2000 THEN 1 END) as very_long_texts,
            COUNT(CASE WHEN length(text) < 500 THEN 1 END) as short_texts
        FROM cast_embeddings
        ",
    )
    .fetch_one(database.pool())
    .await?;

    let (total, long_texts, very_long_texts, short_texts) = stats;

    // Get average text length
    let avg_length: f64 =
        sqlx::query_scalar::<_, Option<f64>>("SELECT AVG(length(text)) FROM cast_embeddings")
            .fetch_one(database.pool())
            .await?
            .unwrap_or(0.0);

    let analysis = MigrationAnalysis {
        total_embeddings: total as usize,
        long_text_count: long_texts as usize,
        very_long_text_count: very_long_texts as usize,
        short_text_count: short_texts as usize,
        average_text_length: avg_length as usize,
        migration_recommended: long_texts > total / 4, // Recommend if >25% are long texts
        estimated_migration_time_minutes: (long_texts as f64 / 1000.0) * 2.0, // Rough estimate
    };

    info!("Migration analysis: {:?}", analysis);
    Ok(analysis)
}

/// Analysis results for migration planning
#[derive(Debug, Clone)]
pub struct MigrationAnalysis {
    pub total_embeddings: usize,
    pub long_text_count: usize,
    pub very_long_text_count: usize,
    pub short_text_count: usize,
    pub average_text_length: usize,
    pub migration_recommended: bool,
    pub estimated_migration_time_minutes: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_stats() {
        let stats = MigrationStats {
            total_embeddings: 1000,
            migrated_count: 800,
            skipped_count: 150,
            failed_count: 50,
            long_text_count: 600,
            short_text_count: 200,
        };

        assert_eq!(stats.success_rate(), 80.0);
    }

    #[test]
    fn test_migration_options_default() {
        let options = MigrationOptions::default();
        assert_eq!(options.min_text_length, 1000);
        assert!(options.keep_original);
        assert_eq!(options.batch_size, 100);
    }
}
