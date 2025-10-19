use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirements {
    pub scheme: String,
    pub network: String,
    pub max_amount_required: String,
    pub asset: String,
    pub pay_to: String,
    pub resource: String,
    pub description: String,
    pub mime_type: Option<String>,
    pub max_timeout_seconds: Option<u64>,
    pub extra: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirementsResponse {
    pub x402_version: u32,
    pub error: String,
    pub accepts: Vec<PaymentRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentPayload {
    pub version: String,
    pub scheme: String,
    pub network: String,
    pub payer: String,
    pub payee: String,
    pub asset: String,
    pub amount: String,
    pub nonce: String,
    pub timestamp: u64,
    pub signature: String,
    pub resource: String,
    pub extra: Option<serde_json::Value>,
}

impl PaymentPayload {
    pub fn to_base64(&self) -> Result<String, String> {
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize payment: {}", e))?;
        Ok(base64::encode(json.as_bytes()))
    }
}

/// Create EIP-712 typed data for x402 payment
pub fn create_eip712_typed_data(
    requirements: &PaymentRequirements,
    payer: &str,
    nonce: &str,
    timestamp: u64,
) -> Result<String, String> {
    // Determine chain ID from network
    let chain_id = match requirements.network.as_str() {
        "base-sepolia" => 84532,
        "base-mainnet" | "base" => 8453,
        "ethereum-mainnet" | "ethereum" => 1,
        "ethereum-sepolia" | "sepolia" => 11155111,
        _ => return Err(format!("Unsupported network: {}", requirements.network)),
    };

    // Normalize addresses (lowercase, with 0x prefix)
    let payer_normalized = normalize_address(payer);
    let payee_normalized = normalize_address(&requirements.pay_to);
    let asset_normalized = normalize_address(&requirements.asset);

    let typed_data = serde_json::json!({
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"}
            ],
            "Payment": [
                {"name": "payer", "type": "address"},
                {"name": "payee", "type": "address"},
                {"name": "asset", "type": "address"},
                {"name": "amount", "type": "uint256"},
                {"name": "nonce", "type": "string"},
                {"name": "timestamp", "type": "uint256"},
                {"name": "resource", "type": "string"}
            ]
        },
        "primaryType": "Payment",
        "domain": {
            "name": "x402-payment",
            "version": "1",
            "chainId": chain_id
        },
        "message": {
            "payer": payer_normalized,
            "payee": payee_normalized,
            "asset": asset_normalized,
            "amount": requirements.max_amount_required,
            "nonce": nonce,
            "timestamp": timestamp,
            "resource": requirements.resource
        }
    });

    serde_json::to_string(&typed_data)
        .map_err(|e| format!("Failed to serialize typed data: {}", e))
}

/// Normalize Ethereum address (lowercase with 0x prefix)
fn normalize_address(addr: &str) -> String {
    let addr = addr.trim();
    if addr.starts_with("0x") || addr.starts_with("0X") {
        format!("0x{}", addr[2..].to_lowercase())
    } else {
        format!("0x{}", addr.to_lowercase())
    }
}

/// Generate a random nonce
pub fn generate_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    // Use timestamp + random number for nonce
    let random = (js_sys::Math::random() * 1_000_000.0) as u64;
    format!("{}-{}", timestamp, random)
}

/// Get current Unix timestamp in seconds
pub fn get_timestamp() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Create payment payload from requirements and signature
pub fn create_payment_payload(
    requirements: &PaymentRequirements,
    payer: &str,
    signature: &str,
    nonce: &str,
    timestamp: u64,
) -> PaymentPayload {
    PaymentPayload {
        version: "1".to_string(),
        scheme: requirements.scheme.clone(),
        network: requirements.network.clone(),
        payer: normalize_address(payer),
        payee: normalize_address(&requirements.pay_to),
        asset: normalize_address(&requirements.asset),
        amount: requirements.max_amount_required.clone(),
        nonce: nonce.to_string(),
        timestamp,
        signature: signature.to_string(),
        resource: requirements.resource.clone(),
        extra: requirements.extra.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_address() {
        assert_eq!(
            normalize_address("0xABCD1234"),
            "0xabcd1234"
        );
        assert_eq!(
            normalize_address("ABCD1234"),
            "0xabcd1234"
        );
        assert_eq!(
            normalize_address("0X1234"),
            "0x1234"
        );
    }
}

