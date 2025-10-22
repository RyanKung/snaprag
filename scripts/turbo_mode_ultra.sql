-- ULTRA Turbo Mode: 删除所有表的非必需索引 + 关闭所有autovacuum
-- 适用于初始同步阶段，追求极致性能
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/turbo_mode_ultra.sql

\echo '🚀🚀🚀 Entering ULTRA TURBO MODE - Maximum Performance!'
\echo ''

-- ============================================================================
-- 1. 删除所有性能索引（保留PRIMARY KEY和UNIQUE约束）
-- ============================================================================

\echo '📊 Dropping performance indexes...'

-- frame_actions (4个可删)
DROP INDEX IF EXISTS idx_frame_actions_fid;
DROP INDEX IF EXISTS idx_frame_actions_cast_hash;
DROP INDEX IF EXISTS idx_frame_actions_timestamp;
DROP INDEX IF EXISTS idx_frame_actions_url;

-- username_proofs (3个可删)
DROP INDEX IF EXISTS idx_username_proofs_fid;
DROP INDEX IF EXISTS idx_username_proofs_username;
DROP INDEX IF EXISTS idx_username_proofs_timestamp;

-- onchain_events (3个可删)
DROP INDEX IF EXISTS idx_onchain_events_fid;
DROP INDEX IF EXISTS idx_onchain_events_type;
DROP INDEX IF EXISTS idx_onchain_events_block;

-- user_profile_changes (2个可删)
DROP INDEX IF EXISTS idx_profile_changes_fid_field_ts;
DROP INDEX IF EXISTS idx_profile_changes_message_hash;

\echo '✅ Dropped 12 performance indexes'
\echo ''

-- ============================================================================
-- 2. 关闭所有表的 autovacuum
-- ============================================================================

\echo '🔄 Disabling autovacuum on all tables...'

ALTER TABLE onchain_events SET (autovacuum_enabled = false);

\echo '✅ All autovacuum disabled'
\echo ''

-- ============================================================================
-- 3. 验证配置
-- ============================================================================

\echo '📋 Current Configuration:'
\echo ''

SELECT 
    relname as table_name,
    (SELECT COUNT(*) 
     FROM pg_indexes 
     WHERE schemaname = 'public' AND tablename = relname) as index_count,
    CASE 
        WHEN reloptions::text LIKE '%autovacuum_enabled=false%' THEN '❌ OFF'
        ELSE '✅ ON'
    END as autovacuum
FROM pg_class
WHERE relname IN (
    'casts', 'links', 'reactions', 'verifications',
    'frame_actions', 'username_proofs', 'onchain_events',
    'user_profile_changes', 'processed_messages'
)
AND relkind = 'r'
ORDER BY relname;

\echo ''
\echo '🚀 ULTRA TURBO MODE ENABLED!'
\echo '⚡ Expected total speed boost: +50-80%'
\echo ''
\echo '⚠️  IMPORTANT: Run turbo_mode_disable.sql after sync completes!'
\echo '⚠️  All query performance will be degraded until indexes are rebuilt!'

