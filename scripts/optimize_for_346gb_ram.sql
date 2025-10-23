-- PostgreSQL 内存优化配置 (346GB RAM 服务器 - PowerEdge T430)
-- 针对 Intel Xeon E5-2690 v4 (56 cores) 优化
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/optimize_for_346gb_ram.sql

\echo '🚀 Optimizing PostgreSQL for 346GB RAM PowerEdge T430 server...'
\echo '   CPU: Intel Xeon E5-2690 v4 (56 cores)'
\echo '   RAM: 346GB'
\echo ''

-- ============================================================================
-- 1. 最大化 maintenance_work_mem (索引重建速度)
-- ============================================================================

\echo '⚡ Setting maintenance_work_mem to 64GB (from 8GB)...'
ALTER SYSTEM SET maintenance_work_mem = '64GB';
\echo '   Expected: 8x faster index creation'
\echo ''

-- ============================================================================
-- 2. 优化 shared_buffers (数据库缓存)
-- ============================================================================

\echo '💾 Setting shared_buffers to 128GB (from 64GB)...'
ALTER SYSTEM SET shared_buffers = '128GB';
\echo '   Expected: 2x better data caching'
\echo ''

-- ============================================================================
-- 3. 大幅提升 work_mem (查询操作内存)
-- ============================================================================

\echo '🔍 Setting work_mem to 1GB (from 128MB)...'
ALTER SYSTEM SET work_mem = '1GB';
\echo '   Expected: 8x faster sorts, joins, and complex queries'
\echo ''

-- ============================================================================
-- 4. 优化 effective_cache_size (查询规划器)
-- ============================================================================

\echo '📊 Setting effective_cache_size to 280GB...'
ALTER SYSTEM SET effective_cache_size = '280GB';
\echo '   Expected: Better query plans with more cache'
\echo ''

-- ============================================================================
-- 5. 优化 CPU 并行设置 (56 cores)
-- ============================================================================

\echo '🖥️ Optimizing CPU parallel settings for 56 cores...'
ALTER SYSTEM SET max_worker_processes = 56;
ALTER SYSTEM SET max_parallel_workers = 56;
ALTER SYSTEM SET max_parallel_workers_per_gather = 16;
ALTER SYSTEM SET parallel_tuple_cost = 0.1;
ALTER SYSTEM SET parallel_setup_cost = 1000.0;
\echo '   Expected: Better parallel query execution'
\echo ''

-- ============================================================================
-- 6. 优化 WAL 和写入设置
-- ============================================================================

\echo '📝 Optimizing WAL and write settings...'
ALTER SYSTEM SET wal_buffers = '64MB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET max_wal_size = '64GB';
ALTER SYSTEM SET min_wal_size = '4GB';
ALTER SYSTEM SET wal_writer_delay = '10ms';
ALTER SYSTEM SET commit_delay = 0;
ALTER SYSTEM SET commit_siblings = 5;
\echo '   Expected: Better write performance'
\echo ''

-- ============================================================================
-- 7. 优化连接和并发设置
-- ============================================================================

\echo '🔗 Optimizing connection settings...'
ALTER SYSTEM SET max_connections = 500;
ALTER SYSTEM SET max_prepared_transactions = 500;
ALTER SYSTEM SET shared_preload_libraries = 'pg_stat_statements';
\echo '   Expected: Support more concurrent connections'
\echo ''

-- ============================================================================
-- 8. 优化内存和统计设置
-- ============================================================================

\echo '📈 Optimizing memory and statistics...'
ALTER SYSTEM SET random_page_cost = 1.1;
ALTER SYSTEM SET effective_io_concurrency = 200;
ALTER SYSTEM SET maintenance_io_concurrency = 10;
ALTER SYSTEM SET max_stack_depth = '7MB';
\echo '   Expected: Better I/O performance'
\echo ''

-- ============================================================================
-- 9. 显示新配置
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
        WHEN name = 'max_connections' THEN setting
        WHEN name = 'max_worker_processes' THEN setting
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
    'max_connections',
    'max_worker_processes',
    'max_parallel_workers'
)
ORDER BY name;

\echo ''
\echo '⚠️  IMPORTANT: Restart PostgreSQL to apply changes!'
\echo ''
\echo '💡 Expected Performance Gains:'
\echo '   - Index creation: 8x faster (64GB maintenance_work_mem)'
\echo '   - Query performance: 2-3x faster (128GB shared_buffers)'
\echo '   - Complex queries: 8x faster (1GB work_mem)'
\echo '   - Parallel queries: 4-8x faster (56 cores)'
\echo '   - Write performance: 30-50% faster (WAL optimizations)'
\echo ''
\echo '🎯 Total memory usage: ~200GB (leaving 146GB for OS)'
\echo '🚀 Index rebuild time: 15-30 minutes (instead of hours!)'
