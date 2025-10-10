# SnapRAG Usage Guide

## 🎯 Quick Start with RAG

SnapRAG now includes complete RAG (Retrieval-Augmented Generation) functionality for semantic search and intelligent querying of Farcaster data.

## 📋 Available Commands

### 1. RAG Query Commands

#### Execute a RAG Query
```bash
# Basic RAG query (uses LLM to generate answers)
cargo run -- rag query "Find developers interested in AI"

# With options
cargo run -- rag query "Who are blockchain developers in SF?" \
  --limit 10 \
  --method hybrid \
  --temperature 0.7 \
  --max-tokens 2000 \
  --verbose

# Retrieval methods:
# - semantic: Vector similarity search
# - keyword: Text-based search
# - hybrid: Combined approach
# - auto: Automatically select best method (default)
```

#### Search Profiles (Without LLM)
```bash
# Quick profile search (no LLM generation)
cargo run -- rag search "rust developers"

# With method selection
cargo run -- rag search "AI researchers" --method semantic --limit 20
```

### 2. Embeddings Commands

#### Generate Embeddings for All Profiles
```bash
# Backfill embeddings (⚠️  requires API key and may incur costs)
cargo run -- embeddings backfill --force

# With custom batch size
cargo run -- embeddings backfill --force --batch-size 50
```

#### Generate Embeddings for Specific Profile
```bash
# Generate for single FID
cargo run -- embeddings generate --fid 12345

# With verbose output
cargo run -- embeddings generate --fid 12345 --verbose
```

#### Test Embedding Generation
```bash
# Test with sample text
cargo run -- embeddings test "Hello, I am a blockchain developer"
```

#### Check Embedding Statistics
```bash
# See embedding coverage
cargo run -- embeddings stats
```

## ⚙️ Configuration

### OpenAI Setup
Edit `config.toml`:
```toml
[embeddings]
dimension = 1536
model = "text-embedding-ada-002"

[llm]
llm_endpoint = "https://api.openai.com/v1"
llm_key = "sk-your-api-key-here"
```

### Ollama Setup (Local)
```toml
[embeddings]
dimension = 768  # or model-specific dimension
model = "nomic-embed-text"

[llm]
llm_endpoint = "http://localhost:11434"
llm_key = "ollama"
```

## 🚀 Usage Examples

### Example 1: Find Similar Profiles
```bash
# Generate embeddings first
cargo run -- embeddings backfill --force

# Search with semantic similarity
cargo run -- rag search "AI and machine learning" --method semantic
```

### Example 2: RAG Query with Context
```bash
# Ask a question that requires understanding context
cargo run -- rag query "Who are the most active developers in the crypto space?" \
  --limit 15 \
  --verbose
```

### Example 3: Hybrid Search
```bash
# Combine vector and keyword matching
cargo run -- rag search "blockchain engineer @SF" --method hybrid
```

### Example 4: Generate Embeddings for New Profiles
```bash
# After syncing new profiles, generate their embeddings
cargo run -- embeddings generate --fid 99999
```

## 📊 Workflow Recommendations

### First Time Setup
1. **Sync Data**: `cargo run -- sync start`
2. **Generate Embeddings**: `cargo run -- embeddings backfill --force`
3. **Check Coverage**: `cargo run -- embeddings stats`
4. **Test RAG**: `cargo run -- rag query "test query"`

### Regular Usage
1. **Search Profiles**: Use `rag search` for quick lookups
2. **Ask Questions**: Use `rag query` for intelligent answers
3. **Monitor Embeddings**: Check `embeddings stats` periodically
4. **Update New Profiles**: Run `embeddings generate` for new FIDs

## 🔍 Search Methods Explained

### Semantic Search
- Uses vector embeddings to find **meaning-based** matches
- Best for: Conceptual queries, finding similar interests
- Example: "developers interested in decentralized systems"

### Keyword Search
- Uses traditional text matching
- Best for: Exact terms, usernames, specific phrases
- Example: "rust blockchain @vitalik"

### Hybrid Search
- Combines both methods with intelligent scoring
- Best for: Most queries, balanced approach
- Example: "AI researcher in San Francisco"

### Auto
- Automatically selects the best method
- Recommended for general use

## 💡 Tips & Best Practices

1. **Generate Embeddings First**: RAG queries require embeddings to work
2. **Monitor API Costs**: OpenAI embeddings have per-token costs
3. **Use Ollama for Local**: Free local embeddings with nomic-embed-text
4. **Batch Processing**: Backfill processes in batches to avoid rate limits
5. **Hybrid for Complex Queries**: Use hybrid search for best results
6. **Temperature Control**: Lower (0.1-0.3) for factual, higher (0.7-0.9) for creative

## 🐛 Troubleshooting

### No Embeddings Found
```bash
# Check coverage
cargo run -- embeddings stats

# If 0%, generate them
cargo run -- embeddings backfill --force
```

### API Key Errors
```bash
# Verify config
cargo run -- config

# Check API endpoint and key are correct in config.toml
```

### Low Quality Results
- Ensure embeddings are generated for all profiles
- Try different retrieval methods
- Adjust similarity thresholds
- Use more context (higher --limit)

## 📚 Next Steps

1. **Integrate with Applications**: Use the Rust API directly
2. **Build Web Interface**: Create HTTP endpoints for RAG queries
3. **Add Custom Prompts**: Modify prompt templates in `src/llm/prompts.rs`
4. **Optimize Performance**: Tune vector indexes in PostgreSQL
5. **Add Monitoring**: Track query performance and quality

## 🔗 API Usage

```rust
use snaprag::rag::RagService;
use snaprag::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;
    let rag = RagService::new(&config).await?;
    
    // Execute RAG query
    let response = rag.query("Find crypto developers").await?;
    println!("Answer: {}", response.answer);
    
    // Print sources
    for (i, source) in response.sources.iter().enumerate() {
        println!("{}. @{} (score: {:.3})", 
            i + 1, 
            source.profile.username.as_deref().unwrap_or("unknown"),
            source.score
        );
    }
    
    Ok(())
}
```

## 🎉 Features Completed

✅ Embedding generation (OpenAI/Ollama)  
✅ LLM integration (GPT/Llama)  
✅ Semantic/Keyword/Hybrid search  
✅ RAG pipeline (Retrieve → Rank → Generate)  
✅ CLI commands for all operations  
✅ Batch processing and backfill  
✅ Profile/Bio/Interests embeddings  
✅ Context assembly and prompt templates  

---

For more information, see the main [README.md](README.md) or run `cargo run -- --help`

