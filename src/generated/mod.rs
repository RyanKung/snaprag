//! Generated protobuf code
//!
//! This module contains the generated Rust code from the protobuf definitions.
//! The code is generated during the build process by the build.rs script.

// Allow warnings from generated protobuf code
#![allow(unused_lifetimes)]
#![allow(clippy::all)]

// Include the generated protobuf code
// Each proto file generates its own .rs file
pub mod admin_rpc;
pub mod blocks;
pub mod gossip;
pub mod hub_event;
pub mod message;
pub mod node_state;
pub mod onchain_event;
pub mod replication;
pub mod request_response;
pub mod rpc;
pub mod sync_trie;
pub mod username_proof;

// Include the generated gRPC client code
// The gRPC client code is generated in the src/generated directory
pub mod grpc_client {
    include!("_.rs");
}

// Re-export the gRPC client
pub use grpc_client::hub_service_client;
