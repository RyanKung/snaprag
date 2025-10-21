#!/bin/bash

# 恢复 PostgreSQL 正常配置（同步完成后执行）
# 使用方法：./restore_normal_config.sh

echo "🔄 正在恢复 PostgreSQL 正常配置..."
echo ""

DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"

psql "$DB_URL" << 'SQL'
-- 恢复统计收集
ALTER SYSTEM SET track_io_timing = 'on';
ALTER SYSTEM SET track_counts = 'on';
ALTER SYSTEM SET track_activities = 'on';

-- 恢复 autovacuum
ALTER SYSTEM SET autovacuum_naptime = '10s';

-- 可选：恢复更保守的 checkpoint
-- ALTER SYSTEM SET checkpoint_timeout = '30min';

-- 应用配置
SELECT pg_reload_conf();

\echo ''
\echo '✅ 配置已恢复'
SQL

echo ""
echo "✅ PostgreSQL 配置已恢复！"
echo ""
echo "📊 下一步："
echo "1. 运行：snaprag index set"
echo "2. 等待索引重建完成"
echo "3. 系统恢复正常运行"
echo ""

