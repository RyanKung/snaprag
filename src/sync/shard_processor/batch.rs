use std::collections::HashMap;

use tracing::warn;

use super::types::BatchedData;
use crate::database::Database;
use crate::Result;

/// Flush batched data to database
pub(super) async fn flush_batched_data(database: &Database, batched: BatchedData) -> Result<()> {
    let start = std::time::Instant::now();
    tracing::trace!(
        "Flushing batch: {} FIDs, {} casts, {} links, {} reactions, {} verifications, {} profile updates",
        batched.fids_to_ensure.len(),
        batched.casts.len(),
        batched.links.len(),
        batched.reactions.len(),
        batched.verifications.len(),
        batched.profile_updates.len()
    );

    // Start a transaction for the entire batch
    let mut tx = database.pool().begin().await?;
    
    // 🔧 Set statement timeout to prevent long-running transactions from blocking
    sqlx::query("SET LOCAL statement_timeout = '30s'")
        .execute(&mut *tx)
        .await?;

    // Batch insert FIDs (split into chunks to avoid parameter limit)
    if !batched.fids_to_ensure.is_empty() {
        let now = chrono::Utc::now();

        const PARAMS_PER_ROW: usize = 3;
        const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~21666 rows per chunk

        // 🔧 Sort FIDs to ensure consistent lock acquisition order (reduce deadlocks)
        let mut fids: Vec<i64> = batched.fids_to_ensure.iter().copied().collect();
        fids.sort_unstable();

        // Split FIDs into chunks
        for chunk in fids.chunks(CHUNK_SIZE) {
            // Build dynamic query for batch insert
            let mut query = String::from(
                "INSERT INTO user_profiles (fid, last_updated_timestamp, last_updated_at) VALUES ",
            );

            let value_clauses: Vec<String> = (0..chunk.len())
                .map(|i| {
                    let base = i * PARAMS_PER_ROW;
                    format!("(${}, ${}, ${})", base + 1, base + 2, base + 3)
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(" ON CONFLICT (fid) DO NOTHING");

            let mut q = sqlx::query(&query);
            for _fid in chunk {
                q = q.bind(_fid).bind(0i64).bind(now);
            }

            let result = q.execute(&mut *tx).await?;
            let profiles_created = result.rows_affected();

            if profiles_created > 0 {
                tracing::debug!("Created {} new profiles", profiles_created);
            }
        }
    }

    // Batch insert casts (split into chunks to avoid parameter limit)
    if !batched.casts.is_empty() {
        tracing::trace!(
            "Batch inserting {} casts (before dedup)",
            batched.casts.len()
        );

        // 🚀 CRITICAL FIX: Deduplicate by message_hash to avoid "affect row a second time" error
        // Keep the latest version of each cast (by timestamp)
        let mut casts_map: HashMap<Vec<u8>, _> = HashMap::new();
        let original_count = batched.casts.len();
        for cast in &batched.casts {
            let hash = cast.3.clone(); // message_hash
            casts_map.insert(hash, cast.clone());
        }
        let deduped_casts: Vec<_> = casts_map.into_values().collect();
        let deduped_count = deduped_casts.len();
        if original_count != deduped_count {
            tracing::debug!(
                "Deduplicated casts: {} -> {} ({} duplicates removed)",
                original_count,
                deduped_count,
                original_count - deduped_count
            );
        }

        const PARAMS_PER_ROW: usize = 8;
        const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~8125 rows per chunk

        // Split casts into chunks
        for chunk in deduped_casts.chunks(CHUNK_SIZE) {
            // Build dynamic query
            let mut query = String::from(
                "INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions) VALUES "
            );

            let value_clauses: Vec<String> = (0..chunk.len())
                .map(|i| {
                    let base = i * PARAMS_PER_ROW;
                    format!(
                        "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5,
                        base + 6,
                        base + 7,
                        base + 8
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(
                " ON CONFLICT (message_hash) DO UPDATE SET \
                fid = EXCLUDED.fid, \
                text = EXCLUDED.text, \
                timestamp = EXCLUDED.timestamp, \
                parent_hash = EXCLUDED.parent_hash, \
                root_hash = EXCLUDED.root_hash, \
                embeds = EXCLUDED.embeds, \
                mentions = EXCLUDED.mentions",
            );

            let mut q = sqlx::query(&query);
            for (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions) in
                chunk
            {
                q = q
                    .bind(fid)
                    .bind(text)
                    .bind(timestamp)
                    .bind(message_hash)
                    .bind(parent_hash)
                    .bind(root_hash)
                    .bind(embeds)
                    .bind(mentions);
            }

            q.execute(&mut *tx).await?;
        }
    }

    // Batch insert links (split into chunks to avoid parameter limit)
    if !batched.links.is_empty() {
        tracing::trace!("Batch inserting {} links", batched.links.len());

        const PARAMS_PER_ROW: usize = 7; // fid, target_fid, link_type, timestamp, message_hash, shard_id, block_height
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.links.chunks(CHUNK_SIZE) {
            let mut query = String::from(
                "INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash, shard_id, block_height) VALUES "
            );

            let value_clauses: Vec<String> = (0..chunk.len())
                .map(|i| {
                    let base = i * PARAMS_PER_ROW;
                    format!(
                        "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5,
                        base + 6,
                        base + 7
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for (fid, target_fid, link_type, timestamp, message_hash, shard_block_info) in chunk {
                q = q
                    .bind(fid)
                    .bind(target_fid)
                    .bind(link_type)
                    .bind(timestamp)
                    .bind(message_hash)
                    .bind(shard_block_info.shard_id as i32)
                    .bind(shard_block_info.block_height as i64);
            }

            q.execute(&mut *tx).await?;
        }
    }

    // Batch insert reactions (split into chunks to avoid parameter limit)
    if !batched.reactions.is_empty() {
        tracing::trace!("Batch inserting {} reactions", batched.reactions.len());

        const PARAMS_PER_ROW: usize = 9; // fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, shard_id, block_height, transaction_fid
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.reactions.chunks(CHUNK_SIZE) {
            let mut query = String::from(
                "INSERT INTO reactions (fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, shard_id, block_height, transaction_fid) VALUES "
            );

            let value_clauses: Vec<String> = (0..chunk.len())
                .map(|i| {
                    let base = i * PARAMS_PER_ROW;
                    format!(
                        "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5,
                        base + 6,
                        base + 7,
                        base + 8,
                        base + 9
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for (
                fid,
                target_cast_hash,
                target_fid,
                reaction_type,
                timestamp,
                message_hash,
                shard_block_info,
            ) in chunk
            {
                q = q
                    .bind(fid)
                    .bind(target_cast_hash)
                    .bind(target_fid)
                    .bind(reaction_type)
                    .bind(timestamp)
                    .bind(message_hash)
                    .bind(shard_block_info.shard_id as i32)
                    .bind(shard_block_info.block_height as i64)
                    .bind(shard_block_info.transaction_fid as i64);
            }

            q.execute(&mut *tx).await?;
        }
    }

    // Batch insert verifications (split into chunks to avoid parameter limit)
    if !batched.verifications.is_empty() {
        tracing::trace!(
            "Batch inserting {} verifications",
            batched.verifications.len()
        );

        const PARAMS_PER_ROW: usize = 11; // fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, shard_id, block_height, transaction_fid
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.verifications.chunks(CHUNK_SIZE) {
            let mut query = String::from(
                "INSERT INTO verifications (fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, shard_id, block_height, transaction_fid) VALUES "
            );

            let value_clauses: Vec<String> = (0..chunk.len())
                .map(|i| {
                    let base = i * PARAMS_PER_ROW;
                    format!(
                        "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                        base + 1,
                        base + 2,
                        base + 3,
                        base + 4,
                        base + 5,
                        base + 6,
                        base + 7,
                        base + 8,
                        base + 9,
                        base + 10,
                        base + 11
                    )
                })
                .collect();

            query.push_str(&value_clauses.join(", "));
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for (
                fid,
                address,
                claim_signature,
                block_hash,
                verification_type,
                chain_id,
                timestamp,
                message_hash,
                shard_block_info,
            ) in chunk
            {
                q = q
                    .bind(fid)
                    .bind(address)
                    .bind(claim_signature)
                    .bind(block_hash)
                    .bind(verification_type)
                    .bind(chain_id)
                    .bind(timestamp)
                    .bind(message_hash)
                    .bind(shard_block_info.shard_id as i32)
                    .bind(shard_block_info.block_height as i64)
                    .bind(shard_block_info.transaction_fid as i64);
            }

            q.execute(&mut *tx).await?;
        }
    }

    // ❌ Removed: user_activity_timeline table dropped
    // Activities tracking was removed for performance (356GB, WAL bottleneck)
    // All necessary data is already in specialized tables (casts, links, reactions, etc.)

    // 🚀 OPTIMIZATION: Simplified profile updates using multiple simple UPDATEs
    // Instead of complex CASE statements, use multiple targeted updates
    // This is faster in Rust (less string allocation) and clearer
    if !batched.profile_updates.is_empty() {
        tracing::trace!(
            "Batch updating {} profile fields",
            batched.profile_updates.len()
        );

        // Group updates by field name
        let mut updates_by_field: HashMap<String, Vec<(i64, Option<String>, i64)>> = HashMap::new();

        for (fid, field_name, value, timestamp) in batched.profile_updates {
            updates_by_field
                .entry(field_name)
                .or_insert_with(Vec::new)
                .push((fid, value, timestamp));
        }

        let now = chrono::Utc::now();

        // 🚀 Use unnest() for batch updates - much faster!
        for (field_name, mut updates) in updates_by_field {
            if updates.is_empty() {
                continue;
            }

            // 🔧 Sort by FID to ensure consistent lock order (reduce deadlocks)
            updates.sort_by_key(|(fid, _, _)| *fid);

            let mut fids = Vec::with_capacity(updates.len());
            let mut values = Vec::with_capacity(updates.len());
            let mut timestamps = Vec::with_capacity(updates.len());

            for (fid, value, timestamp) in updates {
                fids.push(fid);
                values.push(value);
                timestamps.push(timestamp);
            }

            // Dynamic SQL with timestamp check - only update if newer or equal
            let sql = format!(
                r#"
                UPDATE user_profiles AS up
                SET {} = CASE 
                        WHEN data.timestamp >= up.last_updated_timestamp THEN data.value
                        ELSE up.{}
                    END,
                    last_updated_timestamp = GREATEST(data.timestamp, up.last_updated_timestamp),
                    last_updated_at = CASE 
                        WHEN data.timestamp >= up.last_updated_timestamp THEN $4
                        ELSE up.last_updated_at
                    END
                FROM unnest($1::bigint[], $2::text[], $3::bigint[]) 
                    AS data(fid, value, timestamp)
                WHERE up.fid = data.fid
                "#,
                field_name, field_name
            );

            sqlx::query(&sql)
                .bind(&fids)
                .bind(&values)
                .bind(&timestamps)
                .bind(now)
                .execute(&mut *tx)
                .await?;
        }
    }

    // Commit the transaction
    tx.commit().await?;

    let elapsed = start.elapsed();
    if elapsed.as_millis() > 1000 {
        warn!("Batch flush took {}ms (slow!)", elapsed.as_millis());
    } else {
        tracing::trace!("Batch flush completed in {}ms", elapsed.as_millis());
    }

    Ok(())
}
