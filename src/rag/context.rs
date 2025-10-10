//! Context assembly from retrieved documents

use crate::rag::SearchResult;
use std::collections::HashMap;

/// Assembler for creating context from search results
pub struct ContextAssembler {
    max_context_length: usize,
}

impl ContextAssembler {
    /// Create a new context assembler
    pub fn new(max_context_length: usize) -> Self {
        Self {
            max_context_length,
        }
    }

    /// Assemble context from search results
    pub fn assemble(&self, results: &[SearchResult]) -> String {
        let mut context = String::new();
        let mut total_length = 0;

        for (idx, result) in results.iter().enumerate() {
            let profile_text = self.format_profile(&result.profile);
            let entry = format!("\n[Profile {}]\n{}\n", idx + 1, profile_text);

            if total_length + entry.len() > self.max_context_length {
                break;
            }

            context.push_str(&entry);
            total_length += entry.len();
        }

        context
    }

    /// Assemble context with metadata
    pub fn assemble_with_metadata(&self, results: &[SearchResult]) -> (String, Vec<HashMap<String, String>>) {
        let mut context = String::new();
        let mut metadata = Vec::new();
        let mut total_length = 0;

        for (idx, result) in results.iter().enumerate() {
            let profile_text = self.format_profile(&result.profile);
            let entry = format!("\n[Profile {}]\n{}\n", idx + 1, profile_text);

            if total_length + entry.len() > self.max_context_length {
                break;
            }

            context.push_str(&entry);
            total_length += entry.len();

            // Add metadata
            let mut meta = HashMap::new();
            meta.insert("fid".to_string(), result.profile.fid.to_string());
            meta.insert("username".to_string(), result.profile.username.clone().unwrap_or_default());
            meta.insert("score".to_string(), result.score.to_string());
            meta.insert("match_type".to_string(), format!("{:?}", result.match_type));
            metadata.push(meta);
        }

        (context, metadata)
    }

    /// Format a single profile for context
    fn format_profile(&self, profile: &crate::models::UserProfile) -> String {
        let mut parts = Vec::new();

        if let Some(username) = &profile.username {
            parts.push(format!("Username: {}", username));
        }
        if let Some(display_name) = &profile.display_name {
            parts.push(format!("Display Name: {}", display_name));
        }
        if let Some(bio) = &profile.bio {
            parts.push(format!("Bio: {}", bio));
        }
        if let Some(location) = &profile.location {
            parts.push(format!("Location: {}", location));
        }
        if let Some(twitter) = &profile.twitter_username {
            parts.push(format!("Twitter: {}", twitter));
        }
        if let Some(github) = &profile.github_username {
            parts.push(format!("GitHub: {}", github));
        }

        parts.join("\n")
    }

    /// Create a summary of the retrieved profiles
    pub fn create_summary(&self, results: &[SearchResult]) -> String {
        if results.is_empty() {
            return "No profiles found.".to_string();
        }

        let mut summary = format!("Found {} relevant profile(s):\n\n", results.len());

        for (idx, result) in results.iter().enumerate().take(5) {
            let username = result
                .profile
                .username
                .as_deref()
                .unwrap_or("unknown");
            let display_name = result
                .profile
                .display_name
                .as_deref()
                .unwrap_or("No name");
            let bio_preview = result
                .profile
                .bio
                .as_deref()
                .map(|b| {
                    if b.len() > 100 {
                        format!("{}...", &b[..100])
                    } else {
                        b.to_string()
                    }
                })
                .unwrap_or_default();

            summary.push_str(&format!(
                "{}. @{} ({}) - Score: {:.2}\n   {}\n\n",
                idx + 1,
                username,
                display_name,
                result.score,
                bio_preview
            ));
        }

        summary
    }
}

impl Default for ContextAssembler {
    fn default() -> Self {
        Self::new(4000) // Default max context length
    }
}

