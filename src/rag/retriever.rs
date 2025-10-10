//! Retrieval module for semantic and hybrid search

use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::errors::Result;
use crate::models::UserProfile;
use crate::rag::{MatchType, SearchResult};
use std::sync::Arc;
use tracing::debug;

/// Retriever for semantic and hybrid search
pub struct Retriever {
    database: Arc<Database>,
    embedding_service: Arc<EmbeddingService>,
}

impl Retriever {
    /// Create a new retriever
    pub fn new(database: Arc<Database>, embedding_service: Arc<EmbeddingService>) -> Self {
        Self {
            database,
            embedding_service,
        }
    }

    /// Semantic search using vector embeddings
    pub async fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Performing semantic search: {}", query);

        // Generate query embedding
        let query_embedding = self.embedding_service.generate(query).await?;

        // Search in database
        let profiles = self
            .database
            .semantic_search_profiles(query_embedding, limit as i64, threshold)
            .await?;

        // Convert to search results with scores
        let results = profiles
            .into_iter()
            .enumerate()
            .map(|(idx, profile)| SearchResult {
                profile,
                score: 1.0 - (idx as f32 / limit as f32), // Decreasing score based on rank
                match_type: MatchType::Semantic,
            })
            .collect();

        Ok(results)
    }

    /// Keyword search using text matching
    pub async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        debug!("Performing keyword search: {}", query);

        let profiles = self
            .database
            .list_user_profiles(crate::models::UserProfileQuery {
                fid: None,
                username: None,
                display_name: None,
                bio: None,
                location: None,
                twitter_username: None,
                github_username: None,
                limit: Some(limit as i64),
                offset: None,
                start_timestamp: None,
                end_timestamp: None,
                sort_by: None,
                sort_order: None,
                search_term: Some(query.to_string()),
            })
            .await?;

        let results = profiles
            .into_iter()
            .enumerate()
            .map(|(idx, profile)| SearchResult {
                profile,
                score: 1.0 - (idx as f32 / limit as f32),
                match_type: MatchType::Keyword,
            })
            .collect();

        Ok(results)
    }

    /// Hybrid search combining semantic and keyword matching
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        debug!("Performing hybrid search: {}", query);

        // Generate query embedding
        let query_embedding = self.embedding_service.generate(query).await?;

        // Perform hybrid search
        let profiles = self
            .database
            .hybrid_search_profiles(Some(query_embedding), Some(query.to_string()), limit as i64)
            .await?;

        let results = profiles
            .into_iter()
            .enumerate()
            .map(|(idx, profile)| SearchResult {
                profile,
                score: 1.0 - (idx as f32 / limit as f32),
                match_type: MatchType::Hybrid,
            })
            .collect();

        Ok(results)
    }

    /// Search with automatic method selection
    pub async fn auto_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // For now, default to hybrid search
        // TODO: Implement smarter selection based on query type
        self.hybrid_search(query, limit).await
    }
}

/// Rerank search results using various strategies
pub struct Reranker;

impl Reranker {
    /// Reciprocal Rank Fusion (RRF) for combining multiple result sets
    pub fn reciprocal_rank_fusion(
        results_sets: Vec<Vec<SearchResult>>,
        k: f32,
    ) -> Vec<SearchResult> {
        use std::collections::HashMap;

        let mut scores: HashMap<i64, (f32, UserProfile, MatchType)> = HashMap::new();

        for results in results_sets {
            for (rank, result) in results.into_iter().enumerate() {
                let rrf_score = 1.0 / (k + rank as f32 + 1.0);
                let entry = scores
                    .entry(result.profile.fid)
                    .or_insert((0.0, result.profile.clone(), result.match_type));
                entry.0 += rrf_score;
            }
        }

        let mut final_results: Vec<_> = scores
            .into_iter()
            .map(|(_, (score, profile, match_type))| SearchResult {
                profile,
                score,
                match_type,
            })
            .collect();

        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results
    }

    /// Simple score-based reranking
    pub fn rerank_by_score(mut results: Vec<SearchResult>) -> Vec<SearchResult> {
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }
}

