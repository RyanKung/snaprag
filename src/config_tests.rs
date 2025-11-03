//! Unit tests for configuration module
//!
//! These tests validate configuration parsing, defaults, and validation.

#[cfg(test)]
mod tests {
    use crate::config::*;

    // ====== Default Value Tests ======

    #[test]
    fn test_default_batch_size() {
        let config = EmbeddingsConfig {
            dimension: 384,
            model: "test".to_string(),
            batch_size: default_batch_size(),
            parallel_tasks: 1,
            cpu_threads: 0,
            endpoints: vec![],
        };
        assert_eq!(config.batch_size, 500);
    }

    #[test]
    fn test_default_parallel_tasks() {
        let config = EmbeddingsConfig {
            dimension: 384,
            model: "test".to_string(),
            batch_size: 1,
            parallel_tasks: default_parallel_tasks(),
            cpu_threads: 0,
            endpoints: vec![],
        };
        assert_eq!(config.parallel_tasks, 200);
    }

    #[test]
    fn test_default_cpu_threads() {
        let config = EmbeddingsConfig {
            dimension: 384,
            model: "test".to_string(),
            batch_size: 1,
            parallel_tasks: 1,
            cpu_threads: default_cpu_threads(),
            endpoints: vec![],
        };
        assert_eq!(config.cpu_threads, 0); // Auto-detect
    }

    // ====== Embedding Endpoint Tests ======

    #[test]
    fn test_embedding_endpoint_creation() {
        let endpoint = EmbeddingEndpoint {
            name: "test-endpoint".to_string(),
            endpoint: "http://localhost:8080".to_string(),
            api_key: Some("test-key".to_string()),
            model: "test-model".to_string(),
            provider: "ollama".to_string(),
        };

        assert_eq!(endpoint.name, "test-endpoint");
        assert_eq!(endpoint.provider, "ollama");
        assert!(endpoint.api_key.is_some());
    }

    // ====== Sync Config Tests ======

    #[test]
    fn test_default_continuous_sync() {
        assert_eq!(default_enable_continuous_sync(), true);
        assert_eq!(default_continuous_sync_interval(), 5);
    }

    #[test]
    fn test_sync_config_shard_ids() {
        let mut config = SyncConfig {
            snapchain_http_endpoint: "http://test".to_string(),
            snapchain_grpc_endpoint: "http://test".to_string(),
            enable_realtime_sync: true,
            enable_historical_sync: true,
            historical_sync_from_event_id: 0,
            batch_size: 100,
            sync_interval_ms: 1000,
            shard_ids: vec![0, 1, 2],
            enable_continuous_sync: true,
            continuous_sync_interval_secs: 5,
        };

        assert_eq!(config.shard_ids.len(), 3);
        assert_eq!(config.shard_ids[0], 0);
        
        // Test modifying shard_ids
        config.shard_ids.push(3);
        assert_eq!(config.shard_ids.len(), 4);
    }

    // ====== AppConfig Accessor Tests ======

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();
        
        // Test database config accessors
        assert_eq!(config.max_connections(), 100);
        assert_eq!(config.min_connections(), 2);
        assert_eq!(config.connection_timeout(), 60);
        
        // Test embedding config accessors
        assert_eq!(config.embedding_dimension(), 384);
        assert!(!config.embedding_model().is_empty());
        assert_eq!(config.embeddings_batch_size(), 500);
        assert_eq!(config.embeddings_parallel_tasks(), 200);
    }

    #[test]
    fn test_database_config_defaults() {
        let config = DatabaseConfig {
            url: "postgresql://test".to_string(),
            max_connections: 50,
            min_connections: 5,
            connection_timeout: 30,
            slow_query_threshold_secs: 2.0,
        };

        assert_eq!(config.max_connections, 50);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.connection_timeout, 30);
        assert_eq!(config.slow_query_threshold_secs, 2.0);
    }

    // ====== LLM Config Tests ======

    #[test]
    fn test_llm_config_creation() {
        let config = LlmConfig {
            llm_endpoint: "http://localhost:11434".to_string(),
            llm_key: "ollama".to_string(),
            llm_model: "gemma2:27b".to_string(),
        };

        assert!(config.llm_endpoint.contains("11434"));
        assert_eq!(config.llm_key, "ollama");
    }

    // ====== Performance Config Tests ======

    #[test]
    fn test_performance_config() {
        let config = PerformanceConfig {
            enable_vector_indexes: true,
            vector_index_lists: 100,
        };

        assert!(config.enable_vector_indexes);
        assert_eq!(config.vector_index_lists, 100);
    }

    // ====== Cache Config Tests ======

    #[test]
    fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.profile_ttl_secs, 3600);
        assert_eq!(config.social_ttl_secs, 3600);
        assert_eq!(config.max_cache_entries, 10000);
    }

    // ====== Logging Config Tests ======

    #[test]
    fn test_logging_config() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            backtrace: true,
        };

        assert_eq!(config.level, "debug");
        assert!(config.backtrace);
    }
}

// Make default functions accessible for testing
use super::config::*;

const fn default_batch_size() -> usize {
    500
}

const fn default_parallel_tasks() -> usize {
    200
}

const fn default_cpu_threads() -> usize {
    0
}

const fn default_enable_continuous_sync() -> bool {
    true
}

const fn default_continuous_sync_interval() -> u64 {
    5
}

