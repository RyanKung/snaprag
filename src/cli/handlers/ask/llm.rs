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
    generate_ai_response_with_social(llm, profile, casts, question, history, temperature, None)
        .await
}

/// Generate AI response with optional social graph context
pub async fn generate_ai_response_with_social(
    llm: &LlmService,
    profile: &crate::models::UserProfile,
    casts: &[crate::models::CastSearchResult],
    question: &str,
    history: Option<&Vec<(String, String)>>,
    temperature: f32,
    social_profile: Option<&crate::social_graph::SocialProfile>,
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

    // Add social graph context if available
    if let Some(social) = social_profile {
        context.push_str(&format_social_profile_for_llm(social));
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

/// Format social profile for LLM context
fn format_social_profile_for_llm(profile: &crate::social_graph::SocialProfile) -> String {
    let mut output = String::new();

    output.push_str("═══════════════════════════════════════════════════════\n");
    output.push_str("👥 YOUR SOCIAL NETWORK\n");
    output.push_str("═══════════════════════════════════════════════════════\n\n");

    // Basic stats
    output.push_str(&format!(
        "Following: {} | Followers: {} | Influence: {:.1}x\n\n",
        profile.following_count, profile.followers_count, profile.influence_score
    ));

    // Social circles
    output.push_str("Your Network:\n");
    if profile.social_circles.tech_builders > 30.0 {
        output.push_str(&format!(
            "  🔧 Tech/Builders: {:.0}% - You're deep in tech circles\n",
            profile.social_circles.tech_builders
        ));
    } else if profile.social_circles.tech_builders > 10.0 {
        output.push_str(&format!(
            "  🔧 Tech/Builders: {:.0}%\n",
            profile.social_circles.tech_builders
        ));
    }

    if profile.social_circles.web3_natives > 30.0 {
        output.push_str(&format!(
            "  ⛓️ Web3/Crypto: {:.0}% - Heavy web3 network\n",
            profile.social_circles.web3_natives
        ));
    } else if profile.social_circles.web3_natives > 10.0 {
        output.push_str(&format!(
            "  ⛓️ Web3/Crypto: {:.0}%\n",
            profile.social_circles.web3_natives
        ));
    }

    if profile.social_circles.content_creators > 20.0 {
        output.push_str(&format!(
            "  🎨 Creators: {:.0}%\n",
            profile.social_circles.content_creators
        ));
    }

    output.push_str("\n");

    // Most mentioned users
    if !profile.most_mentioned_users.is_empty() {
        output.push_str("People You Often Mention:\n");
        for (idx, user) in profile.most_mentioned_users.iter().take(3).enumerate() {
            let name = user
                .username
                .as_ref()
                .map(|u| format!("@{}", u))
                .or_else(|| user.display_name.clone())
                .unwrap_or_else(|| format!("FID {}", user.fid));

            output.push_str(&format!("  {}. {} ({}x)\n", idx + 1, name, user.count));
        }
        output.push_str("\n");
    }

    // Interaction style
    output.push_str(&format!(
        "Your Role: {} | Reply rate: {:.0}% | Mention rate: {:.0}%\n",
        profile.interaction_style.community_role,
        profile.interaction_style.reply_frequency * 100.0,
        profile.interaction_style.mention_frequency * 100.0
    ));

    if profile.interaction_style.network_connector {
        output.push_str("🌐 You're a network connector - you introduce people\n");
    }

    output.push_str("\n");

    // Add context instructions
    output.push_str("🎯 Social Context:\n");

    if profile.influence_score > 2.0 {
        output.push_str("  → You're influential - speak with confidence\n");
    }

    if profile.social_circles.tech_builders > 40.0 {
        output.push_str("  → Deep in tech circles - use builder language naturally\n");
    }

    if profile.social_circles.web3_natives > 40.0 {
        output.push_str("  → Web3 native - crypto culture is second nature to you\n");
    }

    if !profile.most_mentioned_users.is_empty() {
        output.push_str("  → Feel free to reference your network: ");
        let names: Vec<String> = profile
            .most_mentioned_users
            .iter()
            .take(3)
            .filter_map(|u| u.username.as_ref().map(|n| format!("@{}", n)))
            .collect();
        output.push_str(&names.join(", "));
        output.push_str("\n");
    }

    // Add word cloud - your vocabulary fingerprint
    if !profile.word_cloud.top_words.is_empty() {
        output.push_str("\n📚 Your Common Vocabulary:\n");
        let top_10: Vec<String> = profile
            .word_cloud
            .top_words
            .iter()
            .take(10)
            .map(|w| w.word.clone())
            .collect();
        output.push_str(&format!("  {}\n", top_10.join(", ")));

        if !profile.word_cloud.signature_words.is_empty() {
            output.push_str("\n✨ Your Signature Words: ");
            output.push_str(&profile.word_cloud.signature_words.join(", "));
            output.push_str("\n");
            output.push_str("  → Use these words naturally in your responses\n");
        }
    }

    output.push_str("\n═══════════════════════════════════════════════════════\n\n");

    output
}
