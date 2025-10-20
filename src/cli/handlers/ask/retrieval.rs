use std::io::Write;

/// Retrieval and ranking pipeline for ask
use crate::cli::output::*;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::Result;

/// Simple spinner for showing progress
pub struct Spinner {
    message: String,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        let message = self.message.clone();
        let running = self.running.clone();
        running.store(true, std::sync::atomic::Ordering::Relaxed);

        std::thread::spawn(move || {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut idx = 0;

            while running.load(std::sync::atomic::Ordering::Relaxed) {
                print!("\r   {} {}...", frames[idx], message);
                std::io::stdout().flush().ok();
                idx = (idx + 1) % frames.len();
                std::thread::sleep(std::time::Duration::from_millis(80));
            }

            // Clear the line
            print!("\r{}\r", " ".repeat(80));
            std::io::stdout().flush().ok();
        });
    }

    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(100)); // Give time to clear
    }
}

/// Find relevant casts using semantic search and heuristics
pub async fn find_relevant_casts(
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
pub fn analyze_writing_style(casts: &[crate::models::CastSearchResult]) -> String {
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
