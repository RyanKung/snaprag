//! API server handlers

use std::sync::Arc;

use crate::cli::output::*;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

pub async fn handle_serve_api(
    config: &AppConfig,
    host: String,
    port: u16,
    cors: bool,
    #[cfg(feature = "payment")] payment: bool,
    #[cfg(feature = "payment")] payment_address: Option<String>,
    #[cfg(feature = "payment")] testnet: Option<bool>,
) -> Result<()> {
    use crate::api::serve_api;

    println!("🚀 Starting SnapRAG API Server");
    println!("===============================\n");
    println!("📍 Host: {host}");
    println!("🔌 Port: {port}");
    println!("🌐 CORS: {}", if cors { "Enabled" } else { "Disabled" });

    #[cfg(feature = "payment")]
    // CLI arguments take priority over config
    let testnet_final = testnet.unwrap_or(config.x402.use_testnet);

    #[cfg(feature = "payment")]
    // CLI argument takes priority over config
    let payment_final = payment || config.x402.enabled;

    #[cfg(feature = "payment")]
    // Helper function to normalize Ethereum address
    fn normalize_address(addr: &str) -> String {
        let addr = addr.trim();
        if addr.starts_with("0x") || addr.starts_with("0X") {
            format!("0x{}", addr[2..].to_lowercase())
        } else {
            format!("0x{}", addr.to_lowercase())
        }
    }

    #[cfg(feature = "payment")]
    // Get payment address: prefer CLI argument, fall back to config
    // Read from config even if payment is disabled (for potential future use)
    let payment_address_final = if let Some(addr) = payment_address {
        let normalized = normalize_address(&addr);
        println!("🔧 Using CLI payment address (normalized): {normalized}");
        Some(normalized)
    } else if !config.x402.payment_address.is_empty() {
        let normalized = normalize_address(&config.x402.payment_address);
        // Check if payment.toml exists to show correct source
        let config_source = if std::path::Path::new("payment.toml").exists() {
            "payment.toml"
        } else {
            "config.toml"
        };
        println!("🔧 Using payment address from {config_source} (normalized): {normalized}");
        Some(normalized)
    } else {
        println!("⚠️ No payment address found in CLI or config");
        None
    };

    #[cfg(feature = "payment")]
    if payment_final {
        println!("💰 Payment: ENABLED");
        if let Some(addr) = &payment_address_final {
            println!("📍 Payment Address: {addr}");
        }
        println!(
            "🌐 Network: {}",
            if testnet_final {
                "base-sepolia (testnet)"
            } else {
                "base (mainnet)"
            }
        );
        println!("🔍 Facilitator URL: {}", config.x402.facilitator_url);
        if let Some(rpc) = &config.x402.rpc_url {
            println!("⛓️  RPC URL: {rpc}");
        }
    } else {
        println!("💰 Payment: DISABLED");
    }

    #[cfg(not(feature = "payment"))]
    println!("💡 Payment: Not compiled (use --features payment)");

    println!();

    // Start the API server
    serve_api(
        config,
        host,
        port,
        cors,
        #[cfg(feature = "payment")]
        payment_final,
        #[cfg(feature = "payment")]
        payment_address_final,
        #[cfg(feature = "payment")]
        testnet_final,
    )
    .await?;

    Ok(())
}
