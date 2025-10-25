/// Chat-related API handlers
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use super::AppState;
use crate::api::types::ApiResponse;
use crate::api::types::ChatMessageRequest;
use crate::api::types::ChatMessageResponse;
use crate::api::types::CreateChatRequest;
use crate::api::types::CreateChatResponse;
use crate::api::types::SessionInfoResponse;

/// Parse user identifier (FID or username) and return FID
async fn parse_user_identifier(
    identifier: &str,
    database: &crate::database::Database,
) -> crate::Result<u64> {
    let trimmed = identifier.trim();

    // Check if it starts with @ (username)
    if trimmed.starts_with('@') {
        // Remove @ and query by username
        let username = trimmed.trim_start_matches('@');

        // Query database for username
        let profile = database
            .get_user_profile_by_username(username)
            .await?
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!("Username @{username} not found in database"))
            })?;

        Ok(profile.fid as u64)
    } else {
        // Try to parse as FID number
        trimmed.parse::<u64>().map_err(|_| {
            crate::SnapRagError::Custom(format!(
                "Invalid user identifier '{identifier}'. Use FID (e.g., '99') or username (e.g., '@jesse.base.eth')"
            ))
        })
    }
}

/// Build chat context for LLM
fn build_chat_context(
    profile: &crate::models::UserProfile,
    casts: &[crate::models::CastSearchResult],
    session: &crate::api::session::ChatSession,
    message: &str,
) -> String {
    let mut context = String::new();

    context.push_str(&format!(
        "You are role-playing as {}, a Farcaster user",
        profile.display_name.as_deref().unwrap_or("Unknown")
    ));

    if let Some(username) = &profile.username {
        context.push_str(&format!(" (username: @{username})"));
    }

    context.push_str(&format!(". Your FID is {}.\n\n", profile.fid));

    if let Some(bio) = &profile.bio {
        context.push_str(&format!("Your bio: {bio}\n\n"));
    }

    // Add writing style analysis and examples
    if !casts.is_empty() {
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
        context.push_str("─────────────────────────────────────────────────────\n");
        context.push_str(&format!("Average length: {avg_length} characters\n\n"));

        context.push_str("🎯 CRITICAL RULES:\n\n");

        if avg_length < 50 {
            context.push_str(
                "⚠️ ULTRA-SHORT: Response MUST be under 50 characters. 1 sentence max.\n",
            );
        } else if avg_length < 100 {
            context.push_str("⚠️ CONCISE: Keep under 100 chars. 1-2 short sentences only.\n");
        } else if avg_length < 200 {
            context.push_str("📝 MODERATE: 100-200 chars. 2-3 sentences max.\n");
        } else {
            context.push_str("📚 DETAILED: 200-300 chars. Thoughtful explanations okay.\n");
        }

        context.push_str("\n1. MATCH LENGTH shown in examples\n");
        context.push_str("2. USE SAME vocabulary and phrases\n");
        context.push_str("3. COPY tone (casual/professional/technical)\n");
        context.push_str("4. EMOJIS: If examples have them, USE THEM. If not, DON'T.\n");
        context.push_str("5. MATCH punctuation (!,?, etc.)\n");
        context.push_str("6. KEEP slang if present (lol, fr, ngl, etc.)\n\n");

        context.push_str("⚡ Ask: \"Does this sound EXACTLY like my examples?\"\n\n");
        context.push_str("═══════════════════════════════════════════════════════\n\n");
    }

    // Add conversation history if available
    if !session.conversation_history.is_empty() {
        context.push_str("Previous conversation:\n\n");
        for message in &session.conversation_history {
            context.push_str(&format!("{}: {}\n", message.role, message.content));
        }
        context.push('\n');
    }

    context.push_str("═══ THE QUESTION ═══\n\n");
    context.push_str(&format!("User: {message}\n\n"));
    context.push_str("You (RESPOND IN YOUR EXACT STYLE):");

    context
}

/// Create chat session
pub async fn create_chat_session(
    State(state): State<AppState>,
    Json(req): Json<CreateChatRequest>,
) -> Result<Json<ApiResponse<CreateChatResponse>>, StatusCode> {
    info!("POST /api/chat/create - user: {}", req.user);

    // Parse user identifier (FID or username)
    let fid = match parse_user_identifier(&req.user, &state.database).await {
        Ok(fid) => fid,
        Err(e) => {
            error!("Failed to parse user identifier: {}", e);
            return Ok(Json(ApiResponse::error(format!(
                "Invalid user identifier: {e}"
            ))));
        }
    };

    // Get user profile
    let profile = match state.database.get_user_profile(fid as i64).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Ok(Json(ApiResponse::error(format!("User {fid} not found"))));
        }
        Err(e) => {
            error!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Count user's casts - optimized query
    let casts_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts WHERE fid = $1")
        .bind(fid as i64)
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0) as usize;

    // Create session
    let session = state.session_manager.create_session(
        fid as i64,
        profile.username.clone(),
        profile.display_name.clone(),
        req.context_limit,
        req.temperature,
    );

    info!(
        "Created chat session: {} for FID {}",
        session.session_id, fid
    );

    Ok(Json(ApiResponse::success(CreateChatResponse {
        session_id: session.session_id,
        fid: fid as i64,
        username: profile.username,
        display_name: profile.display_name,
        bio: profile.bio,
        total_casts: casts_count,
    })))
}

/// Send a message in a chat session
pub async fn send_chat_message(
    State(state): State<AppState>,
    Json(req): Json<ChatMessageRequest>,
) -> Result<Json<ApiResponse<ChatMessageResponse>>, StatusCode> {
    info!("POST /api/chat/message - session: {}", req.session_id);

    // Get session
    let mut session = match state.session_manager.get_session(&req.session_id) {
        Some(s) => s,
        None => {
            return Ok(Json(ApiResponse::error("Session not found or expired")));
        }
    };

    // Get user profile
    let profile = match state.database.get_user_profile(session.fid).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Ok(Json(ApiResponse::error(format!(
                "User {} not found",
                session.fid
            ))));
        }
        Err(e) => {
            error!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Generate query embedding
    let query_embedding = match state.embedding_service.generate(&req.message).await {
        Ok(emb) => emb,
        Err(e) => {
            error!("Embedding generation failed: {}", e);
            return Ok(Json(ApiResponse::error("Failed to process question")));
        }
    };

    // Search for relevant casts
    let search_limit = (session.context_limit * 5).max(100);
    let search_results = match state
        .database
        .semantic_search_casts_simple(query_embedding, search_limit as i64, Some(0.3))
        .await
    {
        Ok(results) => results,
        Err(e) => {
            error!("Vector search failed: {}", e);
            return Ok(Json(ApiResponse::error("Failed to search context")));
        }
    };

    // Filter to this user and prioritize substantial content
    let mut user_casts: Vec<_> = search_results
        .into_iter()
        .filter(|r| r.fid == session.fid)
        .collect();

    // Calculate current timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Sort by: relevance * substance * recency
    user_casts.sort_by(|a, b| {
        // Recency factor: newer posts (< 30 days) = 1.0, older (> 1 year) = 0.5
        let age_a_days = ((now - a.timestamp) as f32) / 86400.0;
        let age_b_days = ((now - b.timestamp) as f32) / 86400.0;
        let recency_a = (1.0 - (age_a_days / 365.0).min(0.5)).max(0.5);
        let recency_b = (1.0 - (age_b_days / 365.0).min(0.5)).max(0.5);

        // Combined score: similarity * substance * recency
        let score_a = a.similarity * (a.text.len() as f32).ln().max(1.0) * recency_a;
        let score_b = b.similarity * (b.text.len() as f32).ln().max(1.0) * recency_b;

        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    user_casts.truncate(session.context_limit);

    // Build context
    let context = build_chat_context(&profile, &user_casts, &session, &req.message);

    // Generate response
    let response_text = match state
        .llm_service
        .generate_with_params(&context, session.temperature, 2000)
        .await
    {
        Ok(text) => text,
        Err(e) => {
            error!("LLM generation failed: {}", e);
            return Ok(Json(ApiResponse::error("Failed to generate response")));
        }
    };

    // Add to conversation history
    session.add_message("user", req.message.clone());
    session.add_message("assistant", response_text.clone());

    // Update session
    state.session_manager.update_session(session.clone());

    Ok(Json(ApiResponse::success(ChatMessageResponse {
        session_id: session.session_id,
        message: response_text,
        relevant_casts_count: user_casts.len(),
        conversation_length: session.conversation_history.len(),
    })))
}

/// Get session information
pub async fn get_chat_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ApiResponse<SessionInfoResponse>>, StatusCode> {
    info!("GET /api/chat/session/{}", session_id);

    match state.session_manager.get_session(&session_id) {
        Some(session) => Ok(Json(ApiResponse::success(SessionInfoResponse {
            session_id: session.session_id,
            fid: session.fid,
            username: session.username.clone(),
            display_name: session.display_name.clone(),
            conversation_history: session.conversation_history.clone(),
            created_at: session.created_at,
            last_activity: session.last_activity,
        }))),
        None => Ok(Json(ApiResponse::error("Session not found or expired"))),
    }
}

/// Delete chat session
pub async fn delete_chat_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("DELETE /api/chat/session/{}", session_id);

    state.session_manager.delete_session(&session_id);
    Ok(Json(ApiResponse::success("Session deleted".to_string())))
}
