//! Ask command handler - AI role-playing as a specific user

use std::io::Write;
use std::io::{
    self,
};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

// Import the new ask module
use crate::cli::handlers::ask::args::parse_user_identifier;
use crate::cli::handlers::ask::llm::generate_ai_response;
use crate::cli::handlers::ask::output::display_response;
use crate::cli::handlers::ask::output::print_wrapped;
use crate::cli::handlers::ask::retrieval::analyze_writing_style;
use crate::cli::handlers::ask::retrieval::find_relevant_casts;
use crate::cli::handlers::ask::retrieval::Spinner;
use crate::cli::output::*;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::social_graph::SocialGraphAnalyzer;
use crate::sync::client::SnapchainClient;
use crate::sync::lazy_loader::LazyLoader;
use crate::AppConfig;
use crate::Result;

pub async fn handle_ask(
    config: &AppConfig,
    user_identifier: String,
    question: Option<String>,
    chat: bool,
    fetch_casts: bool,
    context_limit: usize,
    temperature: f32,
    verbose: bool,
) -> Result<()> {
    // Initialize services once
    let database = Arc::new(Database::from_config(config).await?);
    let snapchain_client = Arc::new(SnapchainClient::from_config(config).await?);
    let lazy_loader = LazyLoader::new(database.clone(), snapchain_client);
    let embedding_service = Arc::new(EmbeddingService::new(config)?);
    let llm_service = Arc::new(LlmService::new(config)?);

    // Parse user identifier (FID or username)
    let fid = parse_user_identifier(&user_identifier, &database).await?;

    // Load profile and casts once
    let (profile, casts) = load_user_data(
        &lazy_loader,
        &database,
        &embedding_service,
        fid,
        fetch_casts,
        verbose,
    )
    .await?;

    // Interactive chat mode
    if chat {
        run_interactive_chat(
            &database,
            &embedding_service,
            &llm_service,
            fid,
            &profile,
            &casts,
            context_limit,
            temperature,
            verbose,
        )
        .await?;
    } else {
        // Single question mode
        let q = question.ok_or_else(|| {
            crate::SnapRagError::Custom("Question required in non-chat mode".to_string())
        })?;

        answer_single_question(
            &database,
            &embedding_service,
            &llm_service,
            fid,
            &profile,
            &casts,
            &q,
            context_limit,
            temperature,
            verbose,
        )
        .await?;
    }

    Ok(())
}

/// Load user data (profile and casts with embeddings)
async fn load_user_data(
    lazy_loader: &LazyLoader,
    database: &Database,
    embedding_service: &EmbeddingService,
    fid: u64,
    fetch_casts: bool,
    verbose: bool,
) -> Result<(crate::models::UserProfile, Vec<crate::models::Cast>)> {
    print_info("🤖 Loading user data...");
    println!();

    // 1. Fetch user profile
    print_info(&format!("📋 Step 1/3: Fetching profile for FID {}...", fid));
    let profile = lazy_loader
        .get_user_profile_smart(fid as i64)
        .await?
        .ok_or_else(|| crate::SnapRagError::Custom(format!("User {} not found", fid)))?;

    let username = profile
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| format!("FID {}", fid));
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");

    println!("   ✅ Found: {} ({})", display_name, username);
    if let Some(bio) = &profile.bio {
        let bio_preview = if bio.len() > 100 {
            format!("{}...", &bio[..100])
        } else {
            bio.clone()
        };
        println!("   📝 Bio: {}", bio_preview);
    }
    println!();

    // 2. Fetch and ensure embeddings for casts
    print_info(&format!("📚 Step 2/3: Loading casts for {}...", username));
    let mut casts = lazy_loader.get_user_casts_smart(fid as i64).await?;

    if casts.is_empty() && fetch_casts {
        print_info("   No casts found in database, fetching from Snapchain...");
        casts = lazy_loader.fetch_user_casts(fid).await?;
    }

    if casts.is_empty() {
        print_error(&format!("No casts found for {}.", username));
        return Err(crate::SnapRagError::Custom(
            "No casts available".to_string(),
        ));
    }

    println!("   ✅ Loaded {} casts", casts.len());

    // Check how many casts need embeddings
    let cast_hashes: Vec<_> = casts
        .iter()
        .filter(|c| {
            c.text
                .as_ref()
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false)
        })
        .map(|c| c.message_hash.clone())
        .collect();

    // Efficiently check which casts are missing embeddings
    let missing_hashes = database.get_missing_embeddings(&cast_hashes).await?;

    let casts_without_embeddings: Vec<_> = casts
        .iter()
        .filter(|cast| missing_hashes.contains(&cast.message_hash))
        .cloned()
        .collect();

    let casts_with_embeddings = cast_hashes.len() - casts_without_embeddings.len();

    println!(
        "   📊 Embeddings: {} existing, {} missing",
        casts_with_embeddings,
        casts_without_embeddings.len()
    );

    // Generate missing embeddings
    if !casts_without_embeddings.is_empty() {
        print_info(&format!(
            "🔮 Step 3/3: Generating embeddings for {} casts...",
            casts_without_embeddings.len()
        ));

        let mut success = 0;
        let total = casts_without_embeddings.len();

        for (idx, cast) in casts_without_embeddings.iter().enumerate() {
            if let Some(ref text) = cast.text {
                if !text.trim().is_empty() {
                    match embedding_service.generate(text).await {
                        Ok(embedding) => {
                            if let Err(e) = database
                                .store_cast_embedding(
                                    &cast.message_hash,
                                    cast.fid,
                                    text,
                                    &embedding,
                                )
                                .await
                            {
                                tracing::warn!("Failed to store embedding: {}", e);
                            } else {
                                success += 1;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to generate embedding: {}", e);
                        }
                    }

                    // Progress update
                    let processed = idx + 1;
                    let percentage = (processed as f64 / total as f64 * 100.0) as u32;
                    let bar_width = 30;
                    let filled = (processed as f64 / total as f64 * bar_width as f64) as usize;
                    let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

                    print!(
                        "\r   Progress: [{}] {}% ({}/{})",
                        bar, percentage, processed, total
                    );
                    io::stdout().flush().ok();
                }
            }
        }

        println!();
        println!("   ✅ Generated {} embeddings", success);
    } else {
        print_info("Step 3/3: All casts have embeddings ✓");
    }
    println!();

    Ok((profile, casts))
}

/// Answer a single question
async fn answer_single_question(
    database: &Database,
    embedding_service: &EmbeddingService,
    llm_service: &LlmService,
    fid: u64,
    profile: &crate::models::UserProfile,
    casts: &[crate::models::Cast],
    question: &str,
    context_limit: usize,
    temperature: f32,
    verbose: bool,
) -> Result<()> {
    // Find relevant casts (spinner shows "Searching...")
    let relevant_casts = find_relevant_casts(
        database,
        embedding_service,
        fid,
        question,
        context_limit,
        verbose,
    )
    .await?;

    if relevant_casts.is_empty() {
        print_warning("No relevant casts found. The AI will answer based only on the profile.");
    } else {
        println!("   ✅ Found {} relevant casts", relevant_casts.len());
    }

    // Show writing style in verbose mode
    if verbose && !relevant_casts.is_empty() {
        let style = analyze_writing_style(&relevant_casts);
        println!("   📝 Writing style: {}", style);
    }
    println!();

    // Generate response (spinner shows "Thinking...")
    let response = generate_ai_response(
        llm_service,
        profile,
        &relevant_casts,
        question,
        None, // No conversation history
        temperature,
    )
    .await?;

    // Display response
    display_response(profile, &response, casts.len(), relevant_casts.len());

    Ok(())
}

/// Interactive chat mode with conversation history
async fn run_interactive_chat(
    database: &Database,
    embedding_service: &EmbeddingService,
    llm_service: &LlmService,
    fid: u64,
    profile: &crate::models::UserProfile,
    casts: &[crate::models::Cast],
    context_limit: usize,
    temperature: f32,
    verbose: bool,
) -> Result<()> {
    let username = profile
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| format!("FID {}", fid));
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║  💬 Interactive Chat Mode                                     ║");
    println!(
        "║  Chatting with: {} ({})                      ",
        display_name, username
    );
    if let Some(bio) = &profile.bio {
        let bio_preview = if bio.len() > 50 {
            format!("{}...", &bio[..50])
        } else {
            bio.clone()
        };
        println!(
            "║  Bio: {}                                     ",
            bio_preview
        );
    }
    println!("║  Commands: 'exit', 'quit', 'style' (show style), Ctrl+C      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    // Show style hint
    print_info(&format!(
        "💡 Based on {} casts - AI will mimic their writing style",
        casts.len()
    ));
    println!();

    let mut conversation_history: Vec<(String, String)> = Vec::new(); // (question, answer)

    loop {
        // Prompt for question
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let question = input.trim();

        // Check for special commands
        if question.is_empty() {
            continue;
        }
        if question.eq_ignore_ascii_case("exit")
            || question.eq_ignore_ascii_case("quit")
            || question.eq_ignore_ascii_case("q")
        {
            println!();
            print_success("👋 Conversation ended. Goodbye!");
            break;
        }
        if question.eq_ignore_ascii_case("style") {
            println!();
            // Get some recent casts for style analysis
            let recent_casts = database
                .get_casts_by_fid(fid as i64, Some(20), Some(0))
                .await?;

            // Convert to CastSearchResult format for analysis
            let sample_casts: Vec<crate::models::CastSearchResult> = recent_casts
                .into_iter()
                .map(|c| crate::models::CastSearchResult {
                    message_hash: c.message_hash,
                    fid: c.fid,
                    text: c.text.unwrap_or_default(),
                    timestamp: c.timestamp,
                    parent_hash: c.parent_hash,
                    embeds: c.embeds,
                    mentions: c.mentions,
                    similarity: 1.0,
                    reply_count: 0,
                    reaction_count: 0,
                })
                .collect();

            let style = analyze_writing_style(&sample_casts);
            println!("📝 Writing style analysis: {}", style);
            println!();
            println!("Sample posts:");
            for (idx, cast) in sample_casts.iter().take(5).enumerate() {
                println!("  {}. {}", idx + 1, cast.text);
            }
            println!();
            continue;
        }

        println!();

        // Find relevant casts (spinner shows "Searching...")
        let relevant_casts = find_relevant_casts(
            database,
            embedding_service,
            fid,
            question,
            context_limit,
            verbose,
        )
        .await?;

        if verbose && !relevant_casts.is_empty() {
            println!(
                "   ✅ Using {} relevant casts as context",
                relevant_casts.len()
            );
        } else if !relevant_casts.is_empty() {
            println!("   ✅ Found {} relevant casts", relevant_casts.len());
        }

        // Generate response with conversation history (spinner shows "Thinking...")
        let response = generate_ai_response(
            llm_service,
            profile,
            &relevant_casts,
            question,
            Some(&conversation_history),
            temperature,
        )
        .await?;

        // Display response
        println!();
        println!("{}:", display_name);
        println!();

        // Word wrap the response
        print_wrapped(&response, 70);

        println!();
        println!("─────────────────────────────────────────────────────────────────");
        println!();

        // Add to conversation history
        conversation_history.push((question.to_string(), response.clone()));

        // Limit history to last 5 exchanges to avoid context overflow
        if conversation_history.len() > 5 {
            conversation_history.remove(0);
        }
    }

    Ok(())
}
