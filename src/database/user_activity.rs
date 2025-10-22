use super::Database;
use crate::models::UserActivityTimeline;
use crate::Result;

impl Database {
    /// Get user activity timeline - aggregated from original tables (casts, links, reactions, etc.)
    /// No longer uses user_activity_timeline table (removed for performance)
    pub async fn get_user_activity_timeline(
        &self,
        fid: i64,
        activity_type: Option<String>,
        _start_timestamp: Option<i64>,
        _end_timestamp: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<UserActivityTimeline>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        // Aggregate from original tables using UNION ALL
        let query = if let Some(act_type) = activity_type {
            match act_type.as_str() {
                "cast_add" => {
                    r"
                    SELECT 
                        id,
                        fid,
                        'cast_add' as activity_type,
                        jsonb_build_object('text', text, 'parent_hash', parent_hash) as activity_data,
                        timestamp,
                        message_hash,
                        created_at,
                        shard_id,
                        block_height,
                        transaction_fid
                    FROM casts
                    WHERE fid = $1
                    ORDER BY timestamp DESC
                    LIMIT $2 OFFSET $3
                    "
                }
                "link_add" => {
                    r"
                    WITH latest_links AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_fid 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM links
                        WHERE fid = $1
                    )
                    SELECT 
                        id,
                        fid,
                        'link_add' as activity_type,
                        jsonb_build_object('target_fid', target_fid, 'link_type', link_type) as activity_data,
                        timestamp,
                        message_hash,
                        created_at,
                        shard_id,
                        block_height,
                        transaction_fid
                    FROM latest_links
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $2 OFFSET $3
                    "
                }
                "reaction_add" => {
                    r"
                    WITH latest_reactions AS (
                        SELECT *, ROW_NUMBER() OVER (
                            PARTITION BY fid, target_cast_hash 
                            ORDER BY timestamp DESC
                        ) as rn
                        FROM reactions
                        WHERE fid = $1
                    )
                    SELECT 
                        id,
                        fid,
                        'reaction_add' as activity_type,
                        jsonb_build_object('reaction_type', reaction_type, 'target_fid', target_fid) as activity_data,
                        timestamp,
                        message_hash,
                        created_at,
                        shard_id,
                        block_height,
                        transaction_fid
                    FROM latest_reactions
                    WHERE rn = 1 AND event_type = 'add'
                    ORDER BY timestamp DESC
                    LIMIT $2 OFFSET $3
                    "
                }
                "id_register" => {
                    r"
                    SELECT 
                        id,
                        fid,
                        'id_register' as activity_type,
                        event_data as activity_data,
                        block_timestamp as timestamp,
                        NULL::bytea as message_hash,
                        created_at,
                        shard_id,
                        shard_block_height as block_height,
                        NULL::bigint as transaction_fid
                    FROM onchain_events
                    WHERE fid = $1 AND event_type = 3
                    ORDER BY block_timestamp DESC
                    LIMIT $2 OFFSET $3
                    "
                }
                _ => {
                    // Unsupported activity type, return empty
                    return Ok(vec![]);
                }
            }
        } else {
            // Query all activities from all tables
            r"
            WITH all_activities AS (
                SELECT 
                    id,
                    fid,
                    'cast_add' as activity_type,
                    jsonb_build_object('text', COALESCE(text, '')) as activity_data,
                    timestamp,
                    message_hash,
                    created_at,
                    shard_id,
                    block_height,
                    transaction_fid
                FROM casts
                WHERE fid = $1
                
                UNION ALL
                
                SELECT 
                    l.id,
                    l.fid,
                    'link_add' as activity_type,
                    jsonb_build_object('target_fid', l.target_fid, 'link_type', l.link_type) as activity_data,
                    l.timestamp,
                    l.message_hash,
                    l.created_at,
                    l.shard_id,
                    l.block_height,
                    l.transaction_fid
                FROM (
                    SELECT *, ROW_NUMBER() OVER (
                        PARTITION BY fid, target_fid 
                        ORDER BY timestamp DESC
                    ) as rn
                    FROM links
                    WHERE fid = $1
                ) l
                WHERE l.rn = 1 AND l.event_type = 'add'
                
                UNION ALL
                
                SELECT 
                    r.id,
                    r.fid,
                    'reaction_add' as activity_type,
                    jsonb_build_object('reaction_type', r.reaction_type, 'target_fid', r.target_fid) as activity_data,
                    r.timestamp,
                    r.message_hash,
                    r.created_at,
                    r.shard_id,
                    r.block_height,
                    r.transaction_fid
                FROM (
                    SELECT *, ROW_NUMBER() OVER (
                        PARTITION BY fid, target_cast_hash 
                        ORDER BY timestamp DESC
                    ) as rn
                    FROM reactions
                    WHERE fid = $1
                ) r
                WHERE r.rn = 1 AND r.event_type = 'add'
            )
            SELECT * FROM all_activities
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "
        };

        let activities = sqlx::query_as::<_, UserActivityTimeline>(query)
            .bind(fid)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok(activities)
    }

    /// Record user activity - DEPRECATED
    /// This function is no longer used as user_activity_timeline table was removed
    #[deprecated(note = "user_activity_timeline table removed for performance")]
    pub async fn record_user_activity(
        &self,
        _fid: i64,
        _activity_type: String,
        _activity_data: Option<serde_json::Value>,
        _timestamp: i64,
        _message_hash: Option<Vec<u8>>,
    ) -> Result<UserActivityTimeline> {
        Err(crate::SnapRagError::Custom(
            "user_activity_timeline table removed for performance".to_string(),
        ))
    }

    /// Batch insert user activities - DEPRECATED
    #[deprecated(note = "user_activity_timeline table removed for performance")]
    pub async fn batch_insert_activities(
        &self,
        _activities: Vec<(i64, String, Option<serde_json::Value>, i64, Option<Vec<u8>>)>,
    ) -> Result<()> {
        Ok(()) // No-op
    }
}
