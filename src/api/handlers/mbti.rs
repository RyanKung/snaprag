//! MBTI personality analysis API handlers
//!
//! Provides RESTful API endpoints for MBTI personality analysis and compatibility.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
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

/// Batch MBTI analysis for multiple users (POST /api/mbti/batch)
pub async fn batch_mbti_analysis(
    State(state): State<AppState>,
    Json(req): Json<BatchMbtiRequest>,
) -> Result<Json<ApiResponse<Vec<MbtiResult>>>, StatusCode> {
    info!("POST /api/mbti/batch - {} FIDs", req.fids.len());

    let mut results = Vec::new();
    let analyzer = if let Some(llm_service) = &state.llm_service {
        MbtiAnalyzer::with_llm(state.database.clone(), llm_service.clone())
    } else {
        MbtiAnalyzer::new(state.database.clone())
    };

    for fid in req.fids {
        // Check cache first
        if let Some(cached_mbti) = state.cache_service.get_mbti(fid).await {
            results.push(MbtiResult {
                fid,
                mbti_profile: Some(cached_mbti),
                error: None,
            });
            continue;
        }

        // Get social profile if available
        let social_profile = state.cache_service.get_social(fid).await;

        // Analyze MBTI
        match analyzer.analyze_mbti(fid, social_profile.as_ref()).await {
            Ok(mbti_profile) => {
                // Cache result
                state
                    .cache_service
                    .set_mbti(fid, mbti_profile.clone())
                    .await;

                results.push(MbtiResult {
                    fid,
                    mbti_profile: Some(mbti_profile),
                    error: None,
                });
            }
            Err(e) => {
                results.push(MbtiResult {
                    fid,
                    mbti_profile: None,
                    error: Some(format!("{}", e)),
                });
            }
        }
    }

    info!(
        "✅ Batch MBTI analysis complete: {}/{} successful",
        results.iter().filter(|r| r.mbti_profile.is_some()).count(),
        results.len()
    );

    Ok(Json(ApiResponse::success(results)))
}

/// Get MBTI statistics (GET /api/mbti/stats)
pub async fn get_mbti_stats(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<MbtiStatsResponse>>, StatusCode> {
    info!("GET /api/mbti/stats");

    // This would require storing MBTI results in database
    // For now, return a placeholder response
    let stats = MbtiStatsResponse {
        total_analyzed: 0,
        type_distribution: std::collections::HashMap::new(),
        average_confidence: 0.0,
        most_common_type: None,
    };

    Ok(Json(ApiResponse::success(stats)))
}

/// Search users by MBTI type (GET /api/mbti/search/:mbti_type)
pub async fn search_by_mbti_type(
    State(_state): State<AppState>,
    Path(mbti_type): Path<String>,
) -> Result<Json<ApiResponse<Vec<MbtiSearchResult>>>, StatusCode> {
    info!("GET /api/mbti/search/{}", mbti_type);

    // Validate MBTI type format (4 letters)
    let mbti_upper = mbti_type.to_uppercase();
    if mbti_upper.len() != 4 {
        return Ok(Json(ApiResponse::error(
            "Invalid MBTI type format. Expected 4 letters (e.g., INTJ, ENFP)".to_string(),
        )));
    }

    // Validate each dimension
    let valid_types = [
        ('E', 'I'), // Extraversion/Introversion
        ('S', 'N'), // Sensing/Intuition
        ('T', 'F'), // Thinking/Feeling
        ('J', 'P'), // Judging/Perceiving
    ];

    let chars: Vec<char> = mbti_upper.chars().collect();
    for (i, (option1, option2)) in valid_types.iter().enumerate() {
        if chars[i] != *option1 && chars[i] != *option2 {
            return Ok(Json(ApiResponse::error(format!(
                "Invalid MBTI type. Position {} must be {} or {}",
                i + 1,
                option1,
                option2
            ))));
        }
    }

    // This would require a database table to store MBTI results
    // For now, return empty results
    info!(
        "MBTI search for type {} - feature requires database persistence",
        mbti_upper
    );
    Ok(Json(ApiResponse::success(vec![])))
}

/// Compare MBTI compatibility between two users (GET /api/mbti/compatibility/:fid1/:fid2)
pub async fn get_mbti_compatibility(
    State(state): State<AppState>,
    Path((fid1, fid2)): Path<(i64, i64)>,
) -> Result<Json<ApiResponse<CompatibilityResponse>>, StatusCode> {
    info!("GET /api/mbti/compatibility/{}/{}", fid1, fid2);

    let analyzer = if let Some(llm_service) = &state.llm_service {
        MbtiAnalyzer::with_llm(state.database.clone(), llm_service.clone())
    } else {
        MbtiAnalyzer::new(state.database.clone())
    };

    // Get MBTI profiles for both users
    let mbti1 = match analyzer.analyze_mbti(fid1, None).await {
        Ok(profile) => profile,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!(
                "Failed to analyze FID {}: {}",
                fid1, e
            ))));
        }
    };

    let mbti2 = match analyzer.analyze_mbti(fid2, None).await {
        Ok(profile) => profile,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!(
                "Failed to analyze FID {}: {}",
                fid2, e
            ))));
        }
    };

    // Calculate compatibility score
    let compatibility = calculate_mbti_compatibility(&mbti1, &mbti2);

    info!(
        "✅ MBTI compatibility: {} ({}) + {} ({}) = {:.0}% ({})",
        fid1,
        mbti1.mbti_type,
        fid2,
        mbti2.mbti_type,
        compatibility.score * 100.0,
        compatibility.level
    );

    Ok(Json(ApiResponse::success(CompatibilityResponse {
        fid1,
        fid2,
        mbti_type1: mbti1.mbti_type,
        mbti_type2: mbti2.mbti_type,
        compatibility_score: compatibility.score,
        compatibility_level: compatibility.level,
        strengths: compatibility.strengths,
        challenges: compatibility.challenges,
        summary: compatibility.summary,
    })))
}

// ====== Request/Response Types ======

/// Batch MBTI request
#[derive(Debug, Deserialize)]
pub struct BatchMbtiRequest {
    pub fids: Vec<i64>,
}

/// MBTI result for batch operation
#[derive(Debug, Serialize)]
pub struct MbtiResult {
    pub fid: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mbti_profile: Option<crate::personality::MbtiProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MBTI statistics response
#[derive(Debug, Serialize)]
pub struct MbtiStatsResponse {
    pub total_analyzed: usize,
    pub type_distribution: std::collections::HashMap<String, usize>,
    pub average_confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_common_type: Option<String>,
}

/// MBTI search result
#[derive(Debug, Serialize)]
pub struct MbtiSearchResult {
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub mbti_type: String,
    pub confidence: f32,
}

/// Compatibility analysis response
#[derive(Debug, Serialize)]
pub struct CompatibilityResponse {
    pub fid1: i64,
    pub fid2: i64,
    pub mbti_type1: String,
    pub mbti_type2: String,
    pub compatibility_score: f32,
    pub compatibility_level: String,
    pub strengths: Vec<String>,
    pub challenges: Vec<String>,
    pub summary: String,
}

// ====== Helper Functions ======

/// Compatibility calculation result
struct CompatibilityAnalysis {
    score: f32,
    level: String,
    strengths: Vec<String>,
    challenges: Vec<String>,
    summary: String,
}

/// Calculate MBTI compatibility between two profiles
///
/// Uses dimension differences to calculate compatibility score.
/// Lower difference = higher compatibility.
fn calculate_mbti_compatibility(
    profile1: &crate::personality::MbtiProfile,
    profile2: &crate::personality::MbtiProfile,
) -> CompatibilityAnalysis {
    let type1 = &profile1.mbti_type;
    let type2 = &profile2.mbti_type;

    // Calculate dimension differences
    let ei_diff = (profile1.dimensions.ei_score - profile2.dimensions.ei_score).abs();
    let sn_diff = (profile1.dimensions.sn_score - profile2.dimensions.sn_score).abs();
    let tf_diff = (profile1.dimensions.tf_score - profile2.dimensions.tf_score).abs();
    let jp_diff = (profile1.dimensions.jp_score - profile2.dimensions.jp_score).abs();

    // Calculate compatibility score (0.0-1.0)
    let avg_diff = (ei_diff + sn_diff + tf_diff + jp_diff) / 4.0;
    let score = 1.0 - avg_diff;

    // Determine compatibility level
    let level = if score > 0.8 {
        "Excellent"
    } else if score > 0.6 {
        "Good"
    } else if score > 0.4 {
        "Moderate"
    } else {
        "Challenging"
    };

    // Analyze strengths and challenges
    let mut strengths = Vec::new();
    let mut challenges = Vec::new();

    // E/I dimension
    if ei_diff < 0.3 {
        strengths.push("Similar energy levels and social preferences".to_string());
    } else {
        challenges.push("Different social energy needs".to_string());
    }

    // S/N dimension
    if sn_diff < 0.3 {
        strengths.push("Shared perspective on information processing".to_string());
    } else {
        challenges.push("Different focus (details vs big picture)".to_string());
    }

    // T/F dimension
    if tf_diff < 0.3 {
        strengths.push("Compatible decision-making approaches".to_string());
    } else if tf_diff > 0.6 {
        strengths.push("Complementary thinking/feeling perspectives".to_string());
    } else {
        challenges.push("Some differences in decision-making style".to_string());
    }

    // J/P dimension
    if jp_diff < 0.3 {
        strengths.push("Similar approach to planning".to_string());
    } else {
        challenges.push("Different organization styles".to_string());
    }

    // Generate summary
    let summary = format!(
        "{} and {} show {} compatibility ({:.0}% match) with {} shared trait{}.",
        type1,
        type2,
        level.to_lowercase(),
        score * 100.0,
        strengths.len(),
        if strengths.len() == 1 { "" } else { "s" }
    );

    CompatibilityAnalysis {
        score,
        level: level.to_string(),
        strengths,
        challenges,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbti_compatibility_same_type() {
        use crate::personality::MbtiDimensions;
        use crate::personality::MbtiProfile;

        let profile1 = MbtiProfile {
            fid: 1,
            mbti_type: "INTJ".to_string(),
            confidence: 0.9,
            dimensions: MbtiDimensions {
                ei_score: 0.8,
                sn_score: 0.9,
                tf_score: 0.8,
                jp_score: 0.2,
                ei_confidence: 0.9,
                sn_confidence: 0.9,
                tf_confidence: 0.8,
                jp_confidence: 0.8,
            },
            traits: vec![],
            analysis: "Test".to_string(),
        };

        let profile2 = profile1.clone();
        let compatibility = calculate_mbti_compatibility(&profile1, &profile2);

        assert_eq!(compatibility.score, 1.0);
        assert_eq!(compatibility.level, "Excellent");
    }

    #[test]
    fn test_mbti_compatibility_opposite_types() {
        use crate::personality::MbtiDimensions;
        use crate::personality::MbtiProfile;

        let profile1 = MbtiProfile {
            fid: 1,
            mbti_type: "INTJ".to_string(),
            confidence: 0.9,
            dimensions: MbtiDimensions {
                ei_score: 0.9,
                sn_score: 0.9,
                tf_score: 0.9,
                jp_score: 0.1,
                ei_confidence: 0.9,
                sn_confidence: 0.9,
                tf_confidence: 0.9,
                jp_confidence: 0.9,
            },
            traits: vec![],
            analysis: "Test".to_string(),
        };

        let profile2 = MbtiProfile {
            fid: 2,
            mbti_type: "ESFP".to_string(),
            confidence: 0.9,
            dimensions: MbtiDimensions {
                ei_score: 0.1,
                sn_score: 0.1,
                tf_score: 0.1,
                jp_score: 0.9,
                ei_confidence: 0.9,
                sn_confidence: 0.9,
                tf_confidence: 0.9,
                jp_confidence: 0.9,
            },
            traits: vec![],
            analysis: "Test".to_string(),
        };

        let compatibility = calculate_mbti_compatibility(&profile1, &profile2);

        assert!(compatibility.score < 0.5);
        assert!(!compatibility.challenges.is_empty());
    }
}
