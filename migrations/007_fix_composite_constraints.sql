-- Fix composite unique constraints that conflict with Farcaster's event-stream design
-- Farcaster allows users to like/unlike/re-like, creating multiple messages for same (fid, target, type)
-- We should record ALL events, not just keep the first one

-- ============================================================================
-- REACTIONS: Remove composite constraint
-- ============================================================================

-- Drop the problematic constraint
-- This constraint prevents recording like -> unlike -> like sequences
ALTER TABLE reactions DROP CONSTRAINT IF EXISTS reactions_fid_target_cast_hash_reaction_type_key;

-- Keep message_hash unique (each message should be stored once)
-- This is already enforced by: message_hash BYTEA UNIQUE NOT NULL

-- ============================================================================
-- VERIFICATIONS: Remove composite constraint  
-- ============================================================================

-- Drop the problematic constraint
-- A user might verify same address multiple times (e.g., re-verification after revoke)
ALTER TABLE verifications DROP CONSTRAINT IF EXISTS verifications_fid_address_key;

-- Keep message_hash unique (each message should be stored once)
-- This is already enforced by: message_hash BYTEA UNIQUE NOT NULL

-- ============================================================================
-- RESULT: Event-stream design
-- ============================================================================

-- Now we can record:
-- - User likes cast A (message 1)
-- - User unlikes cast A (message 2)  
-- - User likes cast A again (message 3)
-- All three messages are preserved!

-- Same for verifications:
-- - User verifies address X (message 1)
-- - User removes verification (message 2)
-- - User verifies address X again (message 3)
-- All three messages are preserved!

