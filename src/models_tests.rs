//! Unit tests for data models
//!
//! Tests validation, serialization, and model behavior.

#[cfg(test)]
mod tests {
    use crate::models::*;

    // ====== Sort Order Tests ======

    #[test]
    fn test_sort_order_equality() {
        assert_eq!(SortOrder::Asc, SortOrder::Asc);
        assert_eq!(SortOrder::Desc, SortOrder::Desc);
        assert_ne!(SortOrder::Asc, SortOrder::Desc);
    }

    #[test]
    fn test_sort_order_debug() {
        let asc = format!("{:?}", SortOrder::Asc);
        let desc = format!("{:?}", SortOrder::Desc);
        assert_eq!(asc, "Asc");
        assert_eq!(desc, "Desc");
    }

    // ====== Profile Sort Tests ======

    #[test]
    fn test_profile_sort_by_variants() {
        let variants = vec![
            ProfileSortBy::Fid,
            ProfileSortBy::LastUpdated,
        ];
        assert_eq!(variants.len(), 2);
    }

    // ====== Cast Sort Tests ======

    #[test]
    fn test_cast_sort_by_variants() {
        let variants = vec![
            CastSortBy::Timestamp,
            CastSortBy::Fid,
        ];
        assert_eq!(variants.len(), 2);
    }

    // ====== Link Sort Tests ======

    #[test]
    fn test_link_sort_by_variants() {
        let variants = vec![
            LinkSortBy::Timestamp,
            LinkSortBy::Fid,
            LinkSortBy::TargetFid,
        ];
        assert_eq!(variants.len(), 3);
    }

    // ====== Query Builder Tests ======

    #[test]
    fn test_user_profile_query_default() {
        let query = UserProfileQuery {
            fid: None,
            username: None,
            display_name: None,
            bio: None,
            location: None,
            twitter_username: None,
            github_username: None,
            limit: Some(20),
            offset: None,
            start_timestamp: None,
            end_timestamp: None,
            sort_by: Some(ProfileSortBy::LastUpdated),
            sort_order: Some(SortOrder::Desc),
            search_term: None,
        };

        assert_eq!(query.limit, Some(20));
        assert_eq!(query.sort_order, Some(SortOrder::Desc));
    }

    #[test]
    fn test_cast_query_with_filters() {
        let query = CastQuery {
            fid: Some(123),
            text_search: Some("test".to_string()),
            parent_hash: None,
            root_hash: None,
            has_mentions: Some(true),
            has_embeds: Some(false),
            start_timestamp: None,
            end_timestamp: None,
            limit: Some(10),
            offset: Some(0),
            sort_by: Some(CastSortBy::Timestamp),
            sort_order: Some(SortOrder::Desc),
        };

        assert_eq!(query.fid, Some(123));
        assert_eq!(query.text_search, Some("test".to_string()));
        assert_eq!(query.has_mentions, Some(true));
        assert_eq!(query.has_embeds, Some(false));
    }

    #[test]
    fn test_link_query_follow_type() {
        let query = LinkQuery {
            fid: Some(100),
            target_fid: None,
            link_type: Some("follow".to_string()),
            start_timestamp: None,
            end_timestamp: None,
            limit: Some(100),
            offset: None,
            sort_by: Some(LinkSortBy::Timestamp),
            sort_order: Some(SortOrder::Desc),
        };

        assert_eq!(query.link_type, Some("follow".to_string()));
        assert_eq!(query.limit, Some(100));
    }

    // ====== Statistics Query Tests ======

    #[test]
    fn test_statistics_query_empty() {
        let query = StatisticsQuery {
            start_date: None,
            end_date: None,
            group_by: None,
        };

        assert!(query.start_date.is_none());
        assert!(query.end_date.is_none());
        assert!(query.group_by.is_none());
    }

    #[test]
    fn test_statistics_query_with_dates() {
        let query = StatisticsQuery {
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-12-31".to_string()),
            group_by: Some("month".to_string()),
        };

        assert!(query.start_date.is_some());
        assert!(query.end_date.is_some());
        assert_eq!(query.group_by, Some("month".to_string()));
    }

    // ====== Shard Block Info Tests ======

    #[test]
    fn test_shard_block_info_creation() {
        let info = ShardBlockInfo {
            shard_id: 0,
            current_block: 1000,
            last_synced_block: Some(950),
        };

        assert_eq!(info.shard_id, 0);
        assert_eq!(info.current_block, 1000);
        assert_eq!(info.last_synced_block, Some(950));
    }

    #[test]
    fn test_shard_block_info_no_sync() {
        let info = ShardBlockInfo {
            shard_id: 1,
            current_block: 100,
            last_synced_block: None,
        };

        assert!(info.last_synced_block.is_none());
    }
}

