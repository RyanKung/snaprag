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
    println!("📍 Host: {}", host);
    println!("🔌 Port: {}", port);
    println!("🌐 CORS: {}", if cors { "Enabled" } else { "Disabled" });

    #[cfg(feature = "payment")]
    let testnet_final = testnet.unwrap_or(config.x402.use_testnet);

    #[cfg(feature = "payment")]
    if payment {
        println!("💰 Payment: ENABLED");
        if let Some(addr) = &payment_address {
            println!("📍 Payment Address: {}", addr);
        }
        println!(
            "🌐 Network: {}",
            if testnet_final {
                "base-sepolia (testnet)"
            } else {
                "base (mainnet)"
            }
        );
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
        payment,
        #[cfg(feature = "payment")]
        payment_address,
        #[cfg(feature = "payment")]
        testnet_final,
    )
    .await?;

    Ok(())
}
