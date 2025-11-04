//! Prompt templates for RAG queries

use std::collections::HashMap;

/// Template for generating prompts
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    template: String,
    variables: Vec<String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let variables = extract_variables(&template);
        Self {
            template,
            variables,
        }
    }

    /// Fill in the template with variables
    #[must_use]
    pub fn render(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for var in &self.variables {
            if let Some(value) = values.get(var) {
                result = result.replace(&format!("{{{{{var}}}}}"), value);
            }
        }
        result
    }

    /// Get required variables
    #[must_use]
    pub fn variables(&self) -> &[String] {
        &self.variables
    }
}

/// Extract variable names from template
fn extract_variables(template: &str) -> Vec<String> {
    let mut variables = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // skip second '{'
            let mut var_name = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next();
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        break;
                    }
                } else {
                    var_name.push(ch);
                    chars.next();
                }
            }
            if !var_name.is_empty() && !variables.contains(&var_name) {
                variables.push(var_name);
            }
        }
    }

    variables
}

/// Standard RAG prompt templates
pub struct RagPrompts;

impl RagPrompts {
    /// Profile search prompt
    #[must_use]
    pub fn profile_search() -> PromptTemplate {
        PromptTemplate::new(
            r"You are helping search for Farcaster user profiles. 
            
Based on the following user profiles:

{{profiles}}

User query: {{query}}

Please provide a summary of the most relevant profiles and explain why they match the query.",
        )
    }

    /// Context-based QA prompt
    #[must_use]
    pub fn context_qa() -> PromptTemplate {
        PromptTemplate::new(
            r"You are an expert on the Farcaster protocol and its community.

Context information from the database:
{{context}}

Question: {{question}}

Please provide a detailed and accurate answer based on the context above. If the context doesn't contain enough information to answer the question, please say so.",
        )
    }

    /// Profile summary prompt
    #[must_use]
    pub fn profile_summary() -> PromptTemplate {
        PromptTemplate::new(
            r"Generate a comprehensive summary of the following Farcaster user profile:

Username: {{username}}
Display Name: {{display_name}}
Bio: {{bio}}
Location: {{location}}
Twitter: {{twitter}}
GitHub: {{github}}

Please provide:
1. A brief overview of the user
2. Key interests and focus areas
3. Professional background (if evident)
4. Community connections and influence",
        )
    }

    /// Semantic search query enhancement
    #[must_use]
    pub fn query_enhancement() -> PromptTemplate {
        PromptTemplate::new(
            r"Enhance the following search query to improve semantic matching:

Original query: {{query}}

Please provide:
1. Expanded query with related terms
2. Key concepts to search for
3. Alternative phrasings

Return only the enhanced query text without explanations.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_variables() {
        let template = PromptTemplate::new("Hello {{name}}, you are {{age}} years old.");
        assert_eq!(template.variables(), &["name", "age"]);
    }

    #[test]
    fn test_template_render() {
        let template = PromptTemplate::new("Hello {{name}}!");
        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        assert_eq!(template.render(&values), "Hello Alice!");
    }
}
