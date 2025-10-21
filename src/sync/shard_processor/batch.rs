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

    // Batch insert FIDs to user_profile_changes (event-sourcing table)
    // 🚀 EVENT-SOURCING MODE: Each FID creates a synthetic "fid_created" event
    // Pure append-only, zero locks
    if !batched.fids_to_ensure.is_empty() {
        let now = chrono::Utc::now();

        const PARAMS_PER_ROW: usize = 5; // fid, field_name, field_value, timestamp, message_hash
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        let mut fids: Vec<i64> = batched.fids_to_ensure.iter().copied().collect();
        fids.sort_unstable();

        for chunk in fids.chunks(CHUNK_SIZE) {
            let estimated_size = 150 + chunk.len() * 40;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO user_profile_changes (fid, field_name, field_value, timestamp, message_hash) VALUES ");

            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4, base + 5
                ));
            }
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for fid in chunk {
                // Create synthetic message_hash for fid_created event
                let synthetic_hash = format!("fid_created_{}", fid).as_bytes().to_vec();
                q = q
                    .bind(fid)
                    .bind("fid_created")
                    .bind::<Option<String>>(None) // No value for fid_created event
                    .bind(0i64)
                    .bind(synthetic_hash);
            }

            let result = q.execute(&mut *tx).await?;
            if result.rows_affected() > 0 {
                tracing::debug!("Created {} FID events", result.rows_affected());
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

    // 🚀 EVENT-SOURCING MODE: Insert individual field changes
    // Each update = one row in user_profile_changes table
    // Pure append-only, zero locks!
    if !batched.profile_updates.is_empty() {
        tracing::trace!(
            "Batch inserting {} profile field changes",
            batched.profile_updates.len()
        );

        // Each update is independent - no grouping needed
        const PARAMS_PER_ROW: usize = 5; // fid, field_name, field_value, timestamp, message_hash
        const MAX_PARAMS: usize = 65000;
        const CHUNK_SIZE: usize = MAX_PARAMS / PARAMS_PER_ROW;

        // Convert to list for chunking
        let updates_list: Vec<_> = batched.profile_updates.into_iter().collect();
        
        for chunk in updates_list.chunks(CHUNK_SIZE) {
            let estimated_size = 200 + chunk.len() * 50;
            let mut query = String::with_capacity(estimated_size);
            query.push_str("INSERT INTO user_profile_changes (fid, field_name, field_value, timestamp, message_hash) VALUES ");

            for i in 0..chunk.len() {
                if i > 0 {
                    query.push_str(", ");
                }
                let base = i * PARAMS_PER_ROW;
                query.push_str(&format!(
                    "(${}, ${}, ${}, ${}, ${})",
                    base + 1, base + 2, base + 3, base + 4, base + 5
                ));
            }
            
            query.push_str(" ON CONFLICT (message_hash) DO NOTHING");

            let mut q = sqlx::query(&query);
            for (fid, field_name, value, timestamp) in chunk {
                // Generate unique message_hash using a simple encoding
                // Format: "profile_{field}_{fid}_{timestamp}_{value_hash}"
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                field_name.hash(&mut hasher);
                fid.hash(&mut hasher);
                timestamp.hash(&mut hasher);
                if let Some(ref v) = value {
                    v.hash(&mut hasher);
                }
                let hash_value = hasher.finish();
                let message_hash = format!("profile_{}_{}", field_name, hash_value).as_bytes().to_vec();
                
                q = q
                    .bind(fid)
                    .bind(field_name)
                    .bind(value)
                    .bind(timestamp)
                    .bind(message_hash);
            }

            q.execute(&mut *tx).await?;
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
