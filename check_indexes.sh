#!/bin/bash

DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"

echo "🔍 检查 reactions 和 links 表的索引..."
echo ""

psql "$DB_URL" << 'SQL'
-- 检查 reactions 表的索引
\echo '📊 Reactions 表索引：'
SELECT 
    indexname,
    indexdef
FROM pg_indexes 
WHERE tablename = 'reactions'
ORDER BY indexname;

\echo ''
\echo '📊 Reactions 表约束：'
SELECT 
    conname,
    contype,
    pg_get_constraintdef(oid)
FROM pg_constraint 
WHERE conrelid = 'reactions'::regclass;

\echo ''
\echo '-----------------------------------'
\echo ''

-- 检查 links 表的索引
\echo '📊 Links 表索引：'
SELECT 
    indexname,
    indexdef
FROM pg_indexes 
WHERE tablename = 'links'
ORDER BY indexname;

\echo ''
\echo '📊 Links 表约束：'
SELECT 
    conname,
    contype,
    pg_get_constraintdef(oid)
FROM pg_constraint 
WHERE conrelid = 'links'::regclass;

\echo ''
\echo '-----------------------------------'
\echo ''

-- 检查表大小和行数
\echo '📊 表大小统计：'
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as total_size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) as table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) as indexes_size,
    n_live_tup as estimated_rows
FROM pg_stat_user_tables
WHERE tablename IN ('reactions', 'links', 'casts', 'user_profiles')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
SQL

