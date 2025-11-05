//! Argument and identifier parsing for ask command
use crate::database::Database;
use crate::Result;

/// Parse user identifier (FID or @username) to FID
pub async fn parse_user_identifier(identifier: &str, database: &Database) -> Result<u64> {
    let trimmed = identifier.trim();
    if trimmed.starts_with('@') {
        let username = trimmed.trim_start_matches('@');
        crate::cli::output::print_info(&format!("🔍 Looking up username: @{username}"));
        let profile = database
            .get_user_profile_by_username(username)
            .await?
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!("Username @{username} not found in database"))
            })?;
        println!("   ✅ Found FID: {}", profile.fid);
        #[allow(clippy::cast_sign_loss)] // FID is guaranteed positive in database
        Ok(profile.fid as u64)
    } else {
        trimmed.parse::<u64>().map_err(|_| {
            crate::SnapRagError::Custom(format!(
                "Invalid user identifier {identifier}. Use FID (e.g., 99) or username (e.g., @jesse.base.eth)"
            ))
        })
    }
}
