//! API request handlers

use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use crate::api::types::*;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::models::UserProfileQuery;
use crate::rag::CastRetriever;
use crate::rag::RagQuery;
use crate::rag::RagService;
use crate::rag::RetrievalMethod;
use crate::rag::Retriever;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
    pub embedding_service: Arc<EmbeddingService>,
    pub llm_service: Arc<LlmService>,
}

/// Health check handler
pub async fn health() -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Get profile by FID
pub async fn get_profile(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<ProfileResponse>>, StatusCode> {
    info!("GET /api/profiles/{}", fid);

    match state.database.get_user_profile(fid).await {
        Ok(Some(profile)) => Ok(Json(ApiResponse::success(ProfileResponse {
            fid: profile.fid,
            username: profile.username,
            display_name: profile.display_name,
            bio: profile.bio,
            pfp_url: profile.pfp_url,
            location: profile.location,
            twitter_username: profile.twitter_username,
            github_username: profile.github_username,
        }))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Error fetching profile: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// List profiles
pub async fn list_profiles(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<ProfileResponse>>>, StatusCode> {
    info!("GET /api/profiles?q={}&limit={}", params.q, params.limit);

    let query = UserProfileQuery {
        fid: None,
        username: None,
        display_name: None,
        bio: None,
        location: None,
        twitter_username: None,
        github_username: None,
        limit: Some(params.limit as i64),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
        sort_by: None,
        sort_order: None,
        search_term: if params.q.is_empty() {
            None
        } else {
            Some(params.q)
        },
    };

    match state.database.list_user_profiles(query).await {
        Ok(profiles) => {
            let response: Vec<ProfileResponse> = profiles
                .into_iter()
                .map(|p| ProfileResponse {
                    fid: p.fid,
                    username: p.username,
                    display_name: p.display_name,
                    bio: p.bio,
                    pfp_url: p.pfp_url,
                    location: p.location,
                    twitter_username: p.twitter_username,
                    github_username: p.github_username,
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error listing profiles: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search profiles semantically
pub async fn search_profiles(
    State(state): State<AppState>,
    Json(req): Json<ProfileSearchRequest>,
) -> Result<Json<ApiResponse<Vec<ProfileResponse>>>, StatusCode> {
    info!("POST /api/search/profiles: {}", req.query);

    let retriever = Retriever::new(state.database.clone(), state.embedding_service.clone());

    let results = match req.method.as_deref() {
        Some("semantic") => retriever.semantic_search(&req.query, req.limit, None).await,
        Some("keyword") => retriever.keyword_search(&req.query, req.limit).await,
        Some("hybrid") => retriever.hybrid_search(&req.query, req.limit).await,
        _ => retriever.auto_search(&req.query, req.limit).await,
    };

    match results {
        Ok(search_results) => {
            let response: Vec<ProfileResponse> = search_results
                .into_iter()
                .map(|r| ProfileResponse {
                    fid: r.profile.fid,
                    username: r.profile.username,
                    display_name: r.profile.display_name,
                    bio: r.profile.bio,
                    pfp_url: r.profile.pfp_url,
                    location: r.profile.location,
                    twitter_username: r.profile.twitter_username,
                    github_username: r.profile.github_username,
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error searching profiles: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search casts semantically
pub async fn search_casts(
    State(state): State<AppState>,
    Json(req): Json<CastSearchRequest>,
) -> Result<Json<ApiResponse<Vec<CastResponse>>>, StatusCode> {
    info!("POST /api/search/casts: {}", req.query);

    let retriever = CastRetriever::new(state.database.clone(), state.embedding_service.clone());

    match retriever
        .semantic_search(&req.query, req.limit, Some(req.threshold))
        .await
    {
        Ok(results) => {
            let response: Vec<CastResponse> = results
                .into_iter()
                .map(|r| CastResponse {
                    message_hash: hex::encode(&r.message_hash),
                    fid: r.fid,
                    text: r.text,
                    timestamp: r.timestamp,
                    similarity: Some(r.similarity),
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error searching casts: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// RAG query
pub async fn rag_query(
    State(state): State<AppState>,
    Json(req): Json<RagQueryRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("POST /api/rag/query: {}", req.question);

    let rag_service = RagService::from_services(
        state.database.clone(),
        state.embedding_service.clone(),
        (*state.llm_service).clone(),
    );

    let method = match req.method.as_deref() {
        Some("semantic") => RetrievalMethod::Semantic,
        Some("keyword") => RetrievalMethod::Keyword,
        Some("hybrid") => RetrievalMethod::Hybrid,
        _ => RetrievalMethod::Auto,
    };

    let query = RagQuery {
        question: req.question,
        retrieval_limit: req.retrieval_limit,
        retrieval_method: method,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
    };

    match rag_service.query_with_options(query).await {
        Ok(response) => Ok(Json(ApiResponse::success(response.answer))),
        Err(e) => {
            error!("Error processing RAG query: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get statistics
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatsResponse>>, StatusCode> {
    info!("GET /api/stats");

    // Get basic counts
    let total_profiles = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

    let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts")
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

    let profiles_with_embeddings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM user_profiles WHERE profile_embedding IS NOT NULL",
    )
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    let casts_with_embeddings =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cast_embeddings")
            .fetch_one(state.database.pool())
            .await
            .unwrap_or(0);

    Ok(Json(ApiResponse::success(StatsResponse {
        total_profiles,
        total_casts,
        profiles_with_embeddings,
        casts_with_embeddings,
    })))
}
