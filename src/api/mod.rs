//! API server module for serving read-only services via REST and MCP

pub mod backend_api_key;
pub mod cache;
pub mod cache_proxy;
pub mod handlers;
pub mod mcp;
#[cfg(feature = "payment")]
pub mod payment_middleware;
pub mod pricing;
pub mod redis_client;
pub mod routes;
pub mod server;
pub mod session;
pub mod types;

pub use server::serve_api;
