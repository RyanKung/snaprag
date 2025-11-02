/// API request handlers
use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use crate::api::cache::CacheService;
use crate::api::types::ApiResponse;
use crate::api::types::FetchResponse;
use crate::api::types::FetchUsersBatchRequest;
use crate::api::types::HealthResponse;
use crate::api::types::ProfileResponse;
use crate::config::AppConfig;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::models::UserProfileQuery;
use crate::rag::CastRetriever;
use crate::rag::RagQuery;
use crate::rag::RagService;
use crate::rag::RetrievalMethod;
use crate::rag::Retriever;
use crate::social_graph::SocialGraphAnalyzer;

// Re-export sub-modules
pub mod chat;
pub mod mbti;
pub mod profile;
pub mod rag;
pub mod search;
pub mod stats;

// Re-export handlers
pub use chat::*;
pub use mbti::*;
pub use profile::*;
pub use rag::*;
pub use search::*;
pub use stats::*;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
    pub embedding_service: Arc<EmbeddingService>,
    pub llm_service: Option<Arc<LlmService>>,
    pub lazy_loader: Option<Arc<crate::sync::LazyLoader>>,
    pub session_manager: Arc<crate::api::session::SessionManager>,
    pub cache_service: Arc<CacheService>,
}

/// Health check handler
pub async fn health() -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Fetch user on-demand (POST /api/fetch/user/:fid)
pub async fn fetch_user(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<FetchResponse>>, StatusCode> {
    info!("POST /api/fetch/user/{}", fid);

    // Check if lazy loader is available
    let loader = match &state.lazy_loader {
        Some(l) => l,
        None => {
            return Ok(Json(ApiResponse::error(
                "Lazy loading is not enabled".to_string(),
            )));
        }
    };

    // Check if user already exists
    let existing_profile = state.database.get_user_profile(fid).await.ok().flatten();
    if let Some(profile) = existing_profile {
        return Ok(Json(ApiResponse::success(FetchResponse {
            profile: ProfileResponse {
                fid: profile.fid,
                username: profile.username,
                display_name: profile.display_name,
                bio: profile.bio,
                pfp_url: profile.pfp_url,
                location: profile.location,
                twitter_username: profile.twitter_username,
                github_username: profile.github_username,
            },
            casts_count: 0, // TODO: get actual count
            embeddings_generated: None,
            source: "database".to_string(),
        })));
    }

    // Fetch user profile
    match loader.fetch_user_profile(fid as u64).await {
        Ok(profile) => {
            info!("✅ Successfully fetched user profile for FID {}", fid);
            Ok(Json(ApiResponse::success(FetchResponse {
                profile: ProfileResponse {
                    fid: profile.fid,
                    username: profile.username,
                    display_name: profile.display_name,
                    bio: profile.bio,
                    pfp_url: profile.pfp_url,
                    location: profile.location,
                    twitter_username: profile.twitter_username,
                    github_username: profile.github_username,
                },
                casts_count: 0, // TODO: get actual count
                embeddings_generated: None,
                source: "snapchain".to_string(),
            })))
        }
        Err(e) => {
            error!("Failed to fetch user profile for FID {}: {}", fid, e);
            Ok(Json(ApiResponse::error(format!(
                "Failed to fetch user profile: {e}"
            ))))
        }
    }
}

/// Fetch multiple users on-demand (POST /api/fetch/users)
pub async fn fetch_users_batch(
    State(state): State<AppState>,
    Json(req): Json<FetchUsersBatchRequest>,
) -> Result<Json<ApiResponse<Vec<FetchResponse>>>, StatusCode> {
    info!("POST /api/fetch/users - {} FIDs", req.fids.len());

    // Check if lazy loader is available
    let loader = match &state.lazy_loader {
        Some(l) => l,
        None => {
            return Ok(Json(ApiResponse::error(
                "Lazy loading is not enabled".to_string(),
            )));
        }
    };

    let mut responses = Vec::new();

    for fid in req.fids {
        // Check if user already exists
        let existing_profile = state
            .database
            .get_user_profile(fid as i64)
            .await
            .ok()
            .flatten();
        if let Some(profile) = existing_profile {
            responses.push(FetchResponse {
                profile: ProfileResponse {
                    fid: profile.fid,
                    username: profile.username,
                    display_name: profile.display_name,
                    bio: profile.bio,
                    pfp_url: profile.pfp_url,
                    location: profile.location,
                    twitter_username: profile.twitter_username,
                    github_username: profile.github_username,
                },
                casts_count: 0, // TODO: get actual count
                embeddings_generated: None,
                source: "database".to_string(),
            });
            continue;
        }

        // Fetch user profile
        match loader.fetch_user_profile(fid).await {
            Ok(profile) => {
                info!("✅ Successfully fetched user profile for FID {}", fid);
                responses.push(FetchResponse {
                    profile: ProfileResponse {
                        fid: profile.fid,
                        username: profile.username,
                        display_name: profile.display_name,
                        bio: profile.bio,
                        pfp_url: profile.pfp_url,
                        location: profile.location,
                        twitter_username: profile.twitter_username,
                        github_username: profile.github_username,
                    },
                    casts_count: 0, // TODO: get actual count
                    embeddings_generated: None,
                    source: "snapchain".to_string(),
                });
            }
            Err(e) => {
                error!("Failed to fetch user profile for FID {}: {}", fid, e);
                // Skip failed users for now
            }
        }
    }

    Ok(Json(ApiResponse::success(responses)))
}

/// Social graph analysis handler (GET /api/social/:fid) with caching
pub async fn get_social_analysis(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = std::time::Instant::now();
    info!("GET /api/social/{}", fid);

    // Check cache first
    tracing::debug!("Checking cache for social analysis FID {}", fid);
    if let Some(cached_social) = state.cache_service.get_social(fid).await {
        let duration = start_time.elapsed();
        info!(
            "📦 Social cache hit for FID {} - {}ms",
            fid,
            duration.as_millis()
        );
        let social_data = serde_json::to_value(&cached_social).unwrap_or_else(|_| {
            serde_json::json!({
                "error": "Failed to serialize cached social profile"
            })
        });
        return Ok(Json(ApiResponse::success(social_data)));
    }
    tracing::debug!(
        "No cache hit for social analysis FID {}, proceeding to analysis",
        fid
    );

    // Get user profile first
    let profile = match state.database.get_user_profile(fid).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            let duration = start_time.elapsed();
            info!(
                "❌ GET /api/social/{} - {}ms - 404 (user not found)",
                fid,
                duration.as_millis()
            );
            return Ok(Json(ApiResponse::error(format!(
                "User with FID {} not found",
                fid
            ))));
        }
        Err(e) => {
            error!("Failed to get user profile for FID {}: {}", fid, e);
            let duration = start_time.elapsed();
            info!(
                "❌ GET /api/social/{} - {}ms - 500 (profile error)",
                fid,
                duration.as_millis()
            );
            return Ok(Json(ApiResponse::error(format!(
                "Failed to get user profile: {}",
                e
            ))));
        }
    };

    // Initialize social graph analyzer
    let analyzer = SocialGraphAnalyzer::new(state.database.clone());

    // Analyze user's social graph
    match analyzer.analyze_user(fid).await {
        Ok(social_profile) => {
            // Cache the social analysis
            tracing::debug!("Caching social analysis response for FID {}", fid);
            state
                .cache_service
                .set_social(fid, social_profile.clone())
                .await;

            // Convert social profile to JSON
            let social_data = serde_json::to_value(&social_profile).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize social profile"
                })
            });

            let duration = start_time.elapsed();
            info!(
                "✅ GET /api/social/{} - {}ms - 200 (cached)",
                fid,
                duration.as_millis()
            );
            Ok(Json(ApiResponse::success(social_data)))
        }
        Err(e) => {
            error!("Failed to analyze social graph for FID {}: {}", fid, e);
            let duration = start_time.elapsed();
            info!(
                "❌ GET /api/social/{} - {}ms - 500 (analysis error)",
                fid,
                duration.as_millis()
            );
            Ok(Json(ApiResponse::error(format!(
                "Failed to analyze social graph: {}",
                e
            ))))
        }
    }
}

/// Social graph analysis handler by username (GET /api/social/username/:username) with caching
pub async fn get_social_analysis_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("GET /api/social/username/{}", username);

    // Get user profile by username first
    let profile = match state.database.get_user_profile_by_username(&username).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            return Ok(Json(ApiResponse::error(format!(
                "User with username {} not found",
                username
            ))));
        }
        Err(e) => {
            error!(
                "Failed to get user profile for username {}: {}",
                username, e
            );
            return Ok(Json(ApiResponse::error(format!(
                "Failed to get user profile: {}",
                e
            ))));
        }
    };

    // Check cache first for the FID
    if let Some(cached_social) = state.cache_service.get_social(profile.fid).await {
        info!(
            "📦 Social cache hit for username {} (FID {})",
            username, profile.fid
        );
        let social_data = serde_json::to_value(&cached_social).unwrap_or_else(|_| {
            serde_json::json!({
                "error": "Failed to serialize cached social profile"
            })
        });
        return Ok(Json(ApiResponse::success(social_data)));
    }

    // Initialize social graph analyzer
    let analyzer = SocialGraphAnalyzer::new(state.database.clone());

    // Analyze user's social graph using the FID from the profile
    match analyzer.analyze_user(profile.fid).await {
        Ok(social_profile) => {
            // Cache the social analysis
            state
                .cache_service
                .set_social(profile.fid, social_profile.clone())
                .await;

            // Convert social profile to JSON
            let social_data = serde_json::to_value(&social_profile).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize social profile"
                })
            });

            Ok(Json(ApiResponse::success(social_data)))
        }
        Err(e) => {
            error!(
                "Failed to analyze social graph for username {} (FID {}): {}",
                username, profile.fid, e
            );
            Ok(Json(ApiResponse::error(format!(
                "Failed to analyze social graph: {}",
                e
            ))))
        }
    }
}
