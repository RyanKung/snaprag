//! API request handlers

use std::sync::Arc;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tracing::error;
use tracing::info;

use crate::api::types::*;
use crate::config::AppConfig;
use crate::database::Database;
use crate::embeddings::EmbeddingService;
use crate::llm::LlmService;
use crate::models::UserProfileQuery;
use crate::rag::CastRetriever;
use crate::rag::RagQuery;
use crate::rag::RagService;
use crate::rag::RetrievalMethod;
use crate::rag::Retriever;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub database: Arc<Database>,
    pub embedding_service: Arc<EmbeddingService>,
    pub llm_service: Arc<LlmService>,
    pub lazy_loader: Option<Arc<crate::sync::LazyLoader>>,
    pub session_manager: Arc<crate::api::session::SessionManager>,
}

/// Health check handler
pub async fn health() -> Json<ApiResponse<HealthResponse>> {
    Json(ApiResponse::success(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Get profile by FID (with automatic lazy loading)
pub async fn get_profile(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
) -> Result<Json<ApiResponse<ProfileResponse>>, StatusCode> {
    info!("GET /api/profiles/{}", fid);

    // Try database first
    let profile = match state.database.get_user_profile(fid).await {
        Ok(Some(p)) => Some(p),
        Ok(None) => {
            // Try lazy loading if available
            if let Some(loader) = &state.lazy_loader {
                info!("⚡ Profile {} not found, attempting lazy load", fid);
                match loader.fetch_user_profile(fid as u64).await {
                    Ok(p) => {
                        info!("✅ Successfully lazy loaded profile {}", fid);
                        Some(p)
                    }
                    Err(e) => {
                        info!("Failed to lazy load profile {}: {}", fid, e);
                        None
                    }
                }
            } else {
                None
            }
        }
        Err(e) => {
            error!("Error fetching profile: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    match profile {
        Some(profile) => Ok(Json(ApiResponse::success(ProfileResponse {
            fid: profile.fid,
            username: profile.username,
            display_name: profile.display_name,
            bio: profile.bio,
            pfp_url: profile.pfp_url,
            location: profile.location,
            twitter_username: profile.twitter_username,
            github_username: profile.github_username,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// List profiles
pub async fn list_profiles(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<ProfileResponse>>>, StatusCode> {
    info!("GET /api/profiles?q={}&limit={}", params.q, params.limit);

    let query = UserProfileQuery {
        fid: None,
        username: None,
        display_name: None,
        bio: None,
        location: None,
        twitter_username: None,
        github_username: None,
        limit: Some(params.limit as i64),
        offset: None,
        start_timestamp: None,
        end_timestamp: None,
        sort_by: None,
        sort_order: None,
        search_term: if params.q.is_empty() {
            None
        } else {
            Some(params.q)
        },
    };

    match state.database.list_user_profiles(query).await {
        Ok(profiles) => {
            let response: Vec<ProfileResponse> = profiles
                .into_iter()
                .map(|p| ProfileResponse {
                    fid: p.fid,
                    username: p.username,
                    display_name: p.display_name,
                    bio: p.bio,
                    pfp_url: p.pfp_url,
                    location: p.location,
                    twitter_username: p.twitter_username,
                    github_username: p.github_username,
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error listing profiles: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search profiles semantically
pub async fn search_profiles(
    State(state): State<AppState>,
    Json(req): Json<ProfileSearchRequest>,
) -> Result<Json<ApiResponse<Vec<ProfileResponse>>>, StatusCode> {
    info!("POST /api/search/profiles: {}", req.query);

    let retriever = Retriever::new(state.database.clone(), state.embedding_service.clone());

    let results = match req.method.as_deref() {
        Some("semantic") => retriever.semantic_search(&req.query, req.limit, None).await,
        Some("keyword") => retriever.keyword_search(&req.query, req.limit).await,
        Some("hybrid") => retriever.hybrid_search(&req.query, req.limit).await,
        _ => retriever.auto_search(&req.query, req.limit).await,
    };

    match results {
        Ok(search_results) => {
            let response: Vec<ProfileResponse> = search_results
                .into_iter()
                .map(|r| ProfileResponse {
                    fid: r.profile.fid,
                    username: r.profile.username,
                    display_name: r.profile.display_name,
                    bio: r.profile.bio,
                    pfp_url: r.profile.pfp_url,
                    location: r.profile.location,
                    twitter_username: r.profile.twitter_username,
                    github_username: r.profile.github_username,
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error searching profiles: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Search casts semantically
pub async fn search_casts(
    State(state): State<AppState>,
    Json(req): Json<CastSearchRequest>,
) -> Result<Json<ApiResponse<Vec<CastResponse>>>, StatusCode> {
    info!("POST /api/search/casts: {}", req.query);

    let retriever = CastRetriever::new(state.database.clone(), state.embedding_service.clone());

    match retriever
        .semantic_search(&req.query, req.limit, Some(req.threshold))
        .await
    {
        Ok(results) => {
            let response: Vec<CastResponse> = results
                .into_iter()
                .map(|r| CastResponse {
                    message_hash: hex::encode(&r.message_hash),
                    fid: r.fid,
                    text: r.text,
                    timestamp: r.timestamp,
                    similarity: Some(r.similarity),
                })
                .collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Error searching casts: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// RAG query
pub async fn rag_query(
    State(state): State<AppState>,
    Json(req): Json<RagQueryRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("POST /api/rag/query: {}", req.question);

    let rag_service = RagService::from_services(
        state.database.clone(),
        state.embedding_service.clone(),
        (*state.llm_service).clone(),
    );

    let method = match req.method.as_deref() {
        Some("semantic") => RetrievalMethod::Semantic,
        Some("keyword") => RetrievalMethod::Keyword,
        Some("hybrid") => RetrievalMethod::Hybrid,
        _ => RetrievalMethod::Auto,
    };

    let query = RagQuery {
        question: req.question,
        retrieval_limit: req.retrieval_limit,
        retrieval_method: method,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
    };

    match rag_service.query_with_options(query).await {
        Ok(response) => Ok(Json(ApiResponse::success(response.answer))),
        Err(e) => {
            error!("Error processing RAG query: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Fetch user on-demand (POST /api/fetch/user/:fid)
pub async fn fetch_user(
    State(state): State<AppState>,
    Path(fid): Path<i64>,
    Json(req): Json<FetchUserRequest>,
) -> Result<Json<ApiResponse<FetchResponse>>, StatusCode> {
    info!(
        "POST /api/fetch/user/{} (with_casts={}, embeddings={})",
        fid, req.with_casts, req.generate_embeddings
    );

    let Some(ref loader) = state.lazy_loader else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    // Check if in database (to determine source)
    let in_db = state
        .database
        .get_user_profile(fid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    // Fetch profile (smart)
    let profile = match loader.get_user_profile_smart(fid).await {
        Ok(Some(p)) => p,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Error fetching profile: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Fetch casts if requested
    let casts = if req.with_casts {
        loader
            .get_user_casts_smart(fid)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        Vec::new()
    };

    // Generate embeddings if requested
    let mut embeddings_generated = None;
    if req.generate_embeddings && !casts.is_empty() {
        let embedding_service = if let Some(ref endpoint_name) = req.embedding_endpoint {
            // Use specified endpoint
            match state.database.pool().acquire().await {
                Ok(_) => {
                    // Create embedding service with specified endpoint
                    Arc::new(
                        crate::embeddings::EmbeddingService::new(
                            &crate::config::AppConfig::load()
                                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                        )
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    )
                }
                Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        } else {
            state.embedding_service.clone()
        };

        let mut success = 0;
        for cast in &casts {
            if let Some(ref text) = cast.text {
                if !text.trim().is_empty() {
                    if let Ok(embedding) = embedding_service.generate(text).await {
                        let _ = state
                            .database
                            .store_cast_embedding(&cast.message_hash, cast.fid, text, &embedding)
                            .await;
                        success += 1;
                    }
                }
            }
        }
        embeddings_generated = Some(success);
    }

    Ok(Json(ApiResponse::success(FetchResponse {
        profile: ProfileResponse {
            fid: profile.fid,
            username: profile.username,
            display_name: profile.display_name,
            bio: profile.bio,
            pfp_url: profile.pfp_url,
            location: profile.location,
            twitter_username: profile.twitter_username,
            github_username: profile.github_username,
        },
        casts_count: casts.len(),
        embeddings_generated,
        source: if in_db {
            "database".to_string()
        } else {
            "snapchain".to_string()
        },
    })))
}

/// Fetch multiple users batch (POST /api/fetch/users)
pub async fn fetch_users_batch(
    State(state): State<AppState>,
    Json(req): Json<FetchUsersBatchRequest>,
) -> Result<Json<ApiResponse<Vec<FetchResponse>>>, StatusCode> {
    info!("POST /api/fetch/users ({} fids)", req.fids.len());

    let Some(ref loader) = state.lazy_loader else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let mut results = Vec::new();

    for fid in req.fids {
        // Fetch profile
        let profile = match loader.get_user_profile_smart(fid as i64).await {
            Ok(Some(p)) => p,
            Ok(None) => continue,
            Err(_) => continue,
        };

        // Fetch casts if requested
        let casts = if req.with_casts {
            loader
                .get_user_casts_smart(fid as i64)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        results.push(FetchResponse {
            profile: ProfileResponse {
                fid: profile.fid,
                username: profile.username,
                display_name: profile.display_name,
                bio: profile.bio,
                pfp_url: profile.pfp_url,
                location: profile.location,
                twitter_username: profile.twitter_username,
                github_username: profile.github_username,
            },
            casts_count: casts.len(),
            embeddings_generated: None,
            source: "mixed".to_string(),
        });
    }

    Ok(Json(ApiResponse::success(results)))
}

/// Get statistics
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatsResponse>>, StatusCode> {
    info!("GET /api/stats");

    // Get basic counts
    let total_profiles = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM user_profiles")
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

    let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts")
        .fetch_one(state.database.pool())
        .await
        .unwrap_or(0);

    let profiles_with_embeddings = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM user_profiles WHERE profile_embedding IS NOT NULL",
    )
    .fetch_one(state.database.pool())
    .await
    .unwrap_or(0);

    let casts_with_embeddings =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cast_embeddings")
            .fetch_one(state.database.pool())
            .await
            .unwrap_or(0);

    Ok(Json(ApiResponse::success(StatsResponse {
        total_profiles,
        total_casts,
        profiles_with_embeddings,
        casts_with_embeddings,
    })))
}

/// Create a new chat session
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
                "Invalid user identifier: {}",
                e
            ))));
        }
    };

    // Get user profile
    let profile = match state.database.get_user_profile(fid as i64).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return Ok(Json(ApiResponse::error(format!("User {} not found", fid))));
        }
        Err(e) => {
            error!("Database error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Count user's casts
    let casts_count = match state
        .database
        .get_casts_by_fid(fid as i64, Some(1), Some(0))
        .await
    {
        Ok(casts) => {
            // Get actual count
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts WHERE fid = $1")
                .bind(fid as i64)
                .fetch_one(state.database.pool())
                .await
                .unwrap_or(0) as usize
        }
        Err(_) => 0,
    };

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
    Query(req): Query<GetSessionRequest>,
) -> Result<Json<ApiResponse<SessionInfoResponse>>, StatusCode> {
    info!("GET /api/chat/session - session: {}", req.session_id);

    match state.session_manager.get_session(&req.session_id) {
        Some(session) => Ok(Json(ApiResponse::success(SessionInfoResponse {
            session_id: session.session_id,
            fid: session.fid,
            username: session.username,
            display_name: session.display_name,
            conversation_history: session.conversation_history,
            created_at: session.created_at,
            last_activity: session.last_activity,
        }))),
        None => Ok(Json(ApiResponse::error("Session not found or expired"))),
    }
}

/// Delete a chat session
pub async fn delete_chat_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    info!("DELETE /api/chat/session/{}", session_id);

    state.session_manager.delete_session(&session_id);
    Ok(Json(ApiResponse::success(())))
}

/// Parse user identifier (FID or username)
async fn parse_user_identifier(identifier: &str, database: &Database) -> crate::Result<u64> {
    let trimmed = identifier.trim();

    if trimmed.starts_with('@') {
        let username = trimmed.trim_start_matches('@');
        let profile = database
            .get_user_profile_by_username(username)
            .await?
            .ok_or_else(|| {
                crate::SnapRagError::Custom(format!("Username @{} not found", username))
            })?;
        Ok(profile.fid as u64)
    } else {
        trimmed.parse::<u64>().map_err(|_| {
            crate::SnapRagError::Custom(format!("Invalid user identifier: {}", identifier))
        })
    }
}

/// Build chat context from profile, casts, and history
fn build_chat_context(
    profile: &crate::models::UserProfile,
    relevant_casts: &[crate::models::CastSearchResult],
    session: &crate::api::session::ChatSession,
    question: &str,
) -> String {
    let display_name = profile.display_name.as_deref().unwrap_or("Unknown");
    let username = profile.username.as_deref();

    let mut context = String::new();
    context.push_str(&format!("You are {}, a Farcaster user", display_name));
    if let Some(username) = username {
        context.push_str(&format!(" (@{})", username));
    }
    context.push_str(&format!(". FID: {}.\n\n", profile.fid));

    if let Some(bio) = &profile.bio {
        context.push_str(&format!("Bio: {}\n\n", bio));
    }

    // Add relevant casts as style reference
    if !relevant_casts.is_empty() {
        let avg_length: usize = relevant_casts.iter().map(|c| c.text.len()).sum::<usize>()
            / relevant_casts.len().max(1);

        context.push_str("===== YOUR ACTUAL POSTS =====\n\n");
        for (idx, result) in relevant_casts.iter().take(15).enumerate() {
            context.push_str(&format!("{}. {}\n", idx + 1, result.text));
        }
        context.push_str(&format!("\nAverage length: {} chars\n\n", avg_length));

        context.push_str("===== CRITICAL STYLE RULES =====\n");
        if avg_length < 80 {
            context.push_str(
                "⚠️ You write VERY SHORT posts. KEEP YOUR ANSWER BRIEF (under 100 chars)!\n",
            );
            context.push_str("Examples: '5-10 words', 'one short sentence', 'super concise'.\n");
        } else if avg_length < 150 {
            context.push_str("You're concise. Keep answers to 1-2 sentences.\n");
        } else {
            context.push_str("You write detailed posts. 2-3 sentences is fine.\n");
        }
        context.push_str("MATCH the examples: same length, same emoji usage, same energy.\n\n");
    }

    // Add conversation history
    if !session.conversation_history.is_empty() {
        context.push_str("Previous conversation:\n\n");
        for msg in &session.conversation_history {
            let role_label = if msg.role == "user" { "User" } else { "You" };
            context.push_str(&format!("{}: {}\n", role_label, msg.content));
        }
        context.push_str("\n");
    }

    context.push_str(&format!("User: {}\n\nYou:", question));

    context
}
