//! API route definitions

use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::handlers::AppState;
use super::handlers::{
    self,
};

/// Create `RESTful` API router
pub fn api_routes(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health))
        // Profile endpoints
        .route("/profiles", get(handlers::list_profiles))
        .route("/profiles/:fid", get(handlers::get_profile))
        // Fetch endpoints (lazy loading)
        .route("/fetch/user/:fid", post(handlers::fetch_user))
        .route("/fetch/users", post(handlers::fetch_users_batch))
        // Search endpoints
        .route("/search/profiles", post(handlers::search_profiles))
        .route("/search/casts", post(handlers::search_casts))
        // RAG endpoints
        .route("/rag/query", post(handlers::rag_query))
        // Chat endpoints (interactive AI role-play)
        .route("/chat/create", post(handlers::create_chat_session))
        .route("/chat/message", post(handlers::send_chat_message))
        .route("/chat/session", get(handlers::get_chat_session))
        .route(
            "/chat/session/:session_id",
            delete(handlers::delete_chat_session),
        )
        // Statistics
        .route("/stats", get(handlers::get_stats))
        .with_state(state)
}
