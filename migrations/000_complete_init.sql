-- Complete database initialization for SnapRAG
-- This script creates all tables with the exact structure from production
-- Run with: snaprag init --force

-- Enable pgvector extension (requires superuser)
-- If this fails, run on DB server: sudo -u postgres psql -d snaprag -c 'CREATE EXTENSION IF NOT EXISTS vector;'
CREATE EXTENSION IF NOT EXISTS vector;

-- ==============================================================================
-- 1. USER PROFILES
-- ==============================================================================

CREATE TABLE IF NOT EXISTS user_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT UNIQUE NOT NULL,
    username VARCHAR(255),
    display_name VARCHAR(255),
    bio TEXT,
    pfp_url TEXT,
    banner_url TEXT,
    location VARCHAR(255),
    website_url TEXT,
    twitter_username VARCHAR(255),
    github_username VARCHAR(255),
    primary_address_ethereum VARCHAR(42),
    primary_address_solana VARCHAR(44),
    profile_token VARCHAR(255),
    profile_embedding VECTOR(768),
    bio_embedding VECTOR(768),
    interests_embedding VECTOR(768),
    last_updated_timestamp BIGINT NOT NULL,
    last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT
);

CREATE TABLE IF NOT EXISTS user_profile_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    snapshot_timestamp BIGINT NOT NULL,
    message_hash BYTEA NOT NULL,
    username VARCHAR(255),
    display_name VARCHAR(255),
    bio TEXT,
    pfp_url TEXT,
    banner_url TEXT,
    location VARCHAR(255),
    website_url TEXT,
    twitter_username VARCHAR(255),
    github_username VARCHAR(255),
    primary_address_ethereum VARCHAR(42),
    primary_address_solana VARCHAR(44),
    profile_token VARCHAR(255),
    profile_embedding VECTOR(768),
    bio_embedding VECTOR(768),
    interests_embedding VECTOR(768),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT,
    UNIQUE(fid, snapshot_timestamp)
);

-- ==============================================================================
-- 2. ACTIVITY AND CHANGES
-- ==============================================================================

CREATE TABLE IF NOT EXISTS user_activity_timeline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    activity_type VARCHAR(50) NOT NULL,
    activity_data JSONB,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT
);

-- Add tracking columns if they don't exist (for existing tables)
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_activity_timeline ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

CREATE TABLE IF NOT EXISTS user_data_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    data_type SMALLINT NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    change_timestamp BIGINT NOT NULL,
    message_hash BYTEA NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    activity_type VARCHAR(50) NOT NULL,
    activity_data JSONB,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ==============================================================================
-- 3. CASTS AND LINKS
-- ==============================================================================

CREATE TABLE IF NOT EXISTS casts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    text TEXT,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA UNIQUE NOT NULL,
    parent_hash BYTEA,
    root_hash BYTEA,
    embeds JSONB,
    mentions JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT
);

-- Add tracking columns if they don't exist
ALTER TABLE casts ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE casts ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE casts ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

CREATE TABLE IF NOT EXISTS links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    target_fid BIGINT NOT NULL,
    link_type VARCHAR(50) NOT NULL DEFAULT 'follow',
    timestamp BIGINT NOT NULL,
    message_hash BYTEA UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT
);

-- Add tracking columns if they don't exist
ALTER TABLE links ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE links ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE links ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

CREATE TABLE IF NOT EXISTS user_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    data_type SMALLINT NOT NULL,
    value TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT
);

-- Add tracking columns if they don't exist
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS shard_id INTEGER;
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS block_height BIGINT;
ALTER TABLE user_data ADD COLUMN IF NOT EXISTS transaction_fid BIGINT;

-- ==============================================================================
-- 4. EMBEDDINGS
-- ==============================================================================

CREATE TABLE IF NOT EXISTS cast_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_hash BYTEA UNIQUE NOT NULL,
    fid BIGINT NOT NULL,
    text TEXT NOT NULL,
    embedding VECTOR(768),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create trigger function for updated_at
CREATE OR REPLACE FUNCTION update_cast_embeddings_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger
DROP TRIGGER IF EXISTS trigger_cast_embeddings_updated_at ON cast_embeddings;
CREATE TRIGGER trigger_cast_embeddings_updated_at
    BEFORE UPDATE ON cast_embeddings
    FOR EACH ROW
    EXECUTE FUNCTION update_cast_embeddings_updated_at();

-- ==============================================================================
-- 5. SYNC TRACKING
-- ==============================================================================

CREATE TABLE IF NOT EXISTS sync_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shard_id INTEGER UNIQUE NOT NULL,
    last_processed_height BIGINT DEFAULT 0,
    status VARCHAR(20) DEFAULT 'idle',
    error_message TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sync_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shard_id INTEGER UNIQUE NOT NULL,
    total_messages BIGINT DEFAULT 0,
    total_blocks BIGINT DEFAULT 0,
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS processed_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_hash BYTEA UNIQUE NOT NULL,
    shard_id INTEGER NOT NULL,
    block_height BIGINT NOT NULL,
    transaction_fid BIGINT NOT NULL,
    message_type VARCHAR(50) NOT NULL,
    fid BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    content_hash BYTEA,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ==============================================================================
-- 6. OTHER TABLES
-- ==============================================================================

CREATE TABLE IF NOT EXISTS username_proofs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    username VARCHAR(255) NOT NULL,
    username_type SMALLINT NOT NULL,
    owner_address VARCHAR(42) NOT NULL,
    signature BYTEA NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(fid, username_type)
);

CREATE TABLE IF NOT EXISTS user_profile_trends (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    trend_period VARCHAR(20) NOT NULL,
    trend_date DATE NOT NULL,
    profile_changes_count INTEGER DEFAULT 0,
    bio_changes_count INTEGER DEFAULT 0,
    username_changes_count INTEGER DEFAULT 0,
    activity_score FLOAT DEFAULT 0.0,
    engagement_score FLOAT DEFAULT 0.0,
    profile_embedding VECTOR(768),
    bio_embedding VECTOR(768),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(fid, trend_period, trend_date)
);

-- ==============================================================================
-- 7. ESSENTIAL INDEXES ONLY (for write performance)
-- ==============================================================================

-- user_profiles (already has UNIQUE index on fid)
CREATE INDEX IF NOT EXISTS idx_user_profiles_username ON user_profiles(username);
CREATE INDEX IF NOT EXISTS idx_user_profiles_display_name ON user_profiles(display_name);

-- user_activity_timeline (critical for sync)
CREATE INDEX IF NOT EXISTS idx_activity_timeline_fid_timestamp 
ON user_activity_timeline(fid, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_activity_fid_register 
ON user_activity_timeline(fid, activity_type) 
WHERE activity_type = 'id_register';

-- casts (essential only)
CREATE INDEX IF NOT EXISTS idx_casts_fid ON casts(fid);

-- processed_messages (for sync tracking)
CREATE INDEX IF NOT EXISTS idx_processed_shard_height 
ON processed_messages(shard_id, block_height DESC);

CREATE INDEX IF NOT EXISTS idx_processed_messages_hash 
ON processed_messages(message_hash);

-- sync_progress
CREATE INDEX IF NOT EXISTS idx_sync_progress_shard_id ON sync_progress(shard_id);

-- cast_embeddings
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_message_hash ON cast_embeddings(message_hash);
CREATE INDEX IF NOT EXISTS idx_cast_embeddings_fid ON cast_embeddings(fid);

-- ==============================================================================
-- 8. FOREIGN KEY (if needed)
-- ==============================================================================

-- Add FK constraint for cast_embeddings (optional, can be slow on HDD)
-- ALTER TABLE cast_embeddings 
-- ADD CONSTRAINT fk_cast_embeddings_message_hash 
-- FOREIGN KEY (message_hash) REFERENCES casts(message_hash) ON DELETE CASCADE;

-- ==============================================================================
-- 9. TABLE OPTIMIZATION
-- ==============================================================================

-- Optimize autovacuum for high-write tables
ALTER TABLE user_activity_timeline SET (
    autovacuum_vacuum_scale_factor = 0.01,
    autovacuum_analyze_scale_factor = 0.005
);

ALTER TABLE casts SET (
    autovacuum_vacuum_scale_factor = 0.02,
    autovacuum_analyze_scale_factor = 0.01
);

ALTER TABLE processed_messages SET (
    autovacuum_vacuum_scale_factor = 0.01,
    autovacuum_analyze_scale_factor = 0.005
);

-- Update statistics
ANALYZE;

SELECT 'SnapRAG database initialized successfully!' as status;


