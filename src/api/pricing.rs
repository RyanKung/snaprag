//! API pricing configuration for x402 payments

#[cfg(feature = "payment")]
use std::str::FromStr;

#[cfg(feature = "payment")]
use rust_decimal::Decimal;

/// Pricing tiers for different API endpoints
#[derive(Debug, Clone)]
pub struct PricingConfig {
    /// Free endpoints (no payment required)
    pub free_endpoints: Vec<String>,
    /// Basic tier pricing ($0.001)
    pub basic_endpoints: Vec<String>,
    /// Premium tier pricing ($0.01)
    pub premium_endpoints: Vec<String>,
    /// Enterprise tier pricing ($0.1)
    pub enterprise_endpoints: Vec<String>,
}

impl Default for PricingConfig {
    /// Create default pricing configuration
    /// NOTE: Paths should be WITHOUT /api prefix (middleware receives nested paths)
    fn default() -> Self {
        Self {
            free_endpoints: vec![
                "/health".to_string(),
                "/stats".to_string(),
                "/".to_string(),          // MCP root
                "/resources".to_string(), // MCP resources
                "/tools".to_string(),     // MCP tools list
            ],
            basic_endpoints: vec!["/profiles".to_string(), "/profiles/:fid".to_string()],
            premium_endpoints: vec![
                "/search/profiles".to_string(),
                "/search/casts".to_string(),
                "/tools/call".to_string(), // MCP tool calls
            ],
            enterprise_endpoints: vec!["/rag/query".to_string()],
        }
    }
}

impl PricingConfig {
    /// Get price for a specific endpoint path
    #[cfg(feature = "payment")]
    #[must_use]
    pub fn get_price(&self, path: &str) -> Option<Decimal> {
        // Normalize path - remove /api prefix if present
        let normalized_path = path.strip_prefix("/api").unwrap_or(path);

        // Exact match first for better performance
        // Check if it's a free endpoint
        if self.free_endpoints.contains(&normalized_path.to_string()) {
            return None;
        }

        // Check enterprise tier (exact match or pattern)
        if self
            .enterprise_endpoints
            .contains(&normalized_path.to_string())
            || self
                .enterprise_endpoints
                .iter()
                .any(|p| normalized_path.starts_with(p))
        {
            return Some(Decimal::from_str("0.1").unwrap()); // $0.1
        }

        // Check premium tier
        if self
            .premium_endpoints
            .contains(&normalized_path.to_string())
            || self
                .premium_endpoints
                .iter()
                .any(|p| normalized_path.starts_with(p))
        {
            return Some(Decimal::from_str("0.01").unwrap()); // $0.01
        }

        // Check basic tier
        if self.basic_endpoints.contains(&normalized_path.to_string())
            || self
                .basic_endpoints
                .iter()
                .any(|p| self.matches_pattern(p, normalized_path))
        {
            return Some(Decimal::from_str("0.001").unwrap()); // $0.001
        }

        // Default: no payment required (be conservative)
        None
    }

    /// Check if a path matches a pattern (supports :param placeholders)
    fn matches_pattern(&self, pattern: &str, path: &str) -> bool {
        // Normalize paths - remove /api prefix if present
        let normalized_pattern = pattern.strip_prefix("/api").unwrap_or(pattern);
        let normalized_path = path.strip_prefix("/api").unwrap_or(path);

        let pattern_parts: Vec<&str> = normalized_pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let path_parts: Vec<&str> = normalized_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if pattern_parts.len() != path_parts.len() {
            // Also check if pattern is a prefix (for wildcard matching)
            return normalized_path.starts_with(normalized_pattern)
                || normalized_pattern.starts_with(normalized_path);
        }

        pattern_parts
            .iter()
            .zip(path_parts.iter())
            .all(|(p, t)| p.starts_with(':') || p == t)
    }

    /// Get description for an endpoint
    #[must_use]
    pub fn get_description(&self, path: &str) -> String {
        match path {
            p if p.contains("/health") => "Health check endpoint".to_string(),
            p if p.contains("/stats") => "Statistics endpoint".to_string(),
            p if p.contains("/profiles") && !p.contains("search") => {
                "User profiles query".to_string()
            }
            p if p.contains("/search/profiles") => "Semantic profile search".to_string(),
            p if p.contains("/search/casts") => "Semantic cast search".to_string(),
            p if p.contains("/rag/query") => "RAG query with LLM generation".to_string(),
            p if p.contains("/mcp") => "MCP protocol endpoint".to_string(),
            _ => "API endpoint".to_string(),
        }
    }
}

#[cfg(feature = "payment")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_config() {
        let pricing = PricingConfig::default();

        // Free endpoints (paths without /api prefix)
        assert_eq!(pricing.get_price("/health"), None);
        assert_eq!(pricing.get_price("/stats"), None);
        assert_eq!(pricing.get_price("/"), None);

        // Basic tier - $0.0001 (100 atomic units)
        assert_eq!(
            pricing.get_price("/profiles"),
            Some(Decimal::from_str("0.000100").unwrap())
        );
        assert_eq!(
            pricing.get_price("/profiles/123"),
            Some(Decimal::from_str("0.000100").unwrap())
        );

        // Premium tier - $0.001 (1000 atomic units)
        assert_eq!(
            pricing.get_price("/search/profiles"),
            Some(Decimal::from_str("0.001000").unwrap())
        );
        assert_eq!(
            pricing.get_price("/search/casts"),
            Some(Decimal::from_str("0.001000").unwrap())
        );
        assert_eq!(
            pricing.get_price("/tools/call"),
            Some(Decimal::from_str("0.001000").unwrap())
        );

        // Enterprise tier - $0.01 (10000 atomic units)
        assert_eq!(
            pricing.get_price("/rag/query"),
            Some(Decimal::from_str("0.010000").unwrap())
        );
    }

    #[test]
    fn test_path_matching() {
        let pricing = PricingConfig::default();

        // Test pattern matching with :param placeholders
        assert!(pricing.matches_pattern("/profiles/:fid", "/profiles/123"));

        // Test exact matches
        assert!(pricing.matches_pattern("/health", "/health"));

        // Test non-matches
        assert!(!pricing.matches_pattern("/profiles", "/search"));

        // Test prefix matching
        assert!(pricing.matches_pattern("/search/profiles", "/search/profiles/query"));
    }
}
