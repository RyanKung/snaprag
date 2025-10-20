-- Prepare for links re-synchronization
-- This script prepares the database for efficient link data recovery

-- ============================================================================
-- SITUATION ANALYSIS
-- ============================================================================
-- We have:
-- - 188,987,863 link_add activities (events logged)
-- - 0 links in links table (target_fid not captured)
-- - Code now fixed to properly parse and store links
--
-- Goal: Populate links table efficiently
--
-- Options:
-- 1. Re-sync specific shards that have link messages (FAST, targeted)
-- 2. Delete link activities and re-sync all (SLOW, complete)  
-- 3. Keep activities, add marker for re-processing (HYBRID)
-- ============================================================================

-- Check current state
DO $$
DECLARE
    link_activities_count BIGINT;
    links_count BIGINT;
    earliest_link BIGINT;
    latest_link BIGINT;
BEGIN
    SELECT COUNT(*) INTO link_activities_count 
    FROM user_activity_timeline 
    WHERE activity_type = 'link_add';
    
    SELECT COUNT(*) INTO links_count FROM links;
    
    SELECT MIN(block_height), MAX(block_height) 
    INTO earliest_link, latest_link
    FROM user_activity_timeline 
    WHERE activity_type = 'link_add' 
    AND block_height IS NOT NULL;
    
    RAISE NOTICE '================================================';
    RAISE NOTICE 'LINKS DATA STATUS';
    RAISE NOTICE '================================================';
    RAISE NOTICE 'Link activities: %', link_activities_count;
    RAISE NOTICE 'Links in table: %', links_count;
    RAISE NOTICE 'Block range: % to %', earliest_link, latest_link;
    RAISE NOTICE '================================================';
    RAISE NOTICE '';
    RAISE NOTICE 'RECOMMENDED ACTION:';
    RAISE NOTICE '1. Code is now fixed to capture link target_fid';
    RAISE NOTICE '2. Run incremental sync to process new links going forward';
    RAISE NOTICE '3. For historical data, two options:';
    RAISE NOTICE '   A) Re-sync from earliest block (complete but slow)';
    RAISE NOTICE '   B) Delete link activities, re-sync (faster, cleaner)';
    RAISE NOTICE '';
    RAISE NOTICE 'Option B (RECOMMENDED for speed):';
    RAISE NOTICE '  -- This will clear link activity logs';
    RAISE NOTICE '  DELETE FROM user_activity_timeline WHERE activity_type IN (''link_add'', ''link_remove'');';
    RAISE NOTICE '  -- Then re-sync will reprocess and populate links table';
    RAISE NOTICE '';
    RAISE NOTICE 'After this migration, the sync will be much faster since';
    RAISE NOTICE 'it only needs to re-process link messages, not all data.';
    RAISE NOTICE '================================================';
END $$;

-- ============================================================================
-- OPTION: Clear link activities to enable clean re-sync
-- ============================================================================
-- Uncomment below to execute the cleanup:

-- BEGIN;
--
-- -- Backup count before delete
-- DO $$
-- DECLARE
--     count_before BIGINT;
-- BEGIN
--     SELECT COUNT(*) INTO count_before FROM user_activity_timeline 
--     WHERE activity_type IN ('link_add', 'link_remove');
--     RAISE NOTICE 'Deleting % link activity records...', count_before;
-- END $$;
--
-- -- Delete link activities (this allows re-sync to reprocess them)
-- DELETE FROM user_activity_timeline 
-- WHERE activity_type IN ('link_add', 'link_remove');
--
-- -- Verify
-- DO $$
-- DECLARE
--     count_after BIGINT;
-- BEGIN
--     SELECT COUNT(*) INTO count_after FROM user_activity_timeline 
--     WHERE activity_type IN ('link_add', 'link_remove');
--     RAISE NOTICE 'Remaining link activities: %', count_after;
--     RAISE NOTICE 'Ready for re-sync!';
-- END $$;
--
-- COMMIT;

-- ============================================================================
-- AFTER RUNNING THIS:
-- ============================================================================
-- 1. Run: snaprag sync start --realtime
--    This will process new link messages going forward
--
-- 2. For historical data, run targeted re-sync:
--    snaprag sync start --from-block <earliest_block>
--    (This will reprocess all link messages and populate links table)
--
-- 3. The fixed code will now properly extract target_fid and populate links table
-- ============================================================================

