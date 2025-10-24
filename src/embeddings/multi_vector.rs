//! Multi-vector embedding service for handling chunked text embeddings
//! 
//! This module provides support for:
//! - Chunking long texts into multiple pieces
//! - Storing multiple embeddings per cast
//! - Aggregating multiple embeddings into single vectors
//! - Searching across chunked embeddings

use crate::errors::{Result, SnapRagError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strategy for chunking text into multiple pieces
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkStrategy {
    /// Single chunk (original behavior)
    Single,
    /// Split by paragraphs
    Paragraph,
    /// Split by sentences
    Sentence,
    /// Split by importance scoring
    Importance,
    /// Sliding window approach
    SlidingWindow,
}

/// Strategy for aggregating multiple embeddings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationStrategy {
    /// Use only the first chunk
    FirstChunk,
    /// Average all embeddings
    Mean,
    /// Weighted average based on chunk importance
    WeightedMean,
    /// Maximum values across all embeddings
    Max,
    /// Concatenate embeddings (requires dimension adjustment)
    Concatenate,
}

/// Metadata for a text chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_index: usize,
    pub chunk_text: String,
    pub chunk_length: usize,
    pub chunk_strategy: ChunkStrategy,
    pub importance_score: Option<f32>,
}

/// Result of chunked embedding generation
#[derive(Debug, Clone)]
pub struct ChunkedEmbeddingResult {
    pub message_hash: Vec<u8>,
    pub fid: i64,
    pub original_text: String,
    pub chunks: Vec<(ChunkMetadata, Vec<f32>)>,
    pub aggregated_embedding: Option<Vec<f32>>,
    pub aggregation_strategy: AggregationStrategy,
}

/// Multi-vector embedding service
pub struct MultiVectorEmbeddingService {
    embedding_service: crate::embeddings::EmbeddingService,
    default_chunk_size: usize,
    default_strategy: ChunkStrategy,
    default_aggregation: AggregationStrategy,
}

impl MultiVectorEmbeddingService {
    pub fn new(
        embedding_service: crate::embeddings::EmbeddingService,
        default_chunk_size: usize,
        default_strategy: ChunkStrategy,
        default_aggregation: AggregationStrategy,
    ) -> Self {
        Self {
            embedding_service,
            default_chunk_size,
            default_strategy,
            default_aggregation,
        }
    }

    /// Generate embeddings for a cast with multi-vector support
    pub async fn generate_cast_embeddings(
        &self,
        message_hash: Vec<u8>,
        fid: i64,
        text: &str,
        strategy: Option<ChunkStrategy>,
        aggregation: Option<AggregationStrategy>,
    ) -> Result<ChunkedEmbeddingResult> {
        let strategy = strategy.unwrap_or_else(|| self.default_strategy.clone());
        let aggregation = aggregation.unwrap_or_else(|| self.default_aggregation.clone());

        // Generate chunks based on strategy
        let chunks = self.generate_chunks(text, &strategy)?;
        
        if chunks.is_empty() {
            return Err(SnapRagError::EmbeddingError("No valid chunks generated".to_string()));
        }

        tracing::debug!("Generated {} chunks for text of {} chars", chunks.len(), text.len());
        for (i, (metadata, chunk_text)) in chunks.iter().enumerate() {
            tracing::debug!("Chunk {}: {} chars, preview: {:?}", i, chunk_text.len(), &chunk_text[..chunk_text.len().min(100)]);
        }

        // Generate embeddings for each chunk
        let mut chunk_embeddings = Vec::new();
        for (metadata, chunk_text) in chunks {
            tracing::debug!("Generating embedding for chunk {} ({} chars)", metadata.chunk_index, chunk_text.len());
            let embedding = self.embedding_service.generate(&chunk_text).await?;
            chunk_embeddings.push((metadata, embedding));
        }

        // Generate aggregated embedding if requested
        let aggregated_embedding = if chunk_embeddings.len() > 1 {
            Some(self.aggregate_embeddings(&chunk_embeddings, &aggregation)?)
        } else {
            None
        };

        Ok(ChunkedEmbeddingResult {
            message_hash,
            fid,
            original_text: text.to_string(),
            chunks: chunk_embeddings,
            aggregated_embedding,
            aggregation_strategy: aggregation,
        })
    }

    /// Generate text chunks based on strategy
    fn generate_chunks(
        &self,
        text: &str,
        strategy: &ChunkStrategy,
    ) -> Result<Vec<(ChunkMetadata, String)>> {
        match strategy {
            ChunkStrategy::Single => {
                let processed = crate::embeddings::preprocess_text_for_embedding(text)?;
                Ok(vec![(
                    ChunkMetadata {
                        chunk_index: 0,
                        chunk_text: processed.clone(),
                        chunk_length: processed.len(),
                        chunk_strategy: ChunkStrategy::Single,
                        importance_score: None,
                    },
                    processed,
                )])
            }
            ChunkStrategy::Paragraph => self.chunk_by_paragraphs(text),
            ChunkStrategy::Sentence => self.chunk_by_sentences(text),
            ChunkStrategy::Importance => self.chunk_by_importance(text),
            ChunkStrategy::SlidingWindow => self.chunk_by_sliding_window(text),
        }
    }

    /// Chunk text by paragraphs
    fn chunk_by_paragraphs(&self, text: &str) -> Result<Vec<(ChunkMetadata, String)>> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();

        for (i, paragraph) in paragraphs.iter().enumerate() {
            let processed = crate::embeddings::preprocess_text_for_embedding(paragraph)?;
            if !processed.trim().is_empty() {
                chunks.push((
                    ChunkMetadata {
                        chunk_index: i,
                        chunk_text: processed.clone(),
                        chunk_length: processed.len(),
                        chunk_strategy: ChunkStrategy::Paragraph,
                        importance_score: None,
                    },
                    processed,
                ));
            }
        }

        Ok(chunks)
    }

    /// Chunk text by sentences
    fn chunk_by_sentences(&self, text: &str) -> Result<Vec<(ChunkMetadata, String)>> {
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_index = 0;

        for sentence in sentences {
            let trimmed = sentence.trim();
            if current_chunk.len() + trimmed.len() + 1 <= self.default_chunk_size {
                if !current_chunk.is_empty() {
                    current_chunk.push_str(". ");
                }
                current_chunk.push_str(trimmed);
            } else {
                if !current_chunk.is_empty() {
                    let processed = crate::embeddings::preprocess_text_for_embedding(&current_chunk)?;
                    chunks.push((
                        ChunkMetadata {
                            chunk_index,
                            chunk_text: processed.clone(),
                            chunk_length: processed.len(),
                            chunk_strategy: ChunkStrategy::Sentence,
                            importance_score: None,
                        },
                        processed,
                    ));
                    chunk_index += 1;
                }
                current_chunk = trimmed.to_string();
            }
        }

        // Add the last chunk
        if !current_chunk.is_empty() {
            let processed = crate::embeddings::preprocess_text_for_embedding(&current_chunk)?;
            chunks.push((
                ChunkMetadata {
                    chunk_index,
                    chunk_text: processed.clone(),
                    chunk_length: processed.len(),
                    chunk_strategy: ChunkStrategy::Sentence,
                    importance_score: None,
                },
                processed,
            ));
        }

        Ok(chunks)
    }

    /// Chunk text by importance scoring
    fn chunk_by_importance(&self, text: &str) -> Result<Vec<(ChunkMetadata, String)>> {
        // Use the existing importance-based chunking from text_preprocessing
        let chunks = crate::embeddings::generate_text_chunks(text, self.default_chunk_size)?;
        
        let mut result = Vec::new();
        for (i, chunk_text) in chunks.into_iter().enumerate() {
            let importance_score = self.calculate_importance_score(&chunk_text);
            result.push((
                ChunkMetadata {
                    chunk_index: i,
                    chunk_text: chunk_text.clone(),
                    chunk_length: chunk_text.len(),
                    chunk_strategy: ChunkStrategy::Importance,
                    importance_score: Some(importance_score),
                },
                chunk_text,
            ));
        }

        Ok(result)
    }

    /// Chunk text using sliding window approach
    fn chunk_by_sliding_window(&self, text: &str) -> Result<Vec<(ChunkMetadata, String)>> {
        let processed = crate::embeddings::preprocess_text_for_embedding(text)?;
        let mut chunks = Vec::new();
        
        let window_size = self.default_chunk_size;
        let overlap = window_size / 4; // 25% overlap
        
        let mut start = 0;
        let mut chunk_index = 0;
        
        while start < processed.len() {
            let end = (start + window_size).min(processed.len());
            let chunk_text = processed[start..end].to_string();
            
            chunks.push((
                ChunkMetadata {
                    chunk_index,
                    chunk_text: chunk_text.clone(),
                    chunk_length: chunk_text.len(),
                    chunk_strategy: ChunkStrategy::SlidingWindow,
                    importance_score: None,
                },
                chunk_text,
            ));
            
            start += window_size - overlap;
            chunk_index += 1;
        }

        Ok(chunks)
    }

    /// Calculate importance score for a text chunk
    fn calculate_importance_score(&self, text: &str) -> f32 {
        let important_terms = [
            "TL;DR", "summary", "conclusion", "key", "important", "main", "primary",
            "first", "second", "third", "finally", "overall", "in summary", "to summarize",
            "the main", "the key", "the primary", "the most", "the best", "the worst",
            "however", "but", "although", "despite", "nevertheless", "furthermore",
            "additionally", "moreover", "therefore", "thus", "consequently", "as a result"
        ];

        let score = important_terms.iter()
            .map(|term| text.to_lowercase().matches(term).count())
            .sum::<usize>() as f32;

        // Normalize score (0.0 to 1.0)
        (score / 10.0).min(1.0)
    }

    /// Aggregate multiple embeddings into a single vector
    fn aggregate_embeddings(
        &self,
        chunk_embeddings: &[(ChunkMetadata, Vec<f32>)],
        strategy: &AggregationStrategy,
    ) -> Result<Vec<f32>> {
        if chunk_embeddings.is_empty() {
            return Err(SnapRagError::EmbeddingError("No embeddings to aggregate".to_string()));
        }

        if chunk_embeddings.len() == 1 {
            return Ok(chunk_embeddings[0].1.clone());
        }

        match strategy {
            AggregationStrategy::FirstChunk => Ok(chunk_embeddings[0].1.clone()),
            
            AggregationStrategy::Mean => {
                let dimension = chunk_embeddings[0].1.len();
                let mut aggregated = vec![0.0; dimension];
                
                for (_, embedding) in chunk_embeddings {
                    for (i, &value) in embedding.iter().enumerate() {
                        aggregated[i] += value;
                    }
                }
                
                let count = chunk_embeddings.len() as f32;
                for value in &mut aggregated {
                    *value /= count;
                }
                
                Ok(aggregated)
            }
            
            AggregationStrategy::WeightedMean => {
                let dimension = chunk_embeddings[0].1.len();
                let mut aggregated = vec![0.0; dimension];
                let mut total_weight = 0.0;
                
                for (metadata, embedding) in chunk_embeddings {
                    let weight = metadata.importance_score.unwrap_or(1.0);
                    total_weight += weight;
                    
                    for (i, &value) in embedding.iter().enumerate() {
                        aggregated[i] += value * weight;
                    }
                }
                
                if total_weight > 0.0 {
                    for value in &mut aggregated {
                        *value /= total_weight;
                    }
                }
                
                Ok(aggregated)
            }
            
            AggregationStrategy::Max => {
                let dimension = chunk_embeddings[0].1.len();
                let mut aggregated = vec![f32::NEG_INFINITY; dimension];
                
                for (_, embedding) in chunk_embeddings {
                    for (i, &value) in embedding.iter().enumerate() {
                        aggregated[i] = aggregated[i].max(value);
                    }
                }
                
                Ok(aggregated)
            }
            
            AggregationStrategy::Concatenate => {
                let mut concatenated = Vec::new();
                for (_, embedding) in chunk_embeddings {
                    concatenated.extend(embedding);
                }
                Ok(concatenated)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunk_by_paragraphs() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let service = MultiVectorEmbeddingService::new(
            crate::embeddings::EmbeddingService::new(
                crate::embeddings::EmbeddingClient::Ollama(
                    crate::embeddings::OllamaEmbeddingClient::new("test".to_string(), "http://localhost".to_string())
                ),
                crate::embeddings::EmbeddingConfig::default(),
            ),
            1000,
            ChunkStrategy::Paragraph,
            AggregationStrategy::Mean,
        );

        let chunks = service.chunk_by_paragraphs(text).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].0.chunk_index, 0);
        assert_eq!(chunks[1].0.chunk_index, 1);
        assert_eq!(chunks[2].0.chunk_index, 2);
    }

    #[test]
    fn test_importance_scoring() {
        let service = MultiVectorEmbeddingService::new(
            crate::embeddings::EmbeddingService::new(
                crate::embeddings::EmbeddingClient::Ollama(
                    crate::embeddings::OllamaEmbeddingClient::new("test".to_string(), "http://localhost".to_string())
                ),
                crate::embeddings::EmbeddingConfig::default(),
            ),
            1000,
            ChunkStrategy::Paragraph,
            AggregationStrategy::Mean,
        );

        let high_importance = "TL;DR This is the main point and key finding.";
        let low_importance = "Just some random text without important keywords.";
        
        assert!(service.calculate_importance_score(high_importance) > 
                service.calculate_importance_score(low_importance));
    }
}
