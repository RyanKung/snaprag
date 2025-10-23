-- PostgreSQL 内存优化配置 (300GB RAM 服务器)
-- 大幅提升索引重建和查询性能
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/optimize_for_300gb_ram.sql

\echo '🚀 Optimizing PostgreSQL for 300GB RAM server...'
\echo ''

-- ============================================================================
-- 1. 大幅提升 maintenance_work_mem (索引重建速度)
-- ============================================================================

\echo '⚡ Setting maintenance_work_mem to 32GB (from 8GB)...'
ALTER SYSTEM SET maintenance_work_mem = '32GB';
\echo '   Expected: 3-4x faster index creation'
\echo ''

-- ============================================================================
-- 2. 优化 shared_buffers (数据库缓存)
-- ============================================================================

\echo '💾 Setting shared_buffers to 128GB (from 64GB)...'
ALTER SYSTEM SET shared_buffers = '128GB';
\echo '   Expected: Better data caching, faster queries'
\echo ''

-- ============================================================================
-- 3. 提升 work_mem (查询操作内存)
-- ============================================================================

\echo '🔍 Setting work_mem to 512MB (from 128MB)...'
ALTER SYSTEM SET work_mem = '512MB';
\echo '   Expected: Faster sorts, joins, and complex queries'
\echo ''

-- ============================================================================
-- 4. 优化 effective_cache_size (查询规划器)
-- ============================================================================

\echo '📊 Setting effective_cache_size to 256GB (from 256GB)...'
ALTER SYSTEM SET effective_cache_size = '256GB';
\echo '   Expected: Better query plans'
\echo ''

-- ============================================================================
-- 5. 优化 WAL 相关设置
-- ============================================================================

\echo '📝 Optimizing WAL settings...'
ALTER SYSTEM SET wal_buffers = '64MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET max_wal_size = '32GB';
ALTER SYSTEM SET min_wal_size = '2GB';
\echo '   Expected: Better write performance'
\echo ''

-- ============================================================================
-- 6. 优化连接和并发设置
-- ============================================================================

\echo '🔗 Optimizing connection settings...'
ALTER SYSTEM SET max_connections = 300;
ALTER SYSTEM SET max_prepared_transactions = 300;
\echo '   Expected: Support more concurrent connections'
\echo ''

-- ============================================================================
-- 7. 显示新配置
-- ============================================================================

\echo '📋 New Configuration:'
\echo ''

SELECT 
    name,
    setting as current_value,
    CASE 
        WHEN name = 'shared_buffers' THEN ROUND(setting::numeric * 8 / 1024 / 1024, 0) || ' GB'
        WHEN name = 'maintenance_work_mem' THEN ROUND(setting::numeric / 1024 / 1024, 0) || ' GB'
        WHEN name = 'work_mem' THEN ROUND(setting::numeric / 1024, 0) || ' MB'
        WHEN name = 'effective_cache_size' THEN ROUND(setting::numeric * 8 / 1024 / 1024, 0) || ' GB'
        WHEN name = 'wal_buffers' THEN ROUND(setting::numeric * 8 / 1024, 0) || ' MB'
        WHEN name = 'max_wal_size' THEN ROUND(setting::numeric / 1024 / 1024, 0) || ' GB'
        ELSE setting || ' ' || COALESCE(unit, '')
    END as formatted_value
FROM pg_settings
WHERE name IN (
    'shared_buffers',
    'maintenance_work_mem', 
    'work_mem',
    'effective_cache_size',
    'wal_buffers',
    'max_wal_size',
    'max_connections'
)
ORDER BY name;

\echo ''
\echo '⚠️  IMPORTANT: Restart PostgreSQL to apply changes!'
\echo ''
\echo '💡 Expected Performance Gains:'
\echo '   - Index creation: 3-4x faster (32GB maintenance_work_mem)'
\echo '   - Query performance: 2-3x faster (128GB shared_buffers)'
\echo '   - Complex queries: 2-4x faster (512MB work_mem)'
\echo '   - Write performance: 20-30% faster (WAL optimizations)'
\echo ''
\echo '🎯 Total memory usage: ~200GB (leaving 100GB for OS)'
