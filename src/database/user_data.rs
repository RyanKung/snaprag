use super::Database;
use crate::models::{UserDataQuery, UserData, UserProfile, UserProfileQuery};
use crate::Result;

impl Database {
    /// List user data with filters
    pub async fn list_user_data(&self, query: UserDataQuery) -> Result<Vec<UserData>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Execute query with parameters
        let user_data = match (query.fid, query.data_type) {
            (Some(fid), Some(data_type)) => {
                sqlx::query_as::<_, UserData>(
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
                sqlx::query_as::<_, UserData>(
                    "SELECT * FROM user_data WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
                )
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(data_type)) => {
                sqlx::query_as::<_, UserData>(
                    "SELECT * FROM user_data WHERE data_type = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
                )
                .bind(data_type)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, UserData>(
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
    ) -> Result<Vec<UserData>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let user_data = sqlx::query_as::<_, UserData>(
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
    ) -> Result<Vec<UserData>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let user_data = sqlx::query_as::<_, UserData>(
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
        let mut field_names = Vec::new();

        if profile_embedding.is_some() {
            field_names.push("profile_embedding");
        }
        if bio_embedding.is_some() {
            field_names.push("bio_embedding");
        }
        if interests_embedding.is_some() {
            field_names.push("interests_embedding");
        }

        if field_names.is_empty() {
            return Ok(()); // Nothing to update
        }

        // Insert or update profile_embeddings table (separate from event-sourcing)
        let field_list = field_names.join(", ");
        let placeholders = (2..=field_names.len()+1).map(|n| format!("${n}")).collect::<Vec<_>>().join(", ");
        let updates_clause = field_names.iter().map(|f| format!("{f} = EXCLUDED.{f}")).collect::<Vec<_>>().join(", ");
        
        let query_str = format!(
            "INSERT INTO profile_embeddings (fid, {field_list}) VALUES ($1, {placeholders})
             ON CONFLICT (fid) DO UPDATE SET {updates_clause}"
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

        let profiles = sqlx::query_as(
            r"
            SELECT p.*
            FROM user_profiles p
            JOIN profile_embeddings e ON p.fid = e.fid
            WHERE e.profile_embedding IS NOT NULL
                AND (e.profile_embedding <=> $1::vector) < $2
            ORDER BY e.profile_embedding <=> $1::vector
            LIMIT $3
            ",
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

        let profiles = sqlx::query_as(
            r"
            SELECT p.*
            FROM user_profiles p
            JOIN profile_embeddings e ON p.fid = e.fid
            WHERE e.bio_embedding IS NOT NULL
                AND (e.bio_embedding <=> $1::vector) < $2
            ORDER BY e.bio_embedding <=> $1::vector
            LIMIT $3
            ",
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
                    r"
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
                    ",
                )
                .bind(embedding)
                .bind(format!("%{text}%"))
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
