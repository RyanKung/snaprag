use std::time::Duration;

use axum::body::Body;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::Request;
use axum::http::Response;
use axum::middleware::Next;
use tracing::info;
use tracing::warn;

use crate::api::redis_client::RedisClient;

#[derive(Clone)]
pub struct CacheProxyState {
    pub redis: RedisClient,
    pub upstream_base_url: String,
    pub upstream_api_key: Option<String>,
}

fn cache_key_from_request(req: &Request<Body>) -> String {
    // Key format: GET:/api/.. ?query
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    format!("GET:{path}?{query}")
}

/// Cache proxy middleware for GET requests
///
/// # Errors
/// - Redis connection errors (returns cached response or proxies to upstream)
/// - Upstream HTTP client build errors
/// - Response body conversion errors
///
/// # Panics
/// Panics if the response builder fails to create error responses (extremely unlikely)
pub async fn cache_proxy_middleware(
    state: axum::extract::State<CacheProxyState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Response<Body>> {
    // Only proxy GET; for others, pass through
    if request.method() != axum::http::Method::GET {
        return Ok(next.run(request).await);
    }

    let key = cache_key_from_request(&request);
    let content_type_header = HeaderValue::from_static("application/json");

    // Try Redis first
    match state.redis.get_json(&key).await {
        Ok(Some(cached)) => {
            info!("📦 Cache hit: {}", key);
            let mut resp = Response::new(Body::from(cached));
            resp.headers_mut()
                .insert(axum::http::header::CONTENT_TYPE, content_type_header);
            return Err(resp);
        }
        Ok(None) => {
            info!("🧭 Cache miss: {}, proxying to upstream", key);
        }
        Err(e) => {
            warn!("Redis get error: {} — proxying to upstream", e);
        }
    }

    // Proxy to upstream
    let upstream_url = format!(
        "{}{}{}{}",
        state.upstream_base_url.trim_end_matches('/'),
        request.uri().path(),
        if let Some(q) = request.uri().query() {
            "?"
        } else {
            ""
        },
        request.uri().query().unwrap_or("")
    );

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(60))
        .pool_idle_timeout(Some(Duration::from_secs(120)))
        .pool_max_idle_per_host(8)
        .build()
        .map_err(|e| {
            Response::builder()
                .status(500)
                .body(Body::from(format!("Upstream client build error: {e}")))
                .unwrap()
        })?;

    let mut req_builder = client.get(&upstream_url);
    if let Some(api_key) = &state.upstream_api_key {
        req_builder = req_builder.header("X-API-KEY", api_key);
    }

    // Minimal header forward: accept encoding for compression
    req_builder = req_builder.header("Accept", "application/json");

    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            return Ok(Response::builder()
                .status(axum::http::StatusCode::GATEWAY_TIMEOUT)
                .body(Body::from(format!("Upstream error: {e}")))
                .unwrap());
        }
    };

    let status = upstream_resp.status();
    let headers = upstream_resp.headers().clone();
    let body_bytes = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return Ok(Response::builder()
                .status(axum::http::StatusCode::GATEWAY_TIMEOUT)
                .body(Body::from(format!("Upstream read error: {e}")))
                .unwrap());
        }
    };

    // Cache only 200 OK with application/json
    let is_json = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|v| v.starts_with("application/json"));

    if status.is_success() && is_json {
        if let Err(e) = state
            .redis
            .set_json_with_ttl(&key, &String::from_utf8_lossy(&body_bytes), None)
            .await
        {
            warn!("Redis set error: {}", e);
        }
    }

    let status_http =
        http::StatusCode::from_u16(status.as_u16()).unwrap_or(axum::http::StatusCode::OK);
    let mut resp = Response::builder()
        .status(status_http)
        .body(Body::from(body_bytes))
        .unwrap();
    // set content-type if missing
    if !resp
        .headers()
        .contains_key(axum::http::header::CONTENT_TYPE)
    {
        let mut h = HeaderMap::new();
        h.insert(axum::http::header::CONTENT_TYPE, content_type_header);
        *resp.headers_mut() = h;
    }
    Ok(resp)
}
