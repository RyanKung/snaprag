//! Embedding API clients for various providers

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::info;

use crate::errors::Result;
use crate::errors::SnapragError;

/// Supported embedding providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    /// `OpenAI` embeddings API
    OpenAI,
    /// Ollama local embeddings
    Ollama,
}

/// Client for generating embeddings from various providers
pub struct EmbeddingClient {
    provider: EmbeddingProvider,
    model: String,
    endpoint: String,
    api_key: Option<String>,
    client: Client,
}

impl EmbeddingClient {
    /// Create a new embedding client
    pub fn new(
        provider: EmbeddingProvider,
        model: String,
        endpoint: String,
        api_key: Option<String>,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| SnapragError::HttpError(e.to_string()))?;

        Ok(Self {
            provider,
            model,
            endpoint,
            api_key,
            client,
        })
    }

    /// Generate embedding for a single text
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        match self.provider {
            EmbeddingProvider::OpenAI => self.generate_openai(text).await,
            EmbeddingProvider::Ollama => self.generate_ollama(text).await,
        }
    }

    /// Generate embeddings for multiple texts in batch
    pub async fn generate_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        match self.provider {
            EmbeddingProvider::OpenAI => self.generate_batch_openai(texts).await,
            EmbeddingProvider::Ollama => {
                // Ollama doesn't support batch, so we do it sequentially
                let mut embeddings = Vec::with_capacity(texts.len());
                for text in texts {
                    embeddings.push(self.generate_ollama(text).await?);
                }
                Ok(embeddings)
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

        let result: OpenAIResponse = response.json().await.map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to parse response: {e}"))
        })?;

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

        let result: OpenAIResponse = response.json().await.map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to parse response: {e}"))
        })?;

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

        let result: OllamaResponse = response.json().await.map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to parse response: {e}"))
        })?;

        Ok(result.embedding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key
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
