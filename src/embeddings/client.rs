//! Embedding API clients for various providers

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;

use crate::errors::Result;
use crate::errors::SnapragError;

// Local GPU dependencies would be added here in a real implementation

/// Supported embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    /// `OpenAI` embeddings API
    OpenAI,
    /// Ollama local embeddings
    Ollama,
    /// Local GPU embeddings (BAAI/bge-small-en-v1.5) - requires `local-gpu` feature
    #[cfg(feature = "local-gpu")]
    LocalGPU,
}

/// Client for generating embeddings from various providers
pub struct EmbeddingClient {
    provider: EmbeddingProvider,
    model: String,
    endpoint: String,
    api_key: Option<String>,
    client: Client,
    // Local GPU client (only used when provider is LocalGPU)
    #[cfg(feature = "local-gpu")]
    local_gpu_client: Option<crate::embeddings::local_gpu::LocalGPUClient>,
}

impl EmbeddingClient {
    /// Create a new embedding client
    ///
    /// # Errors
    /// - HTTP client build errors (invalid configuration)
    /// - LocalGPU provider requires async initialization (use `new_async()` instead)
    pub fn new(
        provider: EmbeddingProvider,
        model: String,
        endpoint: String,
        api_key: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // Longer timeout for high concurrency
            .pool_max_idle_per_host(100) // Increase connection pool for high concurrency
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        // Initialize local GPU client if needed
        #[cfg(feature = "local-gpu")]
        let local_gpu_client = if matches!(provider, EmbeddingProvider::LocalGPU) {
            // Note: This will need to be handled differently since new() is now async
            // For now, we'll return an error indicating async initialization is needed
            return Err(SnapragError::ConfigError(
                   "Local GPU client requires async initialization. Use EmbeddingClient::new_async() instead.".to_string()
               ));
        } else {
            None
        };

        Ok(Self {
            provider,
            model,
            endpoint,
            api_key,
            client,
            #[cfg(feature = "local-gpu")]
            local_gpu_client,
        })
    }

    /// Create a new embedding client with async initialization for LocalGPU
    #[cfg(feature = "local-gpu")]
    pub async fn new_async(
        provider: EmbeddingProvider,
        model: String,
        endpoint: String,
        api_key: Option<String>,
        gpu_device_id: Option<usize>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(100)
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        // Initialize local GPU client if needed
        let local_gpu_client = if matches!(provider, EmbeddingProvider::LocalGPU) {
            Some(
                crate::embeddings::local_gpu::LocalGPUClient::new_with_dimension(
                    &model,
                    384,
                    gpu_device_id,
                )
                .await?,
            )
        } else {
            None
        };

        Ok(Self {
            provider,
            model,
            endpoint,
            api_key,
            client,
            local_gpu_client,
        })
    }

    /// Generate embedding for a single text
    ///
    /// # Errors
    /// - API request failures (network errors, timeouts, authentication failures)
    /// - Invalid API responses (malformed JSON, wrong embedding dimensions)
    /// - LocalGPU client not initialized
    /// - Provider-specific errors (rate limits, quota exceeded, invalid model)
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        match self.provider {
            EmbeddingProvider::OpenAI => self.generate_openai(text).await,
            EmbeddingProvider::Ollama => self.generate_ollama(text).await,
            #[cfg(feature = "local-gpu")]
            EmbeddingProvider::LocalGPU => {
                let client = self.local_gpu_client.as_ref().ok_or_else(|| {
                    SnapragError::ConfigError("Local GPU client not initialized".to_string())
                })?;
                client.generate(text).await
            }
        }
    }

    /// Generate embeddings for multiple texts in batch
    ///
    /// # Errors
    /// - API request failures (network errors, timeouts, authentication failures)
    /// - Invalid API responses (malformed JSON, dimension mismatches)
    /// - LocalGPU client not initialized
    /// - Provider-specific errors (rate limits, quota exceeded, batch size limits)
    pub async fn generate_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        match self.provider {
            EmbeddingProvider::OpenAI => self.generate_batch_openai(texts).await,
            EmbeddingProvider::Ollama => {
                // Ollama doesn't support batch, so we do it with high concurrency
                use futures::stream::StreamExt;
                use futures::stream::{
                    self,
                };

                // Aggressive concurrency for maximum performance
                let concurrency = std::cmp::min(texts.len(), 200); // High concurrency for performance
                let results: Vec<Result<Vec<f32>>> = stream::iter(texts.iter())
                    .map(|&text| async move { self.generate_ollama(text).await })
                    .buffered(concurrency)
                    .collect()
                    .await;

                // Convert Vec<Result<T, E>> to Result<Vec<T>, E>
                let mut embeddings = Vec::with_capacity(results.len());
                for result in results {
                    embeddings.push(result?);
                }

                Ok(embeddings)
            }
            #[cfg(feature = "local-gpu")]
            EmbeddingProvider::LocalGPU => {
                let client = self.local_gpu_client.as_ref().ok_or_else(|| {
                    SnapragError::ConfigError("Local GPU client not initialized".to_string())
                })?;
                client.generate_batch(texts).await
            }
        }
    }

    /// Generate embedding using `OpenAI` API
    async fn generate_openai(&self, text: &str) -> Result<Vec<f32>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| SnapragError::ConfigError("OpenAI API key not provided".to_string()))?;

        #[derive(Serialize)]
        struct OpenAIRequest<'a> {
            input: &'a str,
            model: &'a str,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let url = format!("{}/embeddings", self.endpoint);
        debug!("Calling OpenAI embeddings API: {}", url);

        let request = OpenAIRequest {
            input: text,
            model: &self.model,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SnapragError::EmbeddingError(format!(
                "OpenAI API error ({status}): {error_text}"
            )));
        }

        let result: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to parse response: {e}")))?;

        result
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| SnapragError::EmbeddingError("No embedding in response".to_string()))
    }

    /// Generate embeddings in batch using `OpenAI` API
    async fn generate_batch_openai(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| SnapragError::ConfigError("OpenAI API key not provided".to_string()))?;

        #[derive(Serialize)]
        struct OpenAIBatchRequest<'a> {
            input: Vec<&'a str>,
            model: &'a str,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let url = format!("{}/embeddings", self.endpoint);
        debug!("Calling OpenAI batch embeddings API: {} items", texts.len());

        let request = OpenAIBatchRequest {
            input: texts,
            model: &self.model,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SnapragError::EmbeddingError(format!(
                "OpenAI API error ({status}): {error_text}"
            )));
        }

        let result: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to parse response: {e}")))?;

        Ok(result.data.into_iter().map(|d| d.embedding).collect())
    }

    /// Generate embedding using Ollama API
    async fn generate_ollama(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct OllamaRequest<'a> {
            model: &'a str,
            prompt: &'a str,
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            embedding: Vec<f32>,
        }

        let url = format!("{}/api/embeddings", self.endpoint);
        debug!("Calling Ollama embeddings API: {}", url);

        let request = OllamaRequest {
            model: &self.model,
            prompt: text,
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SnapragError::EmbeddingError(format!(
                "Ollama API error ({status}): {error_text}"
            )));
        }

        let result: OllamaResponse = response
            .json()
            .await
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to parse response: {e}")))?;

        Ok(result.embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires API key"]
    async fn test_openai_embedding() {
        let client = EmbeddingClient::new(
            EmbeddingProvider::OpenAI,
            "text-embedding-ada-002".to_string(),
            "https://api.openai.com/v1".to_string(),
            std::env::var("OPENAI_API_KEY").ok(),
        )
        .unwrap();

        let embedding = client.generate("Hello, world!").await.unwrap();
        assert_eq!(embedding.len(), 1536);
    }
}
