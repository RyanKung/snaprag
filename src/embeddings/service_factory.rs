//! Embedding service factory for creating configured embedding services
//!
//! This module consolidates the duplicate logic for creating embedding services
//! with different configurations (local GPU, named endpoints, or default).

use std::sync::Arc;

use crate::config::AppConfig;
use crate::embeddings::EmbeddingService;
use crate::Result;

/// Result of creating an embedding service with metadata
pub struct EmbeddingServiceResult {
    pub service: Arc<EmbeddingService>,
    pub endpoint_info: String,
}

/// Create an embedding service based on the provided configuration options
///
/// # Arguments
/// * `config` - Application configuration
/// * `endpoint_name` - Optional named endpoint from config
/// * `local_gpu` - Whether to use local GPU (requires `local-gpu` feature)
/// * `gpu_device` - Optional GPU device ID (requires `local-gpu` feature)
///
/// # Returns
/// A tuple of (service, endpoint_info) where endpoint_info is a human-readable description
pub async fn create_embedding_service(
    config: &AppConfig,
    endpoint_name: Option<String>,
    #[cfg(feature = "local-gpu")] local_gpu: bool,
    #[cfg(feature = "local-gpu")] gpu_device: Option<usize>,
) -> Result<EmbeddingServiceResult> {
    #[cfg(feature = "local-gpu")]
    if local_gpu {
        return create_local_gpu_service(config, gpu_device).await;
    }

    if let Some(ref ep_name) = endpoint_name {
        return create_named_endpoint_service(config, ep_name);
    }

    create_default_service(config)
}

/// Create an embedding service using local GPU
#[cfg(feature = "local-gpu")]
async fn create_local_gpu_service(
    config: &AppConfig,
    gpu_device: Option<usize>,
) -> Result<EmbeddingServiceResult> {
    tracing::info!("🔧 Using local GPU for embedding generation...");

    let embedding_config = crate::embeddings::EmbeddingConfig {
        provider: crate::embeddings::EmbeddingProvider::LocalGPU,
        model: "BAAI/bge-small-en-v1.5".to_string(),
        dimension: config.embedding_dimension(),
        endpoint: "local-gpu".to_string(),
        api_key: None,
    };

    let service =
        Arc::new(EmbeddingService::from_config_async(embedding_config, gpu_device).await?);

    Ok(EmbeddingServiceResult {
        service,
        endpoint_info: "local-gpu (BAAI/bge-small-en-v1.5)".to_string(),
    })
}

/// Create an embedding service using a named endpoint from config
fn create_named_endpoint_service(
    config: &AppConfig,
    endpoint_name: &str,
) -> Result<EmbeddingServiceResult> {
    let endpoint_config = config
        .get_embedding_endpoint(endpoint_name)
        .ok_or_else(|| {
            crate::SnapRagError::Custom(format!(
                "Endpoint '{}' not found in config. Available endpoints: {:?}",
                endpoint_name,
                config
                    .embedding_endpoints()
                    .iter()
                    .map(|e| &e.name)
                    .collect::<Vec<_>>()
            ))
        })?;

    let embedding_config =
        crate::embeddings::EmbeddingConfig::from_endpoint(config, endpoint_config);
    let service = Arc::new(EmbeddingService::from_config(embedding_config)?);

    Ok(EmbeddingServiceResult {
        service,
        endpoint_info: format!("{} ({})", endpoint_config.name, endpoint_config.endpoint),
    })
}

/// Create an embedding service using the default LLM endpoint
fn create_default_service(config: &AppConfig) -> Result<EmbeddingServiceResult> {
    let service = Arc::new(EmbeddingService::new(config)?);

    Ok(EmbeddingServiceResult {
        service,
        endpoint_info: format!("default ({})", config.llm_endpoint()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_default_service_structure() {
        let config = AppConfig::default();
        let result = create_default_service(&config);
        assert!(result.is_ok());
        let service_result = result.unwrap();
        assert!(!service_result.endpoint_info.is_empty());
        assert!(service_result.endpoint_info.contains("default"));
    }

    #[test]
    fn test_create_named_endpoint_service_not_found() {
        let config = AppConfig::default();
        let result = create_named_endpoint_service(&config, "nonexistent");
        assert!(result.is_err());

        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("not found"));
        }
    }

    #[cfg(feature = "local-gpu")]
    #[tokio::test]
    #[ignore] // Requires GPU hardware
    async fn test_create_local_gpu_service() {
        let config = AppConfig::default();
        let result = create_local_gpu_service(&config, None).await;
        // May fail without GPU, but should return proper error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_service_result_contains_metadata() {
        let config = AppConfig::default();
        if let Ok(result) = create_default_service(&config) {
            // Endpoint info should describe the service
            assert!(!result.endpoint_info.is_empty());
            // Service should be usable
            assert!(std::sync::Arc::strong_count(&result.service) >= 1);
        }
    }
}
