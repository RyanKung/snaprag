//! HTTP server implementation

use std::sync::Arc;

use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::handlers::AppState;
use crate::api::mcp;
#[cfg(feature = "payment")]
use crate::api::payment_middleware::smart_payment_middleware;
#[cfg(feature = "payment")]
use crate::api::payment_middleware::PaymentMiddlewareState;
use crate::api::routes;
use crate::config::AppConfig;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::Result;

/// Start the API server
pub async fn serve_api(
    config: &AppConfig,
    host: String,
    port: u16,
    enable_cors: bool,
    #[cfg(feature = "payment")] payment_enabled: bool,
    #[cfg(feature = "payment")] payment_address: Option<String>,
    #[cfg(feature = "payment")] testnet: bool,
) -> Result<()> {
    info!("🚀 Starting SnapRAG API server...");

    // Initialize services
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);
    let llm_service = Arc::new(LlmService::new(config)?);

    let state = AppState {
        database,
        embedding_service,
        llm_service,
    };

    // Build API routes
    let api_router = routes::api_routes(state.clone());
    let mcp_router = mcp::mcp_routes(state.clone());

    // Combine routes with optional payment middleware
    let mut app = Router::new();

    #[cfg(feature = "payment")]
    if payment_enabled {
        let payment_addr = payment_address.ok_or_else(|| {
            crate::SnapRagError::Custom("Payment address required when payment is enabled".into())
        })?;

        info!("💰 Payment enabled");
        info!("📍 Payment address: {}", payment_addr);
        info!(
            "🌐 Network: {}",
            if testnet {
                "base-sepolia (testnet)"
            } else {
                "base (mainnet)"
            }
        );

        // Create payment middleware state
        let payment_state = PaymentMiddlewareState::new(payment_addr.clone(), testnet);

        // Apply payment middleware to API routes
        let protected_api = api_router.layer(axum::middleware::from_fn_with_state(
            payment_state.clone(),
            smart_payment_middleware,
        ));

        // MCP routes also protected
        let protected_mcp = mcp_router.layer(axum::middleware::from_fn_with_state(
            payment_state,
            smart_payment_middleware,
        ));

        app = Router::new()
            .nest("/api", protected_api)
            .nest("/mcp", protected_mcp);

        info!("🔒 Payment middleware applied to /api and /mcp routes");
    } else {
        app = Router::new()
            .nest("/api", api_router)
            .nest("/mcp", mcp_router);
        info!("💡 Payment disabled - all endpoints are free");
    }

    #[cfg(not(feature = "payment"))]
    {
        app = Router::new()
            .nest("/api", api_router)
            .nest("/mcp", mcp_router);
        info!("💡 Payment feature not compiled - all endpoints are free");
    }

    // Add middleware layers
    app = app
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());

    // Add CORS if enabled
    if enable_cors {
        info!("✅ CORS enabled");
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        app = app.layer(cors);
    }

    // Start server
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("🌐 API server listening on http://{}", addr);
    info!("📋 RESTful API available at http://{}/api", addr);
    info!("🔌 MCP service available at http://{}/mcp", addr);
    info!("");

    #[cfg(feature = "payment")]
    if payment_enabled {
        info!("💰 Payment Information:");
        info!("  Free:     /api/health, /api/stats");
        info!("  $0.001:   /api/profiles");
        info!("  $0.01:    /api/search/*");
        info!("  $0.1:     /api/rag/query");
        info!("");
    }

    info!("Available endpoints:");
    info!("  GET  /api/health         - Health check");
    info!("  GET  /api/profiles       - List profiles");
    info!("  GET  /api/profiles/:fid  - Get profile by FID");
    info!("  POST /api/search/profiles - Search profiles");
    info!("  POST /api/search/casts   - Search casts");
    info!("  POST /api/rag/query      - RAG query");
    info!("  GET  /api/stats          - Statistics");
    info!("");
    info!("  GET  /mcp/               - MCP server info");
    info!("  GET  /mcp/resources      - List MCP resources");
    info!("  GET  /mcp/tools          - List MCP tools");
    info!("  POST /mcp/tools/call     - Call MCP tool");

    axum::serve(listener, app).await?;

    Ok(())
}
