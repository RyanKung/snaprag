//! Backfill embeddings for existing data

use std::sync::Arc;

use tracing::info;
use tracing::warn;

use super::generator::EmbeddingService;
use crate::database::Database;
use crate::errors::Result;

/// Backfill embeddings for all user profiles
pub async fn backfill_embeddings(
    db: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
) -> Result<BackfillStats> {
    info!("Starting embeddings backfill");

    let mut stats = BackfillStats::default();

    // Get all profiles that need embeddings
    let profiles = db
        .list_user_profiles(crate::models::UserProfileQuery {
            fid: None,
            username: None,
            display_name: None,
            bio: None,
            location: None,
            twitter_username: None,
            github_username: None,
            limit: None, // Get all
            offset: None,
            start_timestamp: None,
            end_timestamp: None,
            sort_by: None,
            sort_order: None,
            search_term: None,
        })
        .await?;

    stats.total_profiles = profiles.len();
    info!("Found {} profiles to process", profiles.len());

    // Process in batches (profiles have multiple embeddings each)
    const BATCH_SIZE: usize = 100; // Increased for better GPU utilization
    for (batch_idx, chunk) in profiles.chunks(BATCH_SIZE).enumerate() {
        info!(
            "Processing batch {}/{} ({} profiles)",
            batch_idx + 1,
            profiles.len().div_ceil(BATCH_SIZE),
            chunk.len()
        );

        for profile in chunk {
            match backfill_profile_embeddings(&db, &embedding_service, profile).await {
                Ok(updated) => {
                    if updated {
                        stats.updated += 1;
                    } else {
                        stats.skipped += 1;
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to backfill embeddings for FID {}: {}",
                        profile.fid, e
                    );
                    stats.failed += 1;
                }
            }
        }

        // Small delay between batches to avoid rate limiting
        if batch_idx < profiles.len().div_ceil(BATCH_SIZE) - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    info!(
        "Backfill complete: {} updated, {} skipped, {} failed",
        stats.updated, stats.skipped, stats.failed
    );

    Ok(stats)
}

/// Backfill embeddings for a single profile
async fn backfill_profile_embeddings(
    db: &Database,
    embedding_service: &EmbeddingService,
    profile: &crate::models::UserProfile,
) -> Result<bool> {
    // Check if embeddings already exist
    if profile.profile_embedding.is_some()
        && profile.bio_embedding.is_some()
        && profile.interests_embedding.is_some()
    {
        return Ok(false); // Skip if all embeddings exist
    }

    // Generate profile embedding
    let profile_embedding = if profile.profile_embedding.is_none() {
        Some(
            embedding_service
                .generate_profile_embedding(
                    profile.username.as_deref(),
                    profile.display_name.as_deref(),
                    profile.bio.as_deref(),
                    profile.location.as_deref(),
                )
                .await?,
        )
    } else {
        None
    };

    // Generate bio embedding
    let bio_embedding = if profile.bio_embedding.is_none() {
        Some(
            embedding_service
                .generate_bio_embedding(profile.bio.as_deref())
                .await?,
        )
    } else {
        None
    };

    // Generate interests embedding
    let interests_embedding = if profile.interests_embedding.is_none() {
        Some(
            embedding_service
                .generate_interests_embedding(
                    profile.bio.as_deref(),
                    profile.twitter_username.as_deref(),
                    profile.github_username.as_deref(),
                )
                .await?,
        )
    } else {
        None
    };

    // Update database
    db.update_profile_embeddings(
        profile.fid,
        profile_embedding,
        bio_embedding,
        interests_embedding,
    )
    .await?;

    Ok(true)
}

/// Statistics from backfill operation
#[derive(Debug, Default)]
pub struct BackfillStats {
    pub total_profiles: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl BackfillStats {
    #[must_use]
    pub fn success_rate(&self) -> f64 {
        if self.total_profiles == 0 {
            0.0
        } else {
            (self.updated as f64 / self.total_profiles as f64) * 100.0
        }
    }
}
