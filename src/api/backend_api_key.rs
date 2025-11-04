use axum::body::Body;
use axum::http::Request;
use axum::http::Response;
use axum::middleware::Next;

#[derive(Clone)]
pub struct ApiKeyState {
    pub expected_key: String,
}

/// Backend API key authentication middleware
///
/// # Panics
/// Panics if the response builder fails to create an UNAUTHORIZED response (extremely unlikely)
pub async fn backend_api_key_middleware(
    state: axum::extract::State<ApiKeyState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Response<Body>> {
    let header = request.headers().get("X-API-KEY");
    match header.and_then(|h| h.to_str().ok()) {
        Some(k) if k == state.expected_key => Ok(next.run(request).await),
        _ => Err(Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body(Body::from("Unauthorized"))
            .unwrap()),
    }
}
