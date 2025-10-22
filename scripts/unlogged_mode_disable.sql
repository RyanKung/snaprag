-- UNLOGGED MODE Disable: 恢复正常的WAL日志
-- ⚠️ 警告：这会触发完整表重写，可能需要数小时
-- 
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/unlogged_mode_disable.sql

\echo ''
\echo '🔄 Converting UNLOGGED tables back to LOGGED...'
\echo ''
\echo '⚠️  This will rewrite all tables and may take several hours!'
\echo '⚠️  Ensure you have enough disk space (2x table size)'
\echo ''
\echo 'Press Ctrl+C to cancel, or wait 5 seconds to continue...'
SELECT pg_sleep(5);
\echo ''

\echo '📝 Converting tables to LOGGED (this will take time)...'

-- 核心数据表
\echo '  → casts...'
ALTER TABLE casts SET LOGGED;

\echo '  → links...'
ALTER TABLE links SET LOGGED;

\echo '  → reactions...'
ALTER TABLE reactions SET LOGGED;

\echo '  → verifications...'
ALTER TABLE verifications SET LOGGED;

-- 其他高频表
\echo '  → user_profile_changes...'
ALTER TABLE user_profile_changes SET LOGGED;

\echo '  → onchain_events...'
ALTER TABLE onchain_events SET LOGGED;

\echo '  → username_proofs...'
ALTER TABLE username_proofs SET LOGGED;

\echo '  → frame_actions...'
ALTER TABLE frame_actions SET LOGGED;

\echo '  → processed_messages...'
ALTER TABLE processed_messages SET LOGGED;

-- 支持表
\echo '  → user_data_changes...'
ALTER TABLE user_data_changes SET LOGGED;

\echo ''
\echo '✅ All tables converted back to LOGGED!'
\echo ''
\echo '💡 Next steps:'
\echo '   1. Run VACUUM ANALYZE to update statistics'
\echo '   2. Rebuild indexes with turbo_mode_disable.sql'
\echo '   3. Re-enable autovacuum if needed'
\echo ''

-- 验证
SELECT 
    relname as table_name,
    CASE relpersistence
        WHEN 'u' THEN '🔥 UNLOGGED'
        WHEN 'p' THEN '✅ LOGGED'
        ELSE '❓ OTHER'
    END as persistence,
    pg_size_pretty(pg_total_relation_size(oid)) as total_size
FROM pg_class
WHERE relname IN (
    'casts', 'links', 'reactions', 'verifications',
    'user_profile_changes', 'onchain_events', 'username_proofs',
    'frame_actions', 'processed_messages', 'user_data_changes'
)
AND relkind = 'r'
ORDER BY relname;

