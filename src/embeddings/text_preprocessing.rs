//! Text preprocessing utilities for embedding generation
//!
//! Provides utilities for cleaning, normalizing, and chunking text before embedding generation.

use tracing::debug;
use tracing::warn;

use crate::errors::SnapRagError;

/// Preprocess text for embedding generation with intelligent chunking
///
/// This function handles:
/// - Normalizing whitespace and newlines
/// - Removing or replacing invalid characters
/// - Intelligent chunking for long texts
/// - Basic sanitization
pub fn preprocess_text_for_embedding(text: &str) -> Result<String, SnapRagError> {
    if text.is_empty() {
        return Err(SnapRagError::EmbeddingError(
            "Empty text provided".to_string(),
        ));
    }

    // Step 1: Normalize whitespace and newlines
    let normalized = normalize_whitespace(text);

    // Step 2: Remove invalid characters
    let sanitized = sanitize_text(&normalized);

    // Step 3: Handle long text intelligently
    if sanitized.len() > 1500 {
        warn!(
            "Text too long ({} chars), applying intelligent chunking",
            sanitized.len()
        );
        return Ok(intelligent_text_chunking(&sanitized, 1500));
    }

    // Step 4: Final validation
    if sanitized.trim().is_empty() {
        return Err(SnapRagError::EmbeddingError(
            "Text contains only whitespace after preprocessing".to_string(),
        ));
    }

    debug!(
        "Preprocessed text: {} -> {} chars",
        text.len(),
        sanitized.len()
    );
    Ok(sanitized)
}

/// Normalize whitespace and newlines
fn normalize_whitespace(text: &str) -> String {
    text
        // Replace various newline types with spaces
        .replace("\r\n", " ") // Windows CRLF
        .replace('\n', " ") // Unix LF
        .replace('\r', " ") // Mac CR
        // Replace tabs with spaces
        .replace('\t', " ")
        // Replace multiple spaces with single space
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Sanitize text by removing or replacing invalid characters
fn sanitize_text(text: &str) -> String {
    text.chars()
        .map(|c| {
            match c {
                // Keep printable ASCII characters
                c if c.is_ascii_graphic() => c,
                // Keep spaces
                ' ' => ' ',
                // Replace control characters with space
                c if c.is_control() => ' ',
                // Keep Unicode letters, numbers, and common punctuation
                c if c.is_alphanumeric() => c,
                c if ".,!?;:()[]{}'\"-".contains(c) => c,
                // Keep emojis and other Unicode symbols (but not control chars)
                c if !c.is_control() => c,
                // Replace control characters with space
                _ => ' ',
            }
        })
        .collect::<String>()
        // Clean up multiple spaces but preserve single spaces
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Intelligent text chunking for long texts
/// Uses multiple strategies to preserve semantic meaning
fn intelligent_text_chunking(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Strategy 1: Try to find natural paragraph breaks
    if let Some(chunk) = chunk_by_paragraphs(text, max_length) {
        return chunk;
    }

    // Strategy 2: Try to find sentence boundaries
    if let Some(chunk) = chunk_by_sentences(text, max_length) {
        return chunk;
    }

    // Strategy 3: Try to find important keywords/phrases
    if let Some(chunk) = chunk_by_importance(text, max_length) {
        return chunk;
    }

    // Strategy 4: Fallback to smart word boundary truncation
    smart_truncate_text(text, max_length)
}

/// Chunk by paragraphs (double newlines or significant breaks)
fn chunk_by_paragraphs(text: &str, max_length: usize) -> Option<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    if paragraphs.len() <= 1 {
        return None;
    }

    let mut result = String::new();
    for paragraph in paragraphs {
        if result.len() + paragraph.len() + 2 <= max_length {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(paragraph);
        } else {
            break;
        }
    }

    if result.len() > max_length * 2 / 3 {
        // Only use if we got a substantial chunk
        Some(result)
    } else {
        None
    }
}

/// Chunk by sentences (period, exclamation, question marks)
fn chunk_by_sentences(text: &str, max_length: usize) -> Option<String> {
    let sentences: Vec<&str> = text
        .split(|c| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if sentences.len() <= 1 {
        return None;
    }

    let mut result = String::new();
    for sentence in sentences {
        let trimmed = sentence.trim();
        if result.len() + trimmed.len() + 1 <= max_length {
            if !result.is_empty() {
                result.push_str(". ");
            }
            result.push_str(trimmed);
        } else {
            break;
        }
    }

    if result.len() > max_length * 2 / 3 {
        Some(result)
    } else {
        None
    }
}

/// Chunk by importance (prioritize sentences with key terms)
fn chunk_by_importance(text: &str, max_length: usize) -> Option<String> {
    // Key terms that indicate important content
    let important_terms = [
        "TL;DR",
        "summary",
        "conclusion",
        "key",
        "important",
        "main",
        "primary",
        "first",
        "second",
        "third",
        "finally",
        "overall",
        "in summary",
        "to summarize",
        "the main",
        "the key",
        "the primary",
        "the most",
        "the best",
        "the worst",
        "however",
        "but",
        "although",
        "despite",
        "nevertheless",
        "furthermore",
        "additionally",
        "moreover",
        "therefore",
        "thus",
        "consequently",
        "as a result",
    ];

    let sentences: Vec<&str> = text
        .split(|c| c == '.' || c == '!' || c == '?')
        .filter(|s| !s.trim().is_empty())
        .collect();

    if sentences.len() <= 1 {
        return None;
    }

    // Score sentences by importance
    let mut scored_sentences: Vec<(usize, &str)> = sentences
        .iter()
        .enumerate()
        .map(|(i, sentence)| {
            let score = important_terms
                .iter()
                .map(|term| sentence.to_lowercase().matches(term).count())
                .sum::<usize>();
            (score, *sentence)
        })
        .collect();

    // Sort by score (descending) and then by position (ascending)
    scored_sentences.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.len().cmp(&b.1.len())));

    let mut result = String::new();
    for (_, sentence) in scored_sentences {
        let trimmed = sentence.trim();
        if result.len() + trimmed.len() + 1 <= max_length {
            if !result.is_empty() {
                result.push_str(". ");
            }
            result.push_str(trimmed);
        } else {
            break;
        }
    }

    if result.len() > max_length * 2 / 3 {
        Some(result)
    } else {
        None
    }
}

/// Smart truncation with word boundary preservation
fn smart_truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Try to truncate at word boundary
    let truncated = &text[..max_length];
    if let Some(last_space) = truncated.rfind(' ') {
        if last_space > max_length * 3 / 4 {
            // Only use word boundary if it's not too far back
            return truncated[..last_space].to_string();
        }
    }

    // Fallback to character truncation
    truncated.to_string()
}

/// Legacy truncate function (kept for backward compatibility)
fn truncate_text(text: &str, max_length: usize) -> String {
    smart_truncate_text(text, max_length)
}

/// Validate text for embedding generation
pub fn validate_text_for_embedding(text: &str) -> Result<(), SnapRagError> {
    if text.is_empty() {
        return Err(SnapRagError::EmbeddingError(
            "Empty text provided".to_string(),
        ));
    }

    if text.trim().is_empty() {
        return Err(SnapRagError::EmbeddingError(
            "Text contains only whitespace".to_string(),
        ));
    }

    if text.len() > 1500 {
        return Err(SnapRagError::EmbeddingError(
            "Text too long (max 1500 chars for BGE model)".to_string(),
        ));
    }

    // Check for excessive control characters
    let control_char_count = text.chars().filter(|c| c.is_control()).count();
    if control_char_count > text.len() / 2 {
        return Err(SnapRagError::EmbeddingError(
            "Text contains too many control characters".to_string(),
        ));
    }

    Ok(())
}

/// Generate multiple chunks for long texts (for advanced use cases)
/// Returns a vector of text chunks that can be processed separately
pub fn generate_text_chunks(
    text: &str,
    max_chunk_size: usize,
) -> Result<Vec<String>, SnapRagError> {
    if text.is_empty() {
        return Err(SnapRagError::EmbeddingError(
            "Empty text provided".to_string(),
        ));
    }

    let normalized = normalize_whitespace(text);
    let sanitized = sanitize_text(&normalized);

    if sanitized.len() <= max_chunk_size {
        return Ok(vec![sanitized]);
    }

    let mut chunks = Vec::new();
    let mut remaining = sanitized.as_str();

    while !remaining.is_empty() {
        let chunk = if remaining.len() <= max_chunk_size {
            remaining.to_string()
        } else {
            // Try different chunking strategies
            if let Some(chunk) = chunk_by_paragraphs(remaining, max_chunk_size) {
                chunk
            } else if let Some(chunk) = chunk_by_sentences(remaining, max_chunk_size) {
                chunk
            } else {
                smart_truncate_text(remaining, max_chunk_size)
            }
        };

        let chunk_len = chunk.len();
        chunks.push(chunk);

        // Move to next part
        remaining = &remaining[chunk_len..];
        remaining = remaining.trim_start();
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("hello\nworld"), "hello world");
        assert_eq!(normalize_whitespace("hello\r\nworld"), "hello world");
        assert_eq!(normalize_whitespace("hello\tworld"), "hello world");
        assert_eq!(normalize_whitespace("hello   world"), "hello world");
        assert_eq!(normalize_whitespace("hello\n\n\nworld"), "hello world");
    }

    #[test]
    fn test_sanitize_text() {
        assert_eq!(sanitize_text("hello world!"), "hello world!");
        assert_eq!(sanitize_text("hello\x00world"), "hello world");
        assert_eq!(sanitize_text("hello\x01world"), "hello world");
        assert_eq!(sanitize_text("hello world 123"), "hello world 123");
    }

    #[test]
    fn test_truncate_text() {
        let result1 = truncate_text("hello world", 5);
        assert!(result1.len() <= 5);
        assert!(result1.starts_with("hello") || result1 == "hello");

        assert_eq!(truncate_text("hello world", 20), "hello world");

        let result2 = truncate_text("hello world test", 10);
        assert!(result2.len() <= 10);
    }

    #[test]
    fn test_preprocess_text_for_embedding() {
        // Valid text
        let result = preprocess_text_for_embedding("Hello world!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");

        // Text with newlines
        let result = preprocess_text_for_embedding("Hello\nworld!");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world!");

        // Empty text
        let result = preprocess_text_for_embedding("");
        assert!(result.is_err());

        // Whitespace only
        let result = preprocess_text_for_embedding("   \n\t   ");
        assert!(result.is_err());

        // Text with control characters
        let result = preprocess_text_for_embedding("Hello\x00world");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[test]
    fn test_validate_text_for_embedding() {
        assert!(validate_text_for_embedding("Hello world").is_ok());
        assert!(validate_text_for_embedding("").is_err());
        assert!(validate_text_for_embedding("   ").is_err());
        assert!(validate_text_for_embedding(&"a".repeat(10001)).is_err());
    }
}
