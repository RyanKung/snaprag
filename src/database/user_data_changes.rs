use super::Database;
use crate::models::*;
use crate::Result;

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
