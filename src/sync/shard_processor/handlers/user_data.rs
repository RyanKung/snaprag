/// UserDataAdd message handler

use crate::Result;

use super::super::types::BatchedData;

/// Handle UserDataAdd message (type 11) - all 13 profile field types
pub(super) fn handle_user_data_add(
    body: &serde_json::Value,
    fid: i64,
    timestamp: i64,
    message_hash: &[u8],
    batched: &mut BatchedData,
) {
    if let Some(user_data_body) = body.get("user_data_body") {
        let data_type = user_data_body
            .get("type")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let value = user_data_body
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Map UserDataType to field names
        let field_name = match data_type {
            1 => Some("pfp_url"),
            2 => Some("display_name"),
            3 => Some("bio"),
            5 => Some("website_url"),
            6 => Some("username"),
            7 => Some("location"),
            8 => Some("twitter_username"),
            9 => Some("github_username"),
            10 => Some("banner_url"),
            11 => Some("primary_address_ethereum"),
            12 => Some("primary_address_solana"),
            13 => Some("profile_token"),
            _ => {
                tracing::warn!("Unknown UserDataType {} for FID {}", data_type, fid);
                None
            }
        };

        if let Some(field) = field_name {
            batched.profile_updates.push((
                fid,
                field.to_string(),
                Some(value.to_string()),
                timestamp,
                message_hash.to_vec(),
            ));

            let display_value = if value.len() > 50 {
                format!("{}...", &value[..50])
            } else {
                value.to_string()
            };
            
            tracing::debug!(
                "Collected profile update: FID {} field {} = {}",
                fid,
                field,
                display_value
            );
        }
    }
}

