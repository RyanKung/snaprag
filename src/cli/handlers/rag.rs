//! RAG (Retrieval-Augmented Generation) handlers

use std::sync::Arc;

use crate::cli::output::print_info;
use crate::cli::output::print_warning;
use crate::cli::output::truncate_str;
use crate::database::Database;
use crate::AppConfig;
use crate::Result;
use crate::SnapRag;

pub async fn handle_rag_query_casts(
    snaprag: &SnapRag,
    query: String,
    limit: usize,
    threshold: f32,
    temperature: f32,
    max_tokens: usize,
    verbose: bool,
) -> Result<()> {
    use std::sync::Arc;

    use crate::embeddings::EmbeddingService;
    use crate::llm::LlmService;
    use crate::rag::CastContextAssembler;
    use crate::rag::CastRetriever;

    print_info(&format!("🤖 RAG Query on Casts: \"{query}\""));

    // Check if we have embeddings
    let embed_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cast_embeddings")
        .fetch_one(snaprag.database().pool())
        .await?;

    if embed_count == 0 {
        print_warning("⚠️  No cast embeddings found. Run: snaprag embeddings backfill-casts");
        return Ok(());
    }

    // Step 1: Retrieve relevant casts using CastRetriever
    println!("\n🔍 Step 1: Retrieving relevant casts...");
    let config = AppConfig::load()?;
    let embedding_service = Arc::new(EmbeddingService::new(&config)?);
    let database = Arc::new(Database::from_config(&config).await?);
    let cast_retriever = CastRetriever::new(database, embedding_service);

    let results = cast_retriever
        .semantic_search(&query, limit, Some(threshold))
        .await?;

    if results.is_empty() {
        print_warning("No relevant casts found");
        return Ok(());
    }

    println!("   ✓ Found {} relevant casts", results.len());

    // Step 2: Assemble context using CastContextAssembler
    println!("🔧 Step 2: Assembling context...");
    let context_assembler = CastContextAssembler::default();
    let context = context_assembler
        .assemble_with_authors(&results, snaprag.database())
        .await?;

    if verbose {
        println!("   Context length: {} chars", context.len());
    }

    // Step 3: Generate answer with LLM using enhanced prompts
    println!("💭 Step 3: Generating answer...");
    let llm_service = LlmService::new(&config)?;

    // Use specialized cast RAG prompt
    let prompt = crate::rag::build_cast_rag_prompt(&query, &context);

    let answer = llm_service
        .generate_with_params(&prompt, temperature, max_tokens)
        .await?;

    // Print results
    println!("\n{}", "═".repeat(100));
    println!("📝 Answer:\n");
    println!("{}", answer.trim());
    println!("\n{}", "═".repeat(100));

    if verbose {
        println!("\n📚 Sources ({} casts):", results.len());
        for (idx, result) in results.iter().enumerate() {
            println!(
                "  {}. FID {} | Similarity: {:.2}% | \"{}...\"",
                idx + 1,
                result.fid,
                result.similarity * 100.0,
                result.text.chars().take(50).collect::<String>()
            );
        }
    } else {
        println!("\n💡 Use --verbose to see source casts");
    }

    Ok(())
}

/// Handle cast embeddings backfill command
pub async fn handle_rag_query(
    config: &AppConfig,
    query: String,
    limit: usize,
    method: String,
    temperature: f32,
    max_tokens: usize,
    verbose: bool,
) -> Result<()> {
    use crate::rag::RagQuery;
    use crate::rag::RagService;
    use crate::rag::RetrievalMethod;

    println!("🤖 SnapRAG Query");
    println!("================\n");
    println!("Question: {query}\n");

    // Parse retrieval method
    let retrieval_method = match method.as_str() {
        "semantic" => RetrievalMethod::Semantic,
        "keyword" => RetrievalMethod::Keyword,
        "hybrid" => RetrievalMethod::Hybrid,
        _ => RetrievalMethod::Auto,
    };

    println!("⏳ Initializing RAG service...");
    let rag_service = RagService::new(config).await?;

    println!("🔍 Retrieving relevant profiles...");
    let rag_query = RagQuery {
        question: query.clone(),
        retrieval_limit: limit,
        retrieval_method,
        temperature,
        max_tokens,
    };

    let response = rag_service.query_with_options(rag_query).await?;

    println!("\n📝 Answer:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("{}", response.answer);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📚 Sources ({} profiles):", response.sources.len());
    for (idx, source) in response.sources.iter().enumerate().take(10) {
        let username = source.profile.username.as_deref().unwrap_or("unknown");
        let display_name = source.profile.display_name.as_deref().unwrap_or("No name");

        println!(
            "  {}. @{} ({}) - FID: {}, Score: {:.3}, Match: {:?}",
            idx + 1,
            username,
            display_name,
            source.profile.fid,
            source.score,
            source.match_type
        );

        if verbose {
            if let Some(bio) = &source.profile.bio {
                let bio_preview = truncate_str(bio, 100);
                println!("     Bio: {bio_preview}");
            }
        }
    }

    if response.sources.len() > 10 {
        println!("  ... and {} more", response.sources.len() - 10);
    }

    Ok(())
}

/// Handle RAG search command
pub async fn handle_rag_search(
    config: &AppConfig,
    query: String,
    limit: usize,
    method: String,
) -> Result<()> {
    use crate::database::Database;
    use crate::embeddings::EmbeddingService;
    use crate::rag::Retriever;

    println!("🔍 SnapRAG Search");
    println!("=================\n");
    println!("Query: {query}\n");

    println!("⏳ Initializing search...");
    let database = Arc::new(Database::from_config(config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);
    let retriever = Retriever::new(database, embedding_service);

    println!("🔎 Searching profiles...");
    let results = match method.as_str() {
        "semantic" => retriever.semantic_search(&query, limit, None).await?,
        "keyword" => retriever.keyword_search(&query, limit).await?,
        "hybrid" => retriever.hybrid_search(&query, limit).await?,
        _ => retriever.auto_search(&query, limit).await?,
    };

    println!("\n✅ Found {} profiles:\n", results.len());

    for (idx, result) in results.iter().enumerate() {
        let username = result.profile.username.as_deref().unwrap_or("unknown");
        let display_name = result.profile.display_name.as_deref().unwrap_or("No name");

        println!(
            "{}. @{} ({}) - FID: {}",
            idx + 1,
            username,
            display_name,
            result.profile.fid
        );
        println!(
            "   Score: {:.3} | Match Type: {:?}",
            result.score, result.match_type
        );

        if let Some(bio) = &result.profile.bio {
            let bio_preview = truncate_str(bio, 150);
            println!("   Bio: {bio_preview}");
        }

        if let Some(location) = &result.profile.location {
            println!("   Location: {location}");
        }

        println!();
    }

    Ok(())
}
