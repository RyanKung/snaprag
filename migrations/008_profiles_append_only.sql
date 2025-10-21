-- Migration 008: Convert user_profiles to append-only model
-- 
-- Strategy:
-- 1. Drop the UNIQUE constraint on fid (allow multiple rows per FID)
-- 2. Add timestamp to primary key (fid + timestamp = unique)
-- 3. INSERT new rows instead of UPDATE
-- 4. Use DISTINCT ON (fid) ORDER BY timestamp DESC to get latest
--
-- Benefits:
-- - No UPDATE locks (only INSERTs)
-- - No ON CONFLICT checks (just append)
-- - Perfect for concurrent workers
-- - Historical profile data preserved

-- Step 1: Rename old table
ALTER TABLE user_profiles RENAME TO user_profiles_old;

-- Step 2: Create new append-only table
CREATE TABLE user_profiles (
    id uuid DEFAULT gen_random_uuid(),
    fid bigint NOT NULL,
    username character varying(255),
    display_name character varying(255),
    bio text,
    pfp_url text,
    banner_url text,
    location character varying(255),
    website_url text,
    twitter_username character varying(255),
    github_username character varying(255),
    primary_address_ethereum character varying(42),
    primary_address_solana character varying(44),
    profile_token character varying(255),
    profile_embedding vector(1536),
    bio_embedding vector(1536),
    interests_embedding vector(1536),
    last_updated_timestamp bigint NOT NULL,
    last_updated_at timestamp with time zone DEFAULT now(),
    shard_id integer,
    block_height bigint,
    transaction_fid bigint,
    -- Primary key: (fid, last_updated_timestamp) for uniqueness
    PRIMARY KEY (fid, last_updated_timestamp)
);

-- Step 3: Create indexes for efficient queries
-- Get latest profile per FID
CREATE INDEX idx_user_profiles_fid_timestamp 
    ON user_profiles(fid, last_updated_timestamp DESC);

-- Search by username
CREATE INDEX idx_user_profiles_username 
    ON user_profiles(username) WHERE username IS NOT NULL;

-- Search by display_name
CREATE INDEX idx_user_profiles_display_name 
    ON user_profiles(display_name) WHERE display_name IS NOT NULL;

-- Shard/block tracking
CREATE INDEX idx_user_profiles_shard_block 
    ON user_profiles(shard_id, block_height);

-- Step 4: Migrate existing data
INSERT INTO user_profiles 
SELECT * FROM user_profiles_old
ON CONFLICT (fid, last_updated_timestamp) DO NOTHING;

-- Step 5: Create view for backward compatibility (gets latest profile per FID)
CREATE OR REPLACE VIEW user_profiles_latest AS
SELECT DISTINCT ON (fid)
    id,
    fid,
    username,
    display_name,
    bio,
    pfp_url,
    banner_url,
    location,
    website_url,
    twitter_username,
    github_username,
    primary_address_ethereum,
    primary_address_solana,
    profile_token,
    profile_embedding,
    bio_embedding,
    interests_embedding,
    last_updated_timestamp,
    last_updated_at,
    shard_id,
    block_height,
    transaction_fid
FROM user_profiles
ORDER BY fid, last_updated_timestamp DESC;

-- Step 6: Drop old table (after verification)
-- DROP TABLE user_profiles_old;  -- Run this manually after confirming migration worked

COMMENT ON TABLE user_profiles IS 'Append-only user profiles table. Multiple rows per FID (one per update). Use user_profiles_latest view for current state.';
COMMENT ON VIEW user_profiles_latest IS 'Latest profile data per FID. Use this for queries instead of user_profiles table.';

