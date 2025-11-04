/// RAG-related API handlers
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use super::AppState;
use crate::api::types::ApiResponse;
use crate::api::types::RagQueryRequest;
use crate::rag::RagQuery;
use crate::rag::RagService;
use crate::rag::RetrievalMethod;

/// RAG query
pub async fn rag_query(
    State(state): State<AppState>,
    Json(req): Json<RagQueryRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("POST /api/rag/query: {}", req.question);

    let llm_service = if let Some(llm) = &state.llm_service {
        llm.clone()
    } else {
        error!("LLM service not configured");
        return Ok(Json(ApiResponse::error(
            "LLM service not configured. Please check your configuration.".to_string(),
        )));
    };

    let rag_service = RagService::from_services(
        state.database.clone(),
        state.embedding_service.clone(),
        (*llm_service).clone(),
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
