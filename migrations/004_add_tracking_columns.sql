-- Add tracking columns to existing tables
-- These columns track which shard/block/transaction a record came from

-- Add columns to user_profiles if they don't exist
ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add columns to user_profile_snapshots if they don't exist  
ALTER TABLE user_profile_snapshots ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_profile_snapshots ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_profile_snapshots ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add columns to user_activity_timeline if they don't exist
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add columns to casts if they don't exist
ALTER TABLE casts ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE casts ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE casts ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add columns to links if they don't exist
ALTER TABLE links ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE links ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE links ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add columns to user_data if they don't exist
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- Add indexes for tracking queries
CREATE INDEX IF NOT EXISTS idx_user_profiles_shard_block ON user_profiles(shard_id, block_height);
CREATE INDEX IF NOT EXISTS idx_casts_shard_block ON casts(shard_id, block_height);
CREATE INDEX IF NOT EXISTS idx_activity_shard_block ON user_activity_timeline(shard_id, block_height);

