//! Payment functionality tests

#![cfg(feature = "payment")]

use reqwest::Client;
use serde_json::json;

const API_URL: &str = "http://127.0.0.1:3000";

#[tokio::test]
#[ignore] // Run with: cargo test --features payment --test payment_tests -- --ignored --nocapture
async fn test_payment_required_response() {
    println!("\n🧪 Testing payment required responses...");
    println!("==========================================\n");

    let client = Client::new();

    // Test each pricing tier
    let test_cases = vec![
        ("/api/profiles", "Basic", 1_000u64),
        ("/api/search/profiles", "Premium", 10_000u64),
        ("/api/rag/query", "Enterprise", 100_000u64),
    ];

    for (endpoint, tier, expected_amount) in test_cases {
        println!("Testing {} tier: {}", tier, endpoint);

        let response = if endpoint.contains("search") || endpoint.contains("rag") {
            client
                .post(format!("{}{}", API_URL, endpoint))
                .json(&json!({"query": "test", "question": "test"}))
                .send()
                .await
                .expect("Failed to send request")
        } else {
            client
                .get(format!("{}{}?q=&limit=1", API_URL, endpoint))
                .send()
                .await
                .expect("Failed to send request")
        };

        assert_eq!(response.status(), 402, "{} should return 402", endpoint);

        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");

        // Verify response structure
        assert_eq!(body["x402Version"], 1);
        assert_eq!(body["error"], "Payment required");
        assert!(body["accepts"].is_array());

        // Verify payment requirements
        let requirements = &body["accepts"][0];
        assert_eq!(requirements["scheme"], "exact");
        assert_eq!(requirements["network"], "base-sepolia");

        let amount: String = requirements["maxAmountRequired"]
            .as_str()
            .expect("maxAmountRequired should be string")
            .to_string();
        assert_eq!(
            amount.parse::<u64>().unwrap(),
            expected_amount,
            "{} should require {} atomic units",
            endpoint,
            expected_amount
        );

        println!("✅ {} tier: {} (amount: {})", tier, endpoint, amount);
    }

    println!("\n✅ All payment required tests passed!\n");
}

#[tokio::test]
#[ignore]
async fn test_free_endpoints_with_payment_enabled() {
    println!("\n🧪 Testing free endpoints still work with payment enabled...");
    println!("=============================================================\n");

    let client = Client::new();

    // Test free endpoints
    let free_endpoints = vec!["/api/health", "/api/stats"];

    for endpoint in free_endpoints {
        let response = client
            .get(format!("{}{}", API_URL, endpoint))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(
            response.status(),
            200,
            "{} should return 200 (free)",
            endpoint
        );

        let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
        assert_eq!(body["success"], true);

        println!("✅ {} is free (200 OK)", endpoint);
    }

    println!("\n✅ All free endpoints work correctly!\n");
}

#[tokio::test]
#[ignore]
async fn test_payment_requirements_format() {
    println!("\n🧪 Testing PaymentRequirements format compliance...");
    println!("===================================================\n");

    let client = Client::new();
    let response = client
        .post(format!("{}/api/rag/query", API_URL))
        .json(&json!({"question": "test"}))
        .send()
        .await
        .expect("Failed to send request");

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    let req = &body["accepts"][0];

    // Verify all required fields
    let required_fields = vec![
        "scheme",
        "network",
        "maxAmountRequired",
        "asset",
        "payTo",
        "resource",
        "description",
        "mimeType",
        "maxTimeoutSeconds",
        "extra",
    ];

    for field in required_fields {
        assert!(
            req.get(field).is_some(),
            "Missing required field: {}",
            field
        );
    }

    // Verify extra field structure (USDC info)
    let extra = &req["extra"];
    assert_eq!(extra["name"], "USDC");
    assert_eq!(extra["version"], "2");

    println!("✅ PaymentRequirements format is correct");
    println!("   Scheme: {}", req["scheme"]);
    println!("   Network: {}", req["network"]);
    println!("   Asset: {}", req["asset"]);
    println!("   Amount: {}", req["maxAmountRequired"]);
    println!("\n✅ Format compliance test passed!\n");
}
