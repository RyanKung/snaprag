//! Local GPU embedding client for nomic-embed-text-v1.5
//!
//! This module provides local GPU-accelerated embedding generation using the
//! nomic-embed-text-v1.5 model from HuggingFace.
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

/// Complete NomicBERT model implementation for nomic-embed-text-v1.5
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertModel {
    pub embeddings: Embedding,
    pub token_type_embeddings: Embedding,
    pub layer_norm: LayerNorm,
    pub dropout: Dropout,
    pub layers: Vec<BertLayer>,
    pub device: Device,
    pub config: BertConfig,
}

/// Configuration for NomicBERT model
#[cfg(feature = "local-gpu")]
#[derive(Debug, Clone)]
pub struct BertConfig {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub intermediate_size: usize,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub layer_norm_eps: f64,
    pub hidden_dropout_prob: f64,
    pub attention_probs_dropout_prob: f64,
}

/// Single transformer layer
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertLayer {
    pub attention: BertAttention,
    pub intermediate: BertIntermediate,
    pub output: BertOutput,
}

/// Multi-head attention mechanism
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertAttention {
    pub self_attention: BertSelfAttention,
    pub output: BertSelfOutput,
}

/// Self-attention mechanism with Wqkv (combined Q/K/V)
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertSelfAttention {
    pub wqkv: Linear,
    pub out_proj: Linear,
    pub dropout: Dropout,
    pub num_attention_heads: usize,
    pub attention_head_size: usize,
}

/// Self-attention output layer
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertSelfOutput {
    pub dense: Linear,
    pub layer_norm: LayerNorm,
    pub dropout: Dropout,
}

/// Intermediate MLP layer with SwiGLU activation
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertIntermediate {
    pub fc11: Linear,  // First part of SwiGLU
    pub fc12: Linear,  // Second part of SwiGLU
}

/// Output layer for transformer block
#[cfg(feature = "local-gpu")]
#[derive(Clone)]
pub struct BertOutput {
    pub dense: Linear,
    pub layer_norm: LayerNorm,
    pub dropout: Dropout,
}

#[cfg(feature = "local-gpu")]
impl BertModel {
    pub fn forward_with_mask(
        &self,
        input_ids: &Tensor,
        attention_mask: &Tensor,
    ) -> candle_core::Result<Tensor> {
        let batch_size = input_ids.dim(0)?;
        let seq_len = input_ids.dim(1)?;

        // Get embeddings (no position embeddings - using Rotary PE)
        let token_embeddings = self.embeddings.forward(input_ids)?;
        
        // Token type embeddings (all zeros for single sequence)
        let token_type_ids = Tensor::zeros((batch_size, seq_len), DType::U32, &self.device)?;
        let token_type_embeddings = self.token_type_embeddings.forward(&token_type_ids)?;
        
        // Combine embeddings
        let embeddings = (token_embeddings + token_type_embeddings)?;
        let embeddings = self.layer_norm.forward(&embeddings)?;
        let embeddings = self.dropout.forward(&embeddings, false)?;

        // Pass through transformer layers
        let mut hidden_states = embeddings;
        for layer in &self.layers {
            hidden_states = layer.forward(&hidden_states, attention_mask)?;
        }

        Ok(hidden_states)
    }
}

#[cfg(feature = "local-gpu")]
impl BertLayer {
    pub fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
        let attention_output = self.attention.forward(hidden_states, attention_mask)?;
        let layer_output = self.output.forward(&attention_output, hidden_states)?;
        Ok(layer_output)
    }
}

#[cfg(feature = "local-gpu")]
impl BertAttention {
    pub fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
        let self_output = self.self_attention.forward(hidden_states, attention_mask)?;
        let attention_output = self.output.forward(&self_output, hidden_states)?;
        Ok(attention_output)
    }
}

#[cfg(feature = "local-gpu")]
impl BertSelfAttention {
    pub fn forward(&self, hidden_states: &Tensor, attention_mask: &Tensor) -> candle_core::Result<Tensor> {
        let batch_size = hidden_states.dim(0)?;
        let seq_len = hidden_states.dim(1)?;
        
        // Combined Q/K/V projection
        let qkv = self.wqkv.forward(hidden_states)?;
        
        // Reshape for multi-head attention
        let qkv = qkv.reshape((
            batch_size,
            seq_len,
            3, // Q, K, V
            self.num_attention_heads,
            self.attention_head_size,
        ))?;
        
        // Split into Q, K, V
        let q = qkv.i((.., .., 0, .., ..))?;
        let k = qkv.i((.., .., 1, .., ..))?;
        let v = qkv.i((.., .., 2, .., ..))?;
        
        // Transpose for attention computation
        let q = q.transpose(1, 2)?; // [batch, heads, seq_len, head_size]
        let k = k.transpose(1, 2)?;
        let v = v.transpose(1, 2)?;
        
        // Compute attention scores
        let attention_scores = q.matmul(&k.transpose(2, 3)?)?;
        let attention_scores = (attention_scores / (self.attention_head_size as f64).sqrt())?;
        
        // Apply attention mask
        let attention_mask = attention_mask.unsqueeze(1)?.unsqueeze(2)?;
        let attention_mask_f32 = attention_mask.to_dtype(DType::F32)?;
        let attention_mask_scaled = ((1.0 - attention_mask_f32)? * -10000.0)?;
        let attention_scores = attention_scores.broadcast_add(&attention_mask_scaled)?;
        
        // Softmax
        let attention_probs = candle_nn::ops::softmax(&attention_scores, 3)?;
        let attention_probs = self.dropout.forward(&attention_probs, false)?;
        
        // Apply attention to values
        let context_layer = attention_probs.matmul(&v)?;
        
        // Reshape back
        let context_layer = context_layer.transpose(1, 2)?;
        let context_layer = context_layer.reshape((batch_size, seq_len, self.num_attention_heads * self.attention_head_size))?;
        
        // Output projection
        let context_layer = self.out_proj.forward(&context_layer)?;
        
        Ok(context_layer)
    }
}

#[cfg(feature = "local-gpu")]
impl BertSelfOutput {
    pub fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> candle_core::Result<Tensor> {
        let hidden_states = self.dense.forward(hidden_states)?;
        let hidden_states = self.dropout.forward(&hidden_states, false)?;
        let hidden_states = self.layer_norm.forward(&(hidden_states + input_tensor)?)?;
        Ok(hidden_states)
    }
}

#[cfg(feature = "local-gpu")]
impl BertIntermediate {
    pub fn forward(&self, hidden_states: &Tensor) -> candle_core::Result<Tensor> {
        // SwiGLU activation: SwiGLU(x) = Swish(xW1) * (xW2)
        // where Swish(x) = x * sigmoid(x)
        let fc11_out = self.fc11.forward(hidden_states)?;
        let fc12_out = self.fc12.forward(hidden_states)?;
        
        // Swish activation: x * sigmoid(x)
        // sigmoid(x) = 1 / (1 + exp(-x))
        let neg_fc11 = fc11_out.neg()?;
        let exp_neg = neg_fc11.exp()?;
        let one_plus_exp = (1.0 + exp_neg)?;
        let sigmoid = (1.0 / one_plus_exp)?;
        let swish = (fc11_out * sigmoid)?;
        
        // Element-wise multiplication
        let output = (swish * fc12_out)?;
        
        Ok(output)
    }
}

#[cfg(feature = "local-gpu")]
impl BertOutput {
    pub fn forward(&self, hidden_states: &Tensor, input_tensor: &Tensor) -> candle_core::Result<Tensor> {
        let hidden_states = self.dense.forward(hidden_states)?;
        let hidden_states = self.dropout.forward(&hidden_states, false)?;
        let hidden_states = self.layer_norm.forward(&(hidden_states + input_tensor)?)?;
        Ok(hidden_states)
    }
}

#[cfg(feature = "local-gpu")]
use hf_hub::api::tokio::Api;
#[cfg(feature = "local-gpu")]
use tokenizers::Tokenizer;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::errors::Result;
use crate::errors::SnapragError;

/// Local GPU client for nomic-embed-text-v1.5 embeddings
#[cfg(feature = "local-gpu")]
pub struct LocalGPUClient {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    model_path: PathBuf,
    embedding_dim: usize, // Matryoshka dimension support
}

#[cfg(feature = "local-gpu")]
impl LocalGPUClient {
    /// Create a new local GPU client with default 768 dimensions
    pub async fn new(model_name: &str) -> Result<Self> {
        Self::new_with_dimension(model_name, 768).await
    }
    
    /// Create a new local GPU client with specified embedding dimension
    /// Supported dimensions: 768, 512, 256, 128, 64 (Matryoshka representation learning)
    pub async fn new_with_dimension(model_name: &str, embedding_dim: usize) -> Result<Self> {
        info!("Initializing local GPU client for model: {} with dimension: {}", model_name, embedding_dim);
        
        // Validate embedding dimension
        if !matches!(embedding_dim, 768 | 512 | 256 | 128 | 64) {
            return Err(SnapragError::EmbeddingError(
                format!("Unsupported embedding dimension: {}. Supported dimensions: 768, 512, 256, 128, 64", embedding_dim)
            ));
        }

        // Determine device (CUDA > Metal > CPU)
        let device = Self::get_best_device()?;
        info!("Using device: {:?}", device);

        // Download model if needed
        let model_path = Self::download_model(model_name).await?;

        // Load tokenizer
        let tokenizer = Self::load_tokenizer(&model_path)?;

        // Load model
        let model = Self::load_model(&model_path, &device)?;

        Ok(Self {
            model,
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
        
        // Try to initialize with different configurations
        let api = Api::new().map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to initialize HuggingFace API: {}", e))
        })?;
        
        info!("Creating model repository reference for: {}", model_name);
        let repo = api.model(model_name.to_string());

        // Download model files using direct URL to ensure complete download
        // Use a stable cache location to avoid temp-dir cleanup and cross-run mixing
        info!("Downloading model: {}", model_name);
        let cache_root = std::env::var("SNAPRAG_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".cache").join("snaprag")
            });
        // Store under hf/models/<repo_id>/main to separate different repos and revisions
        let model_dir = cache_root
            .join("hf")
            .join("models")
            .join(model_name.replace("/", "--"))
            .join("main");
        // If directory exists but belongs to a different model, purge to avoid mixing
        if model_dir.exists() {
            let meta_path = model_dir.join(".snaprag-model.json");
            if let Ok(meta_str) = std::fs::read_to_string(&meta_path) {
                if !meta_str.contains(model_name) {
                    warn!("Cache dir exists but for a different model, purging: {:?}", model_dir);
                    std::fs::remove_dir_all(&model_dir).map_err(|e| {
                        SnapragError::EmbeddingError(format!("Failed to purge stale model directory: {}", e))
                    })?;
                }
            }
        }

        std::fs::create_dir_all(&model_dir).map_err(|e| {
            SnapragError::EmbeddingError(format!("Failed to create model directory: {}", e))
        })?;
        
        let model_path = model_dir.join("model.safetensors");
        
        // Check if model.safetensors already exists
        if model_path.exists() {
            info!("Model.safetensors already exists, skipping download: {:?}", model_path);
        } else {
            // Try direct download first
            let url = format!("https://huggingface.co/{}/resolve/main/model.safetensors", model_name);
            info!("Downloading model.safetensors from: {}", url);
            
            match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    let content = response.bytes().await.map_err(|e| {
                        SnapragError::EmbeddingError(format!("Failed to read model response: {}", e))
                    })?;
                    
                    std::fs::write(&model_path, content).map_err(|e| {
                        SnapragError::EmbeddingError(format!("Failed to write model.safetensors: {}", e))
                    })?;
                    
                    info!("Successfully downloaded model.safetensors via direct URL");
                } else {
                    return Err(SnapragError::EmbeddingError(format!("Failed to download model.safetensors: {}", response.status())));
                }
            }
            Err(e) => {
                // Fallback to API download
                warn!("Direct download failed: {}. Trying API download.", e);
                let api_model_path = repo.get("model.safetensors").await.map_err(|e| {
                    SnapragError::EmbeddingError(format!("Failed to download model.safetensors via API: {}", e))
                })?;
                
                // Copy from API download to our directory
                std::fs::copy(&api_model_path, &model_path).map_err(|e| {
                    SnapragError::EmbeddingError(format!("Failed to copy model.safetensors: {}", e))
                })?;
            }
        }
        }
        
        // Try to download config.json
        let model_dir = model_path.parent().unwrap();
        let config_path = model_dir.join("config.json");
        
        // Check if config.json already exists
        if config_path.exists() {
            info!("Config.json already exists, skipping download: {:?}", config_path);
        } else {
            match repo.get("config.json").await {
            Ok(_config_path) => {
                info!("Config file downloaded successfully");
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
        
        // Try to download tokenizer files with better error handling
        let tokenizer_files = ["tokenizer.json", "tokenizer_config.json", "vocab.txt"];
        let mut tokenizer_downloaded = false;
        
        // Check if any tokenizer file already exists
        for tokenizer_file in &tokenizer_files {
            let tokenizer_path = model_dir.join(tokenizer_file);
            if tokenizer_path.exists() {
                info!("Tokenizer file {} already exists, skipping download: {:?}", tokenizer_file, tokenizer_path);
                tokenizer_downloaded = true;
                break;
            }
        }
        
        if !tokenizer_downloaded {
            for tokenizer_file in &tokenizer_files {
                info!("Attempting to download: {}", tokenizer_file);
                match repo.get(tokenizer_file).await {
                Ok(_tokenizer_path) => {
                    info!("Tokenizer file {} downloaded successfully", tokenizer_file);
                    tokenizer_downloaded = true;
                    break;
                }
                Err(e) => {
                    warn!("Failed to download {}: {}. Trying next file.", tokenizer_file, e);
                }
            }
        }
        
        if !tokenizer_downloaded {
            warn!("No tokenizer files could be downloaded. This may cause issues.");
            // Let's try a different approach - maybe the issue is with the API configuration
            info!("Attempting alternative download method...");
            
            // Try downloading with explicit URL construction
            let model_dir = model_path.parent().unwrap();
            for tokenizer_file in &tokenizer_files {
                let url = format!("https://huggingface.co/{}/resolve/main/{}", model_name, tokenizer_file);
                info!("Trying direct download from: {}", url);
                
                match reqwest::get(&url).await {
                    Ok(response) => {
                        if response.status().is_success() {
                            let content = response.bytes().await.map_err(|e| {
                                SnapragError::EmbeddingError(format!("Failed to read response: {}", e))
                            })?;
                            
                            let file_path = model_dir.join(tokenizer_file);
                            std::fs::write(&file_path, content).map_err(|e| {
                                SnapragError::EmbeddingError(format!("Failed to write {}: {}", tokenizer_file, e))
                            })?;
                            
                            info!("Successfully downloaded {} via direct URL", tokenizer_file);
                            tokenizer_downloaded = true;
                            break;
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
        
        // Write model metadata to prevent cross-model mixing
        let meta_path = model_dir.join(".snaprag-model.json");
        let meta = serde_json::json!({
            "model": model_name,
            "revision": "main"
        });
        let _ = std::fs::write(&meta_path, serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string()));

        info!("Model downloaded successfully");
        Ok(model_path.parent().unwrap().to_path_buf())
    }

    /// Load tokenizer from model path
    fn load_tokenizer(model_path: &PathBuf) -> Result<Tokenizer> {
        // Try different tokenizer file formats
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

    /// Load model from model path with complete NomicBERT architecture
    fn load_model(model_path: &PathBuf, device: &Device) -> Result<BertModel> {
        let config_path = model_path.join("config.json");
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| SnapragError::Io(e))?;

        // Parse the nomic-bert config
        let nomic_config: serde_json::Value = serde_json::from_str(&config_content)
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to parse config: {}", e)))?;

        let vocab_size = nomic_config["vocab_size"].as_u64().unwrap_or(30528) as usize;
        let hidden_size = nomic_config["n_embd"].as_u64().unwrap_or(768) as usize;
        let num_attention_heads = nomic_config["n_head"].as_u64().unwrap_or(12) as usize;
        let num_hidden_layers = nomic_config["n_layer"].as_u64().unwrap_or(12) as usize;
        let intermediate_size = nomic_config["n_inner"].as_u64().unwrap_or(3072) as usize;
        let max_position_embeddings = nomic_config["n_positions"].as_u64().unwrap_or(8192) as usize;
        let type_vocab_size = nomic_config["type_vocab_size"].as_u64().unwrap_or(2) as usize;
        let layer_norm_eps = nomic_config["layer_norm_epsilon"].as_f64().unwrap_or(1e-12);
        let hidden_dropout_prob = nomic_config["embd_pdrop"].as_f64().unwrap_or(0.0);
        let attention_probs_dropout_prob = nomic_config["attn_pdrop"].as_f64().unwrap_or(0.0);

        let config = BertConfig {
            vocab_size,
            hidden_size,
            num_attention_heads,
            num_hidden_layers,
            intermediate_size,
            max_position_embeddings,
            type_vocab_size,
            layer_norm_eps,
            hidden_dropout_prob,
            attention_probs_dropout_prob,
        };

        let model_file_path = model_path.join("model.safetensors");
        let weights = candle_core::safetensors::load(&model_file_path, device)?;
        
        if std::env::var("SNAPRAG_DEBUG_TENSORS").ok().as_deref() == Some("1") {
            let total = weights.len();
            info!("Loaded {} tensors. All tensor names:", total);
            for (k, _v) in &weights {
                info!("tensor: {}", k);
            }
        }
        
        // Create VarBuilder from weights
        let vb = VarBuilder::from_tensors(weights, candle_core::DType::F32, device);

        // Load embeddings
        let embeddings = Self::load_embeddings(&vb, &config)?;
        let token_type_embeddings = Self::load_token_type_embeddings(&vb, &config)?;
        
        // Load layer norm and dropout
        let layer_norm = Self::load_layer_norm(&vb, &config)?;
        let dropout = Dropout::new(config.hidden_dropout_prob as f32);
        
        // Load transformer layers
        let layers = Self::load_transformer_layers(&vb, &config)?;

        Ok(BertModel { 
            embeddings, 
            token_type_embeddings,
            layer_norm,
            dropout,
            layers,
            device: device.clone(),
            config,
        })
    }

    /// Load word embeddings
    fn load_embeddings(vb: &VarBuilder, config: &BertConfig) -> Result<Embedding> {
        let embedding_tensor = vb.get((config.vocab_size, config.hidden_size), "embeddings.word_embeddings.weight")
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get word embeddings: {}", e)))?;
        Ok(Embedding::new(embedding_tensor, config.hidden_size))
    }

    /// Load token type embeddings
    fn load_token_type_embeddings(vb: &VarBuilder, config: &BertConfig) -> Result<Embedding> {
        let token_type_tensor = vb.get((config.type_vocab_size, config.hidden_size), "embeddings.token_type_embeddings.weight")
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get token type embeddings: {}", e)))?;
        Ok(Embedding::new(token_type_tensor, config.hidden_size))
    }

    /// Load layer normalization
    fn load_layer_norm(vb: &VarBuilder, config: &BertConfig) -> Result<LayerNorm> {
        let weight = vb.get(config.hidden_size, "emb_ln.weight")
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get layer norm weight: {}", e)))?;
        let bias = vb.get(config.hidden_size, "emb_ln.bias")
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get layer norm bias: {}", e)))?;
        Ok(LayerNorm::new(weight, bias, config.layer_norm_eps))
    }

    /// Load all transformer layers
    fn load_transformer_layers(vb: &VarBuilder, config: &BertConfig) -> Result<Vec<BertLayer>> {
        let mut layers = Vec::new();
        
        for i in 0..config.num_hidden_layers {
            let layer = Self::load_transformer_layer(vb, config, i)?;
            layers.push(layer);
        }
        
        Ok(layers)
    }

    /// Load a single transformer layer
    fn load_transformer_layer(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertLayer> {
        let attention = Self::load_attention(vb, config, layer_idx)?;
        let intermediate = Self::load_intermediate(vb, config, layer_idx)?;
        let output = Self::load_output(vb, config, layer_idx)?;
        
        Ok(BertLayer {
            attention,
            intermediate,
            output,
        })
    }

    /// Load attention mechanism
    fn load_attention(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertAttention> {
        let self_attention = Self::load_self_attention(vb, config, layer_idx)?;
        let output = Self::load_self_output(vb, config, layer_idx)?;
        
        Ok(BertAttention {
            self_attention,
            output,
        })
    }

    /// Load self-attention mechanism
    fn load_self_attention(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertSelfAttention> {
        let attention_head_size = config.hidden_size / config.num_attention_heads;
        
        let wqkv = Self::load_linear(vb, config, &format!("encoder.layers.{}.attn.Wqkv", layer_idx))?;
        let out_proj = Self::load_linear(vb, config, &format!("encoder.layers.{}.attn.out_proj", layer_idx))?;
        let dropout = Dropout::new(config.attention_probs_dropout_prob as f32);
        
        Ok(BertSelfAttention {
            wqkv,
            out_proj,
            dropout,
            num_attention_heads: config.num_attention_heads,
            attention_head_size,
        })
    }

    /// Load self-attention output layer
    fn load_self_output(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertSelfOutput> {
        let dense = Self::load_linear(vb, config, &format!("encoder.layers.{}.attn.out_proj", layer_idx))?;
        let layer_norm = Self::load_layer_norm_layer(vb, config, &format!("encoder.layers.{}.norm1", layer_idx))?;
        let dropout = Dropout::new(config.hidden_dropout_prob as f32);
        
        Ok(BertSelfOutput {
            dense,
            layer_norm,
            dropout,
        })
    }

    /// Load intermediate MLP layer
    fn load_intermediate(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertIntermediate> {
        let fc11 = Self::load_linear(vb, config, &format!("encoder.layers.{}.mlp.fc11", layer_idx))?;
        let fc12 = Self::load_linear(vb, config, &format!("encoder.layers.{}.mlp.fc12", layer_idx))?;
        
        Ok(BertIntermediate {
            fc11,
            fc12,
        })
    }

    /// Load output layer
    fn load_output(vb: &VarBuilder, config: &BertConfig, layer_idx: usize) -> Result<BertOutput> {
        let dense = Self::load_linear(vb, config, &format!("encoder.layers.{}.mlp.fc2", layer_idx))?;
        let layer_norm = Self::load_layer_norm_layer(vb, config, &format!("encoder.layers.{}.norm2", layer_idx))?;
        let dropout = Dropout::new(config.hidden_dropout_prob as f32);
        
        Ok(BertOutput {
            dense,
            layer_norm,
            dropout,
        })
    }

    /// Load linear layer
    fn load_linear(vb: &VarBuilder, config: &BertConfig, name: &str) -> Result<Linear> {
        // Try different shapes based on layer type
        let weight = vb.get((config.hidden_size, config.hidden_size), &format!("{}.weight", name))
            .or_else(|_| vb.get((config.intermediate_size, config.hidden_size), &format!("{}.weight", name)))
            .or_else(|_| vb.get((config.hidden_size, config.intermediate_size), &format!("{}.weight", name)))
            .or_else(|_| vb.get((3 * config.hidden_size, config.hidden_size), &format!("{}.weight", name)))
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get {} weight: {}", name, e)))?;
        
        // Try to get bias, but it might not exist for some layers
        let bias = vb.get(config.hidden_size, &format!("{}.bias", name))
            .or_else(|_| vb.get(config.intermediate_size, &format!("{}.bias", name)))
            .or_else(|_| vb.get(3 * config.hidden_size, &format!("{}.bias", name)))
            .ok(); // Make bias optional
        
        Ok(Linear::new(weight, bias))
    }

    /// Load layer normalization for a specific layer
    fn load_layer_norm_layer(vb: &VarBuilder, config: &BertConfig, name: &str) -> Result<LayerNorm> {
        let weight = vb.get(config.hidden_size, &format!("{}.weight", name))
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get {} weight: {}", name, e)))?;
        let bias = vb.get(config.hidden_size, &format!("{}.bias", name))
            .map_err(|e| SnapragError::EmbeddingError(format!("Failed to get {} bias: {}", name, e)))?;
        Ok(LayerNorm::new(weight, bias, config.layer_norm_eps))
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
            .encode(prefixed_text.as_str(), true)
            .map_err(|e| SnapragError::EmbeddingError(format!("Tokenization failed: {}", e)))?;

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)?;
        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)?;

        // Add batch dimension
        let input_ids = input_ids.unsqueeze(0)?;
        let attention_mask = attention_mask.unsqueeze(0)?;

        // Generate embeddings
        // ModernBERT requires attention mask
        let embeddings = self.model.forward_with_mask(&input_ids, &attention_mask)?;

        // Mean pooling
        let pooled = self.mean_pooling(&embeddings, &attention_mask)?;

        // Normalize
        let normalized = self.normalize(&pooled)?;

        // Convert to Vec<f32>
        let mut embedding_vec: Vec<f32> = normalized.to_vec1()?;
        
        // Apply Matryoshka dimension adjustment for nomic-embed-text-v1.5
        if self.embedding_dim < 768 {
            embedding_vec.truncate(self.embedding_dim);
            debug!("Truncated embedding to {} dimensions", self.embedding_dim);
        }

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
                self.tokenizer.encode(text.as_str(), true).map_err(|e| {
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
        let embeddings = self.model.forward_with_mask(&input_ids, &attention_mask)?;

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
        let mask_expanded = attention_mask.unsqueeze(attention_mask.dims().len())?.expand(embeddings.shape())?;
        let sum_embeddings = (embeddings * &mask_expanded)?.sum(1)?;
        let sum_mask = mask_expanded.sum(1)?.clamp(1e-9, f32::MAX)?;
        Ok(sum_embeddings.broadcast_div(&sum_mask)?)
    }

    /// Normalize embeddings to unit length
    fn normalize(&self, embeddings: &Tensor) -> Result<Tensor> {
        let norm = embeddings.sqr()?.sum(1)?.sqrt()?;
        Ok(embeddings.broadcast_div(&norm)?)
    }
}

#[cfg(feature = "local-gpu")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires GPU and model download
    async fn test_matryoshka_dimensions() {
        // Test different embedding dimensions
        for dim in [768, 512, 256, 128, 64] {
            let client = LocalGPUClient::new_with_dimension("nomic-ai/nomic-embed-text-v1.5", dim).await.unwrap();
            let embedding = client.generate("search_query: What is machine learning?").await.unwrap();
            assert_eq!(embedding.len(), dim, "Expected dimension {} but got {}", dim, embedding.len());
        }
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