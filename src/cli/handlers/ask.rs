//! Ask command handler - AI role-playing as a specific user

use std::io::Write;
use std::io::{
    self,
};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::cli::output::*;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::sync::client::SnapchainClient;
use crate::sync::lazy_loader::LazyLoader;
use crate::AppConfig;
use crate::Result;

/// Simple spinner for showing progress
struct Spinner {
    message: String,
    running: Arc<AtomicBool>,
}

impl Spinner {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    fn start(&self) {
        let message = self.message.clone();
        let running = self.running.clone();
        running.store(true, Ordering::Relaxed);

        std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut idx = 0;

            while running.load(Ordering::Relaxed) {
                print!("\r   {} {}...", frames[idx], message);
                io::stdout().flush().ok();
                idx = (idx + 1) % frames.len();
                std::thread::sleep(Duration::from_millis(80));
            }

            // Clear the line
            print!("\r{}\r", " ".repeat(80));
            io::stdout().flush().ok();
        });
    }

    fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(100)); // Give time to clear
    }
}

/// Parse user identifier (FID or username) and return FID
async fn parse_user_identifier(identifier: &str, database: &Database) -> Result<u64> {
    let trimmed = identifier.trim();

    // Check if it starts with @ (username)
    if trimmed.starts_with('@') {
        // Remove @ and query by username
        let username = trimmed.trim_start_matches('@');

        print_info(&format!("🔍 Looking up username: @{}", username));

        // Query database for username
        let profile = database
            .get_user_profile_by_username(username)
            .await?
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!("Username @{} not found in database", username))
            })?;

        println!("   ✅ Found FID: {}", profile.fid);
        Ok(profile.fid as u64)
    } else {
        // Try to parse as FID number
        trimmed.parse::<u64>().map_err(|_| {
            crate::SnapRagError::Custom(format!(
                "Invalid user identifier '{}'. Use FID (e.g., '99') or username (e.g., '@jesse.base.eth')",
                identifier
            ))
        })
    }
}

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

/// Find relevant casts using semantic search, preferring longer/more substantive posts
async fn find_relevant_casts(
    database: &Database,
    embedding_service: &EmbeddingService,
    fid: u64,
    question: &str,
    context_limit: usize,
    verbose: bool,
) -> Result<Vec<crate::models::CastSearchResult>> {
    // Generate query embedding with spinner
    let spinner = Spinner::new("Searching");
    spinner.start();

    tracing::debug!("Generating query embedding...");
    let query_embedding = embedding_service.generate(question).await?;
    tracing::debug!("Query embedding generated");

    // Search for semantically similar casts (use simple version without engagement metrics)
    // Use a larger limit initially to ensure we get enough from this FID
    let search_limit = (context_limit * 5).max(100);
    tracing::debug!(
        "Executing vector search with limit={}, threshold=0.3",
        search_limit
    );
    let search_results = database
        .semantic_search_casts_simple(query_embedding, search_limit as i64, Some(0.3))
        .await?;
    tracing::debug!(
        "Vector search completed, found {} results",
        search_results.len()
    );

    spinner.stop();

    // Filter to only include casts from this FID
    let mut user_casts: Vec<_> = search_results
        .into_iter()
        .filter(|result| result.fid == fid as i64)
        .collect();

    // Calculate recency score - newer posts get higher weight
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Prioritize: relevance + substance + recency
    // Score = similarity * log(length) * recency_factor
    user_casts.sort_by(|a, b| {
        // Recency factor: 1.0 for recent (< 30 days), decays to 0.5 for old (> 1 year)
        let age_a_days = ((now - a.timestamp) as f32) / 86400.0;
        let age_b_days = ((now - b.timestamp) as f32) / 86400.0;
        let recency_a = (1.0 - (age_a_days / 365.0).min(0.5)).max(0.5);
        let recency_b = (1.0 - (age_b_days / 365.0).min(0.5)).max(0.5);

        // Combine: similarity (most important) + substance + recency
        let score_a = a.similarity * (a.text.len() as f32).ln().max(1.0) * recency_a;
        let score_b = b.similarity * (b.text.len() as f32).ln().max(1.0) * recency_b;

        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top results
    user_casts.truncate(context_limit);

    let user_relevant_casts = user_casts;

    if verbose && !user_relevant_casts.is_empty() {
        println!();
        println!("   📋 Top relevant casts (sorted by: relevance × substance × recency):");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for (idx, result) in user_relevant_casts.iter().take(5).enumerate() {
            let preview = if result.text.len() > 80 {
                format!("{}...", &result.text[..80])
            } else {
                result.text.clone()
            };

            // Calculate days ago
            let age_days = ((now - result.timestamp) as f32) / 86400.0;
            let age_str = if age_days < 1.0 {
                "today".to_string()
            } else if age_days < 7.0 {
                format!("{:.0}d ago", age_days)
            } else if age_days < 30.0 {
                format!("{:.0}d ago", age_days)
            } else if age_days < 365.0 {
                format!("{:.0}mo ago", age_days / 30.0)
            } else {
                format!("{:.1}y ago", age_days / 365.0)
            };

            println!(
                "      {}. [sim: {:.2}, len: {}, age: {}] {}",
                idx + 1,
                result.similarity,
                result.text.len(),
                age_str,
                preview
            );
        }
        println!();
    }

    Ok(user_relevant_casts)
}

/// Analyze user's writing style from their casts
fn analyze_writing_style(casts: &[crate::models::CastSearchResult]) -> String {
    if casts.is_empty() {
        return "casual and friendly".to_string();
    }

    let mut style_notes = Vec::new();

    // Analyze emoji usage
    let total_emojis: usize = casts
        .iter()
        .map(|c| {
            c.text
                .chars()
                .filter(|ch| {
                    // Simple emoji detection (Unicode ranges)
                    matches!(*ch as u32, 0x1F300..=0x1F9FF | 0x2600..=0x26FF | 0x2700..=0x27BF)
                })
                .count()
        })
        .sum();

    let emoji_per_post = total_emojis as f32 / casts.len() as f32;
    if emoji_per_post > 2.0 {
        style_notes.push("frequently uses emojis (2-3+ per post)");
    } else if emoji_per_post > 0.5 {
        style_notes.push("uses emojis moderately");
    } else if emoji_per_post > 0.0 {
        style_notes.push("occasionally uses emojis");
    } else {
        style_notes.push("text-focused, no emojis");
    }

    // Analyze sentence length - focus on longer posts
    let substantive_casts: Vec<_> = casts.iter().filter(|c| c.text.len() > 50).collect();

    if !substantive_casts.is_empty() {
        let avg_length: usize = substantive_casts
            .iter()
            .map(|c| c.text.len())
            .sum::<usize>()
            / substantive_casts.len();

        if avg_length > 200 {
            style_notes.push("writes detailed explanations");
        } else if avg_length > 100 {
            style_notes.push("moderately detailed");
        } else {
            style_notes.push("concise but informative");
        }
    }

    // Check for informal markers
    let informal_count = casts
        .iter()
        .filter(|c| {
            let lower = c.text.to_lowercase();
            lower.contains("lol")
                || lower.contains("lmao")
                || lower.contains("omg")
                || lower.contains("tbh")
                || lower.contains("ngl")
                || lower.contains("fr")
                || lower.contains("gonna")
                || lower.contains("wanna")
        })
        .count();

    if informal_count > casts.len() / 2 {
        style_notes.push("very casual and informal");
    } else if informal_count > casts.len() / 4 {
        style_notes.push("relaxed and conversational");
    } else {
        style_notes.push("professional and articulate");
    }

    // Check for technical language
    let tech_count = casts
        .iter()
        .filter(|c| {
            let lower = c.text.to_lowercase();
            lower.contains("build")
                || lower.contains("dev")
                || lower.contains("code")
                || lower.contains("api")
                || lower.contains("tech")
                || lower.contains("protocol")
                || lower.contains("onchain")
                || lower.contains("contract")
        })
        .count();

    if tech_count > casts.len() / 2 {
        style_notes.push("highly technical and builder-focused");
    } else if tech_count > casts.len() / 4 {
        style_notes.push("tech-aware");
    }

    // Check for enthusiasm/energy
    let exclamation_count = casts.iter().filter(|c| c.text.contains('!')).count();

    if exclamation_count > casts.len() / 2 {
        style_notes.push("enthusiastic and energetic");
    }

    style_notes.join(", ")
}

/// Generate AI response based on profile, casts, and conversation history
async fn generate_ai_response(
    llm_service: &LlmService,
    profile: &crate::models::UserProfile,
    relevant_casts: &[crate::models::CastSearchResult],
    question: &str,
    conversation_history: Option<&Vec<(String, String)>>,
    temperature: f32,
) -> Result<String> {
    let fid = profile.fid;
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");
    let username = profile.username.as_deref();

    // Analyze writing style from casts
    let writing_style = analyze_writing_style(relevant_casts);

    // Build context
    let mut context = String::new();
    context.push_str(&format!(
        "You are role-playing as {}, a Farcaster user",
        display_name
    ));
    if let Some(username) = username {
        context.push_str(&format!(" (username: @{})", username));
    }
    context.push_str(&format!(". Your FID is {}.\n\n", fid));

    if let Some(bio) = &profile.bio {
        context.push_str(&format!("Your bio: {}\n\n", bio));
    }

    // Add writing style analysis and examples
    if !relevant_casts.is_empty() {
        // Calculate average length of relevant posts for style matching
        let avg_length: usize = relevant_casts.iter().map(|c| c.text.len()).sum::<usize>()
            / relevant_casts.len().max(1);

        context.push_str("===== YOUR ACTUAL POSTS (EXAMPLES TO COPY) =====\n\n");
        for (idx, result) in relevant_casts.iter().take(15).enumerate() {
            context.push_str(&format!("{}. {}\n", idx + 1, result.text));
        }
        context.push_str("\n");
        context.push_str(&format!(
            "CRITICAL - Your average post length: ~{} characters\n",
            avg_length
        ));
        context.push_str(&format!("Style analysis: {}\n\n", writing_style));
        context.push_str("===== HOW TO ANSWER =====\n");
        context.push_str("STUDY the examples above. Notice:\n");
        context.push_str("- How SHORT or LONG are they?\n");
        context.push_str("- What WORDS do you use?\n");
        context.push_str("- How DIRECT or EXPLANATORY are you?\n");
        context.push_str("- Do you use emojis? How many?\n");
        context.push_str("- What's your ENERGY level?\n\n");

        if avg_length < 80 {
            context.push_str("⚠️ Your posts are VERY SHORT (under 80 chars). Keep your answer similarly brief!\n");
            context.push_str(
                "Don't write paragraphs if you typically write 1-2 sentences or less.\n\n",
            );
        } else if avg_length < 150 {
            context.push_str("Your posts are concise. Keep answers to 2-3 sentences max.\n\n");
        }

        context.push_str(
            "MIMIC THE EXACT STYLE. If your posts are 5-10 words, your answer should be too.\n",
        );
        context.push_str("If you use emojis, add them. If you're casual, stay casual.\n");
        context.push_str("MATCH THE LENGTH AND ENERGY of the examples above.\n\n");
    }

    // Add conversation history if available
    if let Some(history) = conversation_history {
        if !history.is_empty() {
            context.push_str("Previous conversation:\n\n");
            for (q, a) in history {
                context.push_str(&format!("User: {}\n", q));
                context.push_str(&format!("You: {}\n\n", a));
            }
        }
    }

    context.push_str("===== THE QUESTION =====\n\n");
    context.push_str(&format!("User: {}\n\n", question));
    context.push_str("You (RESPOND IN YOUR STYLE - match examples above!):");

    // Log context in debug mode for troubleshooting
    tracing::debug!("=== LLM PROMPT ===\n{}\n=== END PROMPT ===", context);

    // Generate response with spinner
    let spinner = Spinner::new("Thinking");
    spinner.start();

    // Use lower temperature for very concise styles
    let adjusted_temp = if !relevant_casts.is_empty() {
        let avg_len: usize = relevant_casts.iter().map(|c| c.text.len()).sum::<usize>()
            / relevant_casts.len().max(1);
        if avg_len < 80 {
            temperature.min(0.5) // Lower temp for brief styles
        } else {
            temperature
        }
    } else {
        temperature
    };

    let response = llm_service
        .generate_with_params(&context, adjusted_temp, 2000)
        .await?;

    spinner.stop();

    Ok(response)
}

/// Display a formatted response
fn display_response(
    profile: &crate::models::UserProfile,
    response: &str,
    total_casts: usize,
    relevant_casts: usize,
) {
    let username = profile
        .username
        .as_ref()
        .map(|u| format!("@{}", u))
        .unwrap_or_else(|| format!("FID {}", profile.fid));
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!(
        "║  {} ({})                                           ",
        display_name, username
    );
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    print_wrapped(response, 70);

    println!();
    println!("─────────────────────────────────────────────────────────────────");
    println!(
        "💬 Based on {} casts  |  🎯 Context: {} relevant casts",
        total_casts, relevant_casts
    );
    println!("─────────────────────────────────────────────────────────────────");
}

/// Word wrap text to specified width
fn print_wrapped(text: &str, max_width: usize) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
        if current_line.len() + word.len() + 1 > max_width {
            println!("{}", current_line);
            current_line = word.to_string();
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        println!("{}", current_line);
    }
}
