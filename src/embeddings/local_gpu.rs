//! Local GPU embedding client for nomic-embed-text-v1
//!
//! This module provides local GPU-accelerated embedding generation using the
//! nomic-embed-text-v1 model from HuggingFace.
//!
//! This module is only available when the `local-gpu` feature is enabled.

use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "local-gpu")]
use candle_core::Device;
#[cfg(feature = "local-gpu")]
use candle_core::Tensor;
#[cfg(feature = "local-gpu")]
use candle_nn::VarBuilder;
#[cfg(feature = "local-gpu")]
use candle_transformers::models::bert::BertModel;
#[cfg(feature = "local-gpu")]
use candle_transformers::models::bert::Config;
#[cfg(feature = "local-gpu")]
use hf_hub::api::tokio::Api;
#[cfg(feature = "local-gpu")]
use tokenizers::Tokenizer;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::errors::Result;
use crate::errors::SnapragError;

/// Local GPU client for nomic-embed-text-v1 embeddings
#[cfg(feature = "local-gpu")]
pub struct LocalGPUClient {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    model_path: PathBuf,
}

#[cfg(feature = "local-gpu")]
impl LocalGPUClient {
    /// Create a new local GPU client
    pub fn new(model_name: &str) -> Result<Self> {
        info!("Initializing local GPU client for model: {}", model_name);

        // Determine device (CUDA > Metal > CPU)
        let device = Self::get_best_device()?;
        info!("Using device: {:?}", device);

        // Download model if needed
        let model_path = Self::download_model(model_name)?;

        // Load tokenizer
        let tokenizer = Self::load_tokenizer(&model_path)?;

        // Load model
        let model = Self::load_model(&model_path, &device)?;

        Ok(Self {
            model,
            tokenizer,
            device,
            model_path,
        })
    }

    /// Get the best available device (CUDA > Metal > CPU)
    fn get_best_device() -> Result<Device> {
        if Device::cuda_if_available(0).is_ok() {
            info!("Using CUDA device");
            Ok(Device::cuda_if_available(0)?)
        } else if Device::metal_if_available(0).is_ok() {
            info!("Using Metal device");
            Ok(Device::metal_if_available(0)?)
        } else {
            info!("Using CPU device");
            Ok(Device::Cpu)
        }
    }

    /// Download model from HuggingFace if not already cached
    fn download_model(model_name: &str) -> Result<PathBuf> {
        let api = Api::new()?;
        let repo = api.model(model_name.to_string());

        // Check if model is already cached
        let model_path = repo.get("model.safetensors");
        let config_path = repo.get("config.json");

        match (model_path, config_path) {
            (Ok(model_path), Ok(config_path)) => {
                info!("Model already cached at: {:?}", model_path.parent());
                Ok(model_path.parent().unwrap().to_path_buf())
            }
            _ => {
                info!("Downloading model: {}", model_name);
                let model_path = repo.get("model.safetensors")?;
                let _config_path = repo.get("config.json")?;
                info!("Model downloaded successfully");
                Ok(model_path.parent().unwrap().to_path_buf())
            }
        }
    }

    /// Load tokenizer from model path
    fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
        let tokenizer_path = model_path.join("tokenizer.json");
        if tokenizer_path.exists() {
            Tokenizer::from_file(&tokenizer_path).map_err(|e| {
                SnapragError::EmbeddingError(format!("Failed to load tokenizer: {}", e))
            })
        } else {
            // Fallback to BERT tokenizer
            Tokenizer::from_pretrained("bert-base-uncased", None).map_err(|e| {
                SnapragError::EmbeddingError(format!("Failed to load BERT tokenizer: {}", e))
            })
        }
    }

    /// Load model from model path
    fn load_model(model_path: &PathBuf, device: &Device) -> Result<BertModel> {
        let config_path = model_path.join("config.json");
        let config = std::fs::read_to_string(&config_path).map_err(|e| SnapragError::Io(e))?;

        let config: Config = serde_json::from_str(&config)
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to parse config: {}", e)))?;

        let model_path = model_path.join("model.safetensors");
        let weights = unsafe { candle_core::safetensors::load(&model_path, device)? };
        let vb = VarBuilder::from_tensors(weights, candle_core::DType::F32, device);

        BertModel::load(&vb, &config)
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to load model: {}", e)))
    }

    /// Generate embedding for a single text
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text: {}", text);

        // Add task prefix if not present
        let prefixed_text = if !text.starts_with("search_document:")
            && !text.starts_with("search_query:")
            && !text.starts_with("clustering:")
            && !text.starts_with("classification:")
        {
            format!("search_document: {}", text)
        } else {
            text.to_string()
        };

        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(prefixed_text, true)
            .map_err(|e| SnapragError::EmbeddingError(format!("Tokenization failed: {}", e)))?;

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)?;
        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)?;

        // Add batch dimension
        let input_ids = input_ids.unsqueeze(0)?;
        let attention_mask = attention_mask.unsqueeze(0)?;

        // Generate embeddings
        let embeddings = self.model.forward(&input_ids, &attention_mask)?;

        // Mean pooling
        let pooled = self.mean_pooling(&embeddings, &attention_mask)?;

        // Normalize
        let normalized = self.normalize(&pooled)?;

        // Convert to Vec<f32>
        let embedding_vec: Vec<f32> = normalized.to_vec1()?;

        debug!(
            "Generated embedding with {} dimensions",
            embedding_vec.len()
        );
        Ok(embedding_vec)
    }

    /// Generate embeddings for multiple texts in batch
    pub async fn generate_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        debug!("Generating embeddings for {} texts", texts.len());

        let mut embeddings = Vec::with_capacity(texts.len());

        // Process in smaller batches to avoid memory issues
        const BATCH_SIZE: usize = 32;

        for chunk in texts.chunks(BATCH_SIZE) {
            let chunk_embeddings = self.generate_batch_chunk(chunk).await?;
            embeddings.extend(chunk_embeddings);
        }

        debug!("Generated {} embeddings", embeddings.len());
        Ok(embeddings)
    }

    /// Generate embeddings for a chunk of texts
    async fn generate_batch_chunk(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Add task prefixes
        let prefixed_texts: Vec<String> = texts
            .iter()
            .map(|&text| {
                if !text.starts_with("search_document:")
                    && !text.starts_with("search_query:")
                    && !text.starts_with("clustering:")
                    && !text.starts_with("classification:")
                {
                    format!("search_document: {}", text)
                } else {
                    text.to_string()
                }
            })
            .collect();

        // Tokenize all texts
        let encodings: Result<Vec<_>> = prefixed_texts
            .iter()
            .map(|text| {
                self.tokenizer.encode(text, true).map_err(|e| {
                    SnapragError::EmbeddingError(format!("Tokenization failed: {}", e))
                })
            })
            .collect();
        let encodings = encodings?;

        // Find max length for padding
        let max_len = encodings.iter().map(|e| e.len()).max().unwrap_or(0);

        // Create batch tensors
        let batch_size = texts.len();
        let mut input_ids = Vec::with_capacity(batch_size * max_len);
        let mut attention_mask = Vec::with_capacity(batch_size * max_len);

        for encoding in &encodings {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();

            // Pad to max length
            for i in 0..max_len {
                input_ids.push(if i < ids.len() { ids[i] } else { 0 });
                attention_mask.push(if i < mask.len() { mask[i] } else { 0 });
            }
        }

        let input_ids = Tensor::new(&input_ids, &self.device)?.reshape((batch_size, max_len))?;
        let attention_mask =
            Tensor::new(&attention_mask, &self.device)?.reshape((batch_size, max_len))?;

        // Generate embeddings
        let embeddings = self.model.forward(&input_ids, &attention_mask)?;

        // Mean pooling for each item in batch
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let item_embeddings = embeddings.i(i)?;
            let item_mask = attention_mask.i(i)?;
            let pooled = self.mean_pooling(&item_embeddings, &item_mask)?;
            let normalized = self.normalize(&pooled)?;
            let embedding_vec: Vec<f32> = normalized.to_vec1()?;
            results.push(embedding_vec);
        }

        Ok(results)
    }

    /// Mean pooling of embeddings
    fn mean_pooling(&self, embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        let mask_expanded = attention_mask.unsqueeze(-1)?.expand_as(embeddings)?;
        let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
        let sum_mask = mask_expanded.sum(1)?.clamp(1e-9, f32::MAX)?;
        sum_embeddings.broadcast_div(&sum_mask)
    }

    /// Normalize embeddings to unit length
    fn normalize(&self, embeddings: &Tensor) -> Result<Tensor> {
        let norm = embeddings.sqr()?.sum(1)?.sqrt()?;
        embeddings.broadcast_div(&norm)
    }
}

#[cfg(feature = "local-gpu")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires GPU and model download
    async fn test_local_gpu_embedding() {
        let client = LocalGPUClient::new("nomic-ai/nomic-embed-text-v1").unwrap();

        let embedding = client.generate("Hello, world!").await.unwrap();
        assert_eq!(embedding.len(), 768); // nomic-embed-text-v1 dimension

        let embeddings = client.generate_batch(vec!["Hello", "World"]).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 768);
    }
}

// Stub implementation when local-gpu feature is disabled
#[cfg(not(feature = "local-gpu"))]
pub struct LocalGPUClient;

#[cfg(not(feature = "local-gpu"))]
impl LocalGPUClient {
    pub fn new(_model_name: &str) -> Result<Self> {
        Err(SnapragError::ConfigError(
            "Local GPU support not compiled. Enable 'local-gpu' feature to use local GPU embeddings.".to_string()
        ))
    }

    pub async fn generate(&self, _text: &str) -> Result<Vec<f32>> {
        Err(SnapragError::ConfigError(
            "Local GPU support not compiled. Enable 'local-gpu' feature to use local GPU embeddings.".to_string()
        ))
    }

    pub async fn generate_batch(&self, _texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        Err(SnapragError::ConfigError(
            "Local GPU support not compiled. Enable 'local-gpu' feature to use local GPU embeddings.".to_string()
        ))
    }
}
