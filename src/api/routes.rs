//! API route definitions

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::handlers::AppState;
use super::handlers::{
    self,
};

/// Create RESTful API router
pub fn api_routes(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Profile endpoints
        .route("/profiles", get(handlers::list_profiles))
        .route("/profiles/:fid", get(handlers::get_profile))
        // Search endpoints
        .route("/search/profiles", post(handlers::search_profiles))
        .route("/search/casts", post(handlers::search_casts))
        // RAG endpoints
        .route("/rag/query", post(handlers::rag_query))
        // Statistics
        .route("/stats", get(handlers::get_stats))
        .with_state(state)
}
