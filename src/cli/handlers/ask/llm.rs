use super::retrieval::analyze_writing_style;
use super::retrieval::Spinner;
/// LLM interaction (prompt building and calling)
use crate::llm::LlmService;
use crate::Result;

pub struct AiResponse {
    pub text: String,
}

pub async fn generate_ai_response(
    llm: &LlmService,
    profile: &crate::models::UserProfile,
    casts: &[crate::models::CastSearchResult],
    question: &str,
    history: Option<&Vec<(String, String)>>,
    temperature: f32,
) -> Result<String> {
    let fid = profile.fid;
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");
    let username = profile.username.as_deref();

    // Analyze writing style from casts
    let writing_style = analyze_writing_style(casts);

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
    if !casts.is_empty() {
        // Calculate average length of relevant posts for style matching
        let avg_length: usize =
            casts.iter().map(|c| c.text.len()).sum::<usize>() / casts.len().max(1);

        context.push_str("\n═══════════════════════════════════════════════════════\n");
        context.push_str("🎭 YOUR WRITING STYLE - STUDY THESE EXAMPLES CAREFULLY\n");
        context.push_str("═══════════════════════════════════════════════════════\n\n");

        context.push_str("These are YOUR actual posts. This is HOW YOU WRITE:\n\n");
        for (idx, result) in casts.iter().take(15).enumerate() {
            context.push_str(&format!("{}. \"{}\"\n", idx + 1, result.text));
        }

        context.push_str("\n─────────────────────────────────────────────────────\n");
        context.push_str("📊 STYLE ANALYSIS\n");
        context.push_str("─────────────────────────────────────────────────────\n\n");
        context.push_str(&format!("Average length: {} characters\n\n", avg_length));
        context.push_str(&format!("{}\n\n", writing_style));

        context.push_str("─────────────────────────────────────────────────────\n");
        context.push_str("🎯 CRITICAL RULES - YOU MUST FOLLOW THESE:\n");
        context.push_str("─────────────────────────────────────────────────────\n\n");

        if avg_length < 50 {
            context.push_str("⚠️ ULTRA-SHORT MODE ACTIVATED ⚠️\n");
            context.push_str("Your posts are EXTREMELY brief (under 50 chars).\n");
            context.push_str("➤ Your response MUST be under 50 characters\n");
            context.push_str("➤ Use 1 short sentence or just a few words\n");
            context.push_str("➤ NO lengthy explanations - be ULTRA concise\n\n");
        } else if avg_length < 100 {
            context.push_str("⚠️ CONCISE MODE ⚠️\n");
            context.push_str("Your posts are very short (50-100 chars).\n");
            context.push_str("➤ Keep your response under 100 characters\n");
            context.push_str("➤ Maximum 1-2 short sentences\n");
            context.push_str("➤ Get straight to the point\n\n");
        } else if avg_length < 200 {
            context.push_str("📝 MODERATE MODE\n");
            context.push_str("Your posts are moderately sized (100-200 chars).\n");
            context.push_str("➤ Keep response around 100-200 characters\n");
            context.push_str("➤ 2-3 sentences maximum\n");
            context.push_str("➤ Stay focused and clear\n\n");
        } else {
            context.push_str("📚 DETAILED MODE\n");
            context.push_str("You write detailed explanations (200+ chars).\n");
            context.push_str("➤ Feel free to write 200-300 characters\n");
            context.push_str("➤ Multiple sentences are okay\n");
            context.push_str("➤ Provide thoughtful explanations\n\n");
        }

        context.push_str("🔥 MANDATORY STYLE RULES:\n\n");
        context.push_str("1. LENGTH: Match the length shown in examples above\n");
        context.push_str("2. WORDS: Use the same vocabulary and phrases you see\n");
        context.push_str("3. TONE: Match the energy level (casual/professional/technical)\n");
        context.push_str("4. EMOJIS: If examples have emojis, USE THEM. If not, DON'T.\n");
        context.push_str("5. PUNCTUATION: Match exclamation marks, questions, etc.\n");
        context.push_str("6. SLANG: If you use slang (lol, fr, ngl), keep using it\n");
        context.push_str("7. TECHNICAL: Match the technical depth shown in examples\n\n");

        context.push_str("⚡ BEFORE YOU RESPOND:\n");
        context.push_str("Ask yourself: \"Does this sound EXACTLY like the examples above?\"\n");
        context.push_str("If not, REWRITE to match the style more closely.\n\n");

        context.push_str("═══════════════════════════════════════════════════════\n\n");
    }

    // Add conversation history if available
    if let Some(history) = history {
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
    let adjusted_temp = if !casts.is_empty() {
        let avg_len: usize = casts.iter().map(|c| c.text.len()).sum::<usize>() / casts.len().max(1);
        if avg_len < 80 {
            temperature.min(0.5) // Lower temp for brief styles
        } else {
            temperature
        }
    } else {
        temperature
    };

    let response = llm
        .generate_with_params(&context, adjusted_temp, 2000)
        .await?;

    spinner.stop();

    Ok(response)
}
