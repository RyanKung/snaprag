-- PostgreSQL 优化配置（针对 300GB 内存服务器）
-- 执行前请确保有超级用户权限
-- 建议：先备份当前配置！

-- ============================================
-- 第一部分：立即生效的配置（不需要重启）
-- ============================================

-- 1. WAL 写入优化（对批量写入影响最大）⭐⭐⭐
ALTER SYSTEM SET wal_buffers = '64MB';           -- 增加 WAL 缓冲（默认 -1/auto，通常 16MB）
ALTER SYSTEM SET checkpoint_timeout = '1h';      -- 从默认 5min 增加到 1小时（减少 checkpoint 频率）
ALTER SYSTEM SET max_wal_size = '32GB';          -- 从默认 1GB 增加到 32GB（允许更长的事务）
ALTER SYSTEM SET min_wal_size = '8GB';           -- 从默认 80MB 增加（保持 WAL 文件，避免重建）
ALTER SYSTEM SET wal_compression = 'on';         -- 启用 WAL 压缩（减少磁盘 I/O）

-- 2. 后台写入优化 ⭐⭐
ALTER SYSTEM SET bgwriter_delay = '200ms';       -- 默认 200ms，可以保持
ALTER SYSTEM SET bgwriter_lru_maxpages = 1000;   -- 默认 100，增加 10 倍
ALTER SYSTEM SET bgwriter_lru_multiplier = 4.0;  -- 默认 2.0，更积极的后台写入

-- 3. 并发优化 ⭐⭐
ALTER SYSTEM SET max_locks_per_transaction = 512;    -- 默认 64，增加以支持更多并发锁
ALTER SYSTEM SET max_pred_locks_per_transaction = 512; -- 默认 64

-- 4. 维护工作内存 ⭐⭐⭐
ALTER SYSTEM SET maintenance_work_mem = '8GB';   -- 默认 64MB，用于 VACUUM、CREATE INDEX
ALTER SYSTEM SET autovacuum_work_mem = '8GB';    -- 默认 -1（使用 maintenance_work_mem）

-- 5. 查询执行内存（针对 300GB 内存）⭐⭐
ALTER SYSTEM SET work_mem = '256MB';             -- 默认 4MB，用于排序、哈希表（单个操作）
ALTER SYSTEM SET hash_mem_multiplier = 2.0;      -- 默认 1.0，哈希表使用更多内存

-- 6. 成本估算（帮助查询规划器）⭐
ALTER SYSTEM SET effective_cache_size = '225GB'; -- 默认 4GB，设置为 RAM 的 75%（300GB × 0.75）
ALTER SYSTEM SET random_page_cost = 1.1;         -- 默认 4.0，如果使用 SSD/NVMe，降低到 1.1

-- 7. 并行查询（对大表查询有帮助）⭐
ALTER SYSTEM SET max_parallel_workers_per_gather = 8;  -- 默认 2
ALTER SYSTEM SET max_parallel_workers = 16;            -- 默认 8
ALTER SYSTEM SET max_worker_processes = 32;            -- 默认 8

-- 8. 临时禁用不必要的统计收集（bulk sync 期间）⭐
-- 注意：同步完成后要恢复！
ALTER SYSTEM SET track_io_timing = 'off';        -- 减少统计开销
ALTER SYSTEM SET track_functions = 'none';       -- 默认 none

-- 应用以上配置（立即生效，除了标记为"需要重启"的）
SELECT pg_reload_conf();

\echo '✅ 第一部分配置已应用（不需要重启）'
\echo ''

-- ============================================
-- 第二部分：需要重启 PostgreSQL 的配置
-- ============================================

-- 9. 共享内存（最重要的配置）⭐⭐⭐
-- 建议：RAM 的 25-40%（300GB → 75-120GB）
-- 保守建议：60GB（避免 OOM）
ALTER SYSTEM SET shared_buffers = '60GB';        -- 默认 128MB!!!

-- 10. WAL 段文件大小（PG 11+ 支持）⭐
-- 默认 16MB，可以增加到 64MB 或 128MB（减少文件切换）
-- ALTER SYSTEM SET wal_segment_size = '128MB';  -- 需要 initdb 时指定，无法动态修改

-- 11. 最大连接数（根据需求调整）
-- ALTER SYSTEM SET max_connections = 200;      -- 默认 100，如果需要更多并发可以增加

\echo '⚠️  第二部分配置已设置，但需要重启 PostgreSQL 才能生效！'
\echo '   重启命令：sudo systemctl restart postgresql'
\echo ''

-- ============================================
-- 第三部分：bulk sync 期间的临时优化
-- ============================================

-- 12. 临时降低 WAL 级别（如果不需要复制）⚠️ 危险
-- 注意：需要重启，且会丢失复制能力！
-- ALTER SYSTEM SET wal_level = 'minimal';      -- 默认 replica，降低到 minimal 可以提速
-- ALTER SYSTEM SET max_wal_senders = 0;        -- 默认 10，如果不需要复制可以设置为 0

-- 13. 临时禁用 fsync（极度危险！崩溃会丢数据）❌ 不推荐
-- ALTER SYSTEM SET fsync = 'off';              -- 默认 on，绝对不要在生产环境这样做！

\echo '⚠️  第三部分是高风险优化，已注释掉，请根据需要谨慎启用'
\echo ''

-- ============================================
-- 监控和验证
-- ============================================

\echo '📊 当前配置验证：'
\echo ''

-- 查看关键配置
SELECT name, setting, unit, context 
FROM pg_settings 
WHERE name IN (
    'shared_buffers',
    'effective_cache_size',
    'work_mem',
    'maintenance_work_mem',
    'wal_buffers',
    'checkpoint_timeout',
    'max_wal_size',
    'max_connections',
    'max_worker_processes',
    'synchronous_commit'
)
ORDER BY name;

\echo ''
\echo '============================================'
\echo '预期性能提升汇总（基于 300GB 内存）'
\echo '============================================'
\echo '1. WAL 优化：           +30-50% 写入速度'
\echo '2. shared_buffers 60GB: +20-30% 整体性能'
\echo '3. work_mem 256MB:      +10-20% 复杂查询'
\echo '4. 并发优化：           +5-15% 多 worker 场景'
\echo '============================================'
\echo '总计预期提升：60-100% 🚀'
\echo ''
\echo '⚠️  重要提醒：'
\echo '1. shared_buffers 需要重启 PostgreSQL'
\echo '2. 同步完成后恢复 track_io_timing = on'
\echo '3. 同步完成后运行 snaprag index set'
\echo '4. 建议先备份：pg_dumpall > backup.sql'
