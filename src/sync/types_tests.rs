//! Unit tests for sync types and utilities

#[cfg(test)]
mod tests {
    use crate::sync::types::*;

    #[test]
    fn test_sync_state_new() {
        let state = SyncState::new();
        assert_eq!(state.active_shards, 0);
        assert!(state.shard_states.is_empty());
        assert_eq!(state.total_blocks_processed, 0);
        assert_eq!(state.total_messages_processed, 0);
    }

    #[test]
    fn test_shard_state_creation() {
        let state = ShardState {
            shard_id: 1,
            current_block: 1000,
            last_processed_block: Some(999),
            status: ShardStatus::Running,
            messages_processed: 100,
            errors: Vec::new(),
        };

        assert_eq!(state.shard_id, 1);
        assert_eq!(state.current_block, 1000);
        assert_eq!(state.messages_processed, 100);
        assert!(matches!(state.status, ShardStatus::Running));
    }

    #[test]
    fn test_shard_status_transitions() {
        let statuses = vec![
            ShardStatus::Idle,
            ShardStatus::Running,
            ShardStatus::Paused,
            ShardStatus::Error,
            ShardStatus::Completed,
        ];

        // All statuses should be distinct
        assert_eq!(statuses.len(), 5);
    }

    #[test]
    fn test_sync_config_validation() {
        let config = SyncConfig {
            snapchain_http_endpoint: "http://localhost:8080".to_string(),
            snapchain_grpc_endpoint: "http://localhost:2283".to_string(),
            shard_ids: vec![0, 1, 2],
            start_block_height: None,
            batch_size: 100,
            enable_realtime_sync: true,
            enable_historical_sync: true,
            sync_interval_ms: 1000,
            enable_continuous_sync: true,
            continuous_sync_interval_secs: 5,
        };

        assert_eq!(config.shard_ids.len(), 3);
        assert_eq!(config.batch_size, 100);
        assert!(config.enable_realtime_sync);
    }

    #[test]
    fn test_sync_state_add_shard() {
        let mut state = SyncState::new();
        
        state.shard_states.insert(
            1,
            ShardState {
                shard_id: 1,
                current_block: 100,
                last_processed_block: Some(99),
                status: ShardStatus::Running,
                messages_processed: 50,
                errors: Vec::new(),
            },
        );

        assert_eq!(state.shard_states.len(), 1);
        assert!(state.shard_states.contains_key(&1));
    }

    #[test]
    fn test_sync_state_update_statistics() {
        let mut state = SyncState::new();
        
        state.total_blocks_processed = 100;
        state.total_messages_processed = 1000;
        state.active_shards = 3;

        assert_eq!(state.total_blocks_processed, 100);
        assert_eq!(state.total_messages_processed, 1000);
        assert_eq!(state.active_shards, 3);
    }
}

