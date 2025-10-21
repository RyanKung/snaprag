//! Session management for interactive chat

use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use dashmap::DashMap;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Chat message in conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: u64,
}

/// Chat session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub session_id: String,
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub conversation_history: Vec<ChatMessage>,
    pub created_at: u64,
    pub last_activity: u64,
    pub context_limit: usize,
    pub temperature: f32,
}

impl ChatSession {
    #[must_use] 
    pub fn new(
        fid: i64,
        username: Option<String>,
        display_name: Option<String>,
        context_limit: usize,
        temperature: f32,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            session_id: Uuid::new_v4().to_string(),
            fid,
            username,
            display_name,
            conversation_history: Vec::new(),
            created_at: now,
            last_activity: now,
            context_limit,
            temperature,
        }
    }

    pub fn add_message(&mut self, role: &str, content: String) {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.conversation_history.push(ChatMessage {
            role: role.to_string(),
            content,
            timestamp,
        });

        self.last_activity = timestamp;

        // Keep only last 10 exchanges (20 messages) to avoid context overflow
        if self.conversation_history.len() > 20 {
            self.conversation_history.drain(0..2);
        }
    }

    #[must_use] 
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - self.last_activity > timeout_secs
    }
}

/// Session manager with automatic cleanup
pub struct SessionManager {
    sessions: Arc<DashMap<String, ChatSession>>,
    session_timeout: Duration,
}

impl SessionManager {
    #[must_use] 
    pub fn new(session_timeout_secs: u64) -> Self {
        let sessions = Arc::new(DashMap::new());
        let session_timeout = Duration::from_secs(session_timeout_secs);

        // Start cleanup task
        let sessions_clone = sessions.clone();
        let timeout_secs = session_timeout_secs;
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Self::cleanup_expired_sessions(&sessions_clone, timeout_secs);
            }
        });

        Self {
            sessions,
            session_timeout,
        }
    }

    #[must_use] 
    pub fn create_session(
        &self,
        fid: i64,
        username: Option<String>,
        display_name: Option<String>,
        context_limit: usize,
        temperature: f32,
    ) -> ChatSession {
        let session = ChatSession::new(fid, username, display_name, context_limit, temperature);
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        session
    }

    #[must_use] 
    pub fn get_session(&self, session_id: &str) -> Option<ChatSession> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub fn update_session(&self, session: ChatSession) {
        self.sessions.insert(session.session_id.clone(), session);
    }

    pub fn delete_session(&self, session_id: &str) {
        self.sessions.remove(session_id);
    }

    #[must_use] 
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    fn cleanup_expired_sessions(sessions: &DashMap<String, ChatSession>, timeout_secs: u64) {
        let expired: Vec<String> = sessions
            .iter()
            .filter(|entry| entry.value().is_expired(timeout_secs))
            .map(|entry| entry.key().clone())
            .collect();

        for session_id in expired {
            sessions.remove(&session_id);
            tracing::info!("Cleaned up expired session: {}", session_id);
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(3600) // 1 hour timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = ChatSession::new(
            99,
            Some("jesse.base.eth".to_string()),
            Some("Jesse Pollak".to_string()),
            20,
            0.7,
        );

        assert_eq!(session.fid, 99);
        assert_eq!(session.conversation_history.len(), 0);
        assert!(session.session_id.len() > 0);
    }

    #[test]
    fn test_add_message() {
        let mut session = ChatSession::new(99, None, None, 20, 0.7);

        session.add_message("user", "Hello".to_string());
        session.add_message("assistant", "Hi there!".to_string());

        assert_eq!(session.conversation_history.len(), 2);
        assert_eq!(session.conversation_history[0].role, "user");
        assert_eq!(session.conversation_history[1].role, "assistant");
    }

    #[test]
    fn test_message_limit() {
        let mut session = ChatSession::new(99, None, None, 20, 0.7);

        // Add 25 messages
        for i in 0..25 {
            session.add_message("user", format!("Message {}", i));
        }

        // Should only keep last 20
        assert_eq!(session.conversation_history.len(), 20);
    }
}
