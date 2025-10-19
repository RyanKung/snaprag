-- ==============================================================================
-- Performance Optimization Indexes for Statistics Queries
-- ==============================================================================

-- 1. Optimize MAX(timestamp) query on user_activity_timeline (CRITICAL!)
-- The existing index is (fid, timestamp DESC) but we need just (timestamp DESC)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_activity_timeline_timestamp_desc 
ON user_activity_timeline(timestamp DESC);

-- 2. Partial indexes for common WHERE conditions on user_profiles
-- These dramatically speed up COUNT queries with NOT NULL checks

-- Username check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_username 
ON user_profiles(fid) 
WHERE username IS NOT NULL AND username != '';

-- Display name check  
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_display_name 
ON user_profiles(fid) 
WHERE display_name IS NOT NULL AND display_name != '';

-- Bio check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_bio 
ON user_profiles(fid) 
WHERE bio IS NOT NULL AND bio != '';

-- PFP check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_pfp 
ON user_profiles(fid) 
WHERE pfp_url IS NOT NULL AND pfp_url != '';

-- Website check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_website 
ON user_profiles(fid) 
WHERE website_url IS NOT NULL AND website_url != '';

-- Location check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_location 
ON user_profiles(fid) 
WHERE location IS NOT NULL AND location != '';

-- Twitter check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_twitter 
ON user_profiles(fid) 
WHERE twitter_username IS NOT NULL AND twitter_username != '';

-- GitHub check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_github 
ON user_profiles(fid) 
WHERE github_username IS NOT NULL AND github_username != '';

-- Ethereum address check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_ethereum 
ON user_profiles(fid) 
WHERE primary_address_ethereum IS NOT NULL AND primary_address_ethereum != '';

-- Solana address check
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_has_solana 
ON user_profiles(fid) 
WHERE primary_address_solana IS NOT NULL AND primary_address_solana != '';

-- Complete profile check (has all three: username, display_name, bio)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_user_profiles_complete 
ON user_profiles(fid) 
WHERE username IS NOT NULL AND username != ''
  AND display_name IS NOT NULL AND display_name != ''
  AND bio IS NOT NULL AND bio != '';

-- 3. Index for activity type grouping (used in statistics)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_activity_timeline_type 
ON user_activity_timeline(activity_type);

-- 4. Optimize the JOIN query for top usernames
-- Covering index for the JOIN
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_activity_timeline_fid_for_stats 
ON user_activity_timeline(fid) INCLUDE (id);

-- Update table statistics
ANALYZE user_profiles;
ANALYZE user_activity_timeline;
ANALYZE casts;

SELECT 'Performance indexes created successfully!' as status;

