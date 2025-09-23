pub mod config;
pub mod database;
pub mod errors;
pub mod generated;
pub mod grpc_client;
pub mod logging;
pub mod models;
pub mod sync;

/// Farcaster epoch constant (January 1, 2021 UTC in milliseconds)
pub const FARCASTER_EPOCH: u64 = 1609459200000;

/// Convert Farcaster timestamp (seconds since Farcaster epoch) to Unix timestamp (seconds since Unix epoch)
pub fn farcaster_to_unix_timestamp(farcaster_timestamp: u64) -> u64 {
    farcaster_timestamp + (FARCASTER_EPOCH / 1000)
}

/// Convert Unix timestamp (seconds since Unix epoch) to Farcaster timestamp (seconds since Farcaster epoch)
pub fn unix_to_farcaster_timestamp(unix_timestamp: u64) -> u64 {
    unix_timestamp - (FARCASTER_EPOCH / 1000)
}

#[cfg(test)]
pub mod tests;

pub use config::AppConfig;
pub use errors::*;
