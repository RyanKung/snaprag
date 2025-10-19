-- Optimize vector indexes for better performance
-- Run this after adding embeddings to improve query performance

\echo 'Optimizing cast_embeddings table...'

-- Update statistics for the query planner
ANALYZE cast_embeddings;

-- Optional: VACUUM to clean up dead tuples
VACUUM ANALYZE cast_embeddings;

-- Show table statistics
SELECT 
    schemaname,
    tablename,
    n_live_tup as "Live Rows",
    n_dead_tup as "Dead Rows",
    last_vacuum,
    last_analyze
FROM pg_stat_user_tables 
WHERE tablename = 'cast_embeddings';

\echo 'Done! Vector indexes optimized.'

