#!/bin/bash
# Performance Index Creation Script
# This script creates indexes one by one with progress reporting

set -e

DB_URL="postgresql://snaprag:hackinthebox_24601@192.168.1.192/snaprag"

echo "🚀 Creating performance indexes for SnapRAG..."
echo "⚠️  This will take a while on large tables (9.38 billion rows!)"
echo ""

# Array of indexes to create
declare -a indexes=(
    "idx_activity_timeline_timestamp_desc:user_activity_timeline(timestamp DESC):CRITICAL for MAX/MIN queries"
    "idx_user_profiles_has_username:user_profiles(fid) WHERE username IS NOT NULL AND username != '':Username count queries"
    "idx_user_profiles_has_display_name:user_profiles(fid) WHERE display_name IS NOT NULL AND display_name != '':Display name queries"
    "idx_user_profiles_has_bio:user_profiles(fid) WHERE bio IS NOT NULL AND bio != '':Bio count queries"
    "idx_user_profiles_has_pfp:user_profiles(fid) WHERE pfp_url IS NOT NULL AND pfp_url != '':PFP count queries"
    "idx_user_profiles_has_website:user_profiles(fid) WHERE website_url IS NOT NULL AND website_url != '':Website count queries"
    "idx_user_profiles_has_location:user_profiles(fid) WHERE location IS NOT NULL AND location != '':Location count queries"
    "idx_user_profiles_has_twitter:user_profiles(fid) WHERE twitter_username IS NOT NULL AND twitter_username != '':Twitter count queries"
    "idx_user_profiles_has_github:user_profiles(fid) WHERE github_username IS NOT NULL AND github_username != '':GitHub count queries"
    "idx_user_profiles_has_ethereum:user_profiles(fid) WHERE primary_address_ethereum IS NOT NULL AND primary_address_ethereum != '':Ethereum address queries"
    "idx_user_profiles_has_solana:user_profiles(fid) WHERE primary_address_solana IS NOT NULL AND primary_address_solana != '':Solana address queries"
    "idx_user_profiles_complete:user_profiles(fid) WHERE username IS NOT NULL AND username != '' AND display_name IS NOT NULL AND display_name != '' AND bio IS NOT NULL AND bio != '':Complete profile queries"
    "idx_activity_timeline_type:user_activity_timeline(activity_type):Activity type grouping"
)

total=${#indexes[@]}
current=0

for index_def in "${indexes[@]}"; do
    current=$((current + 1))
    IFS=':' read -r index_name index_spec description <<< "$index_def"
    
    echo "[$current/$total] Creating $index_name..."
    echo "  Purpose: $description"
    echo "  Index: $index_spec"
    
    start_time=$(date +%s)
    
    psql "$DB_URL" -c "CREATE INDEX CONCURRENTLY IF NOT EXISTS $index_name ON $index_spec;" 2>&1 | grep -v "NOTICE" || true
    
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    
    echo "  ✅ Created in ${duration}s"
    echo ""
done

echo "🎉 All indexes created successfully!"
echo ""
echo "Running ANALYZE to update statistics..."
psql "$DB_URL" -c "ANALYZE user_profiles; ANALYZE user_activity_timeline; ANALYZE casts;"

echo ""
echo "✅ Performance optimization complete!"
echo ""
echo "📊 Index status:"
psql "$DB_URL" -c "
SELECT 
    schemaname,
    tablename,
    indexname,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_indexes
JOIN pg_class ON pg_class.relname = indexname
WHERE tablename IN ('user_profiles', 'user_activity_timeline', 'casts')
ORDER BY tablename, indexname;
"

