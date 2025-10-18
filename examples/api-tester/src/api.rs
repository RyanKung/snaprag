use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub name: String,
    pub description: String,
    pub tier: String,
    pub requires_payment: bool,
    pub default_body: Option<String>,
}

pub fn get_endpoints() -> Vec<EndpointInfo> {
    vec![
        EndpointInfo {
            path: "/api/health".to_string(),
            method: "GET".to_string(),
            name: "Health Check".to_string(),
            description: "Check API server health status".to_string(),
            tier: "Free".to_string(),
            requires_payment: false,
            default_body: None,
        },
        EndpointInfo {
            path: "/api/stats".to_string(),
            method: "GET".to_string(),
            name: "Statistics".to_string(),
            description: "Get database statistics (profiles, casts, links)".to_string(),
            tier: "Free".to_string(),
            requires_payment: false,
            default_body: None,
        },
        EndpointInfo {
            path: "/api/profiles?q=&limit=10".to_string(),
            method: "GET".to_string(),
            name: "List Profiles".to_string(),
            description: "Search and list user profiles".to_string(),
            tier: "Basic".to_string(),
            requires_payment: true,
            default_body: None,
        },
        EndpointInfo {
            path: "/api/profiles/3".to_string(),
            method: "GET".to_string(),
            name: "Get Profile".to_string(),
            description: "Get specific user profile by FID".to_string(),
            tier: "Basic".to_string(),
            requires_payment: true,
            default_body: None,
        },
        EndpointInfo {
            path: "/api/search/profiles".to_string(),
            method: "POST".to_string(),
            name: "Search Profiles".to_string(),
            description: "Semantic search for profiles".to_string(),
            tier: "Premium".to_string(),
            requires_payment: true,
            default_body: Some(r#"{
  "query": "AI developers",
  "limit": 10
}"#
            .to_string()),
        },
        EndpointInfo {
            path: "/api/search/casts".to_string(),
            method: "POST".to_string(),
            name: "Search Casts".to_string(),
            description: "Semantic search for casts".to_string(),
            tier: "Premium".to_string(),
            requires_payment: true,
            default_body: Some(r#"{
  "query": "rust programming",
  "limit": 10
}"#
            .to_string()),
        },
        EndpointInfo {
            path: "/api/rag/query".to_string(),
            method: "POST".to_string(),
            name: "RAG Query".to_string(),
            description: "AI-powered RAG query with context".to_string(),
            tier: "Enterprise".to_string(),
            requires_payment: true,
            default_body: Some(r#"{
  "question": "Who are the most active developers in the Farcaster community?"
}"#
            .to_string()),
        },
        EndpointInfo {
            path: "/mcp/".to_string(),
            method: "GET".to_string(),
            name: "MCP Server Info".to_string(),
            description: "Get MCP server information".to_string(),
            tier: "Free".to_string(),
            requires_payment: false,
            default_body: None,
        },
        EndpointInfo {
            path: "/mcp/resources".to_string(),
            method: "GET".to_string(),
            name: "MCP Resources".to_string(),
            description: "List available MCP resources".to_string(),
            tier: "Free".to_string(),
            requires_payment: false,
            default_body: None,
        },
        EndpointInfo {
            path: "/mcp/tools".to_string(),
            method: "GET".to_string(),
            name: "MCP Tools".to_string(),
            description: "List available MCP tools".to_string(),
            tier: "Free".to_string(),
            requires_payment: false,
            default_body: None,
        },
        EndpointInfo {
            path: "/mcp/tools/call".to_string(),
            method: "POST".to_string(),
            name: "MCP Tool Call".to_string(),
            description: "Call an MCP tool".to_string(),
            tier: "Premium".to_string(),
            requires_payment: true,
            default_body: Some(r#"{
  "name": "search_profiles",
  "arguments": {
    "query": "rust developers"
  }
}"#
            .to_string()),
        },
    ]
}

pub async fn make_request(
    base_url: &str,
    endpoint: &EndpointInfo,
    body: Option<String>,
    payment_header: Option<String>,
) -> Result<ApiResponse, String> {
    let url = format!("{}{}", base_url, endpoint.path);

    let mut opts = RequestInit::new();
    opts.method(&endpoint.method);
    opts.mode(RequestMode::Cors);

    // Add body for POST requests
    if endpoint.method == "POST" {
        if let Some(body_str) = body {
            opts.set_body(&wasm_bindgen::JsValue::from_str(&body_str));
        }
    }

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;

    // Set headers
    let headers = request.headers();
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| format!("Failed to set Content-Type: {:?}", e))?;

    // Add payment header if provided
    if let Some(payment) = payment_header {
        headers
            .set("X-PAYMENT", &payment)
            .map_err(|e| format!("Failed to set X-PAYMENT header: {:?}", e))?;
    }

    let window = web_sys::window().ok_or("No window object")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    let status = resp.status();
    let status_text = resp.status_text();

    // Get response headers
    let mut response_headers = Vec::new();
    let headers_iter = resp.headers().entries();
    
    if let Some(iterator) = js_sys::try_iter(&headers_iter).ok().flatten() {
        for entry in iterator {
            if let Ok(entry) = entry {
                if let Ok(array) = entry.dyn_into::<js_sys::Array>() {
                    if array.length() == 2 {
                        let key = array.get(0).as_string().unwrap_or_default();
                        let value = array.get(1).as_string().unwrap_or_default();
                        response_headers.push((key, value));
                    }
                }
            }
        }
    }

    // Get response body
    let text = JsFuture::from(resp.text().map_err(|e| format!("No text method: {:?}", e))?)
        .await
        .map_err(|e| format!("Failed to get text: {:?}", e))?;

    let body = text.as_string().unwrap_or_default();

    Ok(ApiResponse {
        status,
        status_text,
        headers: response_headers,
        body,
    })
}

