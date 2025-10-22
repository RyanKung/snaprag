-- PostgreSQL TURBO Configuration for Maximum Write Performance
-- 适用于初始数据同步阶段，同步完成后恢复默认设置
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/postgresql_turbo_config.sql

\echo '🚀 Applying PostgreSQL TURBO Configuration...'
\echo ''

-- ============================================================================
-- 1. 关闭同步提交（最大性能提升）
-- ============================================================================

\echo '⚡ Setting synchronous_commit = off'
ALTER DATABASE snaprag SET synchronous_commit = 'off';

-- 需要重新连接才能生效，或者在会话中设置
SET synchronous_commit = 'off';

\echo '   Expected gain: +20-50% write speed'
\echo '   Risk: May lose last few seconds of data on crash (acceptable for sync)'
\echo ''

-- ============================================================================
-- 2. 检查当前配置
-- ============================================================================

\echo '📋 Current Write-Related Configuration:'
\echo ''

SELECT 
    name,
    setting,
    COALESCE(unit, '') as unit,
    short_desc
FROM pg_settings
WHERE name IN (
    'synchronous_commit',
    'fsync',
    'shared_buffers',
    'work_mem',
    'maintenance_work_mem',
    'effective_cache_size',
    'checkpoint_timeout',
    'max_wal_size',
    'wal_buffers'
)
ORDER BY name;

\echo ''
\echo '✅ TURBO Configuration Applied!'
\echo ''
\echo '⚠️  Remember to restore after sync completes:'
\echo '    ALTER DATABASE snaprag SET synchronous_commit = ''on'';'
\echo ''
\echo '💡 Additional manual optimizations (requires postgresql.conf edit):'
\echo '    - checkpoint_timeout = 30min (default 5min)'
\echo '    - max_wal_size = 16GB+ (for large syncs)'
\echo '    - shared_buffers = 25% of RAM'
\echo '    - effective_cache_size = 75% of RAM'

