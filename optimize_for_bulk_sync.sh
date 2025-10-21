#!/bin/bash

# 一键优化 PostgreSQL 用于批量同步
# 使用方法：./optimize_for_bulk_sync.sh

echo "🚀 正在优化 PostgreSQL 配置用于批量同步..."
echo ""

# 数据库连接
DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"

# 应用优化配置
psql "$DB_URL" << 'SQL'
-- 内存优化
ALTER SYSTEM SET effective_cache_size = '225GB';
ALTER SYSTEM SET work_mem = '128MB';

-- WAL 优化
ALTER SYSTEM SET checkpoint_timeout = '1h';

-- 并发优化
ALTER SYSTEM SET max_locks_per_transaction = 512;

-- 临时禁用统计（减少开销）
ALTER SYSTEM SET track_io_timing = 'off';
ALTER SYSTEM SET track_counts = 'off';
ALTER SYSTEM SET track_activities = 'off';

-- 调整 autovacuum（减少干扰）
ALTER SYSTEM SET autovacuum_naptime = '1min';

-- 应用配置
SELECT pg_reload_conf();

-- 验证配置
\echo ''
\echo '✅ 配置已应用，当前设置：'
\echo ''
SELECT 
    name, 
    setting || COALESCE(unit, '') as value,
    context
FROM pg_settings 
WHERE name IN (
    'effective_cache_size',
    'work_mem',
    'checkpoint_timeout',
    'max_locks_per_transaction',
    'track_io_timing',
    'track_counts',
    'autovacuum_naptime'
)
ORDER BY name;
SQL

echo ""
echo "✅ PostgreSQL 优化完成！"
echo ""
echo "📊 下一步："
echo "1. 测试不同 workers 数量："
echo "   snaprag sync start --workers 1"
echo "   snaprag sync start --workers 2"
echo ""
echo "2. 同步完成后，运行恢复脚本："
echo "   ./restore_normal_config.sh"
echo ""

