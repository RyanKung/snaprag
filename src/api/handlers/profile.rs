/// Profile-related API handlers
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use super::AppState;
use crate::api::types::ApiResponse;
use crate::api::types::ProfileResponse;
use crate::api::types::SearchQuery;
use crate::models::UserProfileQuery;

/// Get profile by FID (with automatic lazy loading and caching)
pub async fn get_profile(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<ProfileResponse>>, StatusCode> {
    let start_time = std::time::Instant::now();
    info!("GET /api/profiles/{}", fid);

    // Check cache first if enabled
    tracing::debug!("Checking cache for profile FID {}", fid);
    if let Some(cached_profile) = state.cache_service.get_profile(fid).await {
        let duration = start_time.elapsed();
        info!("📦 Profile cache hit for FID {} - {}ms", fid, duration.as_millis());
        return Ok(Json(ApiResponse::success(cached_profile)));
    }
    tracing::debug!("No cache hit for profile FID {}, proceeding to database", fid);

    // Try database first
    let profile = match state.database.get_user_profile(fid).await {
        Ok(Some(p)) => Some(p),
        Ok(None) => {
            // Try lazy loading if available
            if let Some(loader) = &state.lazy_loader {
                info!("⚡ Profile {} not found, attempting lazy load", fid);
                match loader.fetch_user_profile(fid as u64).await {
                    Ok(p) => {
                        info!("✅ Successfully lazy loaded profile {}", fid);
                        Some(p)
                    }
                    Err(e) => {
                        info!("Failed to lazy load profile {}: {}", fid, e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            error!("Error fetching profile: {}", e);
            let duration = start_time.elapsed();
            info!("❌ GET /api/profiles/{} - {}ms - 500", fid, duration.as_millis());
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match profile {
        Some(profile) => {
            let response = ProfileResponse {
                fid: profile.fid,
                username: profile.username,
                display_name: profile.display_name,
                bio: profile.bio,
                pfp_url: profile.pfp_url,
                location: profile.location,
                twitter_username: profile.twitter_username,
                github_username: profile.github_username,
            };
            
            // Cache the response
            tracing::debug!("Caching profile response for FID {}", fid);
            state.cache_service.set_profile(fid, response.clone()).await;
            
            let duration = start_time.elapsed();
            info!("✅ GET /api/profiles/{} - {}ms - 200 (cached)", fid, duration.as_millis());
            Ok(Json(ApiResponse::success(response)))
        }
        None => {
            let duration = start_time.elapsed();
            info!("❌ GET /api/profiles/{} - {}ms - 404", fid, duration.as_millis());
            Err(StatusCode::NOT_FOUND)
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

/// Get profile by username (with caching)
pub async fn get_profile_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<ApiResponse<ProfileResponse>>, StatusCode> {
    info!("GET /api/profiles/username/{}", username);

    // Try database first
    let profile = match state.database.get_user_profile_by_username(&username).await {
        Ok(Some(p)) => Some(p),
        Ok(None) => {
            // Try lazy loading if available
            if let Some(loader) = &state.lazy_loader {
                info!("⚡ Profile {} not found, attempting lazy load", username);
                // For username-based lazy loading, we need to find the FID first
                // This is a limitation - we can't lazy load by username directly
                info!("⚠️ Lazy loading by username not supported, user {} not found", username);
                None
            } else {
                None
            }
        }
        Err(e) => {
            error!("Error fetching profile by username: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match profile {
        Some(profile) => {
            let response = ProfileResponse {
                fid: profile.fid,
                username: profile.username,
                display_name: profile.display_name,
                bio: profile.bio,
                pfp_url: profile.pfp_url,
                location: profile.location,
                twitter_username: profile.twitter_username,
                github_username: profile.github_username,
            };
            
            // Cache the response by FID (since username lookups are less common)
            state.cache_service.set_profile(profile.fid, response.clone()).await;
            
            Ok(Json(ApiResponse::success(response)))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
