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

    // Batch insert FIDs (split into chunks to avoid parameter limit)
    // 🚀 APPEND-ONLY MODE: New table structure uses (fid, timestamp) as primary key
    // This allows multiple rows per FID, eliminating lock contention from ON CONFLICT
    if !batched.fids_to_ensure.is_empty() {
        let now = chrono::Utc::now();

        const PARAMS_PER_ROW: usize = 3;
        const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~21666 rows per chunk

        // 🔧 Sort FIDs to ensure consistent insertion order
        let mut fids: Vec<i64> = batched.fids_to_ensure.iter().copied().collect();
        fids.sort_unstable();

        // Split FIDs into chunks
        for chunk in fids.chunks(CHUNK_SIZE) {
            // Build dynamic query for batch insert
            // 🚀 Pre-allocate capacity to reduce allocations
            let estimated_size = 100 + chunk.len() * 20; // Rough estimate
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO user_profiles (fid, last_updated_timestamp, last_updated_at) VALUES ");

            // 🚀 Use direct string building instead of collecting Vec<String>
            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!("(${}, ${}, ${})", base + 1, base + 2, base + 3));
            }
            // 🚀 NO ON CONFLICT - append-only mode with (fid, timestamp) primary key
            // Duplicates are handled by composite primary key constraint
            query.push_str(" ON CONFLICT (fid, last_updated_timestamp) DO NOTHING");

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

        const PARAMS_PER_ROW: usize = 10; // Added shard_id and block_height
        const MAX_PARAMS: usize = 65000; // Keep below u16::MAX (65535)
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW; // ~6500 rows per chunk

        // Split casts into chunks
        for chunk in deduped_casts.chunks(CHUNK_SIZE) {
            // Build dynamic query
            // 🚀 Pre-allocate capacity
            let estimated_size = 150 + chunk.len() * 60;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO casts (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions, shard_id, block_height) VALUES ");

            // 🚀 Direct string building
            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4, base + 5,
                    base + 6, base + 7, base + 8, base + 9, base + 10
                ));
            }

            // 🚀 CRITICAL FIX: Use DO NOTHING for re-sync performance
            // Casts are immutable - if message_hash exists, no need to update
            // This prevents 166M+ unnecessary updates on re-sync
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for (fid, text, timestamp, message_hash, parent_hash, root_hash, embeds, mentions, shard_block_info) in
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
                    .bind(mentions)
                    .bind(shard_block_info.shard_id as i32)
                    .bind(shard_block_info.block_height as i64);
            }

            q.execute(&mut *tx).await?;
        }
    }

    // Batch insert links (split into chunks to avoid parameter limit)
    if !batched.links.is_empty() {
        tracing::info!("📎 Batch inserting {} links", batched.links.len());

        const PARAMS_PER_ROW: usize = 7; // fid, target_fid, link_type, timestamp, message_hash, shard_id, block_height
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.links.chunks(CHUNK_SIZE) {
            // 🚀 Pre-allocate
            let estimated_size = 150 + chunk.len() * 45;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO links (fid, target_fid, link_type, timestamp, message_hash, shard_id, block_height) VALUES ");

            // 🚀 Direct building
            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4,
                    base + 5, base + 6, base + 7
                ));
            }

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
        tracing::info!("❤️  Batch inserting {} reactions", batched.reactions.len());

        const PARAMS_PER_ROW: usize = 9; // fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, shard_id, block_height, transaction_fid
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.reactions.chunks(CHUNK_SIZE) {
            // 🚀 Pre-allocate
            let estimated_size = 200 + chunk.len() * 60;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO reactions (fid, target_cast_hash, target_fid, reaction_type, timestamp, message_hash, shard_id, block_height, transaction_fid) VALUES ");

            // 🚀 Direct building
            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4, base + 5,
                    base + 6, base + 7, base + 8, base + 9
                ));
            }

            // Only check message_hash (composite constraint will be removed in migration 007)
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
        tracing::info!(
            "✅ Batch inserting {} verifications",
            batched.verifications.len()
        );

        const PARAMS_PER_ROW: usize = 11; // fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, shard_id, block_height, transaction_fid
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        for chunk in batched.verifications.chunks(CHUNK_SIZE) {
            // 🚀 Pre-allocate
            let estimated_size = 250 + chunk.len() * 70;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO verifications (fid, address, claim_signature, block_hash, verification_type, chain_id, timestamp, message_hash, shard_id, block_height, transaction_fid) VALUES ");

            // 🚀 Direct building
            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4, base + 5, base + 6,
                    base + 7, base + 8, base + 9, base + 10, base + 11
                ));
            }

            // Only check message_hash (composite constraint will be removed in migration 007)
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

    // 🚀 APPEND-ONLY MODE: Convert UPDATEs to INSERTs
    // New table structure: (fid, timestamp) as primary key
    // This eliminates UPDATE locks entirely - just append new rows!
    if !batched.profile_updates.is_empty() {
        tracing::trace!(
            "Batch inserting {} profile updates (append-only)",
            batched.profile_updates.len()
        );

        const PARAMS_PER_ROW: usize = 4; // fid, field_value, timestamp, created_at
        const MAX_PARAMS: usize = 65000;
        
        // Group updates by field name to batch insert per field type
        let mut updates_by_field: HashMap<String, Vec<(i64, Option<String>, i64)>> = HashMap::new();

        for (fid, field_name, value, timestamp) in batched.profile_updates {
            updates_by_field
                .entry(field_name)
                .or_insert_with(Vec::new)
                .push((fid, value, timestamp));
        }

        let now = chrono::Utc::now();

        // Insert each field type as a batch
        for (field_name, updates) in updates_by_field {
            if updates.is_empty() {
                continue;
            }

            let chunk_size = MAX_PARAMS / PARAMS_PER_ROW;
            
            for chunk in updates.chunks(chunk_size) {
                let estimated_size = 200 + chunk.len() * 40;
                let mut query = String::with_capacity(estimated_size);
                query.push_str(&format!(
                    "INSERT INTO user_profiles (fid, {}, last_updated_timestamp, last_updated_at) VALUES ",
                    field_name
                ));

                for i in 0..chunk.len() {
                    if i > 0 {
                        query.push_str(", ");
                    }
                    let base = i * PARAMS_PER_ROW;
                    query.push_str(&format!(
                        "(${}, ${}, ${}, ${})",
                        base + 1, base + 2, base + 3, base + 4
                    ));
                }
                
                // 🚀 Append-only: Use composite primary key (fid, timestamp)
                // No lock contention - just skip if exact same (fid, timestamp) exists
                query.push_str(" ON CONFLICT (fid, last_updated_timestamp) DO NOTHING");

                let mut q = sqlx::query(&query);
                for (fid, value, timestamp) in chunk {
                    q = q.bind(fid).bind(value).bind(timestamp).bind(now);
                }

                q.execute(&mut *tx).await?;
            }
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
