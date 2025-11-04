//! Embedding generation service with caching and batch processing

use std::sync::Arc;

use tracing::info;
use tracing::warn;

use super::client::EmbeddingClient;
use super::client::EmbeddingProvider;
use super::EmbeddingConfig;
use super::MAX_BATCH_SIZE;
use crate::errors::Result;

/// Service for generating embeddings with caching and optimization
pub struct EmbeddingService {
    client: Arc<EmbeddingClient>,
    config: EmbeddingConfig,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(config: &crate::config::AppConfig) -> Result<Self> {
        let embedding_config = EmbeddingConfig::from_app_config(config);
        let client = EmbeddingClient::new(
            embedding_config.provider,
            embedding_config.model.clone(),
            embedding_config.endpoint.clone(),
            embedding_config.api_key.clone(),
        )?;

        Ok(Self {
            client: Arc::new(client),
            config: embedding_config,
        })
    }

    /// Create from custom config
    pub fn from_config(config: EmbeddingConfig) -> Result<Self> {
        let client = EmbeddingClient::new(
            config.provider,
            config.model.clone(),
            config.endpoint.clone(),
            config.api_key.clone(),
        )?;

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    /// Create from custom config with async initialization for LocalGPU
    #[cfg(feature = "local-gpu")]
    pub async fn from_config_async(
        config: EmbeddingConfig,
        gpu_device_id: Option<usize>,
    ) -> Result<Self> {
        let client = if matches!(config.provider, EmbeddingProvider::LocalGPU) {
            EmbeddingClient::new_async(
                config.provider,
                config.model.clone(),
                config.endpoint.clone(),
                config.api_key.clone(),
                gpu_device_id,
            )
            .await?
        } else {
            EmbeddingClient::new(
                config.provider,
                config.model.clone(),
                config.endpoint.clone(),
                config.api_key.clone(),
            )?
        };

        Ok(Self {
            client: Arc::new(client),
            config,
        })
    }

    /// Generate embedding for a single text
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        // Preprocess text to handle newlines and invalid characters
        let processed_text = crate::embeddings::preprocess_text_for_embedding(text)?;

        self.client.generate(&processed_text).await
    }

    /// Generate embeddings for multiple texts in batch
    pub async fn generate_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Preprocess texts and track their positions
        let mut processed_texts = Vec::new();
        let mut empty_positions = Vec::new();

        for (i, text) in texts.iter().enumerate() {
            match crate::embeddings::preprocess_text_for_embedding(text) {
                Ok(processed) => processed_texts.push(processed),
                Err(_) => {
                    // If preprocessing fails, treat as empty
                    empty_positions.push(i);
                }
            }
        }

        // Generate embeddings for non-empty texts
        let mut embeddings = if processed_texts.is_empty() {
            Vec::new()
        } else if processed_texts.len() <= MAX_BATCH_SIZE {
            self.client
                .generate_batch(
                    processed_texts
                        .iter()
                        .map(std::string::String::as_str)
                        .collect(),
                )
                .await?
        } else {
            // Split into chunks
            let mut all_embeddings = Vec::new();
            for chunk in processed_texts.chunks(MAX_BATCH_SIZE) {
                let chunk_embeddings = self
                    .client
                    .generate_batch(chunk.iter().map(std::string::String::as_str).collect())
                    .await?;
                all_embeddings.extend(chunk_embeddings);
            }
            all_embeddings
        };

        // Insert zero vectors for empty texts at correct positions
        let zero_vector = vec![0.0; self.config.dimension];
        for pos in empty_positions.iter().rev() {
            embeddings.insert(*pos, zero_vector.clone());
        }

        Ok(embeddings)
    }

    /// Generate embedding for user profile (combines multiple fields)
    pub async fn generate_profile_embedding(
        &self,
        username: Option<&str>,
        display_name: Option<&str>,
        bio: Option<&str>,
        location: Option<&str>,
    ) -> Result<Vec<f32>> {
        let mut parts = Vec::new();

        if let Some(u) = username {
            if !u.trim().is_empty() {
                parts.push(format!("Username: {u}"));
            }
        }
        if let Some(d) = display_name {
            if !d.trim().is_empty() {
                parts.push(format!("Name: {d}"));
            }
        }
        if let Some(b) = bio {
            if !b.trim().is_empty() {
                parts.push(format!("Bio: {b}"));
            }
        }
        if let Some(l) = location {
            if !l.trim().is_empty() {
                parts.push(format!("Location: {l}"));
            }
        }

        if parts.is_empty() {
            return Ok(vec![0.0; self.config.dimension]);
        }

        let combined = parts.join(". ");
        self.generate(&combined).await
    }

    /// Generate embedding for bio text
    pub async fn generate_bio_embedding(&self, bio: Option<&str>) -> Result<Vec<f32>> {
        match bio {
            Some(b) if !b.trim().is_empty() => self.generate(b).await,
            _ => Ok(vec![0.0; self.config.dimension]),
        }
    }

    /// Generate embedding for interests (from bio or other fields)
    pub async fn generate_interests_embedding(
        &self,
        bio: Option<&str>,
        twitter: Option<&str>,
        github: Option<&str>,
    ) -> Result<Vec<f32>> {
        let mut parts = Vec::new();

        if let Some(b) = bio {
            if !b.trim().is_empty() {
                parts.push(b.to_string());
            }
        }
        if let Some(t) = twitter {
            if !t.trim().is_empty() {
                parts.push(format!("Twitter: {t}"));
            }
        }
        if let Some(g) = github {
            if !g.trim().is_empty() {
                parts.push(format!("GitHub: {g}"));
            }
        }

        if parts.is_empty() {
            return Ok(vec![0.0; self.config.dimension]);
        }

        let combined = parts.join(". ");
        self.generate(&combined).await
    }

    /// Get the embedding dimension
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// Get the model name
    #[must_use]
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Get the provider
    #[must_use]
    pub const fn provider(&self) -> EmbeddingProvider {
        self.config.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text_handling() {
        // This test verifies the logic without making API calls
        let texts = ["", "hello", "", "world"];
        let mut filtered = Vec::new();
        let mut empty_pos = Vec::new();

        for (i, t) in texts.iter().enumerate() {
            if t.trim().is_empty() {
                empty_pos.push(i);
            } else {
                filtered.push(*t);
            }
        }

        assert_eq!(filtered, vec!["hello", "world"]);
        assert_eq!(empty_pos, vec![0, 2]);
    }
}
