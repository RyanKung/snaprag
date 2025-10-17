//! Custom RAG pipeline example
//!
//! Run with: cargo run --example custom_pipeline

use std::sync::Arc;

use snaprag::{AppConfig, CastContextAssembler, CastRetriever, Database, EmbeddingService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load()?;

    println!("🔧 Custom RAG Pipeline Example\n");

    // Initialize services manually
    let db = Arc::new(Database::from_config(&config).await?);
    let embedding_service = Arc::new(EmbeddingService::new(&config)?);

    // Create custom cast retriever
    let cast_retriever = CastRetriever::new(Arc::clone(&db), Arc::clone(&embedding_service));

    // Step 1: Retrieve casts
    let query = "What are people saying about Warpcast?";
    println!("🔍 Searching: {}\n", query);

    let casts = cast_retriever.semantic_search(query, 10, Some(0.7)).await?;

    println!("Found {} relevant casts:\n", casts.len());

    // Step 2: Show results with engagement metrics
    for (i, cast) in casts.iter().take(5).enumerate() {
        println!("{}. FID {} (similarity: {:.1}%)", i + 1, cast.fid, cast.similarity * 100.0);
        println!("   {}", &cast.text[..cast.text.len().min(100)]);
        println!(
            "   📊 {} replies, {} reactions\n",
            cast.reply_count, cast.reaction_count
        );
    }

    // Step 3: Assemble context with author info
    let context_assembler = CastContextAssembler::new(4096);
    let context = context_assembler
        .assemble_with_authors(&casts, db)
        .await?;

    println!("📦 Context assembled: {} characters", context.len());

    // Step 4: You can now use this context with your own LLM
    println!("\n✅ Context ready for LLM processing!");

    Ok(())
}

