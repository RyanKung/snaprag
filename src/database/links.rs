use super::Database;
use crate::models::Link;
use crate::models::LinkQuery;
use crate::Result;

impl Database {
    /// List links with filters - using window function to get latest event per (fid, `target_fid`)
    pub async fn list_links(&self, query: LinkQuery) -> Result<Vec<Link>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Use window function to get latest event per (fid, target_fid) pair
        let links = match (query.fid, query.target_fid) {
            (Some(fid), Some(target_fid)) => {
                sqlx::query_as::<_, Link>(
                    r"
                    WITH latest_events AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_fid 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM links
                        WHERE fid = $1 AND target_fid = $2
                    )
                    SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                           created_at, shard_id, block_height, transaction_fid
                    FROM latest_events 
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $3 OFFSET $4
                    ",
                )
                .bind(fid)
                .bind(target_fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(fid), None) => {
                sqlx::query_as::<_, Link>(
                    r"
                    WITH latest_events AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_fid 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM links
                        WHERE fid = $1
                    )
                    SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                           created_at, shard_id, block_height, transaction_fid
                    FROM latest_events 
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $2 OFFSET $3
                    ",
                )
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(target_fid)) => {
                sqlx::query_as::<_, Link>(
                    r"
                    WITH latest_events AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_fid 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM links
                        WHERE target_fid = $1
                    )
                    SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                           created_at, shard_id, block_height, transaction_fid
                    FROM latest_events 
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $2 OFFSET $3
                    ",
                )
                .bind(target_fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, Link>(
                    r"
                    WITH latest_events AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_fid 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM links
                    )
                    SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                           created_at, shard_id, block_height, transaction_fid
                    FROM latest_events 
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $1 OFFSET $2
                    ",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        };

        Ok(links)
    }

    /// Get links by FID - using window function to get latest event per target
    pub async fn get_links_by_fid(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            r"
            WITH latest_events AS (
                SELECT *, ROW_NUMBER() OVER (
                    PARTITION BY fid, target_fid 
                    ORDER BY timestamp DESC
                ) as rn
                FROM links
                WHERE fid = $1
            )
            SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                   created_at, shard_id, block_height, transaction_fid
            FROM latest_events 
            WHERE rn = 1 AND event_type = 'add'
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get followers for a user (only active, using window function)
    pub async fn get_followers(
        &self,
        target_fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            r"
            WITH latest_events AS (
                SELECT *, ROW_NUMBER() OVER (
                    PARTITION BY fid, target_fid 
                    ORDER BY timestamp DESC
                ) as rn
                FROM links
                WHERE target_fid = $1 AND link_type = 'follow'
            )
            SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                   created_at, shard_id, block_height, transaction_fid
            FROM latest_events 
            WHERE rn = 1 AND event_type = 'add'
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(target_fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get following for a user (only active, using window function)
    pub async fn get_following(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            r"
            WITH latest_events AS (
                SELECT *, ROW_NUMBER() OVER (
                    PARTITION BY fid, target_fid 
                    ORDER BY timestamp DESC
                ) as rn
                FROM links
                WHERE fid = $1 AND link_type = 'follow'
            )
            SELECT id, fid, target_fid, link_type, event_type, timestamp, message_hash, 
                   created_at, shard_id, block_height, transaction_fid
            FROM latest_events 
            WHERE rn = 1 AND event_type = 'add'
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            ",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }
}
