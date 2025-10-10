//! Streaming response handling

use std::pin::Pin;

use futures::Stream;

use crate::errors::Result;

/// Streaming response from LLM
pub struct StreamingResponse {
    stream: Pin<Box<dyn Stream<Item = Result<String>> + Send>>,
}

impl StreamingResponse {
    pub fn new(stream: Pin<Box<dyn Stream<Item = Result<String>> + Send>>) -> Self {
        Self { stream }
    }

    /// Collect all chunks into a single string
    pub async fn collect_all(mut self) -> Result<String> {
        use futures::StreamExt;
        let mut result = String::new();
        while let Some(chunk) = self.stream.next().await {
            result.push_str(&chunk?);
        }
        Ok(result)
    }

    /// Get the underlying stream
    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = Result<String>> + Send>> {
        self.stream
    }
}
