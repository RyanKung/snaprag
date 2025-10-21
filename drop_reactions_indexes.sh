#!/bin/bash

# 临时脚本：删除 reactions 表的遗留索引
# 这些索引在之前的 index unset 中没有被删除

DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"

echo "🔨 删除 reactions 表的遗留索引..."
echo ""

psql "$DB_URL" << 'SQL'
-- 删除所有非唯一约束的索引（保留 message_hash unique 和 primary key）
DROP INDEX IF EXISTS idx_reactions_engagement CASCADE;
DROP INDEX IF EXISTS idx_reactions_shard_block CASCADE;
DROP INDEX IF EXISTS idx_reactions_target_cast CASCADE;
DROP INDEX IF EXISTS idx_reactions_target_fid CASCADE;
DROP INDEX IF EXISTS idx_reactions_type CASCADE;
DROP INDEX IF EXISTS idx_reactions_user_cast CASCADE;

\echo ''
\echo '✅ 遗留索引已删除'
\echo ''
\echo '📊 Reactions 表剩余索引：'
SELECT indexname FROM pg_indexes WHERE tablename = 'reactions' ORDER BY indexname;
SQL

echo ""
echo "✅ 完成！现在 reactions 表应该只有 2 个索引（primary key + message_hash unique）"
echo ""

