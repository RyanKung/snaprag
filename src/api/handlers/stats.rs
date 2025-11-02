/// Stats-related API handlers
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::info;

use super::AppState;
use crate::api::types::ApiResponse;
use crate::api::types::StatsResponse;

/// Get stats including cache information
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatsResponse>>, StatusCode> {
    info!("GET /api/stats");

    // Get basic counts
    let total_profiles =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(DISTINCT fid) FROM user_profile_changes")
            .fetch_one(state.database.pool())
            .await
            .unwrap_or(0);

    let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts")
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

    let profiles_with_embeddings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM profile_embeddings WHERE profile_embedding IS NOT NULL",
    )
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    let casts_with_embeddings =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cast_embeddings")
            .fetch_one(state.database.pool())
            .await
            .unwrap_or(0);

    // Get cache statistics
    let cache_stats = state.cache_service.get_stats().await;
    let cache_info = state.cache_service.get_cache_info().await;

    Ok(Json(ApiResponse::success(StatsResponse {
        total_profiles,
        total_casts,
        profiles_with_embeddings,
        casts_with_embeddings,
        cache_stats: Some(crate::api::types::CacheStatsResponse {
            hits: cache_stats.hits,
            misses: cache_stats.misses,
            hit_rate: cache_stats.hit_rate(),
            evictions: cache_stats.evictions,
            expired_cleanups: cache_stats.expired_cleanups,
            profile_entries: cache_info.profile_entries,
            social_entries: cache_info.social_entries,
            mbti_entries: cache_info.mbti_entries,
            total_entries: cache_info.total_entries,
            max_entries: cache_info.max_entries,
            usage_percentage: cache_info.usage_percentage(),
        }),
    })))
}
