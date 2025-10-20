//! Social graph analysis for user profiling
//!
//! This module analyzes user relationships and interactions to build
//! a comprehensive social profile for better AI understanding.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

use crate::database::Database;
use crate::Result;

/// Social graph profile for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialProfile {
    pub fid: i64,

    // Following/Follower stats
    pub following_count: usize,
    pub followers_count: usize,
    pub influence_score: f32, // followers / following ratio

    // Network analysis
    pub top_followed_users: Vec<UserMention>,
    pub top_followers: Vec<UserMention>,
    pub most_mentioned_users: Vec<UserMention>,

    // Social circle categorization
    pub social_circles: SocialCircles,

    // Interaction patterns
    pub interaction_style: InteractionStyle,
}

/// User mention with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMention {
    pub fid: i64,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub count: usize,
    pub category: String, // "tech", "creator", "web3", etc.
}

/// Social circles breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialCircles {
    pub tech_builders: f32,    // % following tech people
    pub content_creators: f32, // % following creators
    pub web3_natives: f32,     // % following web3 people
    pub casual_users: f32,     // % following casual users
}

/// Interaction style analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionStyle {
    pub reply_frequency: f32,    // How often user replies to others
    pub mention_frequency: f32,  // How often user mentions others
    pub network_connector: bool, // Actively introduces people
    pub community_role: String,  // "leader", "contributor", "observer"
}

/// Social graph analyzer
pub struct SocialGraphAnalyzer {
    database: Arc<Database>,
}

impl SocialGraphAnalyzer {
    /// Create a new social graph analyzer
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Analyze user's social profile
    pub async fn analyze_user(&self, fid: i64) -> Result<SocialProfile> {
        // Get following list
        let following = self.get_following(fid).await?;
        let followers = self.get_followers(fid).await?;

        // Calculate influence score
        let influence_score = if !following.is_empty() {
            followers.len() as f32 / following.len() as f32
        } else {
            0.0
        };

        // Analyze mentions from user's casts (this works even without links data)
        let mentioned_users = self.analyze_mentions(fid).await?;

        // If we have mentioned users, try to categorize them as a proxy for social circles
        let social_circles = if !mentioned_users.is_empty() {
            self.categorize_from_mentions(&mentioned_users)
        } else if !following.is_empty() {
            self.categorize_social_circles(&following).await?
        } else {
            SocialCircles {
                tech_builders: 0.0,
                content_creators: 0.0,
                web3_natives: 0.0,
                casual_users: 0.0,
            }
        };

        // Analyze interaction patterns
        let interaction_style = self.analyze_interaction_style(fid).await?;

        // Get top users in each category
        let top_followed = self.get_top_users(&following, 5).await?;
        let top_followers = self.get_top_users(&followers, 5).await?;

        Ok(SocialProfile {
            fid,
            following_count: following.len(),
            followers_count: followers.len(),
            influence_score,
            top_followed_users: top_followed,
            top_followers: top_followers,
            most_mentioned_users: mentioned_users,
            social_circles,
            interaction_style,
        })
    }

    /// Get list of users this FID follows
    async fn get_following(&self, fid: i64) -> Result<Vec<i64>> {
        let links = sqlx::query_scalar::<_, i64>(
            "SELECT target_fid FROM links WHERE fid = $1 AND link_type = 'follow'",
        )
        .bind(fid)
        .fetch_all(self.database.pool())
        .await?;

        Ok(links)
    }

    /// Get list of users who follow this FID
    async fn get_followers(&self, fid: i64) -> Result<Vec<i64>> {
        let followers = sqlx::query_scalar::<_, i64>(
            "SELECT fid FROM links WHERE target_fid = $1 AND link_type = 'follow'",
        )
        .bind(fid)
        .fetch_all(self.database.pool())
        .await?;

        Ok(followers)
    }

    /// Analyze users mentioned in casts
    async fn analyze_mentions(&self, fid: i64) -> Result<Vec<UserMention>> {
        // Get casts with mentions
        let casts = sqlx::query!(
            r#"
            SELECT mentions
            FROM casts
            WHERE fid = $1 AND mentions IS NOT NULL
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
            fid
        )
        .fetch_all(self.database.pool())
        .await?;

        // Count mention frequency
        let mut mention_counts: HashMap<i64, usize> = HashMap::new();

        for cast in casts {
            if let Some(mentions_json) = cast.mentions {
                if let Some(mentions_array) = mentions_json.as_array() {
                    for mention in mentions_array {
                        if let Some(mentioned_fid) = mention.as_i64() {
                            *mention_counts.entry(mentioned_fid).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Sort by frequency and get top 10
        let mut sorted_mentions: Vec<_> = mention_counts.into_iter().collect();
        sorted_mentions.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_mentions.truncate(10);

        // Get user profiles for mentioned users
        let mut result = Vec::new();
        for (mentioned_fid, count) in sorted_mentions {
            let profile = self.database.get_user_profile(mentioned_fid).await?;

            result.push(UserMention {
                fid: mentioned_fid,
                username: profile.as_ref().and_then(|p| p.username.clone()),
                display_name: profile.as_ref().and_then(|p| p.display_name.clone()),
                count,
                category: self.categorize_user(mentioned_fid).await?,
            });
        }

        Ok(result)
    }

    /// Get top users with profiles
    async fn get_top_users(&self, fids: &[i64], limit: usize) -> Result<Vec<UserMention>> {
        let mut result = Vec::new();

        for fid in fids.iter().take(limit) {
            let profile = self.database.get_user_profile(*fid).await?;

            result.push(UserMention {
                fid: *fid,
                username: profile.as_ref().and_then(|p| p.username.clone()),
                display_name: profile.as_ref().and_then(|p| p.display_name.clone()),
                count: 1,
                category: self.categorize_user(*fid).await?,
            });
        }

        Ok(result)
    }

    /// Categorize a user based on their content
    async fn categorize_user(&self, fid: i64) -> Result<String> {
        // Get recent casts to analyze content
        let casts = self
            .database
            .get_casts_by_fid(fid, Some(20), Some(0))
            .await?;

        if casts.is_empty() {
            return Ok("unknown".to_string());
        }

        // Analyze content for keywords
        let all_text: String = casts
            .iter()
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let tech_score = count_keywords(
            &all_text,
            &[
                "build",
                "dev",
                "code",
                "api",
                "protocol",
                "github",
                "rust",
                "typescript",
                "solidity",
                "engineering",
            ],
        );

        let web3_score = count_keywords(
            &all_text,
            &[
                "web3",
                "crypto",
                "nft",
                "blockchain",
                "onchain",
                "eth",
                "base",
                "token",
                "defi",
                "dao",
            ],
        );

        let creator_score = count_keywords(
            &all_text,
            &[
                "art", "design", "create", "music", "writing", "video", "content", "story",
                "creative",
            ],
        );

        // Determine primary category
        let max_score = tech_score.max(web3_score).max(creator_score);

        if max_score == 0 {
            Ok("casual".to_string())
        } else if tech_score == max_score {
            Ok("tech".to_string())
        } else if web3_score == max_score {
            Ok("web3".to_string())
        } else {
            Ok("creator".to_string())
        }
    }

    /// Categorize social circles based on mentioned users
    fn categorize_from_mentions(&self, mentioned_users: &[UserMention]) -> SocialCircles {
        if mentioned_users.is_empty() {
            return SocialCircles {
                tech_builders: 0.0,
                content_creators: 0.0,
                web3_natives: 0.0,
                casual_users: 0.0,
            };
        }

        let total_weight: usize = mentioned_users.iter().map(|u| u.count).sum();

        let mut tech_weight = 0;
        let mut web3_weight = 0;
        let mut creator_weight = 0;
        let mut casual_weight = 0;

        for user in mentioned_users {
            match user.category.as_str() {
                "tech" => tech_weight += user.count,
                "web3" => web3_weight += user.count,
                "creator" => creator_weight += user.count,
                _ => casual_weight += user.count,
            }
        }

        let total = total_weight as f32;

        SocialCircles {
            tech_builders: (tech_weight as f32 / total) * 100.0,
            content_creators: (creator_weight as f32 / total) * 100.0,
            web3_natives: (web3_weight as f32 / total) * 100.0,
            casual_users: (casual_weight as f32 / total) * 100.0,
        }
    }

    /// Categorize user's social circles
    async fn categorize_social_circles(&self, following: &[i64]) -> Result<SocialCircles> {
        if following.is_empty() {
            return Ok(SocialCircles {
                tech_builders: 0.0,
                content_creators: 0.0,
                web3_natives: 0.0,
                casual_users: 0.0,
            });
        }

        let mut tech_count = 0;
        let mut web3_count = 0;
        let mut creator_count = 0;
        let mut casual_count = 0;

        // Sample up to 50 users to avoid too many queries
        let sample_size = following.len().min(50);

        for fid in following.iter().take(sample_size) {
            let category = self.categorize_user(*fid).await?;
            match category.as_str() {
                "tech" => tech_count += 1,
                "web3" => web3_count += 1,
                "creator" => creator_count += 1,
                _ => casual_count += 1,
            }
        }

        let total = sample_size as f32;

        Ok(SocialCircles {
            tech_builders: (tech_count as f32 / total) * 100.0,
            content_creators: (creator_count as f32 / total) * 100.0,
            web3_natives: (web3_count as f32 / total) * 100.0,
            casual_users: (casual_count as f32 / total) * 100.0,
        })
    }

    /// Analyze user's interaction style
    async fn analyze_interaction_style(&self, fid: i64) -> Result<InteractionStyle> {
        // Count replies (casts with parent_hash)
        let total_casts = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM casts WHERE fid = $1")
            .bind(fid)
            .fetch_one(self.database.pool())
            .await?;

        let reply_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM casts WHERE fid = $1 AND parent_hash IS NOT NULL",
        )
        .bind(fid)
        .fetch_one(self.database.pool())
        .await?;

        let mention_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM casts WHERE fid = $1 AND mentions IS NOT NULL",
        )
        .bind(fid)
        .fetch_one(self.database.pool())
        .await?;

        let reply_frequency = if total_casts > 0 {
            reply_count as f32 / total_casts as f32
        } else {
            0.0
        };

        let mention_frequency = if total_casts > 0 {
            mention_count as f32 / total_casts as f32
        } else {
            0.0
        };

        // Determine community role
        let following_count = self.get_following(fid).await?.len();
        let followers_count = self.get_followers(fid).await?.len();

        let community_role = if followers_count > 1000 && reply_frequency > 0.3 {
            "leader".to_string()
        } else if reply_frequency > 0.4 || mention_frequency > 0.3 {
            "contributor".to_string()
        } else if following_count > 100 {
            "observer".to_string()
        } else {
            "casual".to_string()
        };

        let network_connector = mention_frequency > 0.3 && reply_frequency > 0.3;

        Ok(InteractionStyle {
            reply_frequency,
            mention_frequency,
            network_connector,
            community_role,
        })
    }

    /// Format social profile as a human-readable string for LLM context
    pub fn format_for_llm(&self, profile: &SocialProfile) -> String {
        let mut output = String::new();

        output.push_str("═══════════════════════════════════════════════════════\n");
        output.push_str("👥 SOCIAL NETWORK PROFILE\n");
        output.push_str("═══════════════════════════════════════════════════════\n\n");

        // Basic stats
        output.push_str(&format!(
            "Following: {} | Followers: {} | Influence: {:.1}x\n\n",
            profile.following_count, profile.followers_count, profile.influence_score
        ));

        // Social circles
        output.push_str("Social Circle Breakdown:\n");
        if profile.social_circles.tech_builders > 30.0 {
            output.push_str(&format!(
                "  🔧 Tech/Builders: {:.0}% - HEAVY tech network\n",
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
                "  ⛓️ Web3/Crypto: {:.0}% - HEAVY web3 network\n",
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
            output.push_str("Most Frequently Mentioned:\n");
            for (idx, user) in profile.most_mentioned_users.iter().take(3).enumerate() {
                let name = user
                    .username
                    .as_ref()
                    .map(|u| format!("@{}", u))
                    .or_else(|| user.display_name.clone())
                    .unwrap_or_else(|| format!("FID {}", user.fid));

                output.push_str(&format!(
                    "  {}. {} ({}x, {})\n",
                    idx + 1,
                    name,
                    user.count,
                    user.category
                ));
            }
            output.push_str("\n");
        }

        // Interaction style
        output.push_str("Interaction Style:\n");
        output.push_str(&format!(
            "  Role: {} | Reply rate: {:.0}% | Mention rate: {:.0}%\n",
            profile.interaction_style.community_role,
            profile.interaction_style.reply_frequency * 100.0,
            profile.interaction_style.mention_frequency * 100.0
        ));

        if profile.interaction_style.network_connector {
            output.push_str("  🌐 Network Connector - actively introduces people\n");
        }

        output.push_str("\n");

        // Add context instructions
        output.push_str("🎯 Social Context Instructions:\n");

        if profile.influence_score > 2.0 {
            output.push_str("  → Influential user - speak with confidence\n");
        } else if profile.influence_score < 0.5 {
            output.push_str("  → Growing account - show learning mindset\n");
        }

        if profile.social_circles.tech_builders > 40.0 {
            output.push_str("  → Deep in tech circles - use builder language\n");
        }

        if profile.social_circles.web3_natives > 40.0 {
            output.push_str("  → Web3 native - understand crypto culture\n");
        }

        if profile.interaction_style.reply_frequency > 0.4 {
            output.push_str("  → Active conversationalist - engage with questions\n");
        }

        if !profile.most_mentioned_users.is_empty() {
            output.push_str("  → You can reference your network: ");
            let names: Vec<String> = profile
                .most_mentioned_users
                .iter()
                .take(3)
                .filter_map(|u| u.username.as_ref().map(|n| format!("@{}", n)))
                .collect();
            output.push_str(&names.join(", "));
            output.push_str("\n");
        }

        output.push_str("\n═══════════════════════════════════════════════════════\n\n");

        output
    }
}

/// Helper function to count keywords in text
fn count_keywords(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .filter(|keyword| text.contains(*keyword))
        .count()
}
