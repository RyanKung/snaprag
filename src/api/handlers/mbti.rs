//! MBTI personality analysis API handlers

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use crate::api::handlers::AppState;
use crate::api::types::ApiResponse;
use crate::personality::MbtiAnalyzer;

/// Get MBTI personality analysis for a user (GET /api/mbti/:fid)
pub async fn get_mbti_analysis(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let start_time = std::time::Instant::now();
    info!("GET /api/mbti/{}", fid);

    // Check cache first
    if let Some(cached_mbti) = state.cache_service.get_mbti(fid).await {
        let duration = start_time.elapsed();
        info!(
            "📦 MBTI cache hit for FID {} - {}ms",
            fid,
            duration.as_millis()
        );
        let mbti_data = serde_json::to_value(&cached_mbti).unwrap_or_else(|_| {
            serde_json::json!({
                "error": "Failed to serialize cached MBTI profile"
            })
        });
        return Ok(Json(ApiResponse::success(mbti_data)));
    }

    // Get user profile first (for validation)
    let profile = match state.database.get_user_profile(fid).await {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            let duration = start_time.elapsed();
            info!(
                "❌ GET /api/mbti/{} - {}ms - 404 (user not found)",
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
                "❌ GET /api/mbti/{} - {}ms - 500 (profile error)",
                fid,
                duration.as_millis()
            );
            return Ok(Json(ApiResponse::error(format!(
                "Failed to get user profile: {}",
                e
            ))));
        }
    };

    // Try to get social profile from cache
    let social_profile = state.cache_service.get_social(fid).await;

    // Initialize MBTI analyzer (with or without LLM)
    let analyzer = if let Some(llm_service) = &state.llm_service {
        MbtiAnalyzer::with_llm(state.database.clone(), llm_service.clone())
    } else {
        MbtiAnalyzer::new(state.database.clone())
    };

    // Analyze MBTI personality
    match analyzer.analyze_mbti(fid, social_profile.as_ref()).await {
        Ok(mbti_profile) => {
            // Cache the MBTI analysis
            state
                .cache_service
                .set_mbti(fid, mbti_profile.clone())
                .await;

            // Convert to JSON
            let mbti_data = serde_json::to_value(&mbti_profile).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize MBTI profile"
                })
            });

            let duration = start_time.elapsed();
            info!(
                "✅ GET /api/mbti/{} - {}ms - 200 (type: {}, confidence: {:.2})",
                fid,
                duration.as_millis(),
                mbti_profile.mbti_type,
                mbti_profile.confidence
            );
            Ok(Json(ApiResponse::success(mbti_data)))
        }
        Err(e) => {
            error!("Failed to analyze MBTI for FID {}: {}", fid, e);
            let duration = start_time.elapsed();
            info!(
                "❌ GET /api/mbti/{} - {}ms - 500 (analysis error)",
                fid,
                duration.as_millis()
            );
            Ok(Json(ApiResponse::error(format!(
                "Failed to analyze MBTI: {}",
                e
            ))))
        }
    }
}

/// Get MBTI personality analysis by username (GET /api/mbti/username/:username)
pub async fn get_mbti_analysis_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("GET /api/mbti/username/{}", username);

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
    if let Some(cached_mbti) = state.cache_service.get_mbti(profile.fid).await {
        info!(
            "📦 MBTI cache hit for username {} (FID {})",
            username, profile.fid
        );
        let mbti_data = serde_json::to_value(&cached_mbti).unwrap_or_else(|_| {
            serde_json::json!({
                "error": "Failed to serialize cached MBTI profile"
            })
        });
        return Ok(Json(ApiResponse::success(mbti_data)));
    }

    // Try to get social profile from cache
    let social_profile = state.cache_service.get_social(profile.fid).await;

    // Initialize MBTI analyzer
    let analyzer = if let Some(llm_service) = &state.llm_service {
        MbtiAnalyzer::with_llm(state.database.clone(), llm_service.clone())
    } else {
        MbtiAnalyzer::new(state.database.clone())
    };

    // Analyze using the FID from the profile
    match analyzer
        .analyze_mbti(profile.fid, social_profile.as_ref())
        .await
    {
        Ok(mbti_profile) => {
            // Cache the result
            state
                .cache_service
                .set_mbti(profile.fid, mbti_profile.clone())
                .await;

            let mbti_data = serde_json::to_value(&mbti_profile).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize MBTI profile"
                })
            });

            info!(
                "✅ GET /api/mbti/username/{} (FID {}) - type: {}, confidence: {:.2}",
                username, profile.fid, mbti_profile.mbti_type, mbti_profile.confidence
            );
            Ok(Json(ApiResponse::success(mbti_data)))
        }
        Err(e) => {
            error!(
                "Failed to analyze MBTI for username {} (FID {}): {}",
                username, profile.fid, e
            );
            Ok(Json(ApiResponse::error(format!(
                "Failed to analyze MBTI: {}",
                e
            ))))
        }
    }
}
