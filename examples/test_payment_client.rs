//! x402 Payment Test Client
//!
//! Test client for making payments to `SnapRAG` API endpoints.
//!
//! Usage:
//! ```bash
//! # Set environment variables
//! export X402_PRIVATE_KEY="0x..."
//! export X402_PAYER_ADDRESS="0x..."
//!
//! # Run test
//! cargo run --features payment --example test_payment_client
//! ```

#[cfg(not(feature = "payment"))]
fn main() {
    println!("❌ This example requires the 'payment' feature to be enabled.");
    println!("Run with: cargo run --features payment --example test_payment_client");
    std::process::exit(1);
}

#[cfg(feature = "payment")]
use reqwest::Client;
#[cfg(feature = "payment")]
use rust_x402::client::X402Client;
#[cfg(feature = "payment")]
use rust_x402::types::PaymentPayload;
#[cfg(feature = "payment")]
use rust_x402::types::PaymentRequirements;
#[cfg(feature = "payment")]
use rust_x402::types::PaymentRequirementsResponse;
#[cfg(feature = "payment")]
use rust_x402::wallet::WalletFactory;

#[cfg(feature = "payment")]
const API_URL: &str = "http://127.0.0.1:3000";

#[cfg(feature = "payment")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("\n🧪 SnapRAG x402 Payment Test Client");
    println!("====================================\n");

    // Get wallet credentials from environment
    let private_key = std::env::var("X402_PRIVATE_KEY").expect("X402_PRIVATE_KEY not set");
    let payer_address = std::env::var("X402_PAYER_ADDRESS").expect("X402_PAYER_ADDRESS not set");

    println!("👤 Payer Address: {payer_address}\n");

    // Test 1: Free endpoints
    println!("═══════════════════════════════════════════════");
    println!("TEST 1: Free Endpoints (No Payment Required)");
    println!("═══════════════════════════════════════════════\n");

    test_free_endpoints().await?;

    // Test 2: Paid endpoint without payment
    println!("\n═══════════════════════════════════════════════");
    println!("TEST 2: Paid Endpoint Without Payment");
    println!("═══════════════════════════════════════════════\n");

    let requirements = test_payment_required().await?;

    // Test 3: Create signed payment
    println!("\n═══════════════════════════════════════════════");
    println!("TEST 3: Create Signed Payment Payload");
    println!("═══════════════════════════════════════════════\n");

    let payment_payload = create_signed_payment(&requirements, &private_key, &payer_address)?;

    // Test 4: Send payment
    println!("\n═══════════════════════════════════════════════");
    println!("TEST 4: Send Request With Payment");
    println!("═══════════════════════════════════════════════\n");

    test_with_payment(&payment_payload).await?;

    println!("\n═══════════════════════════════════════════════");
    println!("🎉 All Tests Passed!");
    println!("═══════════════════════════════════════════════\n");

    Ok(())
}

#[cfg(feature = "payment")]
async fn test_free_endpoints() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client.get(format!("{API_URL}/api/health")).send().await?;

    println!("GET /api/health");
    println!("Status: {}", response.status());
    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json().await?;
    println!("Response: {}", body["data"]["status"]);
    println!("✅ Free endpoint works\n");

    Ok(())
}

#[cfg(feature = "payment")]
async fn test_payment_required() -> Result<PaymentRequirements, Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .post(format!("{API_URL}/api/rag/query"))
        .json(&serde_json::json!({"question": "Who are AI developers?"}))
        .send()
        .await?;

    println!("POST /api/rag/query (no payment)");
    println!("Status: {}", response.status());
    assert_eq!(response.status(), 402);

    let body: PaymentRequirementsResponse = response.json().await?;
    println!("✅ Correctly returned 402 Payment Required\n");

    let requirements = body
        .accepts
        .first()
        .ok_or("No payment requirements")?
        .clone();

    println!("Payment Requirements:");
    println!("  Scheme: {}", requirements.scheme);
    println!("  Network: {}", requirements.network);
    println!(
        "  Amount: {} atomic units",
        requirements.max_amount_required
    );
    println!("  Pay To: {}", requirements.pay_to);
    println!("  Asset: {}", requirements.asset);
    println!("  Description: {}", requirements.description);

    Ok(requirements)
}

#[cfg(feature = "payment")]
fn create_signed_payment(
    requirements: &PaymentRequirements,
    private_key: &str,
    payer_address: &str,
) -> Result<PaymentPayload, Box<dyn std::error::Error>> {
    println!("Creating signed payment payload...");
    println!("  Network: {}", requirements.network);
    println!("  Amount: {}", requirements.max_amount_required);

    // Create wallet
    let wallet = WalletFactory::from_private_key(private_key, &requirements.network)?;

    // Create signed payment payload
    let payment_payload = wallet.create_signed_payment_payload(requirements, payer_address)?;

    println!("✅ Payment payload created with EIP-712 signature");
    println!("  Payer: {payer_address}");
    println!("  Payee: {}", requirements.pay_to);

    Ok(payment_payload)
}

#[cfg(feature = "payment")]
async fn test_with_payment(
    payment_payload: &PaymentPayload,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Encode payment payload
    let payment_header = payment_payload.to_base64()?;
    println!("Payment Header (Base64): {}...", &payment_header[..60]);

    // Send request with payment
    let response = client
        .post(format!("{API_URL}/api/rag/query"))
        .header("X-PAYMENT", payment_header)
        .json(&serde_json::json!({"question": "Who are AI developers?"}))
        .send()
        .await?;

    println!("\nPOST /api/rag/query (with payment)");
    println!("Status: {}", response.status());

    if response.status() == 200 {
        println!("✅ Payment accepted!\n");

        // Check for settlement response
        if let Some(settlement_header) = response.headers().get("X-PAYMENT-RESPONSE") {
            let settlement_b64 = settlement_header.to_str()?;
            let settlement_json = base64::decode(settlement_b64)?;
            let settlement: serde_json::Value = serde_json::from_slice(&settlement_json)?;

            println!("💰 Payment Settlement:");
            println!("  Success: {}", settlement["success"]);
            println!("  Transaction: {}", settlement["transaction"]);
            println!("  Network: {}", settlement["network"]);

            if let Some(tx_hash) = settlement["transaction"].as_str() {
                println!("  View: https://sepolia.basescan.org/tx/{tx_hash}");
            }
        } else {
            println!("⚠️  No X-PAYMENT-RESPONSE header (settlement may be pending)");
        }

        // Get response body
        let body: serde_json::Value = response.json().await?;
        if let Some(answer) = body["data"].as_str() {
            println!("\n📄 Response:");
            println!("  {}", &answer[..200.min(answer.len())]);
            if answer.len() > 200 {
                println!("  ...");
            }
        }

        Ok(())
    } else {
        println!("❌ Payment failed: {}", response.status());
        let body = response.text().await?;
        println!("Response: {body}");
        Err("Payment verification failed".into())
    }
}
