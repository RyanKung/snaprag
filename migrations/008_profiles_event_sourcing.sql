-- Migration 008: Convert user_profiles to event-sourcing model
-- 
-- Key Idea: Store individual field changes, not complete snapshots
-- Benefits:
-- - Pure append-only (no UPDATE, no locks)
-- - Each message = one INSERT (simple, fast)
-- - Query-time merging (DISTINCT ON)
--
-- Trade-off:
-- - Queries slightly slower (need to merge changes)
-- - But inserts MUCH faster (no lock contention)

-- Step 1: Create event-sourcing table for profile changes
CREATE TABLE IF NOT EXISTS user_profile_changes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    fid bigint NOT NULL,
    field_name varchar(50) NOT NULL,  -- 'username', 'display_name', 'bio', etc.
    field_value text,                 -- The new value for this field
    timestamp bigint NOT NULL,        -- Farcaster timestamp
    message_hash bytea NOT NULL UNIQUE,
    shard_id integer,
    block_height bigint,
    transaction_fid bigint,
    created_at timestamp with time zone DEFAULT now()
);

-- Indexes for efficient queries
-- Most important: get latest value for each (fid, field)
CREATE INDEX idx_profile_changes_fid_field_ts 
    ON user_profile_changes(fid, field_name, timestamp DESC);

-- Deduplication
CREATE INDEX idx_profile_changes_message_hash 
    ON user_profile_changes(message_hash);

-- Shard/block tracking
CREATE INDEX idx_profile_changes_shard_block 
    ON user_profile_changes(shard_id, block_height);

-- Field name filtering (if we want to query specific fields)
CREATE INDEX idx_profile_changes_field_name 
    ON user_profile_changes(field_name);

-- Step 2: Migrate existing user_profiles data to changes format
-- For each existing profile, create synthetic change events
DO $$
DECLARE
    field_names text[] := ARRAY['username', 'display_name', 'bio', 'pfp_url', 'location', 
                                'website_url', 'twitter_username', 'github_username'];
    field_name text;
BEGIN
    FOREACH field_name IN ARRAY field_names
    LOOP
        EXECUTE format('
            INSERT INTO user_profile_changes (fid, field_name, field_value, timestamp, message_hash, created_at)
            SELECT 
                fid,
                %L as field_name,
                %I as field_value,
                last_updated_timestamp,
                (''\x'' || md5(%L || ''-'' || fid::text || ''-'' || COALESCE(%I, '''')))::bytea as message_hash,
                last_updated_at
            FROM user_profiles
            WHERE %I IS NOT NULL
            ON CONFLICT (message_hash) DO NOTHING
        ', field_name, field_name, field_name, field_name, field_name);
    END LOOP;
END $$;

-- Step 3: Rename old table (keep for backup)
ALTER TABLE user_profiles RENAME TO user_profiles_old_backup;

-- Step 4: Create view that reconstructs profiles from changes
CREATE OR REPLACE VIEW user_profiles AS
WITH latest_changes AS (
    -- Get the latest value for each (fid, field_name)
    SELECT DISTINCT ON (fid, field_name)
        fid,
        field_name,
        field_value,
        timestamp
    FROM user_profile_changes
    ORDER BY fid, field_name, timestamp DESC
),
profile_data AS (
    -- Pivot field_name/field_value pairs into columns
    SELECT 
        fid,
        MAX(timestamp) as last_updated_timestamp,
        MAX(CASE WHEN field_name = 'username' THEN field_value END) as username,
        MAX(CASE WHEN field_name = 'display_name' THEN field_value END) as display_name,
        MAX(CASE WHEN field_name = 'bio' THEN field_value END) as bio,
        MAX(CASE WHEN field_name = 'pfp_url' THEN field_value END) as pfp_url,
        MAX(CASE WHEN field_name = 'banner_url' THEN field_value END) as banner_url,
        MAX(CASE WHEN field_name = 'location' THEN field_value END) as location,
        MAX(CASE WHEN field_name = 'website_url' THEN field_value END) as website_url,
        MAX(CASE WHEN field_name = 'twitter_username' THEN field_value END) as twitter_username,
        MAX(CASE WHEN field_name = 'github_username' THEN field_value END) as github_username,
        MAX(CASE WHEN field_name = 'primary_address_ethereum' THEN field_value END) as primary_address_ethereum,
        MAX(CASE WHEN field_name = 'primary_address_solana' THEN field_value END) as primary_address_solana
    FROM latest_changes
    GROUP BY fid
)
SELECT 
    gen_random_uuid() as id,
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
    NULL::varchar(255) as profile_token,
    NULL::vector(1536) as profile_embedding,
    NULL::vector(1536) as bio_embedding,
    NULL::vector(1536) as interests_embedding,
    last_updated_timestamp,
    NOW() as last_updated_at,
    NULL::integer as shard_id,
    NULL::bigint as block_height,
    NULL::bigint as transaction_fid
FROM profile_data;

-- Optional: Create materialized view for better query performance
-- CREATE MATERIALIZED VIEW user_profiles_materialized AS SELECT * FROM user_profiles;
-- CREATE UNIQUE INDEX ON user_profiles_materialized(fid);
-- Refresh with: REFRESH MATERIALIZED VIEW user_profiles_materialized;

COMMENT ON TABLE user_profile_changes IS 'Event-sourcing table: each row = single field change. Pure append-only, zero locks.';
COMMENT ON VIEW user_profiles IS 'Reconstructed profiles from user_profile_changes. Compatible with existing queries.';

