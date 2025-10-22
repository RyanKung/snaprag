//! Enhanced prompts for RAG queries

/// Build profile RAG prompt
#[must_use]
pub fn build_profile_rag_prompt(question: &str, context: &str) -> String {
    format!(
        r"You are an expert assistant helping users discover and learn about Farcaster protocol users.

Context: The following are Farcaster user profiles that may be relevant to the question:

{context}

Question: {question}

Instructions:
1. Provide a helpful and accurate answer based on the profiles above
2. If referencing specific users, mention their username or display name
3. If the profiles don't contain relevant information, say so clearly
4. Be concise but informative
5. Focus on the most relevant information

Answer:"
    )
}

/// Build cast RAG prompt
#[must_use]
pub fn build_cast_rag_prompt(question: &str, context: &str) -> String {
    format!(
        r#"You are an expert Farcaster analyst helping users understand discussions, trends, and community sentiment.

Context: The following are relevant Farcaster casts (posts) that may help answer the question:

{context}

Question: {question}

Instructions:
1. Analyze the casts above to provide a comprehensive answer
2. Identify key themes, patterns, and insights
3. Reference specific casts when relevant (e.g., "According to Cast 1...")
4. Highlight any consensus or disagreements in the discussion
5. If the casts don't contain enough information, acknowledge the limitation
6. Be analytical and insightful, not just summarizing
7. Keep your answer concise but substantive

Answer:"#
    )
}

/// Build trend analysis prompt
#[must_use]
pub fn build_trend_analysis_prompt(casts: &str, time_period: &str) -> String {
    format!(
        r"You are a Farcaster trends analyst. Analyze the following casts from {time_period} and identify key trends.

Casts:
{casts}

Task: Provide a comprehensive trend analysis including:
1. Main topics and themes being discussed
2. Emerging patterns or shifts in conversation
3. Notable discussions or debates
4. Key contributors and influencers
5. Overall sentiment and tone
6. Actionable insights

Be data-driven and specific. Use examples from the casts to support your analysis.

Trend Analysis:"
    )
}

/// Build user profiling prompt
#[must_use]
pub fn build_user_profiling_prompt(username: &str, bio: &str, recent_casts: &str) -> String {
    format!(
        r"You are an expert at understanding user personas and community behavior.

User Profile:
- Username: {username}
- Bio: {bio}

Recent Activity:
{recent_casts}

Task: Create a comprehensive profile analysis including:
1. Core interests and areas of expertise
2. Communication style and engagement patterns  
3. Community role and influence level
4. Notable contributions or perspectives
5. Potential collaboration opportunities
6. Overall impression and key takeaways

Be objective and insightful.

Profile Analysis:"
    )
}

/// Build content summarization prompt
#[must_use]
pub fn build_summary_prompt(content: &str, max_length: usize) -> String {
    format!(
        r"Summarize the following Farcaster content concisely.

Content:
{content}

Requirements:
- Maximum {max_length} words
- Capture key points only
- Maintain factual accuracy
- Use clear, direct language

Summary:"
    )
}

/// Build thread context prompt
#[must_use]
pub fn build_thread_context_prompt(thread: &str) -> String {
    format!(
        r"You are analyzing a Farcaster conversation thread.

Thread:
{thread}

Task: Provide a thread summary including:
1. Main topic of discussion
2. Key points from each participant
3. Evolution of the conversation
4. Any conclusions or outcomes
5. Notable insights or perspectives

Thread Analysis:"
    )
}

/// Build comparative analysis prompt
#[must_use]
pub fn build_comparison_prompt(item1: &str, item2: &str, comparison_type: &str) -> String {
    format!(
        r"Compare and contrast the following two {comparison_type}:

Item 1:
{item1}

Item 2:
{item2}

Task: Provide a detailed comparison including:
1. Similarities
2. Differences
3. Strengths and weaknesses
4. Use cases or contexts
5. Recommendations

Comparison Analysis:"
    )
}
