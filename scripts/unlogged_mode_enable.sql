-- UNLOGGED MODE: 极限性能模式（仅适用于首次完整同步）
-- ⚠️ 警告：启用后表数据在PostgreSQL崩溃时会完全丢失！
-- ⚠️ 仅在可以随时重新同步的场景下使用
-- 
-- 性能提升：+100-300% (可能达到 20-45k 条/秒)
-- 风险：PostgreSQL崩溃/断电 = 所有数据丢失
--
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/unlogged_mode_enable.sql

\echo ''
\echo '⚠️⚠️⚠️  WARNING: ENTERING UNLOGGED MODE  ⚠️⚠️⚠️'
\echo ''
\echo '   This will disable WAL logging on all data tables.'
\echo '   If PostgreSQL crashes, ALL data will be lost!'
\echo ''
\echo '   Only use this for initial sync when you can re-run from scratch.'
\echo ''
\echo '   Press Ctrl+C to cancel, or wait 5 seconds to continue...'
SELECT pg_sleep(5);
\echo ''

\echo '🔥 Converting tables to UNLOGGED...'

-- 核心数据表
ALTER TABLE casts SET UNLOGGED;
ALTER TABLE links SET UNLOGGED;
ALTER TABLE reactions SET UNLOGGED;
ALTER TABLE verifications SET UNLOGGED;

-- 其他高频表
ALTER TABLE user_profile_changes SET UNLOGGED;
ALTER TABLE onchain_events SET UNLOGGED;
ALTER TABLE username_proofs SET UNLOGGED;
ALTER TABLE frame_actions SET UNLOGGED;
ALTER TABLE processed_messages SET UNLOGGED;

-- 支持表
ALTER TABLE user_data_changes SET UNLOGGED;

\echo ''
\echo '✅ UNLOGGED MODE Enabled!'
\echo ''
\echo '⚡ Expected performance: 20-45k rows/sec (+100-300%)'
\echo ''
\echo '⚠️  CRITICAL REMINDERS:'
\echo '   1. PostgreSQL crash = ALL DATA LOST'
\echo '   2. Must run unlogged_mode_disable.sql after sync'
\echo '   3. Backup before converting back to LOGGED'
\echo ''

-- 显示当前UNLOGGED表
SELECT 
    relname as table_name,
    CASE relpersistence
        WHEN 'u' THEN '🔥 UNLOGGED'
        WHEN 'p' THEN '✅ LOGGED'
        ELSE '❓ OTHER'
    END as persistence
FROM pg_class
WHERE relname IN (
    'casts', 'links', 'reactions', 'verifications',
    'user_profile_changes', 'onchain_events', 'username_proofs',
    'frame_actions', 'processed_messages', 'user_data_changes'
)
AND relkind = 'r'
ORDER BY relname;

