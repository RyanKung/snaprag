//! MCP (Model Context Protocol) server implementation

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use super::handlers::AppState;

/// MCP protocol version
const MCP_VERSION: &str = "1.0";

/// MCP server information
#[derive(Debug, Serialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub capabilities: McpCapabilities,
}

/// MCP capabilities
#[derive(Debug, Serialize)]
pub struct McpCapabilities {
    pub resources: bool,
    pub tools: bool,
    pub prompts: bool,
}

/// MCP resource
#[derive(Debug, Serialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// MCP tool definition
#[derive(Debug, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool call request
#[derive(Debug, Deserialize)]
pub struct McpToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP tool call response
#[derive(Debug, Serialize)]
pub struct McpToolCallResponse {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

/// MCP content
#[derive(Debug, Serialize)]
pub struct McpContent {
    pub r#type: String,
    pub text: String,
}

/// Get MCP server information
async fn get_server_info() -> Json<McpServerInfo> {
    Json(McpServerInfo {
        name: "SnapRAG MCP Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: MCP_VERSION.to_string(),
        capabilities: McpCapabilities {
            resources: true,
            tools: true,
            prompts: true,
        },
    })
}

/// List available resources
async fn list_resources() -> Json<Vec<McpResource>> {
    Json(vec![
        McpResource {
            uri: "snaprag://profiles".to_string(),
            name: "User Profiles".to_string(),
            description: "Farcaster user profiles database".to_string(),
            mime_type: "application/json".to_string(),
        },
        McpResource {
            uri: "snaprag://casts".to_string(),
            name: "Casts".to_string(),
            description: "Farcaster casts (messages) database".to_string(),
            mime_type: "application/json".to_string(),
        },
    ])
}

/// List available tools
async fn list_tools() -> Json<Vec<McpTool>> {
    Json(vec![
        McpTool {
            name: "search_profiles".to_string(),
            description: "Search for Farcaster profiles using semantic search".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 20
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "search_casts".to_string(),
            description: "Search for casts using semantic search".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results",
                        "default": 20
                    },
                    "threshold": {
                        "type": "number",
                        "description": "Similarity threshold (0.0-1.0)",
                        "default": 0.5
                    }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "get_profile".to_string(),
            description: "Get a user profile by FID".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "fid": {
                        "type": "integer",
                        "description": "Farcaster ID"
                    }
                },
                "required": ["fid"]
            }),
        },
        McpTool {
            name: "rag_query".to_string(),
            description: "Execute a RAG query to get AI-generated answers about Farcaster users"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Question to ask"
                    },
                    "retrieval_limit": {
                        "type": "integer",
                        "description": "Number of profiles to retrieve",
                        "default": 10
                    }
                },
                "required": ["question"]
            }),
        },
    ])
}

/// Call a tool
async fn call_tool(
    State(state): State<AppState>,
    Json(req): Json<McpToolCallRequest>,
) -> Result<Json<McpToolCallResponse>, StatusCode> {
    info!("MCP tool call: {}", req.name);

    match req.name.as_str() {
        "search_profiles" => {
            let query = req
                .arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;
            let limit = req
                .arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;

            // Use the retriever
            use crate::rag::Retriever;
            let retriever = Retriever::new(state.database.clone(), state.embedding_service.clone());

            match retriever.auto_search(query, limit).await {
                Ok(results) => {
                    let text =
                        serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".to_string());
                    Ok(Json(McpToolCallResponse {
                        content: vec![McpContent {
                            r#type: "text".to_string(),
                            text,
                        }],
                        is_error: false,
                    }))
                }
                Err(e) => Ok(Json(McpToolCallResponse {
                    content: vec![McpContent {
                        r#type: "text".to_string(),
                        text: format!("Error: {}", e),
                    }],
                    is_error: true,
                })),
            }
        }
        "get_profile" => {
            let fid = req
                .arguments
                .get("fid")
                .and_then(|v| v.as_i64())
                .ok_or(StatusCode::BAD_REQUEST)?;

            match state.database.get_user_profile(fid).await {
                Ok(Some(profile)) => {
                    let text =
                        serde_json::to_string_pretty(&profile).unwrap_or_else(|_| "{}".to_string());
                    Ok(Json(McpToolCallResponse {
                        content: vec![McpContent {
                            r#type: "text".to_string(),
                            text,
                        }],
                        is_error: false,
                    }))
                }
                Ok(None) => Ok(Json(McpToolCallResponse {
                    content: vec![McpContent {
                        r#type: "text".to_string(),
                        text: "Profile not found".to_string(),
                    }],
                    is_error: true,
                })),
                Err(e) => Ok(Json(McpToolCallResponse {
                    content: vec![McpContent {
                        r#type: "text".to_string(),
                        text: format!("Error: {}", e),
                    }],
                    is_error: true,
                })),
            }
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}

/// Create MCP router
pub fn mcp_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(get_server_info))
        .route("/resources", get(list_resources))
        .route("/tools", get(list_tools))
        .route("/tools/call", post(call_tool))
        .with_state(state)
}
