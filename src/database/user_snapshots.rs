use super::Database;
use crate::models::{UserProfileSnapshot, UserProfile, ProfileSnapshotQuery};
use crate::Result;

impl Database {
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

    /// Create profile snapshot from current profile
    pub(super) async fn create_profile_snapshot_from_profile(
        &self,
        profile: &UserProfile,
        message_hash: Vec<u8>,
    ) -> Result<UserProfileSnapshot> {
        // Use a slightly different timestamp to avoid conflicts
        let snapshot_timestamp = profile.last_updated_timestamp + 1;

        let snapshot = sqlx::query_as::<_, UserProfileSnapshot>(
            r"
            INSERT INTO user_profile_snapshots (
                fid, snapshot_timestamp, message_hash, username, display_name, bio,
                pfp_url, banner_url, location, website_url, twitter_username,
                github_username, primary_address_ethereum, primary_address_solana,
                profile_token, profile_embedding, bio_embedding, interests_embedding
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            RETURNING *
            ",
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
            r"
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
            ",
        )
        .bind(query.fid)
        .bind(query.limit.unwrap_or(100))
        .bind(query.offset.unwrap_or(0))
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
            r"
            SELECT * FROM user_profile_snapshots
            WHERE fid = $1 AND snapshot_timestamp <= $2
            ORDER BY snapshot_timestamp DESC
            LIMIT 1
            ",
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
