#!/bin/bash

# Quick script to optimize vector search performance
# Run this if `snaprag ask` is slow at Step 3

set -e

echo "🔧 Optimizing Vector Search Indexes..."
echo

# Database connection details from config.toml
DB_HOST="192.168.1.192"
DB_USER="snaprag"
DB_NAME="snaprag"
DB_PASSWORD="hackinthebox_24601"

export PGPASSWORD="$DB_PASSWORD"

echo "📊 Checking current table stats..."
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" << 'EOF'
SELECT 
    count(*) as total_embeddings,
    pg_size_pretty(pg_total_relation_size('cast_embeddings')) as table_size
FROM cast_embeddings;
EOF

echo
echo "🔍 Running ANALYZE to update query planner statistics..."
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -c "ANALYZE cast_embeddings;"

echo
echo "🧹 Running VACUUM ANALYZE to optimize..."
psql -h "$DB_HOST" -U "$DB_USER" -d "$DB_NAME" -c "VACUUM ANALYZE cast_embeddings;"

echo
echo "✅ Done! Vector indexes optimized."
echo
echo "Now try running:"
echo "  snaprag ask 99 \"What are your thoughts on building on Base?\""
echo

unset PGPASSWORD

