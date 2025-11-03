//! Unit tests for data models and query builders

#[cfg(test)]
mod tests {
    use crate::models::*;

    // ====== Query Builder Tests ======

    #[test]
    fn test_user_profile_query_builder() {
        let query = UserProfileQuery {
            fid: Some(123),
            username: Some("alice".to_string()),
            display_name: None,
            bio: None,
            location: None,
            twitter_username: None,
            github_username: None,
            limit: Some(10),
            offset: Some(0),
            start_timestamp: None,
            end_timestamp: None,
            sort_by: Some(ProfileSortBy::LastUpdated),
            sort_order: Some(SortOrder::Desc),
            search_term: None,
        };

        assert_eq!(query.fid, Some(123));
        assert_eq!(query.username, Some("alice".to_string()));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_cast_query_text_search() {
        let query = CastQuery {
            fid: None,
            text_search: Some("blockchain".to_string()),
            parent_hash: None,
            root_hash: None,
            has_mentions: Some(true),
            has_embeds: None,
            start_timestamp: Some(1000000),
            end_timestamp: Some(2000000),
            limit: Some(50),
            offset: None,
            sort_by: Some(CastSortBy::Timestamp),
            sort_order: Some(SortOrder::Desc),
        };

        assert!(query.text_search.is_some());
        assert_eq!(query.has_mentions, Some(true));
        assert_eq!(query.start_timestamp, Some(1000000));
    }

    #[test]
    fn test_link_query_follow_type() {
        let query = LinkQuery {
            fid: Some(100),
            target_fid: Some(200),
            link_type: Some("follow".to_string()),
            start_timestamp: None,
            end_timestamp: None,
            limit: Some(100),
            offset: None,
            sort_by: Some(LinkSortBy::Timestamp),
            sort_order: Some(SortOrder::Asc),
        };

        assert_eq!(query.fid, Some(100));
        assert_eq!(query.target_fid, Some(200));
        assert_eq!(query.link_type, Some("follow".to_string()));
    }

    // ====== Sort Enum Tests ======

    #[test]
    fn test_sort_order_equality() {
        assert_eq!(SortOrder::Asc, SortOrder::Asc);
        assert_eq!(SortOrder::Desc, SortOrder::Desc);
        assert_ne!(SortOrder::Asc, SortOrder::Desc);
    }

    #[test]
    fn test_profile_sort_by_enum() {
        let variants = vec![ProfileSortBy::Fid, ProfileSortBy::LastUpdated];
        assert_eq!(variants.len(), 2);
        
        for variant in variants {
            let debug_str = format!("{:?}", variant);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_cast_sort_by_enum() {
        let by_time = CastSortBy::Timestamp;
        let by_fid = CastSortBy::Fid;
        
        assert!(matches!(by_time, CastSortBy::Timestamp));
        assert!(matches!(by_fid, CastSortBy::Fid));
    }

    #[test]
    fn test_link_sort_by_enum() {
        let sorts = vec![
            LinkSortBy::Timestamp,
            LinkSortBy::Fid,
            LinkSortBy::TargetFid,
        ];
        assert_eq!(sorts.len(), 3);
    }

    // ====== Statistics Tests ======

    #[test]
    fn test_statistics_query_empty() {
        let query = StatisticsQuery {
            start_date: None,
            end_date: None,
            group_by: None,
        };

        assert!(query.start_date.is_none());
        assert!(query.end_date.is_none());
    }

    #[test]
    fn test_statistics_query_with_period() {
        let query = StatisticsQuery {
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-12-31".to_string()),
            group_by: Some("month".to_string()),
        };

        assert_eq!(query.start_date.as_deref(), Some("2024-01-01"));
        assert_eq!(query.end_date.as_deref(), Some("2024-12-31"));
        assert_eq!(query.group_by.as_deref(), Some("month"));
    }

    // ====== Shard Block Info Tests ======

    #[test]
    fn test_shard_block_info() {
        let info = ShardBlockInfo {
            shard_id: 0,
            current_block: 1000,
            last_synced_block: Some(990),
        };

        assert_eq!(info.shard_id, 0);
        assert_eq!(info.current_block, 1000);
        assert_eq!(info.last_synced_block, Some(990));
    }

    #[test]
    fn test_shard_block_info_no_sync() {
        let info = ShardBlockInfo {
            shard_id: 2,
            current_block: 500,
            last_synced_block: None,
        };

        assert!(info.last_synced_block.is_none());
        assert_eq!(info.shard_id, 2);
    }

    // ====== Model Default Tests ======

    #[test]
    fn test_query_with_optional_fields() {
        let query = CastQuery {
            fid: None,
            text_search: None,
            parent_hash: None,
            root_hash: None,
            has_mentions: None,
            has_embeds: None,
            start_timestamp: None,
            end_timestamp: None,
            limit: Some(20),
            offset: Some(0),
            sort_by: Some(CastSortBy::Timestamp),
            sort_order: Some(SortOrder::Desc),
        };

        // All optional fields should be None
        assert!(query.fid.is_none());
        assert!(query.text_search.is_none());
        assert!(query.has_mentions.is_none());
        
        // Required pagination should have values
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.offset, Some(0));
    }
}

