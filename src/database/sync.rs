use super::Database;
use crate::Result;

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

impl Database {
    /// Get the last processed height for a shard
    pub async fn get_last_processed_height(&self, shard_id: u32) -> Result<u64> {
        let row = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT last_processed_height FROM sync_progress WHERE shard_id = $1",
        )
        .bind(shard_id as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.flatten().map_or(0, |h| h as u64))
    }

    /// Update the last processed height for a shard
    pub async fn update_last_processed_height(&self, shard_id: u32, height: u64) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO sync_progress (shard_id, last_processed_height, status, updated_at)
            VALUES ($1, $2, 'syncing', NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                last_processed_height = EXCLUDED.last_processed_height,
                status = 'syncing',
                updated_at = NOW()
            ",
        )
        .bind(shard_id as i32)
        .bind(height as i64)
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
        sqlx::query(
            r"
            INSERT INTO sync_progress (shard_id, status, error_message, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                status = EXCLUDED.status,
                error_message = EXCLUDED.error_message,
                updated_at = NOW()
            ",
        )
        .bind(shard_id as i32)
        .bind(status)
        .bind(error_message)
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
        sqlx::query(
            r"
            INSERT INTO processed_messages (
                message_hash, shard_id, block_height, transaction_fid,
                message_type, fid, timestamp, content_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (message_hash) DO NOTHING
            ",
        )
        .bind(message_hash)
        .bind(shard_id as i32)
        .bind(block_height as i64)
        .bind(transaction_fid as i64)
        .bind(message_type)
        .bind(fid as i64)
        .bind(timestamp)
        .bind(content_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if a message has been processed
    pub async fn is_message_processed(&self, message_hash: &[u8]) -> Result<bool> {
        let row = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM processed_messages WHERE message_hash = $1 LIMIT 1",
        )
        .bind(message_hash)
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
        sqlx::query(
            r"
            INSERT INTO sync_stats (shard_id, total_messages, total_blocks, last_updated)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (shard_id)
            DO UPDATE SET
                total_messages = EXCLUDED.total_messages,
                total_blocks = EXCLUDED.total_blocks,
                last_updated = NOW()
            ",
        )
        .bind(shard_id as i32)
        .bind(total_messages as i64)
        .bind(total_blocks as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get sync statistics for all shards
    pub async fn get_sync_stats(&self) -> Result<Vec<SyncStats>> {
        let stats = sqlx::query_as::<_, SyncStats>(
            r"
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
            ",
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
        sqlx::query(
            r"
            INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_hash) DO NOTHING
            ",
        )
        .bind(fid)
        .bind(target_fid)
        .bind(link_type)
        .bind(timestamp)
        .bind(message_hash)
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
        sqlx::query(
            r"
            INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (message_hash) DO NOTHING
            "
        )
        .bind(fid)
        .bind(text)
        .bind(timestamp)
        .bind(message_hash)
        .bind(parent_hash)
        .bind(root_hash)
        .bind(embeds)
        .bind(mentions)
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
        sqlx::query(
            r"
            INSERT INTO user_data (fid, data_type, value, timestamp, message_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (message_hash) DO NOTHING
            ",
        )
        .bind(fid)
        .bind(data_type)
        .bind(value)
        .bind(timestamp)
        .bind(message_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
