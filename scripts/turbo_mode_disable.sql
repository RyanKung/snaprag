-- Turbo Mode Disable: Rebuild all indexes for normal operation
-- Run AFTER bulk sync completes
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/turbo_mode_disable.sql

\echo '🔨 Rebuilding indexes (this may take 30-60 minutes)...'

-- Reactions indexes
\echo '  📊 reactions table...'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_fid ON reactions(fid);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_target_cast ON reactions(target_cast_hash);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_target_fid ON reactions(target_fid);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_type ON reactions(reaction_type);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_timestamp ON reactions(timestamp DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_latest ON reactions(fid, target_cast_hash, timestamp DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_reactions_event_type ON reactions(event_type);

-- Links indexes
\echo '  🔗 links table...'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_latest ON links(fid, target_fid, timestamp DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_event_type ON links(event_type);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_links_fid_type ON links(fid, link_type);

-- Verifications indexes
\echo '  ✅ verifications table...'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_fid ON verifications(fid);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_address ON verifications(address);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_timestamp ON verifications(timestamp DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_latest ON verifications(fid, address, timestamp DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_event_type ON verifications(event_type);

-- Casts indexes
\echo '  📝 casts table...'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_casts_fid ON casts(fid);

-- Processed messages indexes
\echo '  📦 processed_messages table...'
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_processed_shard_height ON processed_messages(shard_id, block_height DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_processed_messages_hash ON processed_messages(message_hash);

-- Re-enable autovacuum
\echo '  🔄 Re-enabling autovacuum...'
ALTER TABLE reactions SET (autovacuum_enabled = true);
ALTER TABLE links SET (autovacuum_enabled = true);
ALTER TABLE verifications SET (autovacuum_enabled = true);
ALTER TABLE casts SET (autovacuum_enabled = true);
ALTER TABLE processed_messages SET (autovacuum_enabled = true);
ALTER TABLE user_profile_changes SET (autovacuum_enabled = true);

-- Vacuum and analyze
\echo '  🧹 Running VACUUM ANALYZE...'
VACUUM ANALYZE reactions;
VACUUM ANALYZE links;
VACUUM ANALYZE verifications;
VACUUM ANALYZE casts;

\echo '✅ Normal operation mode restored!'
\echo '📊 All indexes rebuilt'
\echo '🔄 Autovacuum re-enabled'

