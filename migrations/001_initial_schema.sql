-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

-- Create user_profiles table (current state only)
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
    profile_embedding VECTOR(1536),
    bio_embedding VECTOR(1536),
    interests_embedding VECTOR(1536),
    last_updated_timestamp BIGINT NOT NULL,
    last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create user_profile_snapshots table (historical snapshots)
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
    profile_embedding VECTOR(1536),
    bio_embedding VECTOR(1536),
    interests_embedding VECTOR(1536),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(fid, snapshot_timestamp)
);

-- Create user_data_changes table (change tracking)
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

-- Create username_proofs table
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

-- Create user_activity_timeline table
CREATE TABLE IF NOT EXISTS user_activity_timeline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    activity_type VARCHAR(50) NOT NULL,
    activity_data JSONB,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create user_profile_trends table
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
    profile_embedding VECTOR(1536),
    bio_embedding VECTOR(1536),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(fid, trend_period, trend_date)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_user_profiles_fid ON user_profiles(fid);
CREATE INDEX IF NOT EXISTS idx_user_profiles_username ON user_profiles(username);
CREATE INDEX IF NOT EXISTS idx_user_profiles_display_name ON user_profiles(display_name);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_fid_timestamp ON user_profile_snapshots(fid, snapshot_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_profile_snapshots_timestamp ON user_profile_snapshots(snapshot_timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_data_changes_fid_type ON user_data_changes(fid, data_type, change_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_data_changes_timestamp ON user_data_changes(change_timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_activity_timeline_fid_timestamp ON user_activity_timeline(fid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_activity_timeline_type_timestamp ON user_activity_timeline(activity_type, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_profile_trends_fid_period_date ON user_profile_trends(fid, trend_period, trend_date DESC);

-- Create vector similarity search indexes
CREATE INDEX IF NOT EXISTS idx_user_profiles_profile_embedding ON user_profiles 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_user_profiles_bio_embedding ON user_profiles 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_profile_embedding ON user_profile_snapshots 
USING ivfflat (profile_embedding vector_cosine_ops) WITH (lists = 100);

CREATE INDEX IF NOT EXISTS idx_profile_snapshots_bio_embedding ON user_profile_snapshots 
USING ivfflat (bio_embedding vector_cosine_ops) WITH (lists = 100);
