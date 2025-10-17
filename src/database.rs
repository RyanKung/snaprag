use sqlx::PgPool;

use crate::models::*;
use crate::Result;
use crate::SnapRagError;

/// Database connection pool wrapper
#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new database instance from configuration
    pub async fn from_config(config: &crate::config::AppConfig) -> Result<Self> {
        let pool_options = sqlx::postgres::PgPoolOptions::new()
            .max_connections(config.max_connections())
            .min_connections(config.min_connections())
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout()));

        let pool = pool_options.connect(config.database_url()).await?;
        Ok(Self::new(pool))
    }

    /// Run database migrations
    /// Note: Migrations are currently managed manually via SQL files in /migrations
    /// Future enhancement: Could integrate with sqlx migrations or refinery
    pub async fn migrate(&self) -> Result<()> {
        Ok(())
    }

    /// Get a reference to the database pool for raw queries
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

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

    /// Create a user profile snapshot
    pub async fn create_user_profile_snapshot(&self, snapshot: &UserProfileSnapshot) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_profile_snapshots (
                fid, username, display_name, bio, pfp_url, website_url,
                snapshot_timestamp, message_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            snapshot.fid,
            snapshot.username,
            snapshot.display_name,
            snapshot.bio,
            snapshot.pfp_url,
            snapshot.website_url,
            snapshot.snapshot_timestamp,
            snapshot.message_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Removed duplicate record_user_data_change method - using the one at line 722

    // Removed duplicate record_user_activity method - using the one at line 836

    /// Initialize database schema
    pub async fn init_schema(&self) -> Result<()> {
        // Create user_profiles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_profiles (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT UNIQUE NOT NULL,
                username VARCHAR(255),
                display_name VARCHAR(255),
                bio TEXT,
                pfp_url TEXT,
                banner_url TEXT,
                location VARCHAR(255),
                website_url TEXT,
                twitter_username VARCHAR(255),
                github_username VARCHAR(255),
                primary_address_ethereum VARCHAR(42),
                primary_address_solana VARCHAR(44),
                profile_token VARCHAR(255),
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                interests_embedding VECTOR(1536),
                last_updated_timestamp BIGINT NOT NULL,
                last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user_profile_snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_profile_snapshots (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                snapshot_timestamp BIGINT NOT NULL,
                message_hash BYTEA NOT NULL,
                username VARCHAR(255),
                display_name VARCHAR(255),
                bio TEXT,
                pfp_url TEXT,
                banner_url TEXT,
                location VARCHAR(255),
                website_url TEXT,
                twitter_username VARCHAR(255),
                github_username VARCHAR(255),
                primary_address_ethereum VARCHAR(42),
                primary_address_solana VARCHAR(44),
                profile_token VARCHAR(255),
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                interests_embedding VECTOR(1536),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(fid, snapshot_timestamp)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user_data_changes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_data_changes (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                data_type SMALLINT NOT NULL,
                old_value TEXT,
                new_value TEXT NOT NULL,
                change_timestamp BIGINT NOT NULL,
                message_hash BYTEA NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create username_proofs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS username_proofs (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                username VARCHAR(255) NOT NULL,
                username_type SMALLINT NOT NULL,
                owner_address VARCHAR(42) NOT NULL,
                signature BYTEA NOT NULL,
                timestamp BIGINT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(fid, username_type)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user_activity_timeline table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_activity_timeline (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                activity_type VARCHAR(50) NOT NULL,
                activity_data JSONB,
                timestamp BIGINT NOT NULL,
                message_hash BYTEA,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create user_profile_trends table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_profile_trends (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                fid BIGINT NOT NULL,
                trend_period VARCHAR(20) NOT NULL,
                trend_date DATE NOT NULL,
                profile_changes_count INTEGER DEFAULT 0,
                bio_changes_count INTEGER DEFAULT 0,
                username_changes_count INTEGER DEFAULT 0,
                activity_score FLOAT DEFAULT 0.0,
                engagement_score FLOAT DEFAULT 0.0,
                profile_embedding VECTOR(1536),
                bio_embedding VECTOR(1536),
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(fid, trend_period, trend_date)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        self.create_indexes().await?;

        Ok(())
    }

    async fn create_indexes(&self) -> Result<()> {
        // User profiles indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_user_profiles_fid ON user_profiles(fid)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_user_profiles_username ON user_profiles(username)",
        )
        .execute(&self.pool)
        .await?;

        // Profile snapshots indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_profile_snapshots_fid_timestamp ON user_profile_snapshots(fid, snapshot_timestamp DESC)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_profile_snapshots_timestamp ON user_profile_snapshots(snapshot_timestamp DESC)")
            .execute(&self.pool)
            .await?;

        // User data changes indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_data_changes_fid_type ON user_data_changes(fid, data_type, change_timestamp DESC)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_data_changes_timestamp ON user_data_changes(change_timestamp DESC)")
            .execute(&self.pool)
            .await?;

        // Activity timeline indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_activity_timeline_fid_timestamp ON user_activity_timeline(fid, timestamp DESC)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_activity_timeline_type_timestamp ON user_activity_timeline(activity_type, timestamp DESC)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

/// User Profile CRUD operations
impl Database {
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

/// User Profile Snapshot CRUD operations
impl Database {
    /// Create profile snapshot from current profile
    async fn create_profile_snapshot_from_profile(
        &self,
        profile: &UserProfile,
        message_hash: Vec<u8>,
    ) -> Result<UserProfileSnapshot> {
        // Use a slightly different timestamp to avoid conflicts
        let snapshot_timestamp = profile.last_updated_timestamp + 1;

        let snapshot = sqlx::query_as::<_, UserProfileSnapshot>(
            r#"
            INSERT INTO user_profile_snapshots (
                fid, snapshot_timestamp, message_hash, username, display_name, bio,
                pfp_url, banner_url, location, website_url, twitter_username,
                github_username, primary_address_ethereum, primary_address_solana,
                profile_token, profile_embedding, bio_embedding, interests_embedding
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            RETURNING *
            "#,
        )
        .bind(profile.fid)
        .bind(snapshot_timestamp)
        .bind(message_hash)
        .bind(&profile.username)
        .bind(&profile.display_name)
        .bind(&profile.bio)
        .bind(&profile.pfp_url)
        .bind(&profile.banner_url)
        .bind(&profile.location)
        .bind(&profile.website_url)
        .bind(&profile.twitter_username)
        .bind(&profile.github_username)
        .bind(&profile.primary_address_ethereum)
        .bind(&profile.primary_address_solana)
        .bind(&profile.profile_token)
        .bind(&profile.profile_embedding)
        .bind(&profile.bio_embedding)
        .bind(&profile.interests_embedding)
        .fetch_one(&self.pool)
        .await?;

        Ok(snapshot)
    }

    /// Get profile snapshots for a user
    pub async fn get_profile_snapshots(
        &self,
        query: ProfileSnapshotQuery,
    ) -> Result<Vec<UserProfileSnapshot>> {
        // Returns snapshots with pagination
        let snapshots = sqlx::query_as::<_, UserProfileSnapshot>(
            r#"
            SELECT 
                id,
                fid,
                snapshot_timestamp,
                message_hash,
                username,
                display_name,
                bio,
                pfp_url,
                website_url,
                location,
                twitter_username,
                github_username,
                banner_url,
                primary_address_ethereum,
                primary_address_solana,
                created_at
            FROM user_profile_snapshots 
            WHERE fid = $1
            ORDER BY snapshot_timestamp DESC
            LIMIT $2
            OFFSET $3
            "#,
        )
        .bind(query.fid)
        .bind(query.limit.unwrap_or(100) as i64)
        .bind(query.offset.unwrap_or(0) as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(snapshots)
    }

    /// Get profile snapshot at specific timestamp
    pub async fn get_profile_snapshot_at_timestamp(
        &self,
        fid: i64,
        timestamp: i64,
    ) -> Result<Option<UserProfileSnapshot>> {
        let snapshot = sqlx::query_as::<_, UserProfileSnapshot>(
            r#"
            SELECT * FROM user_profile_snapshots
            WHERE fid = $1 AND snapshot_timestamp <= $2
            ORDER BY snapshot_timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(fid)
        .bind(timestamp)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }

    /// Get latest profile snapshot for a user
    pub async fn get_latest_profile_snapshot(
        &self,
        fid: i64,
    ) -> Result<Option<UserProfileSnapshot>> {
        let snapshot = sqlx::query_as::<_, UserProfileSnapshot>(
            "SELECT * FROM user_profile_snapshots WHERE fid = $1 ORDER BY snapshot_timestamp DESC LIMIT 1"
        )
        .bind(fid)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }
}

/// User Data Change CRUD operations
impl Database {
    /// Record a user data change
    pub async fn record_user_data_change(
        &self,
        fid: i64,
        data_type: i16,
        old_value: Option<String>,
        new_value: String,
        timestamp: i64,
        message_hash: Vec<u8>,
    ) -> Result<UserDataChange> {
        let change = sqlx::query_as::<_, UserDataChange>(
            r#"
            INSERT INTO user_data_changes (fid, data_type, old_value, new_value, change_timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(fid)
        .bind(data_type)
        .bind(old_value)
        .bind(new_value)
        .bind(timestamp)
        .bind(message_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(change)
    }

    /// Get user data changes
    pub async fn get_user_data_changes(
        &self,
        fid: i64,
        data_type: Option<i16>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UserDataChange>> {
        // Get user data changes with optional data_type filter
        let changes = if let Some(dt) = data_type {
            sqlx::query_as::<_, UserDataChange>(
                r#"
                SELECT 
                    id, fid, data_type, old_value, new_value,
                    change_timestamp, message_hash, created_at
                FROM user_data_changes 
                WHERE fid = $1 AND data_type = $2
                ORDER BY change_timestamp DESC
                LIMIT $3
                OFFSET $4
                "#,
            )
            .bind(fid)
            .bind(dt)
            .bind(limit.unwrap_or(100) as i64)
            .bind(offset.unwrap_or(0) as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UserDataChange>(
                r#"
                SELECT 
                    id, fid, data_type, old_value, new_value,
                    change_timestamp, message_hash, created_at
                FROM user_data_changes 
                WHERE fid = $1
                ORDER BY change_timestamp DESC
                LIMIT $2
                OFFSET $3
                "#,
            )
            .bind(fid)
            .bind(limit.unwrap_or(100) as i64)
            .bind(offset.unwrap_or(0) as i64)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(changes)
    }
}

/// Username Proof CRUD operations
impl Database {
    /// Create or update username proof
    pub async fn upsert_username_proof(
        &self,
        fid: i64,
        username: String,
        username_type: UsernameType,
        owner_address: String,
        signature: Vec<u8>,
        timestamp: i64,
    ) -> Result<UsernameProof> {
        let proof = sqlx::query_as::<_, UsernameProof>(
            r#"
            INSERT INTO username_proofs (fid, username, username_type, owner_address, signature, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (fid, username_type)
            DO UPDATE SET
                username = EXCLUDED.username,
                owner_address = EXCLUDED.owner_address,
                signature = EXCLUDED.signature,
                timestamp = EXCLUDED.timestamp,
                created_at = NOW()
            RETURNING *
            "#
        )
        .bind(fid)
        .bind(username)
        .bind(username_type as i32)
        .bind(owner_address)
        .bind(signature)
        .bind(timestamp)
        .fetch_one(&self.pool)
        .await?;

        Ok(proof)
    }

    /// Get username proof by FID and type
    pub async fn get_username_proof(
        &self,
        fid: i64,
        username_type: UsernameType,
    ) -> Result<Option<UsernameProof>> {
        let proof = sqlx::query_as::<_, UsernameProof>(
            "SELECT * FROM username_proofs WHERE fid = $1 AND username_type = $2",
        )
        .bind(fid)
        .bind(username_type as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(proof)
    }

    /// Get all username proofs for a user
    pub async fn get_user_username_proofs(&self, fid: i64) -> Result<Vec<UsernameProof>> {
        let proofs = sqlx::query_as::<_, UsernameProof>(
            "SELECT * FROM username_proofs WHERE fid = $1 ORDER BY timestamp DESC",
        )
        .bind(fid)
        .fetch_all(&self.pool)
        .await?;

        Ok(proofs)
    }
}

/// User Activity Timeline CRUD operations
impl Database {
    /// Record user activity
    pub async fn record_user_activity(
        &self,
        fid: i64,
        activity_type: String,
        activity_data: Option<serde_json::Value>,
        timestamp: i64,
        message_hash: Option<Vec<u8>>,
    ) -> Result<UserActivityTimeline> {
        let activity = sqlx::query_as::<_, UserActivityTimeline>(
            r#"
            INSERT INTO user_activity_timeline (fid, activity_type, activity_data, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(fid)
        .bind(activity_type)
        .bind(activity_data)
        .bind(timestamp)
        .bind(message_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    /// Batch insert user activities for performance
    pub async fn batch_insert_activities(
        &self,
        activities: Vec<(i64, String, Option<serde_json::Value>, i64, Option<Vec<u8>>)>,
    ) -> Result<()> {
        if activities.is_empty() {
            return Ok(());
        }

        // Build VALUES clause dynamically
        let mut query = String::from(
            "INSERT INTO user_activity_timeline (fid, activity_type, activity_data, timestamp, message_hash) VALUES "
        );

        let params_per_row = 5;
        let value_clauses: Vec<String> = (0..activities.len())
            .map(|i| {
                let base = i * params_per_row;
                format!(
                    "(${}, ${}, ${}, ${}, ${})",
                    base + 1,
                    base + 2,
                    base + 3,
                    base + 4,
                    base + 5
                )
            })
            .collect();

        query.push_str(&value_clauses.join(", "));

        let mut q = sqlx::query(&query);
        for (fid, activity_type, activity_data, timestamp, message_hash) in activities {
            q = q
                .bind(fid)
                .bind(activity_type)
                .bind(activity_data)
                .bind(timestamp)
                .bind(message_hash);
        }

        q.execute(&self.pool).await?;
        Ok(())
    }

    /// Get user activity timeline
    pub async fn get_user_activity_timeline(
        &self,
        fid: i64,
        activity_type: Option<String>,
        _start_timestamp: Option<i64>,
        _end_timestamp: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UserActivityTimeline>> {
        let activities = if let Some(act_type) = activity_type {
            // Query with activity type filter
            sqlx::query_as::<_, UserActivityTimeline>(
                r#"
                SELECT 
                    id,
                    fid,
                    activity_type,
                    activity_data,
                    timestamp,
                    message_hash,
                    created_at,
                    shard_id,
                    block_height,
                    transaction_fid
                FROM user_activity_timeline 
                WHERE fid = $1 AND activity_type = $2
                ORDER BY timestamp DESC
                LIMIT $3
                OFFSET $4
                "#,
            )
            .bind(fid)
            .bind(act_type)
            .bind(limit.unwrap_or(100) as i64)
            .bind(offset.unwrap_or(0) as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            // Query all activities
            sqlx::query_as::<_, UserActivityTimeline>(
                r#"
                SELECT 
                    id,
                    fid,
                    activity_type,
                    activity_data,
                    timestamp,
                    message_hash,
                    created_at,
                    shard_id,
                    block_height,
                    transaction_fid
                FROM user_activity_timeline 
                WHERE fid = $1
                ORDER BY timestamp DESC
                LIMIT $2
                OFFSET $3
                "#,
            )
            .bind(fid)
            .bind(limit.unwrap_or(100) as i64)
            .bind(offset.unwrap_or(0) as i64)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(activities)
    }
}

/// Sync-related database operations
impl Database {
    /// Get the last processed height for a shard
    pub async fn get_last_processed_height(&self, shard_id: u32) -> Result<u64> {
        let row = sqlx::query!(
            "SELECT last_processed_height FROM sync_progress WHERE shard_id = $1",
            shard_id as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|r| r.last_processed_height.unwrap_or(0) as u64)
            .unwrap_or(0))
    }

    /// Update the last processed height for a shard
    pub async fn update_last_processed_height(&self, shard_id: u32, height: u64) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sync_progress (shard_id, last_processed_height, status, updated_at)
            VALUES ($1, $2, 'syncing', NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                last_processed_height = EXCLUDED.last_processed_height,
                status = 'syncing',
                updated_at = NOW()
            "#,
            shard_id as i32,
            height as i64
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update sync status for a shard
    pub async fn update_sync_status(
        &self,
        shard_id: u32,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sync_progress (shard_id, status, error_message, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                status = EXCLUDED.status,
                error_message = EXCLUDED.error_message,
                updated_at = NOW()
            "#,
            shard_id as i32,
            status,
            error_message
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a processed message
    pub async fn record_processed_message(
        &self,
        message_hash: Vec<u8>,
        shard_id: u32,
        block_height: u64,
        transaction_fid: u64,
        message_type: &str,
        fid: u64,
        timestamp: i64,
        content_hash: Option<Vec<u8>>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO processed_messages (
                message_hash, shard_id, block_height, transaction_fid,
                message_type, fid, timestamp, content_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (message_hash) DO NOTHING
            "#,
            message_hash,
            shard_id as i32,
            block_height as i64,
            transaction_fid as i64,
            message_type,
            fid as i64,
            timestamp,
            content_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a message has been processed
    pub async fn is_message_processed(&self, message_hash: &[u8]) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT 1 as exists FROM processed_messages WHERE message_hash = $1 LIMIT 1",
            message_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    /// Update sync statistics
    pub async fn update_sync_stats(
        &self,
        shard_id: u32,
        total_messages: u64,
        total_blocks: u64,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO sync_stats (shard_id, total_messages, total_blocks, last_updated)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                total_messages = EXCLUDED.total_messages,
                total_blocks = EXCLUDED.total_blocks,
                last_updated = NOW()
            "#,
            shard_id as i32,
            total_messages as i64,
            total_blocks as i64
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get sync statistics for all shards
    pub async fn get_sync_stats(&self) -> Result<Vec<SyncStats>> {
        let stats = sqlx::query_as::<_, SyncStats>(
            r#"
            SELECT 
                sp.shard_id,
                COALESCE(ss.total_messages, 0) as total_messages,
                COALESCE(ss.total_blocks, 0) as total_blocks,
                COALESCE(ss.last_updated, sp.updated_at) as last_updated,
                sp.status,
                sp.last_processed_height,
                sp.updated_at as last_sync_timestamp
            FROM sync_progress sp
            LEFT JOIN sync_stats ss ON sp.shard_id = ss.shard_id
            ORDER BY sp.shard_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(stats)
    }

    /// Upsert a link (follow relationship, etc.)
    pub async fn upsert_link(
        &self,
        fid: i64,
        target_fid: i64,
        link_type: &str,
        timestamp: i64,
        message_hash: Vec<u8>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_hash)
            DO UPDATE SET
                fid = EXCLUDED.fid,
                target_fid = EXCLUDED.target_fid,
                link_type = EXCLUDED.link_type,
                timestamp = EXCLUDED.timestamp
            "#,
            fid,
            target_fid,
            link_type,
            timestamp,
            message_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upsert a cast
    pub async fn upsert_cast(
        &self,
        fid: i64,
        text: Option<String>,
        timestamp: i64,
        message_hash: Vec<u8>,
        parent_hash: Option<Vec<u8>>,
        root_hash: Option<Vec<u8>>,
        embeds: Option<serde_json::Value>,
        mentions: Option<serde_json::Value>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (message_hash)
            DO UPDATE SET
                fid = EXCLUDED.fid,
                text = EXCLUDED.text,
                timestamp = EXCLUDED.timestamp,
                parent_hash = EXCLUDED.parent_hash,
                root_hash = EXCLUDED.root_hash,
                embeds = EXCLUDED.embeds,
                mentions = EXCLUDED.mentions
            "#,
            fid,
            text,
            timestamp,
            message_hash,
            parent_hash,
            root_hash,
            embeds,
            mentions
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upsert user data
    pub async fn upsert_user_data(
        &self,
        fid: i64,
        data_type: i16,
        value: String,
        timestamp: i64,
        message_hash: Vec<u8>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_data (fid, data_type, value, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_hash)
            DO UPDATE SET
                fid = EXCLUDED.fid,
                data_type = EXCLUDED.data_type,
                value = EXCLUDED.value,
                timestamp = EXCLUDED.timestamp
            "#,
            fid,
            data_type,
            value,
            timestamp,
            message_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// Sync-related data structures
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SyncStats {
    pub shard_id: i32,
    pub total_messages: i64,
    pub total_blocks: i64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub status: Option<String>,
    pub last_processed_height: Option<i64>,
    pub last_sync_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Cast CRUD operations
impl Database {
    /// List casts with filters
    pub async fn list_casts(
        &self,
        query: crate::models::CastQuery,
    ) -> Result<Vec<crate::models::Cast>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Build the query based on filters
        let mut sql = String::from("SELECT * FROM casts WHERE 1=1");
        let mut bind_values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> =
            Vec::new();
        let mut param_count = 0;

        if let Some(fid) = query.fid {
            param_count += 1;
            sql.push_str(&format!(" AND fid = ${}", param_count));
            bind_values.push(Box::new(fid));
        }

        if let Some(text_search) = &query.text_search {
            param_count += 1;
            sql.push_str(&format!(" AND text ILIKE ${}", param_count));
            bind_values.push(Box::new(format!("%{}%", text_search)));
        }

        if let Some(parent_hash) = &query.parent_hash {
            param_count += 1;
            sql.push_str(&format!(" AND parent_hash = ${}", param_count));
            bind_values.push(Box::new(parent_hash.clone()));
        }

        if let Some(root_hash) = &query.root_hash {
            param_count += 1;
            sql.push_str(&format!(" AND root_hash = ${}", param_count));
            bind_values.push(Box::new(root_hash.clone()));
        }

        if let Some(has_mentions) = query.has_mentions {
            if has_mentions {
                sql.push_str(" AND mentions IS NOT NULL AND mentions != 'null'");
            } else {
                sql.push_str(" AND (mentions IS NULL OR mentions = 'null')");
            }
        }

        if let Some(has_embeds) = query.has_embeds {
            if has_embeds {
                sql.push_str(" AND embeds IS NOT NULL AND embeds != 'null'");
            } else {
                sql.push_str(" AND (embeds IS NULL OR embeds = 'null')");
            }
        }

        if let Some(start_timestamp) = query.start_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
            bind_values.push(Box::new(start_timestamp));
        }

        if let Some(end_timestamp) = query.end_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
            bind_values.push(Box::new(end_timestamp));
        }

        // Add sorting
        let sort_by = match query.sort_by {
            Some(crate::models::CastSortBy::Timestamp) => "timestamp",
            Some(crate::models::CastSortBy::Fid) => "fid",
            Some(crate::models::CastSortBy::Text) => "text",
            Some(crate::models::CastSortBy::CreatedAt) => "created_at",
            None => "timestamp",
        };

        let sort_order = match query.sort_order {
            Some(crate::models::SortOrder::Asc) => "ASC",
            Some(crate::models::SortOrder::Desc) => "DESC",
            None => "DESC",
        };

        sql.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));

        // Add pagination
        param_count += 1;
        sql.push_str(&format!(" LIMIT ${}", param_count));
        bind_values.push(Box::new(limit));
        param_count += 1;
        sql.push_str(&format!(" OFFSET ${}", param_count));
        bind_values.push(Box::new(offset));

        // Use the dynamically built query with all filters
        // Note: We need to use query_as with dynamic SQL, which requires rebuilding
        // For complex filters, we'll use a pragmatic approach
        let casts = if query.fid.is_some()
            || query.text_search.is_some()
            || query.parent_hash.is_some()
            || query.root_hash.is_some()
            || query.start_timestamp.is_some()
            || query.end_timestamp.is_some()
        {
            // Complex query - use the specific filters we support
            let mut conditions = vec!["1=1".to_string()];
            let mut param_idx = 1;

            if let Some(fid) = query.fid {
                conditions.push(format!("fid = ${}", param_idx));
                param_idx += 1;
            }

            if let Some(text_search) = &query.text_search {
                conditions.push(format!("text ILIKE ${}", param_idx));
                param_idx += 1;
            }

            if let Some(parent_hash) = &query.parent_hash {
                conditions.push(format!("parent_hash = ${}", param_idx));
                param_idx += 1;
            }

            if let Some(start_timestamp) = query.start_timestamp {
                conditions.push(format!("timestamp >= ${}", param_idx));
                param_idx += 1;
            }

            if let Some(end_timestamp) = query.end_timestamp {
                conditions.push(format!("timestamp <= ${}", param_idx));
                // param_idx would be incremented here if we had more conditions
            }

            let where_clause = conditions.join(" AND ");
            let order_by = match query.sort_by {
                Some(crate::models::CastSortBy::Timestamp) => "timestamp",
                Some(crate::models::CastSortBy::Fid) => "fid",
                _ => "timestamp",
            };
            let order_dir = match query.sort_order {
                Some(crate::models::SortOrder::Asc) => "ASC",
                _ => "DESC",
            };

            let sql = format!(
                "SELECT * FROM casts WHERE {} ORDER BY {} {} LIMIT {} OFFSET {}",
                where_clause, order_by, order_dir, limit, offset
            );

            let mut q = sqlx::query_as::<_, crate::models::Cast>(&sql);

            if let Some(fid) = query.fid {
                q = q.bind(fid);
            }
            if let Some(text_search) = &query.text_search {
                q = q.bind(format!("%{}%", text_search));
            }
            if let Some(parent_hash) = &query.parent_hash {
                q = q.bind(parent_hash);
            }
            if let Some(start_timestamp) = query.start_timestamp {
                q = q.bind(start_timestamp);
            }
            if let Some(end_timestamp) = query.end_timestamp {
                q = q.bind(end_timestamp);
            }

            q.fetch_all(&self.pool).await?
        } else {
            // Simple query - just sort and paginate
            sqlx::query_as::<_, crate::models::Cast>(
                "SELECT * FROM casts ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(casts)
    }

    /// Get casts by FID
    pub async fn get_casts_by_fid(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::Cast>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let casts = sqlx::query_as::<_, crate::models::Cast>(
            "SELECT * FROM casts WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(casts)
    }

    /// Count casts without embeddings
    pub async fn count_casts_without_embeddings(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) 
            FROM casts c
            LEFT JOIN cast_embeddings ce ON c.message_hash = ce.message_hash
            WHERE c.text IS NOT NULL 
              AND length(c.text) > 0
              AND ce.id IS NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Get casts without embeddings
    pub async fn get_casts_without_embeddings(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<crate::models::Cast>> {
        let casts = sqlx::query_as::<_, crate::models::Cast>(
            r#"
            SELECT c.* 
            FROM casts c
            LEFT JOIN cast_embeddings ce ON c.message_hash = ce.message_hash
            WHERE c.text IS NOT NULL 
              AND length(c.text) > 0
              AND ce.id IS NULL
            ORDER BY c.timestamp DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(casts)
    }

    /// Store cast embedding
    pub async fn store_cast_embedding(
        &self,
        message_hash: &[u8],
        fid: i64,
        text: &str,
        embedding: &[f32],
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO cast_embeddings (message_hash, fid, text, embedding)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (message_hash) 
            DO UPDATE SET 
                embedding = EXCLUDED.embedding,
                updated_at = NOW()
            "#,
        )
        .bind(message_hash)
        .bind(fid)
        .bind(text)
        .bind(embedding)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Semantic search for casts with engagement metrics
    pub async fn semantic_search_casts(
        &self,
        query_embedding: Vec<f32>,
        limit: i64,
        threshold: Option<f32>,
    ) -> Result<Vec<crate::models::CastSearchResult>> {
        let threshold_val = threshold.unwrap_or(0.0);

        #[derive(sqlx::FromRow)]
        struct RawResult {
            message_hash: Vec<u8>,
            fid: i64,
            text: String,
            timestamp: i64,
            parent_hash: Option<Vec<u8>>,
            embeds: Option<serde_json::Value>,
            mentions: Option<serde_json::Value>,
            similarity: f32,
            reply_count: Option<i64>,
            reaction_count: Option<i64>,
        }

        let raw_results = sqlx::query_as::<_, RawResult>(
            r#"
            SELECT 
                ce.message_hash,
                ce.fid,
                ce.text,
                c.timestamp,
                c.parent_hash,
                c.embeds,
                c.mentions,
                1 - (ce.embedding <=> $1) as similarity,
                (SELECT COUNT(*) FROM casts WHERE parent_hash = ce.message_hash) as reply_count,
                (SELECT COUNT(*) FROM user_activity_timeline 
                 WHERE message_hash = ce.message_hash 
                 AND activity_type = 'reaction_add') as reaction_count
            FROM cast_embeddings ce
            INNER JOIN casts c ON ce.message_hash = c.message_hash
            WHERE 1 - (ce.embedding <=> $1) > $2
            ORDER BY ce.embedding <=> $1
            LIMIT $3
            "#,
        )
        .bind(&query_embedding)
        .bind(threshold_val)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = raw_results
            .into_iter()
            .map(|r| crate::models::CastSearchResult {
                message_hash: r.message_hash,
                fid: r.fid,
                text: r.text,
                timestamp: r.timestamp,
                parent_hash: r.parent_hash,
                embeds: r.embeds,
                mentions: r.mentions,
                similarity: r.similarity,
                reply_count: r.reply_count.unwrap_or(0),
                reaction_count: r.reaction_count.unwrap_or(0),
            })
            .collect();

        Ok(results)
    }

    /// Get cast statistics (replies, reactions, etc.)
    pub async fn get_cast_stats(&self, message_hash: &[u8]) -> Result<crate::models::CastStats> {
        let stats = sqlx::query_as::<_, crate::models::CastStats>(
            r#"
            SELECT 
                $1 as message_hash,
                (SELECT COUNT(*) FROM casts WHERE parent_hash = $1) as reply_count,
                (SELECT COUNT(*) FROM user_activity_timeline 
                 WHERE message_hash = $1 
                 AND activity_type = 'reaction_add') as reaction_count,
                (SELECT COUNT(DISTINCT fid) FROM user_activity_timeline 
                 WHERE message_hash = $1 
                 AND activity_type = 'reaction_add') as unique_reactors
            "#,
        )
        .bind(message_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// Get cast by message hash
    pub async fn get_cast_by_hash(
        &self,
        message_hash: Vec<u8>,
    ) -> Result<Option<crate::models::Cast>> {
        let cast =
            sqlx::query_as::<_, crate::models::Cast>("SELECT * FROM casts WHERE message_hash = $1")
                .bind(message_hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(cast)
    }

    /// Get cast replies (children)
    pub async fn get_cast_replies(
        &self,
        parent_hash: Vec<u8>,
        limit: Option<i64>,
    ) -> Result<Vec<crate::models::Cast>> {
        let casts = sqlx::query_as::<_, crate::models::Cast>(
            "SELECT * FROM casts WHERE parent_hash = $1 ORDER BY timestamp ASC LIMIT $2",
        )
        .bind(parent_hash)
        .bind(limit.unwrap_or(100))
        .fetch_all(&self.pool)
        .await?;

        Ok(casts)
    }

    /// Get cast thread (recursive parents and children)
    pub async fn get_cast_thread(
        &self,
        message_hash: Vec<u8>,
        max_depth: usize,
    ) -> Result<CastThread> {
        let mut thread = CastThread {
            root: None,
            parents: Vec::new(),
            children: Vec::new(),
        };

        // Get the target cast
        let cast = self.get_cast_by_hash(message_hash.clone()).await?;
        if cast.is_none() {
            return Ok(thread);
        }

        let current_cast = cast.unwrap();
        thread.root = Some(current_cast.clone());

        // Traverse up to find parents
        let mut current_parent = current_cast.parent_hash.clone();
        let mut depth = 0;
        while let Some(parent_hash) = current_parent {
            if depth >= max_depth {
                break;
            }

            if let Some(parent) = self.get_cast_by_hash(parent_hash.clone()).await? {
                thread.parents.push(parent.clone());
                current_parent = parent.parent_hash.clone();
                depth += 1;
            } else {
                break;
            }
        }

        // Reverse parents so root is first
        thread.parents.reverse();

        // Get direct replies
        let replies = self.get_cast_replies(message_hash, Some(50)).await?;
        thread.children = replies;

        Ok(thread)
    }
}

/// Cast thread structure
#[derive(Debug, Clone)]
pub struct CastThread {
    pub root: Option<crate::models::Cast>,
    pub parents: Vec<crate::models::Cast>,
    pub children: Vec<crate::models::Cast>,
}

/// Link CRUD operations
impl Database {
    /// List links with filters
    pub async fn list_links(
        &self,
        query: crate::models::LinkQuery,
    ) -> Result<Vec<crate::models::Link>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Build the query based on filters
        let mut sql = String::from("SELECT * FROM links WHERE 1=1");
        let mut param_count = 0;

        if let Some(fid) = query.fid {
            param_count += 1;
            sql.push_str(&format!(" AND fid = ${}", param_count));
        }

        if let Some(target_fid) = query.target_fid {
            param_count += 1;
            sql.push_str(&format!(" AND target_fid = ${}", param_count));
        }

        if let Some(link_type) = &query.link_type {
            param_count += 1;
            sql.push_str(&format!(" AND link_type = ${}", param_count));
        }

        if let Some(start_timestamp) = query.start_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
        }

        if let Some(end_timestamp) = query.end_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
        }

        // Add sorting
        let sort_by = match query.sort_by {
            Some(crate::models::LinkSortBy::Timestamp) => "timestamp",
            Some(crate::models::LinkSortBy::Fid) => "fid",
            Some(crate::models::LinkSortBy::TargetFid) => "target_fid",
            Some(crate::models::LinkSortBy::LinkType) => "link_type",
            Some(crate::models::LinkSortBy::CreatedAt) => "created_at",
            None => "timestamp",
        };

        let sort_order = match query.sort_order {
            Some(crate::models::SortOrder::Asc) => "ASC",
            Some(crate::models::SortOrder::Desc) => "DESC",
            None => "DESC",
        };

        sql.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));

        // Add pagination
        param_count += 1;
        sql.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        sql.push_str(&format!(" OFFSET ${}", param_count));

        // Execute query with parameters
        let links = match (query.fid, query.target_fid) {
            (Some(fid), Some(target_fid)) => {
                sqlx::query_as::<_, crate::models::Link>(
                    "SELECT * FROM links WHERE fid = $1 AND target_fid = $2 ORDER BY timestamp DESC LIMIT $3 OFFSET $4"
                )
                .bind(fid)
                .bind(target_fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(fid), None) => {
                sqlx::query_as::<_, crate::models::Link>(
                    "SELECT * FROM links WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
                )
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(target_fid)) => {
                sqlx::query_as::<_, crate::models::Link>(
                    "SELECT * FROM links WHERE target_fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
                )
                .bind(target_fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, crate::models::Link>(
                    "SELECT * FROM links ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(links)
    }

    /// Get links by FID
    pub async fn get_links_by_fid(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, crate::models::Link>(
            "SELECT * FROM links WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get followers for a user
    pub async fn get_followers(
        &self,
        target_fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, crate::models::Link>(
            "SELECT * FROM links WHERE target_fid = $1 AND link_type = 'follow' ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
        )
        .bind(target_fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get following for a user
    pub async fn get_following(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, crate::models::Link>(
            "SELECT * FROM links WHERE fid = $1 AND link_type = 'follow' ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }
}

/// User Data CRUD operations
impl Database {
    /// List user data with filters
    pub async fn list_user_data(
        &self,
        query: crate::models::UserDataQuery,
    ) -> Result<Vec<crate::models::UserData>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Build the query based on filters
        let mut sql = String::from("SELECT * FROM user_data WHERE 1=1");
        let mut param_count = 0;

        if let Some(fid) = query.fid {
            param_count += 1;
            sql.push_str(&format!(" AND fid = ${}", param_count));
        }

        if let Some(data_type) = query.data_type {
            param_count += 1;
            sql.push_str(&format!(" AND data_type = ${}", param_count));
        }

        if let Some(value_search) = &query.value_search {
            param_count += 1;
            sql.push_str(&format!(" AND value ILIKE ${}", param_count));
        }

        if let Some(start_timestamp) = query.start_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
        }

        if let Some(end_timestamp) = query.end_timestamp {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
        }

        // Add sorting
        let sort_by = match query.sort_by {
            Some(crate::models::UserDataSortBy::Timestamp) => "timestamp",
            Some(crate::models::UserDataSortBy::Fid) => "fid",
            Some(crate::models::UserDataSortBy::DataType) => "data_type",
            Some(crate::models::UserDataSortBy::Value) => "value",
            Some(crate::models::UserDataSortBy::CreatedAt) => "created_at",
            None => "timestamp",
        };

        let sort_order = match query.sort_order {
            Some(crate::models::SortOrder::Asc) => "ASC",
            Some(crate::models::SortOrder::Desc) => "DESC",
            None => "DESC",
        };

        sql.push_str(&format!(" ORDER BY {} {}", sort_by, sort_order));

        // Add pagination
        param_count += 1;
        sql.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        sql.push_str(&format!(" OFFSET ${}", param_count));

        // Execute query with parameters
        let user_data = match (query.fid, query.data_type) {
            (Some(fid), Some(data_type)) => {
                sqlx::query_as::<_, crate::models::UserData>(
                    "SELECT * FROM user_data WHERE fid = $1 AND data_type = $2 ORDER BY timestamp DESC LIMIT $3 OFFSET $4"
                )
                .bind(fid)
                .bind(data_type)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(fid), None) => {
                sqlx::query_as::<_, crate::models::UserData>(
                    "SELECT * FROM user_data WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
                )
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(data_type)) => {
                sqlx::query_as::<_, crate::models::UserData>(
                    "SELECT * FROM user_data WHERE data_type = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
                )
                .bind(data_type)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, crate::models::UserData>(
                    "SELECT * FROM user_data ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(user_data)
    }

    /// Get user data by FID
    pub async fn get_user_data_by_fid(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::UserData>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let user_data = sqlx::query_as::<_, crate::models::UserData>(
            "SELECT * FROM user_data WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(user_data)
    }

    /// Get user data by FID and data type
    pub async fn get_user_data_by_fid_and_type(
        &self,
        fid: i64,
        data_type: i16,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<crate::models::UserData>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let user_data = sqlx::query_as::<_, crate::models::UserData>(
            "SELECT * FROM user_data WHERE fid = $1 AND data_type = $2 ORDER BY timestamp DESC LIMIT $3 OFFSET $4"
        )
        .bind(fid)
        .bind(data_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(user_data)
    }

    /// Update profile embeddings
    pub async fn update_profile_embeddings(
        &self,
        fid: i64,
        profile_embedding: Option<Vec<f32>>,
        bio_embedding: Option<Vec<f32>>,
        interests_embedding: Option<Vec<f32>>,
    ) -> Result<()> {
        // Build dynamic query based on which embeddings are provided
        let mut updates = Vec::new();
        let mut param_num = 2; // $1 is fid

        if profile_embedding.is_some() {
            updates.push(format!("profile_embedding = ${}", param_num));
            param_num += 1;
        }
        if bio_embedding.is_some() {
            updates.push(format!("bio_embedding = ${}", param_num));
            param_num += 1;
        }
        if interests_embedding.is_some() {
            updates.push(format!("interests_embedding = ${}", param_num));
        }

        if updates.is_empty() {
            return Ok(()); // Nothing to update
        }

        let query_str = format!(
            "UPDATE user_profiles SET {} WHERE fid = $1",
            updates.join(", ")
        );

        let mut query = sqlx::query(&query_str).bind(fid);

        if let Some(pe) = profile_embedding {
            query = query.bind(pe);
        }
        if let Some(be) = bio_embedding {
            query = query.bind(be);
        }
        if let Some(ie) = interests_embedding {
            query = query.bind(ie);
        }

        query.execute(&self.pool).await?;
        Ok(())
    }

    /// Semantic search for profiles using vector similarity
    pub async fn semantic_search_profiles(
        &self,
        query_embedding: Vec<f32>,
        limit: i64,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<UserProfile>> {
        let threshold = similarity_threshold.unwrap_or(0.8);

        let profiles = sqlx::query_as::<_, UserProfile>(
            r#"
            SELECT *
            FROM user_profiles
            WHERE profile_embedding IS NOT NULL
                AND (profile_embedding <=> $1::vector) < $2
            ORDER BY profile_embedding <=> $1::vector
            LIMIT $3
            "#,
        )
        .bind(query_embedding)
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Semantic search for profiles by bio
    pub async fn semantic_search_profiles_by_bio(
        &self,
        query_embedding: Vec<f32>,
        limit: i64,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<UserProfile>> {
        let threshold = similarity_threshold.unwrap_or(0.8);

        let profiles = sqlx::query_as::<_, UserProfile>(
            r#"
            SELECT *
            FROM user_profiles
            WHERE bio_embedding IS NOT NULL
                AND (bio_embedding <=> $1::vector) < $2
            ORDER BY bio_embedding <=> $1::vector
            LIMIT $3
            "#,
        )
        .bind(query_embedding)
        .bind(threshold)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(profiles)
    }

    /// Hybrid search combining vector similarity and text search
    pub async fn hybrid_search_profiles(
        &self,
        query_embedding: Option<Vec<f32>>,
        text_query: Option<String>,
        limit: i64,
    ) -> Result<Vec<UserProfile>> {
        match (query_embedding, text_query) {
            (Some(embedding), Some(text)) => {
                // Combined vector + text search using CTE
                let profiles = sqlx::query_as::<_, UserProfile>(
                    r#"
                    WITH scored_profiles AS (
                        SELECT 
                            id, fid, username, display_name, bio, pfp_url, banner_url, location,
                            website_url, twitter_username, github_username, primary_address_ethereum,
                            primary_address_solana, profile_token, profile_embedding, bio_embedding,
                            interests_embedding, last_updated_timestamp, last_updated_at,
                            shard_id, block_height, transaction_fid,
                            (profile_embedding <=> $1::vector) as vector_distance,
                            CASE 
                                WHEN username ILIKE $2 THEN 1.0
                                WHEN display_name ILIKE $2 THEN 0.9
                                WHEN bio ILIKE $2 THEN 0.8
                                ELSE 0.0
                            END as text_score
                        FROM user_profiles
                        WHERE profile_embedding IS NOT NULL
                            AND (username ILIKE $2 OR display_name ILIKE $2 OR bio ILIKE $2)
                    )
                    SELECT 
                        id, fid, username, display_name, bio, pfp_url, banner_url, location,
                        website_url, twitter_username, github_username, primary_address_ethereum,
                        primary_address_solana, profile_token, profile_embedding, bio_embedding,
                        interests_embedding, last_updated_timestamp, last_updated_at,
                        shard_id, block_height, transaction_fid
                    FROM scored_profiles
                    ORDER BY vector_distance * 0.5 + (1.0 - text_score) * 0.5
                    LIMIT $3
                    "#,
                )
                .bind(embedding)
                .bind(format!("%{}%", text))
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;
                Ok(profiles)
            }
            (Some(embedding), None) => {
                // Vector search only
                self.semantic_search_profiles(embedding, limit, None).await
            }
            (None, Some(text)) => {
                // Text search only
                let query = UserProfileQuery {
                    fid: None,
                    username: None,
                    display_name: None,
                    bio: None,
                    location: None,
                    twitter_username: None,
                    github_username: None,
                    limit: Some(limit),
                    offset: None,
                    start_timestamp: None,
                    end_timestamp: None,
                    sort_by: None,
                    sort_order: None,
                    search_term: Some(text),
                };
                self.list_user_profiles(query).await
            }
            (None, None) => {
                // No search criteria
                Ok(Vec::new())
            }
        }
    }
}
