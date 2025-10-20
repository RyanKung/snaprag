-- Add reactions and verifications tables
-- These tables store ReactionAdd (type 3) and VerificationAdd (type 7) message data

-- ============================================================================
-- REACTIONS TABLE - Store likes and recasts
-- ============================================================================

CREATE TABLE IF NOT EXISTS reactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,                    -- User who reacted
    target_cast_hash BYTEA NOT NULL,        -- Cast being reacted to
    target_fid BIGINT,                       -- Author of target cast (for queries)
    reaction_type SMALLINT NOT NULL,         -- 1=like, 2=recast
    timestamp BIGINT NOT NULL,
    message_hash BYTEA UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT,
    
    -- Prevent duplicate reactions (same user, same cast, same type)
    UNIQUE(fid, target_cast_hash, reaction_type)
);

-- Indexes for reactions
CREATE INDEX IF NOT EXISTS idx_reactions_fid ON reactions(fid);
CREATE INDEX IF NOT EXISTS idx_reactions_target_cast ON reactions(target_cast_hash);
CREATE INDEX IF NOT EXISTS idx_reactions_target_fid ON reactions(target_fid);
CREATE INDEX IF NOT EXISTS idx_reactions_type ON reactions(reaction_type);
CREATE INDEX IF NOT EXISTS idx_reactions_timestamp ON reactions(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_reactions_shard_block ON reactions(shard_id, block_height);

-- Index for finding all reactions by a user to a specific cast
CREATE INDEX IF NOT EXISTS idx_reactions_user_cast ON reactions(fid, target_cast_hash);

-- Index for engagement metrics (count reactions per cast)
CREATE INDEX IF NOT EXISTS idx_reactions_engagement ON reactions(target_cast_hash, reaction_type);

COMMENT ON TABLE reactions IS 'Stores user reactions (likes and recasts) to casts';
COMMENT ON COLUMN reactions.reaction_type IS '1=like, 2=recast';
COMMENT ON COLUMN reactions.target_cast_hash IS 'Hash of the cast being reacted to';

-- ============================================================================
-- VERIFICATIONS TABLE - Store address verifications
-- ============================================================================

CREATE TABLE IF NOT EXISTS verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fid BIGINT NOT NULL,
    address BYTEA NOT NULL,                  -- Verified address
    claim_signature BYTEA,
    block_hash BYTEA,
    verification_type SMALLINT DEFAULT 0,    -- 0=EOA, 1=contract
    chain_id INTEGER,                         -- Ethereum=1, etc.
    timestamp BIGINT NOT NULL,
    message_hash BYTEA UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shard_id INTEGER,
    block_height BIGINT,
    transaction_fid BIGINT,
    
    -- One address per FID (latest verification)
    UNIQUE(fid, address)
);

-- Indexes for verifications
CREATE INDEX IF NOT EXISTS idx_verifications_fid ON verifications(fid);
CREATE INDEX IF NOT EXISTS idx_verifications_address ON verifications(address);
CREATE INDEX IF NOT EXISTS idx_verifications_timestamp ON verifications(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_verifications_shard_block ON verifications(shard_id, block_height);

COMMENT ON TABLE verifications IS 'Stores verified addresses for users';
COMMENT ON COLUMN verifications.address IS 'Verified Ethereum or other blockchain address';

-- ============================================================================
-- CAST_EMBEDS TABLE (OPTIONAL) - Dedicated storage for cast embeds
-- ============================================================================
-- Currently embeds are stored as JSONB in casts.embeds
-- This table would normalize them for better querying

CREATE TABLE IF NOT EXISTS cast_embeds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cast_hash BYTEA NOT NULL REFERENCES casts(message_hash) ON DELETE CASCADE,
    embed_type VARCHAR(20) NOT NULL,         -- 'url' or 'cast_id'
    url TEXT,                                 -- If embed_type='url'
    embed_cast_fid BIGINT,                   -- If embed_type='cast_id'
    embed_cast_hash BYTEA,                   -- If embed_type='cast_id'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cast_embeds_cast_hash ON cast_embeds(cast_hash);
CREATE INDEX IF NOT EXISTS idx_cast_embeds_type ON cast_embeds(embed_type);
CREATE INDEX IF NOT EXISTS idx_cast_embeds_url ON cast_embeds(url) WHERE url IS NOT NULL;

COMMENT ON TABLE cast_embeds IS 'Normalized storage for cast embeds (URLs and quoted casts)';

-- ============================================================================
-- STATISTICS
-- ============================================================================

DO $$
BEGIN
    RAISE NOTICE '================================================';
    RAISE NOTICE 'MIGRATION COMPLETE';
    RAISE NOTICE '================================================';
    RAISE NOTICE 'Created tables:';
    RAISE NOTICE '  - reactions (for likes and recasts)';
    RAISE NOTICE '  - verifications (for address verifications)';
    RAISE NOTICE '  - cast_embeds (optional, for normalized embeds)';
    RAISE NOTICE '';
    RAISE NOTICE 'Next steps:';
    RAISE NOTICE '  1. Update code to populate these tables';
    RAISE NOTICE '  2. Run sync or backfill to populate data';
    RAISE NOTICE '================================================';
END $$;

