-- Turbo Mode: Drop ALL non-essential indexes for maximum write speed
-- Run during bulk sync for 30-50% additional speed boost
-- Usage: psql -h <host> -U snaprag -d snaprag -f scripts/turbo_mode_enable.sql

\echo '🚀 Entering TURBO MODE - dropping non-essential indexes...'

-- Drop reactions non-essential indexes (keep pkey and message_hash UNIQUE)
DROP INDEX IF EXISTS idx_reactions_event_type;
DROP INDEX IF EXISTS idx_reactions_latest;
DROP INDEX IF EXISTS idx_reactions_fid;
DROP INDEX IF EXISTS idx_reactions_target_cast;
DROP INDEX IF EXISTS idx_reactions_target_fid;
DROP INDEX IF EXISTS idx_reactions_type;
DROP INDEX IF EXISTS idx_reactions_timestamp;

-- Drop links non-essential indexes (keep pkey and message_hash UNIQUE)
DROP INDEX IF EXISTS idx_links_event_type;
DROP INDEX IF EXISTS idx_links_latest;
DROP INDEX IF EXISTS idx_links_fid_type;

-- Drop verifications non-essential indexes (keep pkey and message_hash UNIQUE)
DROP INDEX IF EXISTS idx_verifications_event_type;
DROP INDEX IF EXISTS idx_verifications_latest;
DROP INDEX IF EXISTS idx_verifications_fid;
DROP INDEX IF EXISTS idx_verifications_address;
DROP INDEX IF EXISTS idx_verifications_timestamp;

-- Drop casts non-essential indexes (keep pkey and message_hash UNIQUE)
DROP INDEX IF EXISTS idx_casts_fid;

-- Drop processed_messages non-essential indexes (keep pkey and message_hash)
DROP INDEX IF EXISTS idx_processed_shard_height;
DROP INDEX IF EXISTS idx_processed_messages_hash;

\echo '✅ TURBO MODE enabled!'
\echo '⚡ Expected speed boost: +30-50%'
\echo '⚠️  Remember to run turbo_mode_disable.sql after sync completes!'

SELECT 
    'reactions' as table_name,
    COUNT(*) as remaining_indexes
FROM pg_indexes 
WHERE tablename = 'reactions'
UNION ALL
SELECT 'links', COUNT(*) FROM pg_indexes WHERE tablename = 'links'
UNION ALL
SELECT 'verifications', COUNT(*) FROM pg_indexes WHERE tablename = 'verifications'
UNION ALL
SELECT 'casts', COUNT(*) FROM pg_indexes WHERE tablename = 'casts';

