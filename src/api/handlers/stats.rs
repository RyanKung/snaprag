/// Stats-related API handlers
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::info;

use super::AppState;
use crate::api::types::*;

/// Get stats
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
