//! x402 payment middleware for SnapRAG API

#[cfg(feature = "payment")]
use axum::body::Body;
#[cfg(feature = "payment")]
use axum::extract::Request;
#[cfg(feature = "payment")]
use axum::http::HeaderValue;
#[cfg(feature = "payment")]
use axum::http::StatusCode;
#[cfg(feature = "payment")]
use axum::middleware::Next;
#[cfg(feature = "payment")]
use axum::response::IntoResponse;
#[cfg(feature = "payment")]
use axum::response::Response;
#[cfg(feature = "payment")]
use axum::Json;
#[cfg(feature = "payment")]
use rust_x402::middleware::PaymentMiddleware as X402Middleware;
#[cfg(feature = "payment")]
use rust_x402::types::PaymentPayload;
#[cfg(feature = "payment")]
use rust_x402::types::PaymentRequirements;

#[cfg(feature = "payment")]
use crate::api::pricing::PricingConfig;

/// Payment middleware state
#[cfg(feature = "payment")]
#[derive(Clone)]
pub struct PaymentMiddlewareState {
    pub pricing: PricingConfig,
    pub payment_address: String,
    pub testnet: bool,
    pub middleware: X402Middleware,
}

#[cfg(feature = "payment")]
impl PaymentMiddlewareState {
    pub fn new(payment_address: String, testnet: bool) -> Self {
        use std::str::FromStr;

        use rust_decimal::Decimal;

        let pricing = PricingConfig::default();

        // Create default middleware (will be customized per route)
        let middleware = X402Middleware::new(Decimal::from_str("0.01").unwrap(), &payment_address);

        Self {
            pricing,
            payment_address,
            testnet,
            middleware,
        }
    }
}

/// Smart payment middleware that applies different pricing based on path
#[cfg(feature = "payment")]
pub async fn smart_payment_middleware(
    state: axum::extract::State<PaymentMiddlewareState>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let path = request.uri().path().to_string();
    let headers = request.headers().clone();

    // Debug: log the path and pricing check
    tracing::info!("🔍 Payment middleware checking path: '{}'", path);
    let price = state.pricing.get_price(&path);
    tracing::info!("💵 Price for '{}': {:?}", path, price);

    // Get price for this endpoint
    match price {
        None => {
            // Free endpoint, pass through
            tracing::info!("✅ Free endpoint accessed: {}", path);
            next.run(request).await
        }
        Some(amount) => {
            // Paid endpoint - check for payment
            tracing::info!("💰 Paid endpoint accessed: {} (price: ${})", path, amount);

            // Check for X-PAYMENT header
            if let Some(payment_header) = headers.get("X-PAYMENT") {
                if let Ok(payment_str) = payment_header.to_str() {
                    // Parse payment payload
                    match PaymentPayload::from_base64(payment_str) {
                        Ok(payment_payload) => {
                            // Create payment requirements
                            let requirements = create_payment_requirements(
                                &path,
                                amount,
                                &state.payment_address,
                                state.testnet,
                                &state.pricing,
                            );

                            // Verify payment
                            match state
                                .middleware
                                .verify_with_requirements(&payment_payload, &requirements)
                                .await
                            {
                                Ok(true) => {
                                    // Payment valid, proceed
                                    tracing::info!("✅ Payment verified for {}", path);
                                    let mut response = next.run(request).await;

                                    // Settle payment after successful response
                                    if let Ok(settlement) = state
                                        .middleware
                                        .settle_with_requirements(&payment_payload, &requirements)
                                        .await
                                    {
                                        if let Ok(settlement_header) = settlement.to_base64() {
                                            if let Ok(header_value) =
                                                HeaderValue::from_str(&settlement_header)
                                            {
                                                response
                                                    .headers_mut()
                                                    .insert("X-PAYMENT-RESPONSE", header_value);
                                            }
                                        }
                                    }

                                    response
                                }
                                Ok(false) => {
                                    // Payment invalid
                                    tracing::warn!("❌ Payment verification failed for {}", path);
                                    return_payment_required(
                                        &path,
                                        amount,
                                        &state.payment_address,
                                        state.testnet,
                                        &state.pricing,
                                    )
                                }
                                Err(e) => {
                                    // Verification error
                                    tracing::error!("Payment verification error: {}", e);
                                    return_payment_required(
                                        &path,
                                        amount,
                                        &state.payment_address,
                                        state.testnet,
                                        &state.pricing,
                                    )
                                }
                            }
                        }
                        Err(e) => {
                            // Invalid payment payload
                            tracing::warn!("Invalid payment payload: {}", e);
                            return_payment_required(
                                &path,
                                amount,
                                &state.payment_address,
                                state.testnet,
                                &state.pricing,
                            )
                        }
                    }
                } else {
                    return_payment_required(
                        &path,
                        amount,
                        &state.payment_address,
                        state.testnet,
                        &state.pricing,
                    )
                }
            } else {
                // No payment header
                tracing::info!("No payment header for paid endpoint: {}", path);
                return_payment_required(
                    &path,
                    amount,
                    &state.payment_address,
                    state.testnet,
                    &state.pricing,
                )
            }
        }
    }
}

/// Create payment requirements for a resource
#[cfg(feature = "payment")]
fn create_payment_requirements(
    path: &str,
    amount: rust_decimal::Decimal,
    payment_address: &str,
    testnet: bool,
    pricing: &PricingConfig,
) -> PaymentRequirements {
    use rust_x402::types::networks;
    use rust_x402::types::schemes;

    let network = if testnet {
        networks::BASE_SEPOLIA
    } else {
        networks::BASE_MAINNET
    };

    let asset = networks::get_usdc_address(network).unwrap_or_else(|| {
        tracing::error!(
            "Unsupported network: {}, using BASE_SEPOLIA default",
            network
        );
        networks::get_usdc_address(networks::BASE_SEPOLIA)
            .expect("BASE_SEPOLIA should always be supported")
    });

    // Convert amount to atomic units (USDC has 6 decimals)
    use rust_decimal::prelude::*;
    let multiplier = Decimal::from(1_000_000);
    let amount_in_atomic = amount * multiplier;

    // Round to remove fractional parts
    let amount_atomic = amount_in_atomic.round().to_string();

    // Remove decimal point if present
    let amount_atomic = amount_atomic
        .split('.')
        .next()
        .unwrap_or(&amount_atomic)
        .to_string();

    tracing::debug!(
        "Amount conversion: {} USD -> {} atomic units",
        amount,
        amount_atomic
    );

    let mut requirements = PaymentRequirements::new(
        schemes::EXACT,
        network,
        amount_atomic,
        asset,
        payment_address,
        path,
        pricing.get_description(path),
    );

    requirements.mime_type = Some("application/json".to_string());
    requirements.max_timeout_seconds = 60;

    // Set USDC extra info
    let mut extra = serde_json::Map::new();
    extra.insert("name".to_string(), serde_json::json!("USDC"));
    extra.insert("version".to_string(), serde_json::json!("2"));
    requirements.extra = Some(serde_json::Value::Object(extra));

    requirements
}

/// Return 402 Payment Required response
#[cfg(feature = "payment")]
fn return_payment_required(
    path: &str,
    amount: rust_decimal::Decimal,
    payment_address: &str,
    testnet: bool,
    pricing: &PricingConfig,
) -> Response {
    let requirements = create_payment_requirements(path, amount, payment_address, testnet, pricing);

    let response_body = serde_json::json!({
        "x402Version": 1,
        "error": "Payment required",
        "accepts": vec![requirements],
    });

    (StatusCode::PAYMENT_REQUIRED, Json(response_body)).into_response()
}

#[cfg(not(feature = "payment"))]
/// Stub for when payment feature is disabled
pub struct PaymentMiddlewareState;
