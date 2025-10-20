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

        context.push_str("===== YOUR ACTUAL POSTS (EXAMPLES TO COPY) =====\n\n");
        for (idx, result) in casts.iter().take(15).enumerate() {
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
