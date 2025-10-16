//! Cast retrieval module for semantic search

use std::sync::Arc;

use tracing::debug;

use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::errors::Result;
use crate::models::CastSearchResult;

/// Retriever for cast content
pub struct CastRetriever {
    database: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
}

impl CastRetriever {
    /// Create a new cast retriever
    pub fn new(database: Arc<Database>, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            database,
            embedding_service,
        }
    }

    /// Semantic search for casts
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<CastSearchResult>> {
        debug!("Performing cast semantic search: {}", query);

        // Generate query embedding
        let query_embedding = self.embedding_service.generate(query).await?;

        // Search in database
        let results = self
            .database
            .semantic_search_casts(query_embedding, limit as i64, threshold)
            .await?;

        debug!("Found {} matching casts", results.len());
        Ok(results)
    }

    /// Search casts by FID
    pub async fn search_by_fid(&self, fid: i64, limit: usize) -> Result<Vec<crate::models::Cast>> {
        debug!("Searching casts for FID {}", fid);

        let casts = self
            .database
            .get_casts_by_fid(fid, Some(limit as i64), Some(0))
            .await?;

        Ok(casts)
    }

    /// Get cast thread
    pub async fn get_thread(
        &self,
        message_hash: Vec<u8>,
        max_depth: usize,
    ) -> Result<crate::database::CastThread> {
        debug!("Retrieving cast thread");

        let thread = self
            .database
            .get_cast_thread(message_hash, max_depth)
            .await?;

        Ok(thread)
    }

    /// Search recent casts across all users
    pub async fn search_recent(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<crate::models::Cast>> {
        debug!(
            "Searching recent casts (limit: {}, offset: {})",
            limit, offset
        );

        // Use simplified query for recent casts
        let casts = sqlx::query_as::<_, crate::models::Cast>(
            "SELECT * FROM casts WHERE text IS NOT NULL ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(self.database.pool())
        .await?;

        Ok(casts)
    }
}
