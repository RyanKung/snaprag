//! API server module for serving read-only services via REST and MCP

pub mod cache;
pub mod handlers;
pub mod mcp;
#[cfg(feature = "payment")]
pub mod payment_middleware;
pub mod pricing;
pub mod routes;
pub mod server;
pub mod session;
pub mod types;

pub use server::serve_api;
