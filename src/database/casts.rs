use super::Database;
use crate::models::Cast;
use crate::models::CastQuery;
use crate::models::CastSearchResult;
use crate::models::CastSortBy;
use crate::models::CastStats;
use crate::models::SortOrder;
use crate::Result;

/// Cast thread structure
#[derive(Debug, Clone)]
pub struct CastThread {
    pub root: Option<Cast>,
    pub parents: Vec<Cast>,
    pub children: Vec<Cast>,
}

impl Database {
    /// List casts with filters
    pub async fn list_casts(&self, query: CastQuery) -> Result<Vec<Cast>> {
        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

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

            if let Some(_fid) = query.fid {
                conditions.push(format!("fid = ${param_idx}"));
                param_idx += 1;
            }

            if let Some(_text_search) = &query.text_search {
                conditions.push(format!("text ILIKE ${param_idx}"));
                param_idx += 1;
            }

            if let Some(_parent_hash) = &query.parent_hash {
                conditions.push(format!("parent_hash = ${param_idx}"));
                param_idx += 1;
            }

            if let Some(_start_timestamp) = query.start_timestamp {
                conditions.push(format!("timestamp >= ${param_idx}"));
                param_idx += 1;
            }

            if let Some(_end_timestamp) = query.end_timestamp {
                conditions.push(format!("timestamp <= ${param_idx}"));
                // param_idx would be incremented here if we had more conditions
            }

            let where_clause = conditions.join(" AND ");
            let order_by = match query.sort_by {
                Some(CastSortBy::Timestamp) => "timestamp",
                Some(CastSortBy::Fid) => "fid",
                _ => "timestamp",
            };
            let order_dir = match query.sort_order {
                Some(SortOrder::Asc) => "ASC",
                _ => "DESC",
            };

            let sql = format!(
                "SELECT * FROM casts WHERE {where_clause} ORDER BY {order_by} {order_dir} LIMIT {limit} OFFSET {offset}"
            );

            let mut q = sqlx::query_as::<_, Cast>(&sql);

            if let Some(fid) = query.fid {
                q = q.bind(fid);
            }
            if let Some(text_search) = &query.text_search {
                q = q.bind(format!("%{text_search}%"));
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
            sqlx::query_as::<_, Cast>(
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
    ) -> Result<Vec<Cast>> {
        let offset = offset.unwrap_or(0);

        // If no limit specified, fetch all casts (use a very large number)
        // This is more efficient than dynamic SQL construction
        let limit = limit.unwrap_or(1_000_000);

        let casts = sqlx::query_as::<_, Cast>(
            "SELECT * FROM casts WHERE fid = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
        )
        .bind(fid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(casts)
    }

    /// Count casts without embeddings (optimized for large datasets)
    pub async fn count_casts_without_embeddings(&self) -> Result<i64> {
        // For large datasets, it's much faster to calculate:
        // total_casts - existing_embeddings = missing_embeddings
        // This avoids the expensive NOT IN subquery on 200M+ rows
        
        let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts")
            .fetch_one(&self.pool)
            .await?;
            
        let existing_embeddings = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cast_embeddings")
            .fetch_one(&self.pool)
            .await?;
            
        let missing = total_casts - existing_embeddings;
        
        tracing::debug!(
            "Count calculation: {} total casts - {} existing embeddings = {} missing",
            total_casts, existing_embeddings, missing
        );
        
        Ok(missing)
    }

    /// Get casts without embeddings (optimized for large datasets)
    pub async fn get_casts_without_embeddings(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Cast>> {
        let casts = sqlx::query_as::<_, Cast>(
            r"
            SELECT c.* 
            FROM casts c
            WHERE c.message_hash NOT IN (
                SELECT message_hash FROM cast_embeddings
            )
            AND c.text IS NOT NULL 
            AND length(c.text) > 0
            ORDER BY c.timestamp DESC
            LIMIT $1 OFFSET $2
            ",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(casts)
    }

    /// Check which message hashes from a list don't have embeddings
    /// Returns a `HashSet` of message hashes that need embeddings
    pub async fn get_missing_embeddings(
        &self,
        message_hashes: &[Vec<u8>],
    ) -> Result<std::collections::HashSet<Vec<u8>>> {
        if message_hashes.is_empty() {
            return Ok(std::collections::HashSet::new());
        }

        // Get all hashes that already have embeddings
        let existing = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT message_hash FROM cast_embeddings WHERE message_hash = ANY($1)",
        )
        .bind(message_hashes)
        .fetch_all(&self.pool)
        .await?;

        let existing_set: std::collections::HashSet<Vec<u8>> = existing.into_iter().collect();

        // Return hashes that are NOT in the existing set
        let missing: std::collections::HashSet<Vec<u8>> = message_hashes
            .iter()
            .filter(|hash| !existing_set.contains(*hash))
            .cloned()
            .collect();

        Ok(missing)
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
            r"
            INSERT INTO cast_embeddings (message_hash, fid, text, embedding)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (message_hash) 
            DO UPDATE SET 
                embedding = EXCLUDED.embedding,
                updated_at = NOW()
            ",
        )
        .bind(message_hash)
        .bind(fid)
        .bind(text)
        .bind(embedding)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Semantic search for casts (lightweight version without engagement metrics)
    pub async fn semantic_search_casts_simple(
        &self,
        query_embedding: Vec<f32>,
        limit: i64,
        threshold: Option<f32>,
    ) -> Result<Vec<CastSearchResult>> {
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
            similarity: f64, // PostgreSQL returns FLOAT8 (f64) from distance operator
        }

        let raw_results = sqlx::query_as::<_, RawResult>(
            r"
            SELECT 
                ce.message_hash,
                ce.fid,
                ce.text,
                c.timestamp,
                c.parent_hash,
                c.embeds,
                c.mentions,
                1 - (ce.embedding <=> $1::vector) as similarity
            FROM cast_embeddings ce
            INNER JOIN casts c ON ce.message_hash = c.message_hash
            WHERE 1 - (ce.embedding <=> $1::vector) > $2
            ORDER BY ce.embedding <=> $1::vector
            LIMIT $3
            ",
        )
        .bind(&query_embedding)
        .bind(threshold_val)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = raw_results
            .into_iter()
            .map(|r| CastSearchResult {
                message_hash: r.message_hash,
                fid: r.fid,
                text: r.text,
                timestamp: r.timestamp,
                parent_hash: r.parent_hash,
                embeds: r.embeds,
                mentions: r.mentions,
                similarity: r.similarity as f32, // Convert f64 to f32
                reply_count: 0,                  // Not calculated in simple version
                reaction_count: 0,               // Not calculated in simple version
            })
            .collect();

        Ok(results)
    }

    /// Semantic search for casts with engagement metrics
    pub async fn semantic_search_casts(
        &self,
        query_embedding: Vec<f32>,
        limit: i64,
        threshold: Option<f32>,
    ) -> Result<Vec<CastSearchResult>> {
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
            similarity: f64, // PostgreSQL returns FLOAT8 (f64) from distance operator
            reply_count: Option<i64>,
            reaction_count: Option<i64>,
        }

        let raw_results = sqlx::query_as::<_, RawResult>(
            r"
            SELECT 
                ce.message_hash,
                ce.fid,
                ce.text,
                c.timestamp,
                c.parent_hash,
                c.embeds,
                c.mentions,
                1 - (ce.embedding <=> $1::vector) as similarity,
                (SELECT COUNT(*) FROM casts WHERE parent_hash = ce.message_hash) as reply_count,
                (SELECT COUNT(*) FROM (
                    SELECT *, ROW_NUMBER() OVER (
                        PARTITION BY fid, target_cast_hash 
                        ORDER BY timestamp DESC
                    ) as rn
                    FROM reactions
                    WHERE target_cast_hash = ce.message_hash
                ) r WHERE r.rn = 1 AND r.event_type = 'add') as reaction_count
            FROM cast_embeddings ce
            INNER JOIN casts c ON ce.message_hash = c.message_hash
            WHERE 1 - (ce.embedding <=> $1::vector) > $2
            ORDER BY ce.embedding <=> $1::vector
            LIMIT $3
            ",
        )
        .bind(&query_embedding)
        .bind(threshold_val)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = raw_results
            .into_iter()
            .map(|r| CastSearchResult {
                message_hash: r.message_hash,
                fid: r.fid,
                text: r.text,
                timestamp: r.timestamp,
                parent_hash: r.parent_hash,
                embeds: r.embeds,
                mentions: r.mentions,
                similarity: r.similarity as f32, // Convert f64 to f32
                reply_count: r.reply_count.unwrap_or(0),
                reaction_count: r.reaction_count.unwrap_or(0),
            })
            .collect();

        Ok(results)
    }

    /// Get cast statistics (replies, reactions, etc.)
    pub async fn get_cast_stats(&self, message_hash: &[u8]) -> Result<CastStats> {
        let stats = sqlx::query_as::<_, CastStats>(
            r"
            SELECT 
                $1 as message_hash,
                (SELECT COUNT(*) FROM casts WHERE parent_hash = $1) as reply_count,
                (SELECT COUNT(*) FROM (
                    SELECT *, ROW_NUMBER() OVER (
                        PARTITION BY fid, target_cast_hash 
                        ORDER BY timestamp DESC
                    ) as rn
                    FROM reactions
                    WHERE target_cast_hash = $1
                ) r WHERE r.rn = 1 AND r.event_type = 'add') as reaction_count,
                (SELECT COUNT(DISTINCT fid) FROM (
                    SELECT fid, ROW_NUMBER() OVER (
                        PARTITION BY fid, target_cast_hash 
                        ORDER BY timestamp DESC
                    ) as rn
                    FROM reactions
                    WHERE target_cast_hash = $1
                ) r WHERE r.rn = 1 AND r.event_type = 'add') as unique_reactors
            ",
        )
        .bind(message_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    /// Get cast by message hash
    pub async fn get_cast_by_hash(&self, message_hash: Vec<u8>) -> Result<Option<Cast>> {
        let cast = sqlx::query_as::<_, Cast>("SELECT * FROM casts WHERE message_hash = $1")
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
    ) -> Result<Vec<Cast>> {
        let casts = sqlx::query_as::<_, Cast>(
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
