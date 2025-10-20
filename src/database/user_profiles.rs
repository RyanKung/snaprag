use super::Database;
use crate::models::*;
use crate::Result;
use crate::SnapRagError;

impl Database {
    /// Upsert a user profile
    pub async fn upsert_user_profile(&self, profile: &UserProfile) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_profiles (
                fid, username, display_name, bio, pfp_url, website_url, 
                last_updated_timestamp, last_updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (fid)
            DO UPDATE SET
                username = EXCLUDED.username,
                display_name = EXCLUDED.display_name,
                bio = EXCLUDED.bio,
                pfp_url = EXCLUDED.pfp_url,
                website_url = EXCLUDED.website_url,
                last_updated_timestamp = EXCLUDED.last_updated_timestamp,
                last_updated_at = EXCLUDED.last_updated_at
            "#,
            profile.fid,
            profile.username,
            profile.display_name,
            profile.bio,
            profile.pfp_url,
            profile.website_url,
            profile.last_updated_timestamp,
            profile.last_updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a new user profile
    pub async fn create_user_profile(
        &self,
        request: CreateUserProfileRequest,
    ) -> Result<UserProfile> {
        let profile = sqlx::query_as::<_, UserProfile>(
            r#"
            INSERT INTO user_profiles (
                fid, username, display_name, bio, pfp_url, banner_url, location,
                website_url, twitter_username, github_username, primary_address_ethereum,
                primary_address_solana, profile_token, last_updated_timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(request.fid)
        .bind(request.username)
        .bind(request.display_name)
        .bind(request.bio)
        .bind(request.pfp_url)
        .bind(request.banner_url)
        .bind(request.location)
        .bind(request.website_url)
        .bind(request.twitter_username)
        .bind(request.github_username)
        .bind(request.primary_address_ethereum)
        .bind(request.primary_address_solana)
        .bind(request.profile_token)
        .bind(request.created_at)
        .fetch_one(&self.pool)
        .await?;

        // Create initial snapshot
        if let Some(message_hash) = request.message_hash {
            self.create_profile_snapshot_from_profile(&profile, message_hash)
                .await?;
        }

        Ok(profile)
    }

    /// Get user profile by FID
    pub async fn get_user_profile(&self, fid: i64) -> Result<Option<UserProfile>> {
        let profile =
            sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profiles WHERE fid = $1")
                .bind(fid)
                .fetch_optional(&self.pool)
                .await?;

        Ok(profile)
    }

    /// Get user profile by username
    pub async fn get_user_profile_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserProfile>> {
        let profile =
            sqlx::query_as::<_, UserProfile>("SELECT * FROM user_profiles WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;

        Ok(profile)
    }

    /// Update user profile field and create snapshot
    pub async fn update_user_profile(
        &self,
        request: UpdateUserProfileRequest,
    ) -> Result<UserProfile> {
        // Get current profile
        let current_profile = self
            .get_user_profile(request.fid)
            .await?
            .ok_or_else(|| SnapRagError::UserNotFound(request.fid as u64))?;

        // Get old value for the specific field
        let old_value = match request.data_type {
            UserDataType::Username => current_profile.username.clone(),
            UserDataType::Display => current_profile.display_name.clone(),
            UserDataType::Bio => current_profile.bio.clone(),
            UserDataType::Pfp => current_profile.pfp_url.clone(),
            UserDataType::Banner => current_profile.banner_url.clone(),
            UserDataType::Location => current_profile.location.clone(),
            UserDataType::Url => current_profile.website_url.clone(),
            UserDataType::Twitter => current_profile.twitter_username.clone(),
            UserDataType::Github => current_profile.github_username.clone(),
            UserDataType::PrimaryAddressEthereum => {
                current_profile.primary_address_ethereum.clone()
            }
            UserDataType::PrimaryAddressSolana => current_profile.primary_address_solana.clone(),
            UserDataType::ProfileToken => current_profile.profile_token.clone(),
            _ => None,
        };

        // Record the change
        self.record_user_data_change(
            request.fid,
            request.data_type as i16,
            old_value,
            request.new_value.clone(),
            request.timestamp,
            request.message_hash.clone(),
        )
        .await?;

        // Update the profile
        let updated_profile = sqlx::query_as::<_, UserProfile>(
            r#"
            UPDATE user_profiles SET
                username = CASE WHEN $2 = 6 THEN $3 ELSE username END,
                display_name = CASE WHEN $2 = 2 THEN $3 ELSE display_name END,
                bio = CASE WHEN $2 = 3 THEN $3 ELSE bio END,
                pfp_url = CASE WHEN $2 = 1 THEN $3 ELSE pfp_url END,
                banner_url = CASE WHEN $2 = 10 THEN $3 ELSE banner_url END,
                location = CASE WHEN $2 = 7 THEN $3 ELSE location END,
                website_url = CASE WHEN $2 = 5 THEN $3 ELSE website_url END,
                twitter_username = CASE WHEN $2 = 8 THEN $3 ELSE twitter_username END,
                github_username = CASE WHEN $2 = 9 THEN $3 ELSE github_username END,
                primary_address_ethereum = CASE WHEN $2 = 11 THEN $3 ELSE primary_address_ethereum END,
                primary_address_solana = CASE WHEN $2 = 12 THEN $3 ELSE primary_address_solana END,
                profile_token = CASE WHEN $2 = 13 THEN $3 ELSE profile_token END,
                last_updated_timestamp = $4,
                last_updated_at = NOW()
            WHERE fid = $1
            RETURNING *
            "#,
        )
        .bind(request.fid)
        .bind(request.data_type as i16)
        .bind(request.new_value)
        .bind(request.timestamp)
        .fetch_one(&self.pool)
        .await?;

        // Create snapshot
        self.create_profile_snapshot_from_profile(&updated_profile, request.message_hash)
            .await?;

        Ok(updated_profile)
    }

    /// Delete user profile (soft delete by setting fields to NULL)
    pub async fn delete_user_profile(
        &self,
        fid: i64,
        message_hash: Vec<u8>,
        timestamp: i64,
    ) -> Result<UserProfile> {
        // Get current profile for snapshot
        let current_profile = self
            .get_user_profile(fid)
            .await?
            .ok_or_else(|| SnapRagError::UserNotFound(fid as u64))?;

        // Record deletion as changes
        let fields_to_clear = [
            (UserDataType::Username, current_profile.username.clone()),
            (UserDataType::Display, current_profile.display_name.clone()),
            (UserDataType::Bio, current_profile.bio.clone()),
            (UserDataType::Pfp, current_profile.pfp_url.clone()),
            (UserDataType::Banner, current_profile.banner_url.clone()),
            (UserDataType::Location, current_profile.location.clone()),
            (UserDataType::Url, current_profile.website_url.clone()),
            (
                UserDataType::Twitter,
                current_profile.twitter_username.clone(),
            ),
            (
                UserDataType::Github,
                current_profile.github_username.clone(),
            ),
            (
                UserDataType::PrimaryAddressEthereum,
                current_profile.primary_address_ethereum.clone(),
            ),
            (
                UserDataType::PrimaryAddressSolana,
                current_profile.primary_address_solana.clone(),
            ),
            (
                UserDataType::ProfileToken,
                current_profile.profile_token.clone(),
            ),
        ];

        for (data_type, old_value) in fields_to_clear.iter() {
            if old_value.is_some() {
                self.record_user_data_change(
                    fid,
                    *data_type as i16,
                    old_value.clone(),
                    String::new(), // Empty string for deletion
                    timestamp,
                    message_hash.clone(),
                )
                .await?;
            }
        }

        // Clear all profile fields
        let deleted_profile = sqlx::query_as::<_, UserProfile>(
            r#"
            UPDATE user_profiles SET
                username = NULL,
                display_name = NULL,
                bio = NULL,
                pfp_url = NULL,
                banner_url = NULL,
                location = NULL,
                website_url = NULL,
                twitter_username = NULL,
                github_username = NULL,
                primary_address_ethereum = NULL,
                primary_address_solana = NULL,
                profile_token = NULL,
                last_updated_timestamp = $2,
                last_updated_at = NOW()
            WHERE fid = $1
            RETURNING *
            "#,
        )
        .bind(fid)
        .bind(timestamp)
        .fetch_one(&self.pool)
        .await?;

        // Create snapshot
        self.create_profile_snapshot_from_profile(&deleted_profile, message_hash)
            .await?;

        Ok(deleted_profile)
    }

    /// List user profiles with filters
    pub async fn list_user_profiles(&self, query: UserProfileQuery) -> Result<Vec<UserProfile>> {
        // Note: Filters are currently applied in the handler layer
        // This function returns all profiles with basic pagination
        // For complex filtering, use semantic_search_profiles or specific query methods

        let limit = query.limit.unwrap_or(100) as i64;
        let offset = query.offset.unwrap_or(0) as i64;

        // If limit is explicitly None, get ALL profiles (use very large limit)
        let effective_limit = if query.limit.is_none() && offset == 0 {
            i64::MAX // No limit - get all
        } else {
            limit
        };

        let profiles = sqlx::query_as::<_, UserProfile>(
            r#"
            SELECT 
                id, fid, username, display_name, bio, pfp_url, banner_url, location,
                website_url, twitter_username, github_username, primary_address_ethereum,
                primary_address_solana, profile_token, profile_embedding, bio_embedding,
                interests_embedding, last_updated_timestamp, last_updated_at,
                shard_id, block_height, transaction_fid
            FROM user_profiles 
            ORDER BY last_updated_at DESC
            LIMIT $1
            OFFSET $2
            "#,
        )
        .bind(effective_limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(profiles)
    }

    /// List FIDs with advanced filtering
    pub async fn list_fids(&self, query: crate::models::FidQuery) -> Result<Vec<UserProfile>> {
        // Returns all profiles with pagination
        // Filters (has_username, has_display_name) are applied in handler layer
        let profiles = sqlx::query_as::<_, UserProfile>(
            r#"
            SELECT 
                id,
                fid,
                username,
                display_name,
                bio,
                pfp_url,
                banner_url,
                location,
                website_url,
                twitter_username,
                github_username,
                banner_url,
                primary_address_ethereum,
                primary_address_solana,
                profile_token,
                profile_embedding,
                bio_embedding,
                interests_embedding,
                last_updated_timestamp,
                last_updated_at,
                shard_id,
                block_height,
                transaction_fid
            FROM user_profiles 
            ORDER BY fid ASC
            LIMIT $1
            OFFSET $2
            "#,
        )
        .bind(query.limit.unwrap_or(100) as i64)
        .bind(query.offset.unwrap_or(0) as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(profiles)
    }

    /// Get statistics
    pub async fn get_statistics(
        &self,
        query: crate::models::StatisticsQuery,
    ) -> Result<crate::models::StatisticsResult> {
        // Get basic counts
        let total_fids = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
            .fetch_one(&self.pool)
            .await?;

        let profiles_with_username = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE username IS NOT NULL AND username != ''",
        )
        .fetch_one(&self.pool)
        .await?;

        // Complete profiles = has username + display_name + bio (more meaningful)
        let complete_profiles = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM user_profiles 
            WHERE username IS NOT NULL AND username != ''
              AND display_name IS NOT NULL AND display_name != ''
              AND bio IS NOT NULL AND bio != ''
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_display_name = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE display_name IS NOT NULL AND display_name != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_bio = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE bio IS NOT NULL AND bio != ''",
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_pfp = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE pfp_url IS NOT NULL AND pfp_url != ''",
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_website = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE website_url IS NOT NULL AND website_url != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_location = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE location IS NOT NULL AND location != ''",
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_twitter = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE twitter_username IS NOT NULL AND twitter_username != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_github = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE github_username IS NOT NULL AND github_username != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_ethereum_address = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE primary_address_ethereum IS NOT NULL AND primary_address_ethereum != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        let profiles_with_solana_address = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_profiles WHERE primary_address_solana IS NOT NULL AND primary_address_solana != ''"
        )
        .fetch_one(&self.pool)
        .await?;

        // Get recent registrations
        let recent_registrations = sqlx::query_as::<_, crate::models::ProfileRegistration>(
            r#"
            SELECT 
                fid,
                username,
                display_name,
                last_updated_at as created_at
            FROM user_profiles 
            ORDER BY last_updated_at DESC 
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Get activity statistics first (needed for username stats)
        let total_activities =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_activity_timeline")
                .fetch_one(&self.pool)
                .await?;

        // Get top usernames with actual counts
        let top_usernames = sqlx::query_as::<_, crate::models::UsernameStats>(
            r#"
            SELECT 
                up.username,
                COUNT(DISTINCT uat.id) as count,
                (COUNT(DISTINCT uat.id) * 100.0 / NULLIF($1, 0))::float8 as percentage
            FROM user_profiles up
            LEFT JOIN user_activity_timeline uat ON up.fid = uat.fid
            WHERE up.username IS NOT NULL AND up.username != ''
            GROUP BY up.username
            ORDER BY count DESC
            LIMIT 10
            "#,
        )
        .bind(total_activities)
        .fetch_all(&self.pool)
        .await?;

        // Get growth by period (use simplified version for now, CTE with window functions is complex)
        // Future enhancement: could add proper time-series analytics
        let growth_by_period = vec![crate::models::GrowthStats {
            period: "All Time".to_string(),
            new_registrations: total_fids,
            total_fids,
            growth_rate: 0.0,
        }];

        let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts")
            .fetch_one(&self.pool)
            .await?;

        let activities_by_type = sqlx::query_as::<_, crate::models::ActivityTypeStats>(
            r#"
            SELECT 
                activity_type,
                COUNT(*) as count
            FROM user_activity_timeline
            GROUP BY activity_type
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(crate::models::StatisticsResult {
            total_fids,
            total_profiles: total_fids,
            complete_profiles,
            profiles_with_username,
            profiles_with_display_name,
            profiles_with_bio,
            profiles_with_pfp,
            profiles_with_website,
            profiles_with_location,
            profiles_with_twitter,
            profiles_with_github,
            profiles_with_ethereum_address,
            profiles_with_solana_address,
            recent_registrations,
            top_usernames,
            growth_by_period,
            total_activities,
            total_casts,
            activities_by_type,
        })
    }
}
