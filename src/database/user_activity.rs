use super::Database;
use crate::models::*;
use crate::Result;

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
