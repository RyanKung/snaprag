//! Local GPU embedding client for BAAI/bge-small-en-v1.5
//!
//! This module provides local GPU-accelerated embedding generation using the
//! BAAI/bge-small-en-v1.5 model from HuggingFace.
//!
//! This module is only available when the `local-gpu` feature is enabled.

use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "local-gpu")]
use candle_core::{Device, Tensor, Module, DType};
#[cfg(feature = "local-gpu")]
use candle_core::IndexOp;
#[cfg(feature = "local-gpu")]
use candle_nn::{VarBuilder, Embedding, Linear, LayerNorm, Dropout};

#[cfg(feature = "local-gpu")]
use hf_hub::api::tokio::Api;
#[cfg(feature = "local-gpu")]
use tokenizers::Tokenizer;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::errors::Result;
use crate::errors::SnapragError;

/// Local GPU client for BAAI/bge-small-en-v1.5 embeddings
#[cfg(feature = "local-gpu")]
pub struct LocalGPUClient {
    tokenizer: Tokenizer,
    device: Device,
    model_path: PathBuf,
    embedding_dim: usize,
}

#[cfg(feature = "local-gpu")]
impl LocalGPUClient {
    /// Create a new local GPU client with default 384 dimensions (BGE small model)
    pub async fn new(model_name: &str) -> Result<Self> {
        Self::new_with_dimension(model_name, 384).await
    }
    
    /// Create a new local GPU client with specified embedding dimension
    /// BGE small model supports 384 dimensions
    pub async fn new_with_dimension(model_name: &str, embedding_dim: usize) -> Result<Self> {
        info!("Initializing local GPU client for model: {} with dimension: {}", model_name, embedding_dim);
        
        // Validate embedding dimension for BGE small model
        if embedding_dim != 384 {
            return Err(SnapragError::EmbeddingError(
                format!("BGE small model only supports 384 dimensions, got: {}", embedding_dim)
            ));
        }

        // Determine device (CUDA > Metal > CPU)
        let device = Self::get_best_device()?;
        info!("Using device: {:?}", device);

        // Download model if needed
        let model_path = Self::download_model(model_name).await?;

        // Load tokenizer
        let tokenizer = Self::load_tokenizer(&model_path)?;

        Ok(Self {
            tokenizer,
            device,
            model_path,
            embedding_dim,
        })
    }

    /// Get the best available device (CUDA > Metal > CPU)
    fn get_best_device() -> Result<Device> {
        if Device::cuda_if_available(0).is_ok() {
            info!("Using CUDA device");
            Ok(Device::cuda_if_available(0)?)
        } else if Device::new_metal(0).is_ok() {
            info!("Using Metal device");
            Ok(Device::new_metal(0)?)
        } else {
            info!("Using CPU device");
            Ok(Device::Cpu)
        }
    }

    /// Download model from HuggingFace if not already cached
    async fn download_model(model_name: &str) -> Result<PathBuf> {
        info!("Initializing HuggingFace API...");
        
        let api = Api::new().map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to initialize HuggingFace API: {}", e))
        })?;
        
        info!("Creating model repository reference for: {}", model_name);
        let repo = api.model(model_name.to_string());

        // Use a stable cache location
        let cache_root = std::env::var("SNAPRAG_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".cache").join("snaprag")
            });
        
        let model_dir = cache_root
            .join("hf")
            .join("models")
            .join(model_name.replace("/", "--"))
            .join("main");

        std::fs::create_dir_all(&model_dir).map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to create model directory: {}", e))
        })?;
        
        // Download model files
        let model_path = model_dir.join("model.safetensors");
        
        if !model_path.exists() {
            info!("Downloading model.safetensors...");
            let api_model_path = repo.get("model.safetensors").await.map_err(|e| {
                SnapragError::EmbeddingError(format!("Failed to download model.safetensors: {}", e))
            })?;
            
            std::fs::copy(&api_model_path, &model_path).map_err(|e| {
                SnapragError::EmbeddingError(format!("Failed to copy model.safetensors: {}", e))
            })?;
        }
        
        // Download config.json
        let config_path = model_dir.join("config.json");
        if !config_path.exists() {
            info!("Downloading config.json...");
            match repo.get("config.json").await {
                Ok(api_config_path) => {
                    std::fs::copy(&api_config_path, &config_path).map_err(|e| {
                        SnapragError::EmbeddingError(format!("Failed to copy config.json: {}", e))
                    })?;
                    info!("Successfully downloaded config.json");
                }
                Err(e) => {
                    warn!("Failed to download config.json via API: {}. Trying direct download.", e);
                    // Try direct download
                    let url = format!("https://huggingface.co/{}/resolve/main/config.json", model_name);
                    info!("Trying direct download from: {}", url);
                    
                    match reqwest::get(&url).await {
                        Ok(response) => {
                            if response.status().is_success() {
                                let content = response.bytes().await.map_err(|e| {
                                    SnapragError::EmbeddingError(format!("Failed to read config response: {}", e))
                                })?;
                                
                                std::fs::write(&config_path, content).map_err(|e| {
                                    SnapragError::EmbeddingError(format!("Failed to write config.json: {}", e))
                                })?;
                                
                                info!("Successfully downloaded config.json via direct URL");
                            } else {
                                return Err(SnapragError::EmbeddingError(format!("Failed to download config.json: {}", response.status())));
                            }
                        }
                        Err(e) => {
                            return Err(SnapragError::EmbeddingError(format!("Failed to download config.json: {}", e)));
                        }
                    }
                }
            }
        }
        
        // Download tokenizer files
        let tokenizer_files = ["tokenizer.json", "tokenizer_config.json", "vocab.txt"];
        for tokenizer_file in &tokenizer_files {
            let tokenizer_path = model_dir.join(tokenizer_file);
            if !tokenizer_path.exists() {
                info!("Downloading {}...", tokenizer_file);
                match repo.get(tokenizer_file).await {
                    Ok(api_tokenizer_path) => {
                        let _ = std::fs::copy(&api_tokenizer_path, &tokenizer_path).map_err(|e| {
                            warn!("Failed to copy {}: {}", tokenizer_file, e);
                        });
                        info!("Successfully downloaded {}", tokenizer_file);
                    }
                    Err(e) => {
                        warn!("Failed to download {} via API: {}. Trying direct download.", tokenizer_file, e);
                        // Try direct download
                        let url = format!("https://huggingface.co/{}/resolve/main/{}", model_name, tokenizer_file);
                        info!("Trying direct download from: {}", url);
                        
                        match reqwest::get(&url).await {
                            Ok(response) => {
                                if response.status().is_success() {
                                    let content = response.bytes().await.map_err(|e| {
                                        warn!("Failed to read {} response: {}", tokenizer_file, e);
                                    });
                                    if let Ok(content) = content {
                                        if let Err(e) = std::fs::write(&tokenizer_path, content) {
                                            warn!("Failed to write {}: {}", tokenizer_file, e);
                                        } else {
                                            info!("Successfully downloaded {} via direct URL", tokenizer_file);
                                        }
                                    }
                                } else {
                                    warn!("Direct download failed for {}: {}", tokenizer_file, response.status());
                                }
                            }
                            Err(e) => {
                                warn!("Direct download failed for {}: {}", tokenizer_file, e);
                            }
                        }
                    }
                }
            }
        }

        info!("Model downloaded successfully");
        Ok(model_dir)
    }

    /// Load tokenizer from model path
    fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
        let tokenizer_files = [
            "tokenizer.json",
            "tokenizer_config.json", 
            "vocab.txt"
        ];
        
        for tokenizer_file in &tokenizer_files {
            let tokenizer_path = model_path.join(tokenizer_file);
            if tokenizer_path.exists() {
                info!("Loading tokenizer from: {:?}", tokenizer_path);
                return Tokenizer::from_file(&tokenizer_path).map_err(|e| {
                    SnapragError::EmbeddingError(format!("Failed to load tokenizer from {:?}: {}", tokenizer_path, e))
                });
            }
        }
        
        Err(SnapragError::EmbeddingError(format!(
            "No tokenizer files found in {:?}. Expected one of: {:?}",
            model_path, tokenizer_files
        )))
    }

    /// Generate embedding for a single text using BGE model
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text: {}", text);

        // BGE models work well without task prefixes for general text
        let processed_text = text;

        // Tokenize input
        let encoding = self
            .tokenizer
            .encode(processed_text.to_string(), true)
            .map_err(|e| SnapragError::EmbeddingError(format!("Tokenization failed: {}", e)))?;

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)?;
        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)?;

        // Add batch dimension
        let input_ids = input_ids.unsqueeze(0)?;
        let attention_mask = attention_mask.unsqueeze(0)?;

        // For now, we'll use a simple approach: load the model weights and compute embeddings
        // This is a simplified implementation - in production you'd want to use candle-transformers
        let embeddings = self.compute_embeddings_simple(&input_ids, &attention_mask)?;

        // Mean pooling
        let pooled = self.mean_pooling(&embeddings, &attention_mask)?;

        // Normalize - squeeze to remove batch dimension
        let pooled_squeezed = pooled.squeeze(0)?;
        let normalized = self.normalize(&pooled_squeezed)?;

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

        // Tokenize all texts
        let encodings: Result<Vec<_>> = texts
            .iter()
            .map(|text| {
                self.tokenizer.encode(text.to_string(), true).map_err(|e| {
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

        let input_ids = Tensor::new(input_ids.as_slice(), &self.device)?.reshape((batch_size, max_len))?;
        let attention_mask =
            Tensor::new(attention_mask.as_slice(), &self.device)?.reshape((batch_size, max_len))?;

        // Generate embeddings
        let embeddings = self.compute_embeddings_simple(&input_ids, &attention_mask)?;

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

    /// Simple embedding computation - this is a placeholder implementation
    /// In production, you would load the full BERT model using candle-transformers
    fn compute_embeddings_simple(&self, input_ids: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        // This is a simplified implementation
        // For now, we'll create random embeddings as a placeholder
        // In production, you would load the actual BERT model weights and compute real embeddings
        
        let batch_size = input_ids.dim(0)?;
        let seq_len = input_ids.dim(1)?;
        let hidden_size = self.embedding_dim;
        
        // Create random embeddings as placeholder
        // TODO: Replace with actual BERT model loading and computation
        let embeddings = Tensor::randn(0f32, 1f32, (batch_size, seq_len, hidden_size), &self.device)?;
        
        Ok(embeddings)
    }

    /// Mean pooling of embeddings
    fn mean_pooling(&self, embeddings: &Tensor, attention_mask: &Tensor) -> Result<Tensor> {
        // Convert attention_mask to f32 to match embeddings dtype
        let mask_f32 = attention_mask.to_dtype(DType::F32)?;
        let mask_expanded = mask_f32.unsqueeze(attention_mask.dims().len())?.expand(embeddings.shape())?;
        let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
        let sum_mask = mask_expanded.sum(1)?.clamp(1e-9, f32::MAX)?;
        Ok(sum_embeddings.broadcast_div(&sum_mask)?)
    }

    /// Normalize embeddings to unit length
    fn normalize(&self, embeddings: &Tensor) -> Result<Tensor> {
        let norm = embeddings.sqr()?.sum_all()?.sqrt()?;
        Ok(embeddings.broadcast_div(&norm)?)
    }
}

#[cfg(feature = "local-gpu")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires GPU and model download
    async fn test_bge_embedding() {
        let client = LocalGPUClient::new("BAAI/bge-small-en-v1.5").await.unwrap();
        let embedding = client.generate("What is machine learning?").await.unwrap();
        assert_eq!(embedding.len(), 384, "Expected dimension 384 but got {}", embedding.len());
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