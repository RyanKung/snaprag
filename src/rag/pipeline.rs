//! Complete RAG pipeline: Retrieve -> Rank -> Generate

use std::collections::HashMap;
use std::sync::Arc;

use tracing::debug;
use tracing::info;

use crate::config::AppConfig;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::errors::Result;
use crate::llm::ChatMessage;
use crate::llm::LlmService;
use crate::rag::ContextAssembler;
use crate::rag::Retriever;
use crate::rag::SearchResult;

/// Complete RAG service
pub struct RagService {
    retriever: Retriever,
    context_assembler: ContextAssembler,
    llm_service: LlmService,
}

impl RagService {
    /// Create a new RAG service
    ///
    /// # Errors
    /// - Database connection errors
    /// - Embedding service configuration errors (invalid API keys, endpoints)
    /// - LLM service configuration errors (missing or invalid LLM config)
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let database = Arc::new(Database::from_config(config).await?);
        let embedding_service = Arc::new(EmbeddingService::new(config)?);
        let retriever = Retriever::new(database, embedding_service);
        let context_assembler = ContextAssembler::default();
        let llm_service = LlmService::new(config)?;

        Ok(Self {
            retriever,
            context_assembler,
            llm_service,
        })
    }

    /// Create from existing services
    #[must_use]
    pub fn from_services(
        database: Arc<Database>,
        embedding_service: Arc<EmbeddingService>,
        llm_service: LlmService,
    ) -> Self {
        let retriever = Retriever::new(database, embedding_service);
        let context_assembler = ContextAssembler::default();

        Self {
            retriever,
            context_assembler,
            llm_service,
        }
    }

    /// Perform a complete RAG query
    ///
    /// # Errors
    /// - Document retrieval errors (embedding generation, database queries)
    /// - Context assembly errors (text processing, formatting failures)
    /// - LLM generation errors (API failures, rate limits, invalid responses)
    /// - Network errors (timeouts, connection failures)
    pub async fn query(&self, query: &str) -> Result<RagResponse> {
        self.query_with_options(RagQuery {
            question: query.to_string(),
            retrieval_limit: 10,
            retrieval_method: RetrievalMethod::Auto,
            temperature: 0.7,
            max_tokens: 2000,
        })
        .await
    }

    /// Perform RAG query with custom options
    ///
    /// # Errors
    /// - Document retrieval errors (embedding generation, database queries, invalid retrieval method)
    /// - Context assembly errors (text processing, token limit exceeded)
    /// - LLM generation errors (API failures, rate limits, invalid temperature/max_tokens)
    /// - Network errors (timeouts, connection failures)
    /// - Invalid query parameters (negative limits, invalid temperature range)
    pub async fn query_with_options(&self, query: RagQuery) -> Result<RagResponse> {
        info!("Processing RAG query: {}", query.question);

        // Step 1: Retrieve relevant documents
        debug!("Step 1: Retrieving documents");
        let results = match query.retrieval_method {
            RetrievalMethod::Semantic => {
                self.retriever
                    .semantic_search(&query.question, query.retrieval_limit, None)
                    .await?
            }
            RetrievalMethod::Keyword => {
                self.retriever
                    .keyword_search(&query.question, query.retrieval_limit)
                    .await?
            }
            RetrievalMethod::Hybrid => {
                self.retriever
                    .hybrid_search(&query.question, query.retrieval_limit)
                    .await?
            }
            RetrievalMethod::Auto => {
                self.retriever
                    .auto_search(&query.question, query.retrieval_limit)
                    .await?
            }
        };

        debug!("Retrieved {} results", results.len());

        // Step 2: Assemble context
        debug!("Step 2: Assembling context");
        let (context, metadata) = self.context_assembler.assemble_with_metadata(&results);

        // Step 3: Generate answer using LLM
        debug!("Step 3: Generating answer");
        let prompt = self.build_prompt(&query.question, &context);
        let answer = self
            .llm_service
            .generate_with_params(&prompt, query.temperature, query.max_tokens)
            .await?;

        info!("RAG query completed successfully");

        Ok(RagResponse {
            answer,
            sources: results,
            context,
            metadata,
            query: query.question,
        })
    }

    /// Search for profiles without LLM generation
    pub async fn search_profiles(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.retriever.auto_search(query, limit).await
    }

    /// Build prompt for LLM
    fn build_prompt(&self, question: &str, context: &str) -> String {
        format!(
            r"You are an expert assistant helping users discover and learn about Farcaster protocol users.

Context: The following are Farcaster user profiles that may be relevant to the question:

{context}

Question: {question}

Instructions:
1. Provide a helpful and accurate answer based on the profiles above
2. If referencing specific users, mention their username
3. If the profiles don't contain relevant information, say so
4. Be concise but informative

Answer:"
        )
    }

    /// Get retriever reference
    #[must_use]
    pub const fn retriever(&self) -> &Retriever {
        &self.retriever
    }

    /// Get context assembler reference
    #[must_use]
    pub const fn context_assembler(&self) -> &ContextAssembler {
        &self.context_assembler
    }
}

/// RAG query configuration
#[derive(Debug, Clone)]
pub struct RagQuery {
    pub question: String,
    pub retrieval_limit: usize,
    pub retrieval_method: RetrievalMethod,
    pub temperature: f32,
    pub max_tokens: usize,
}

/// Retrieval method for RAG
#[derive(Debug, Clone, Copy)]
pub enum RetrievalMethod {
    /// Semantic search using embeddings
    Semantic,
    /// Keyword search using text matching
    Keyword,
    /// Hybrid search combining both
    Hybrid,
    /// Automatic selection
    Auto,
}

/// RAG response
#[derive(Debug, Clone)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<SearchResult>,
    pub context: String,
    pub metadata: Vec<HashMap<String, String>>,
    pub query: String,
}

impl RagResponse {
    /// Get a formatted string representation
    #[must_use]
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Query: {}\n\n", self.query));
        output.push_str(&format!("Answer:\n{}\n\n", self.answer));
        output.push_str(&format!("Sources ({} profiles):\n", self.sources.len()));

        for (idx, source) in self.sources.iter().enumerate().take(5) {
            let username = source.profile.username.as_deref().unwrap_or("unknown");
            output.push_str(&format!(
                "  {}. @{} (FID: {}, Score: {:.2})\n",
                idx + 1,
                username,
                source.profile.fid,
                source.score
            ));
        }

        output
    }
}
