//! API request and response types

use serde::Deserialize;
use serde::Serialize;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub threshold: Option<f32>,
}

fn default_limit() -> usize {
    20
}

/// Profile search request
#[derive(Debug, Deserialize)]
pub struct ProfileSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub method: Option<String>,
}

/// Cast search request
#[derive(Debug, Deserialize)]
pub struct CastSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

fn default_threshold() -> f32 {
    0.5
}

/// RAG query request
#[derive(Debug, Deserialize)]
pub struct RagQueryRequest {
    pub question: String,
    #[serde(default = "default_rag_limit")]
    pub retrieval_limit: usize,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_rag_limit() -> usize {
    10
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    2000
}

/// Profile response
#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pfp_url: Option<String>,
    pub location: Option<String>,
    pub twitter_username: Option<String>,
    pub github_username: Option<String>,
}

/// Cast response
#[derive(Debug, Serialize)]
pub struct CastResponse {
    pub message_hash: String,
    pub fid: i64,
    pub text: String,
    pub timestamp: i64,
    pub similarity: Option<f32>,
}

/// Statistics response
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_profiles: i64,
    pub total_casts: i64,
    pub profiles_with_embeddings: i64,
    pub casts_with_embeddings: i64,
}
