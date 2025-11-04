//! Personality analysis module - MBTI inference from user behavior
//!
//! This module analyzes user's social behavior, communication patterns,
//! and content to infer their MBTI personality type.
//!
//! MBTI Dimensions:
//! - E/I: Extraversion vs Introversion
//! - S/N: Sensing vs Intuition
//! - T/F: Thinking vs Feeling
//! - J/P: Judging vs Perceiving

use std::collections::HashMap;
use std::sync::Arc;

use lazy_static::lazy_static;
use serde::Deserialize;
use serde::Serialize;

use crate::database::Database;
use crate::llm::LlmService;
use crate::social_graph::SocialProfile;
use crate::Result;

// Load AFINN sentiment lexicon at compile time
const AFINN_LEXICON: &str = include_str!("../data/afinn.txt");

lazy_static! {
    /// AFINN sentiment scores (-5 to +5)
    static ref AFINN_SCORES: HashMap<String, i8> = {
        let mut map = HashMap::new();
        for line in AFINN_LEXICON.lines() {
            if let Some((word, score_str)) = line.split_once('\t') {
                if let Ok(score) = score_str.trim().parse::<i8>() {
                    map.insert(word.to_lowercase(), score);
                }
            }
        }
        map
    };
}

/// MBTI personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbtiProfile {
    pub fid: i64,
    pub mbti_type: String,          // e.g., "INTJ", "ENFP"
    pub confidence: f32,            // 0.0-1.0 confidence score
    pub dimensions: MbtiDimensions, // Individual dimension scores
    pub traits: Vec<String>,        // Key personality traits
    pub analysis: String,           // Detailed analysis
}

/// MBTI dimension scores (0.0 = first letter, 1.0 = second letter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbtiDimensions {
    pub ei_score: f32,      // 0.0 = E (Extravert), 1.0 = I (Introvert)
    pub sn_score: f32,      // 0.0 = S (Sensing), 1.0 = N (Intuition)
    pub tf_score: f32,      // 0.0 = T (Thinking), 1.0 = F (Feeling)
    pub jp_score: f32,      // 0.0 = J (Judging), 1.0 = P (Perceiving)
    pub ei_confidence: f32, // Confidence for E/I dimension
    pub sn_confidence: f32,
    pub tf_confidence: f32,
    pub jp_confidence: f32,
}

/// Behavioral indicators extracted from user data
#[derive(Debug, Clone)]
struct BehavioralIndicators {
    // Social behavior
    social_activity_level: f32, // Posts + interactions frequency
    network_size_ratio: f32,    // Followers/following ratio
    interaction_frequency: f32, // Reply + mention rate
    response_to_others: f32,    // How often they engage with others

    // Communication style
    abstract_word_ratio: f32,  // Abstract vs concrete language
    emotional_word_ratio: f32, // Emotional vs logical language
    question_frequency: f32,   // How often they ask questions
    statement_frequency: f32,  // Definitive statements vs open-ended

    // Content patterns
    future_oriented: f32,    // Future vs present focus
    structured_content: f32, // Organized vs spontaneous
    personal_sharing: f32,   // Personal stories vs general topics

    // Topic preferences
    tech_abstract_score: f32,     // Abstract tech concepts
    practical_content_score: f32, // How-to, practical advice
    tech_discussion_score: f32,   // Technical discussion participation
}

/// MBTI analyzer
pub struct MbtiAnalyzer {
    database: Arc<Database>,
    llm_service: Option<Arc<LlmService>>,
}

impl MbtiAnalyzer {
    /// Create a new MBTI analyzer
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            llm_service: None,
        }
    }

    /// Create with LLM service for enhanced analysis
    pub const fn with_llm(database: Arc<Database>, llm_service: Arc<LlmService>) -> Self {
        Self {
            database,
            llm_service: Some(llm_service),
        }
    }

    /// Analyze user's MBTI personality type
    pub async fn analyze_mbti(
        &self,
        fid: i64,
        social_profile: Option<&SocialProfile>,
    ) -> Result<MbtiProfile> {
        // Get social profile if not provided
        let social_profile_owned;
        let social_profile = if let Some(profile) = social_profile {
            profile
        } else {
            let analyzer = crate::social_graph::SocialGraphAnalyzer::new(self.database.clone());
            social_profile_owned = analyzer.analyze_user(fid).await?;
            &social_profile_owned
        };

        // Extract behavioral indicators
        let indicators = self
            .extract_behavioral_indicators(fid, social_profile)
            .await?;

        // Calculate MBTI dimensions using rule-based approach
        let dimensions = self.calculate_dimensions(&indicators);

        // Determine MBTI type from dimensions
        let mbti_type = self.dimensions_to_type(&dimensions);

        // Calculate overall confidence
        let confidence = (dimensions.ei_confidence
            + dimensions.sn_confidence
            + dimensions.tf_confidence
            + dimensions.jp_confidence)
            / 4.0;

        // Get personality traits
        let traits = self.get_traits_for_type(&mbti_type);

        // Generate detailed analysis using LLM (if available)
        let analysis = if let Some(llm) = &self.llm_service {
            self.generate_llm_analysis(fid, social_profile, &dimensions, &mbti_type, llm)
                .await?
        } else {
            self.generate_rule_based_analysis(social_profile, &dimensions, &mbti_type)
        };

        Ok(MbtiProfile {
            fid,
            mbti_type,
            confidence,
            dimensions,
            traits,
            analysis,
        })
    }

    /// Extract behavioral indicators from user data
    async fn extract_behavioral_indicators(
        &self,
        fid: i64,
        social_profile: &SocialProfile,
    ) -> Result<BehavioralIndicators> {
        // Get recent casts for text analysis
        let casts = self
            .database
            .get_casts_by_fid(fid, Some(100), Some(0))
            .await?;

        // Filter out bot/automated messages
        let filtered_casts: Vec<_> = casts
            .iter()
            .filter(|cast| !is_bot_message(cast.text.as_deref()))
            .collect();

        let total_casts = filtered_casts.len() as f32;
        if total_casts == 0.0 {
            // Return neutral indicators if no data
            return Ok(BehavioralIndicators {
                social_activity_level: 0.5,
                network_size_ratio: 0.5,
                interaction_frequency: 0.5,
                response_to_others: 0.5,
                abstract_word_ratio: 0.5,
                emotional_word_ratio: 0.5,
                question_frequency: 0.5,
                statement_frequency: 0.5,
                future_oriented: 0.5,
                structured_content: 0.5,
                personal_sharing: 0.5,
                tech_abstract_score: 0.5,
                practical_content_score: 0.5,
                tech_discussion_score: 0.5,
            });
        }

        // Combine all text for analysis (using filtered casts)
        let all_text: String = filtered_casts
            .iter()
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect::<Vec<String>>()
            .join(" ")
            .to_lowercase();

        // Social behavior indicators
        let social_activity_level = (total_casts / 100.0).min(1.0); // Normalized to 0-1

        let network_size_ratio = if social_profile.following_count > 0 {
            (social_profile.followers_count as f32 / social_profile.following_count as f32).min(2.0)
                / 2.0 // Normalize to 0-1
        } else {
            0.5
        };

        // Weighted interaction frequency (降低提及率权重)
        // Reply is more genuine social behavior than mentions (which might be for exposure)
        let interaction_frequency = (social_profile.interaction_style.reply_frequency * 0.8
            + social_profile.interaction_style.mention_frequency * 0.2)
            / 1.0;

        let response_to_others = social_profile.interaction_style.reply_frequency;

        // Technical discussion participation (新增技术讨论指标)
        let tech_discussion_score = calculate_tech_discussion_score(&all_text);

        // Communication style analysis
        let abstract_words = [
            "concept",
            "idea",
            "theory",
            "imagine",
            "possible",
            "potential",
            "vision",
            "abstract",
            "pattern",
            "meaning",
            "future",
            "innovation",
            "creative",
        ];
        let abstract_count = count_keywords(&all_text, &abstract_words);

        let concrete_words = [
            "how", "what", "when", "where", "specific", "detail", "fact", "data", "number", "step",
            "process", "example", "real", "actual", "practice",
        ];
        let concrete_count = count_keywords(&all_text, &concrete_words);

        let abstract_word_ratio = if abstract_count + concrete_count > 0 {
            abstract_count as f32 / (abstract_count + concrete_count) as f32
        } else {
            0.5
        };

        // Emotional vs logical language (using AFINN + custom technical words)
        let (emotional_count, logical_count) = calculate_emotional_vs_logical(&all_text);

        let emotional_word_ratio = if emotional_count + logical_count > 0 {
            emotional_count as f32 / (emotional_count + logical_count) as f32
        } else {
            0.5
        };

        // Questions vs statements
        let question_count = all_text.matches('?').count();
        let statement_indicators = all_text.matches('.').count() + all_text.matches('!').count();
        let question_frequency = if question_count + statement_indicators > 0 {
            question_count as f32 / (question_count + statement_indicators) as f32
        } else {
            0.5
        };

        let definitive_words = ["must", "should", "will", "always", "never", "definitely"];
        let definitive_count = count_keywords(&all_text, &definitive_words);
        let statement_frequency = (definitive_count as f32 / total_casts).min(1.0);

        // Future vs present orientation
        let future_words = [
            "will", "going to", "plan", "future", "tomorrow", "next", "soon",
        ];
        let future_count = count_keywords(&all_text, &future_words);

        let present_words = ["now", "today", "currently", "right now", "at the moment"];
        let present_count = count_keywords(&all_text, &present_words);

        let future_oriented = if future_count + present_count > 0 {
            future_count as f32 / (future_count + present_count) as f32
        } else {
            0.5
        };

        // Structured vs spontaneous
        let structure_words = ["first", "second", "step", "plan", "organize", "schedule"];
        let structure_count = count_keywords(&all_text, &structure_words);
        let structured_content = (structure_count as f32 / total_casts).min(1.0);

        // Personal sharing
        let personal_words = ["i", "me", "my", "myself", "i'm"];
        let personal_count = count_keywords(&all_text, &personal_words);
        let personal_sharing =
            (personal_count as f32 / all_text.split_whitespace().count() as f32).min(0.2) / 0.2; // Normalize (assuming 20% is very high)

        // Topic analysis
        let tech_abstract_score = social_profile.social_circles.tech_builders / 100.0;
        let practical_content_score = if social_profile.word_cloud.top_words.is_empty() {
            0.5
        } else {
            // Check for practical/how-to keywords in top words
            let practical_keywords = ["how", "guide", "tutorial", "tip", "use"];
            let practical_in_top_words = social_profile
                .word_cloud
                .top_words
                .iter()
                .filter(|w| practical_keywords.iter().any(|k| w.word.contains(k)))
                .count();
            (practical_in_top_words as f32 / 5.0).min(1.0)
        };

        Ok(BehavioralIndicators {
            social_activity_level,
            network_size_ratio,
            interaction_frequency,
            response_to_others,
            abstract_word_ratio,
            emotional_word_ratio,
            question_frequency,
            statement_frequency,
            future_oriented,
            structured_content,
            personal_sharing,
            tech_abstract_score,
            practical_content_score,
            tech_discussion_score,
        })
    }

    /// Calculate MBTI dimensions from behavioral indicators
    fn calculate_dimensions(&self, indicators: &BehavioralIndicators) -> MbtiDimensions {
        // E/I: Extraversion vs Introversion
        // High social activity, large network, frequent interaction = Extravert (0.0)
        // Low social activity, selective network = Introvert (1.0)
        //
        // Weighted components (adjusted for genuine social behavior):
        // - Reply behavior is more genuine than mentions (mentions might be for exposure)
        // - Technical deep discussion indicates introversion (deep thinking, focused analysis)
        let ei_components = vec![
            (1.0 - indicators.social_activity_level, 1.0), // Weight: 1.0 (activity)
            (1.0 - indicators.interaction_frequency, 1.0), // Weight: 1.0 (weighted reply+mention)
            (1.0 - indicators.response_to_others, 1.0),    // Weight: 1.0 (reply rate)
            (indicators.tech_discussion_score, 1.2), // Weight: 1.2 (tech discussion → I) ⭐ 不反转！
        ];

        let ei_score = weighted_average(&ei_components);
        let ei_confidence = calculate_confidence(
            &ei_components
                .iter()
                .map(|(score, _)| *score)
                .collect::<Vec<_>>(),
        );

        // S/N: Sensing vs Intuition
        // Concrete, practical, present-focused = Sensing (0.0)
        // Abstract, future-focused, conceptual = Intuition (1.0)
        let sn_components = vec![
            indicators.abstract_word_ratio,
            indicators.future_oriented,
            indicators.tech_abstract_score,
            1.0 - indicators.practical_content_score,
        ];
        let sn_score = sn_components.iter().sum::<f32>() / sn_components.len() as f32;
        let sn_confidence = calculate_confidence(&sn_components);

        // T/F: Thinking vs Feeling
        // Logical, objective, analytical = Thinking (0.0)
        // Emotional, subjective, empathetic = Feeling (1.0)
        //
        // Weighted components (adjusted for technical users):
        // - Technical discussion indicates logical thinking (T)
        // - Emotional words indicate feeling (F)
        // - Personal sharing in tech context might just be "I wrote code" (not emotional)
        let tf_components = vec![
            (indicators.emotional_word_ratio, 1.5), // Weight: 1.5 (emotional words → F)
            (indicators.personal_sharing, 0.5),     // Weight: 0.5 (降低，技术人也用"我") ⭐
            (1.0 - indicators.tech_discussion_score, 1.0), // Weight: 1.0 (tech → T) ⭐
        ];
        let tf_score = weighted_average(&tf_components);
        let tf_confidence = calculate_confidence(
            &tf_components
                .iter()
                .map(|(score, _)| *score)
                .collect::<Vec<_>>(),
        );

        // J/P: Judging vs Perceiving
        // Organized, decisive, planned = Judging (0.0)
        // Flexible, exploratory, spontaneous = Perceiving (1.0)
        let jp_components = vec![
            1.0 - indicators.structured_content,
            indicators.question_frequency,
            1.0 - indicators.statement_frequency,
        ];
        let jp_score = jp_components.iter().sum::<f32>() / jp_components.len() as f32;
        let jp_confidence = calculate_confidence(&jp_components);

        MbtiDimensions {
            ei_score,
            sn_score,
            tf_score,
            jp_score,
            ei_confidence,
            sn_confidence,
            tf_confidence,
            jp_confidence,
        }
    }

    /// Convert dimension scores to MBTI type
    fn dimensions_to_type(&self, dimensions: &MbtiDimensions) -> String {
        let e_or_i = if dimensions.ei_score < 0.5 { "E" } else { "I" };
        let s_or_n = if dimensions.sn_score < 0.5 { "S" } else { "N" };
        let t_or_f = if dimensions.tf_score < 0.5 { "T" } else { "F" };
        let j_or_p = if dimensions.jp_score < 0.5 { "J" } else { "P" };

        format!("{e_or_i}{s_or_n}{t_or_f}{j_or_p}")
    }

    /// Get personality traits for a given MBTI type
    fn get_traits_for_type(&self, mbti_type: &str) -> Vec<String> {
        let traits = match mbti_type {
            "INTJ" => vec![
                "Strategic thinker",
                "Independent",
                "Analytical",
                "Innovative",
                "Future-focused",
            ],
            "INTP" => vec![
                "Logical",
                "Curious",
                "Theoretical",
                "Problem solver",
                "Analytical",
            ],
            "ENTJ" => vec![
                "Natural leader",
                "Strategic",
                "Decisive",
                "Efficient",
                "Goal-oriented",
            ],
            "ENTP" => vec![
                "Innovative",
                "Entrepreneurial",
                "Debater",
                "Quick thinker",
                "Versatile",
            ],
            "INFJ" => vec![
                "Insightful",
                "Idealistic",
                "Compassionate",
                "Creative",
                "Purposeful",
            ],
            "INFP" => vec![
                "Idealistic",
                "Empathetic",
                "Creative",
                "Open-minded",
                "Value-driven",
            ],
            "ENFJ" => vec![
                "Charismatic",
                "Inspiring",
                "Empathetic",
                "Organized",
                "Persuasive",
            ],
            "ENFP" => vec![
                "Enthusiastic",
                "Creative",
                "Sociable",
                "Spontaneous",
                "Optimistic",
            ],
            "ISTJ" => vec![
                "Responsible",
                "Organized",
                "Practical",
                "Detail-oriented",
                "Reliable",
            ],
            "ISFJ" => vec![
                "Caring",
                "Loyal",
                "Practical",
                "Detail-oriented",
                "Supportive",
            ],
            "ESTJ" => vec![
                "Organized",
                "Practical",
                "Direct",
                "Efficient",
                "Traditional",
            ],
            "ESFJ" => vec!["Caring", "Social", "Organized", "Cooperative", "Supportive"],
            "ISTP" => vec![
                "Practical",
                "Hands-on",
                "Logical",
                "Adaptable",
                "Problem solver",
            ],
            "ISFP" => vec![
                "Artistic",
                "Gentle",
                "Flexible",
                "Sensitive",
                "Present-focused",
            ],
            "ESTP" => vec![
                "Energetic",
                "Action-oriented",
                "Pragmatic",
                "Sociable",
                "Risk-taker",
            ],
            "ESFP" => vec![
                "Outgoing",
                "Spontaneous",
                "Playful",
                "Enthusiastic",
                "People-focused",
            ],
            _ => vec!["Unique", "Individual"],
        };

        traits.into_iter().map(String::from).collect()
    }

    /// Generate detailed analysis using LLM
    async fn generate_llm_analysis(
        &self,
        fid: i64,
        social_profile: &SocialProfile,
        dimensions: &MbtiDimensions,
        mbti_type: &str,
        llm: &LlmService,
    ) -> Result<String> {
        // Get sample casts for context
        let casts = self
            .database
            .get_casts_by_fid(fid, Some(10), Some(0))
            .await?;

        let sample_casts = casts
            .iter()
            .filter_map(|c| c.text.as_ref())
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n- ");

        let prompt = format!(
            r"You are an expert MBTI personality analyst. Based on the following user behavior data,
provide a detailed MBTI personality analysis.

MBTI Type: {type}

Dimension Scores (0.0-1.0 scale):
- E/I: {ei:.2} (Extraversion ← → Introversion)
- S/N: {sn:.2} (Sensing ← → Intuition)
- T/F: {tf:.2} (Thinking ← → Feeling)  
- J/P: {jp:.2} (Judging ← → Perceiving)

Social Profile:
- Following: {following} | Followers: {followers} | Influence: {influence:.1}x
- Community Role: {role}
- Reply Frequency: {reply:.0}% | Mention Frequency: {mention:.0}%

Top Words: {words}

Sample Posts:
- {posts}

Provide a 3-4 paragraph analysis that:
1. Explains how this {type} type manifests in their online behavior
2. Highlights their communication style and social patterns
3. Discusses their likely strengths and potential blind spots
4. Provides insights on how they engage with their community

Be specific, insightful, and connect observations to MBTI theory.",
            type = mbti_type,
            ei = dimensions.ei_score,
            sn = dimensions.sn_score,
            tf = dimensions.tf_score,
            jp = dimensions.jp_score,
            following = social_profile.following_count,
            followers = social_profile.followers_count,
            influence = social_profile.influence_score,
            role = social_profile.interaction_style.community_role,
            reply = social_profile.interaction_style.reply_frequency * 100.0,
            mention = social_profile.interaction_style.mention_frequency * 100.0,
            words = social_profile
                .word_cloud
                .top_words
                .iter()
                .take(10)
                .map(|w| w.word.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            posts = if sample_casts.is_empty() {
                "No recent posts available"
            } else {
                &sample_casts
            }
        );

        llm.generate(&prompt).await
    }

    /// Generate rule-based analysis without LLM
    fn generate_rule_based_analysis(
        &self,
        social_profile: &SocialProfile,
        dimensions: &MbtiDimensions,
        mbti_type: &str,
    ) -> String {
        let mut analysis = String::new();

        analysis.push_str(&format!(
            "Based on behavioral analysis, this user exhibits characteristics of an {} personality type.\n\n",
            mbti_type
        ));

        // E/I analysis
        if dimensions.ei_score < 0.3 {
            analysis.push_str(
                "Strong Extraversion (E): Highly active socially, frequently engages with others, \
                and maintains broad social networks. Energized by social interaction.\n\n",
            );
        } else if dimensions.ei_score > 0.7 {
            analysis.push_str(
                "Strong Introversion (I): More selective in social engagement, prefers depth over breadth \
                in connections. Reflective communication style.\n\n",
            );
        } else {
            analysis.push_str(
                "Balanced E/I: Shows both extraverted and introverted tendencies depending on context.\n\n",
            );
        }

        // S/N analysis
        if dimensions.sn_score < 0.3 {
            analysis.push_str(
                "Strong Sensing (S): Focuses on concrete, practical information. Present-oriented with \
                attention to details and real-world applications.\n\n",
            );
        } else if dimensions.sn_score > 0.7 {
            analysis.push_str(
                "Strong Intuition (N): Abstract thinker focused on possibilities and future potential. \
                Interested in patterns, concepts, and innovation.\n\n",
            );
        }

        // T/F analysis
        if dimensions.tf_score < 0.3 {
            analysis.push_str(
                "Strong Thinking (T): Logical and analytical approach. Focuses on objective criteria \
                and systematic problem-solving.\n\n",
            );
        } else if dimensions.tf_score > 0.7 {
            analysis.push_str(
                "Strong Feeling (F): Values-driven decision making. Empathetic and considerate of \
                personal impact in communications.\n\n",
            );
        }

        // J/P analysis
        if dimensions.jp_score < 0.3 {
            analysis.push_str(
                "Strong Judging (J): Organized and decisive. Prefers structure and planning. \
                Makes definitive statements and clear conclusions.\n\n",
            );
        } else if dimensions.jp_score > 0.7 {
            analysis.push_str(
                "Strong Perceiving (P): Flexible and exploratory. Open to new information. \
                Asks questions and adapts spontaneously.\n\n",
            );
        }

        analysis.push_str(&format!(
            "Community role: {}. This aligns with typical {} behavior patterns in online communities.",
            social_profile.interaction_style.community_role, mbti_type
        ));

        analysis
    }
}

/// Helper function to count keywords in text
fn count_keywords(text: &str, keywords: &[&str]) -> usize {
    keywords
        .iter()
        .map(|keyword| text.matches(keyword).count())
        .sum()
}

/// Calculate weighted average from (value, weight) pairs
fn weighted_average(components: &[(f32, f32)]) -> f32 {
    let total_weight: f32 = components.iter().map(|(_, w)| w).sum();
    let weighted_sum: f32 = components.iter().map(|(v, w)| v * w).sum();
    weighted_sum / total_weight
}

/// Calculate confidence score from component scores
fn calculate_confidence(components: &[f32]) -> f32 {
    // Confidence is higher when components agree (low variance)
    let mean = components.iter().sum::<f32>() / components.len() as f32;
    let variance =
        components.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / components.len() as f32;

    // Convert variance to confidence (lower variance = higher confidence)
    // Max variance is 0.25 (when half components are 0, half are 1)
    let normalized_variance = variance / 0.25;
    (1.0 - normalized_variance).max(0.3).min(1.0) // Min 30% confidence
}

/// Calculate technical discussion participation score
fn calculate_tech_discussion_score(text: &str) -> f32 {
    // Technical discussion keywords that indicate genuine engagement
    let tech_discussion_keywords = [
        // Technical concepts
        "api",
        "protocol",
        "blockchain",
        "smart contract",
        "encryption",
        "algorithm",
        "optimization",
        "architecture",
        "implementation",
        "debugging",
        "testing",
        "deployment",
        "infrastructure",
        // Problem solving
        "bug",
        "issue",
        "fix",
        "solution",
        "problem",
        "error",
        "exception",
        "workaround",
        "patch",
        "resolve",
        "investigate",
        // Code/Development
        "code",
        "function",
        "class",
        "method",
        "library",
        "framework",
        "repository",
        "commit",
        "pull request",
        "merge",
        "branch",
        // Questions and discussions
        "why",
        "how does",
        "what if",
        "anyone know",
        "has anyone",
        "thoughts on",
        "opinions on",
        "feedback on",
        // Technical reasoning
        "because",
        "therefore",
        "however",
        "although",
        "considering",
        "based on",
        "according to",
        "in my experience",
    ];

    let keyword_count = tech_discussion_keywords
        .iter()
        .map(|keyword| text.matches(keyword).count())
        .sum::<usize>();

    // Normalize: assume 10+ tech keywords = high tech discussion (1.0)
    (keyword_count as f32 / 10.0).min(1.0)
}

/// Calculate emotional vs logical word counts using AFINN lexicon
fn calculate_emotional_vs_logical(text: &str) -> (usize, usize) {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut emotional_count = 0;
    let mut logical_count = 0;

    // Technical/logical keywords (expanded list)
    let logical_keywords = [
        "because",
        "therefore",
        "analyze",
        "logic",
        "rational",
        "reason",
        "evidence",
        "prove",
        "fact",
        "objective",
        "efficient",
        "optimize",
        "system",
        "algorithm",
        "performance",
        "benchmark",
        "measurement",
        "metric",
        "data",
        "statistics",
        "implementation",
        "architecture",
        "design",
        "structure",
        "debug",
        "test",
        "validate",
        "compute",
    ];

    for word in words {
        let word_lower = word.to_lowercase();

        // Check AFINN lexicon for emotional words
        if let Some(&score) = AFINN_SCORES.get(&word_lower) {
            // AFINN scores: -5 to +5
            // Emotional words typically have strong scores (abs >= 2)
            if score.abs() >= 2 {
                emotional_count += 1;
            }
        }

        // Check technical/logical keywords
        if logical_keywords.iter().any(|&kw| word_lower.contains(kw)) {
            logical_count += 1;
        }
    }

    (emotional_count, logical_count)
}

/// Check if a cast is likely a bot/automated message
fn is_bot_message(text: Option<&str>) -> bool {
    let Some(text) = text else {
        return false;
    };

    let text_lower = text.to_lowercase();

    // Bot message patterns to filter out
    let bot_patterns = [
        "ms!t",                                 // microsub bot marker
        "i'm supporting you through /microsub", // microsub support messages
        "please mute the keyword \"ms!t\"",     // microsub mute instruction
        "$degen",                               // degen tip bot (when standalone)
        "minted",                               // NFT mint notifications (alone)
        "you've been tipped",                   // tip notifications
        "airdrop claim",                        // airdrop spam
        "congratulations! you won",             // spam/scam
        "click here to claim",                  // spam/scam
        "limited time offer",                   // spam
        "visit this link",                      // spam
    ];

    // Check for exact bot patterns
    for pattern in &bot_patterns {
        if text_lower.contains(pattern) {
            return true;
        }
    }

    // Additional heuristic: very short automated messages
    // Skip if it's just a tip/support notification
    if text.len() < 50 && text_lower.contains("$degen") && text_lower.contains("supporting") {
        return true;
    }

    // Filter out pure emoji posts without meaningful text (likely automated reactions)
    let has_meaningful_text = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .count()
        > 10;

    if !has_meaningful_text && text.len() < 20 {
        return true; // Likely automated emoji spam
    }

    false
}
