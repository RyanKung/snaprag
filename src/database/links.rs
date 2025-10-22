use super::Database;
use crate::models::{LinkQuery, Link};
use crate::Result;

impl Database {
    /// List links with filters
    pub async fn list_links(&self, query: LinkQuery) -> Result<Vec<Link>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        // Execute query with parameters - only return active links (not removed)
        let links = match (query.fid, query.target_fid) {
            (Some(fid), Some(target_fid)) => {
                sqlx::query_as::<_, Link>(
                    "SELECT * FROM links WHERE fid = $1 AND target_fid = $2 AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $3 OFFSET $4"
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
                    "SELECT * FROM links WHERE fid = $1 AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
                )
                .bind(fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(target_fid)) => {
                sqlx::query_as::<_, Link>(
                    "SELECT * FROM links WHERE target_fid = $1 AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
                )
                .bind(target_fid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, Link>(
                    "SELECT * FROM links WHERE removed_at IS NULL ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
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
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            "SELECT * FROM links WHERE fid = $1 AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get followers for a user (only active, not removed)
    pub async fn get_followers(
        &self,
        target_fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            "SELECT * FROM links WHERE target_fid = $1 AND link_type = 'follow' AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
        )
        .bind(target_fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }

    /// Get following for a user (only active, not removed)
    pub async fn get_following(
        &self,
        fid: i64,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Link>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let links = sqlx::query_as::<_, Link>(
            "SELECT * FROM links WHERE fid = $1 AND link_type = 'follow' AND removed_at IS NULL ORDER BY timestamp DESC LIMIT $2 OFFSET $3"
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(links)
    }
}
