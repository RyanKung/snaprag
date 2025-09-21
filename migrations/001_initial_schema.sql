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

-- Create sync_progress table for tracking sync progress per shard
CREATE TABLE IF NOT EXISTS sync_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shard_id INTEGER NOT NULL UNIQUE,
    last_processed_height BIGINT DEFAULT 0,
    status VARCHAR(20) DEFAULT 'idle',
    error_message TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create processed_messages table for tracking processed messages
CREATE TABLE IF NOT EXISTS processed_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_hash BYTEA NOT NULL UNIQUE,
    shard_id INTEGER NOT NULL,
    block_height BIGINT NOT NULL,
    transaction_fid BIGINT NOT NULL,
    message_type VARCHAR(50) NOT NULL,
    fid BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    content_hash BYTEA,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create sync_stats table for tracking sync statistics
CREATE TABLE IF NOT EXISTS sync_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shard_id INTEGER NOT NULL,
    total_messages BIGINT DEFAULT 0,
    total_blocks BIGINT DEFAULT 0,
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(shard_id)
);

-- Create indexes for sync tables
CREATE INDEX IF NOT EXISTS idx_sync_progress_shard_id ON sync_progress(shard_id);
CREATE INDEX IF NOT EXISTS idx_sync_progress_status ON sync_progress(status);

CREATE INDEX IF NOT EXISTS idx_processed_messages_hash ON processed_messages(message_hash);
CREATE INDEX IF NOT EXISTS idx_processed_messages_shard_height ON processed_messages(shard_id, block_height);
CREATE INDEX IF NOT EXISTS idx_processed_messages_fid_timestamp ON processed_messages(fid, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_sync_stats_shard_id ON sync_stats(shard_id);

-- user_data_changes table already defined above

-- Create user_activities table for basic activity tracking
CREATE TABLE IF NOT EXISTS user_activities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    activity_type VARCHAR(100) NOT NULL,
    activity_data TEXT,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create user_activity_timeline table for detailed activity tracking
CREATE TABLE IF NOT EXISTS user_activity_timeline (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    activity_type VARCHAR(100) NOT NULL,
    activity_data JSONB,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for user_data_changes
CREATE INDEX IF NOT EXISTS idx_user_data_changes_fid ON user_data_changes(fid);
CREATE INDEX IF NOT EXISTS idx_user_data_changes_timestamp ON user_data_changes(change_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_user_data_changes_type ON user_data_changes(data_type);

-- Create indexes for user_activities
CREATE INDEX IF NOT EXISTS idx_user_activities_fid ON user_activities(fid);
CREATE INDEX IF NOT EXISTS idx_user_activities_timestamp ON user_activities(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_user_activities_type ON user_activities(activity_type);

-- Create indexes for user_activity_timeline
CREATE INDEX IF NOT EXISTS idx_user_activity_timeline_fid ON user_activity_timeline(fid);
CREATE INDEX IF NOT EXISTS idx_user_activity_timeline_timestamp ON user_activity_timeline(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_user_activity_timeline_type ON user_activity_timeline(activity_type);

-- Create casts table for storing cast messages
CREATE TABLE IF NOT EXISTS casts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    text TEXT,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA NOT NULL UNIQUE,
    parent_hash BYTEA,
    root_hash BYTEA,
    embeds JSONB,
    mentions JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create links table for storing follow relationships and other links
CREATE TABLE IF NOT EXISTS links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    target_fid BIGINT NOT NULL,
    link_type VARCHAR(50) NOT NULL DEFAULT 'follow',
    timestamp BIGINT NOT NULL,
    message_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create user_data table for storing user profile data
CREATE TABLE IF NOT EXISTS user_data (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    data_type SMALLINT NOT NULL,
    value TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    message_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for casts table
CREATE INDEX IF NOT EXISTS idx_casts_fid ON casts(fid);
CREATE INDEX IF NOT EXISTS idx_casts_timestamp ON casts(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_casts_message_hash ON casts(message_hash);
CREATE INDEX IF NOT EXISTS idx_casts_parent_hash ON casts(parent_hash);
CREATE INDEX IF NOT EXISTS idx_casts_root_hash ON casts(root_hash);

-- Create indexes for links table
CREATE INDEX IF NOT EXISTS idx_links_fid ON links(fid);
CREATE INDEX IF NOT EXISTS idx_links_target_fid ON links(target_fid);
CREATE INDEX IF NOT EXISTS idx_links_type ON links(link_type);
CREATE INDEX IF NOT EXISTS idx_links_timestamp ON links(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_links_message_hash ON links(message_hash);
CREATE INDEX IF NOT EXISTS idx_links_fid_target_type ON links(fid, target_fid, link_type);

-- Create indexes for user_data table
CREATE INDEX IF NOT EXISTS idx_user_data_fid ON user_data(fid);
CREATE INDEX IF NOT EXISTS idx_user_data_type ON user_data(data_type);
CREATE INDEX IF NOT EXISTS idx_user_data_timestamp ON user_data(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_user_data_message_hash ON user_data(message_hash);
CREATE INDEX IF NOT EXISTS idx_user_data_fid_type ON user_data(fid, data_type);
