//! API caching layer for profile and social data
//!
//! This module provides in-memory caching for expensive API operations
//! like profile lookups and social graph analysis.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::api::types::ProfileResponse;
use crate::social_graph::SocialProfile;

/// Cache entry with TTL support
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Default TTL for profile cache entries
    pub profile_ttl: Duration,
    /// Default TTL for social analysis cache entries  
    pub social_ttl: Duration,
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Enable cache statistics
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            profile_ttl: Duration::from_secs(3600), // 1 hour default
            social_ttl: Duration::from_secs(3600),  // 1 hour default
            max_entries: 10000,
            enable_stats: true,
        }
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expired_cleanups: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// In-memory cache service for API responses
pub struct CacheService {
    profile_cache: Arc<RwLock<HashMap<i64, CacheEntry<ProfileResponse>>>>,
    social_cache: Arc<RwLock<HashMap<i64, CacheEntry<SocialProfile>>>>,
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl CacheService {
    /// Create a new cache service with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new cache service with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            profile_cache: Arc::new(RwLock::new(HashMap::new())),
            social_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get cached profile by FID
    pub async fn get_profile(&self, fid: i64) -> Option<ProfileResponse> {
        let mut cache = self.profile_cache.write().await;

        if let Some(entry) = cache.get(&fid) {
            if entry.is_expired() {
                cache.remove(&fid);
                self.increment_miss().await;
                tracing::debug!("Profile cache miss (expired) for FID {}", fid);
                return None;
            }

            self.increment_hit().await;
            tracing::debug!("Profile cache hit for FID {}", fid);
            return Some(entry.data.clone());
        }

        self.increment_miss().await;
        tracing::debug!("Profile cache miss for FID {}", fid);
        None
    }

    /// Cache a profile response
    pub async fn set_profile(&self, fid: i64, profile: ProfileResponse) {
        let mut cache = self.profile_cache.write().await;

        // Check if we need to evict entries
        if cache.len() >= self.config.max_entries {
            self.evict_oldest_entries(&mut cache).await;
        }

        let entry = CacheEntry::new(profile, self.config.profile_ttl);
        cache.insert(fid, entry);
        tracing::debug!("Cached profile for FID {}", fid);
    }

    /// Get cached social analysis by FID
    pub async fn get_social(&self, fid: i64) -> Option<SocialProfile> {
        let mut cache = self.social_cache.write().await;

        if let Some(entry) = cache.get(&fid) {
            if entry.is_expired() {
                cache.remove(&fid);
                self.increment_miss().await;
                tracing::debug!("Social cache miss (expired) for FID {}", fid);
                return None;
            }

            self.increment_hit().await;
            tracing::debug!("Social cache hit for FID {}", fid);
            return Some(entry.data.clone());
        }

        self.increment_miss().await;
        tracing::debug!("Social cache miss for FID {}", fid);
        None
    }

    /// Cache a social analysis response
    pub async fn set_social(&self, fid: i64, social: SocialProfile) {
        let mut cache = self.social_cache.write().await;

        // Check if we need to evict entries
        if cache.len() >= self.config.max_entries {
            self.evict_oldest_entries(&mut cache).await;
        }

        let entry = CacheEntry::new(social, self.config.social_ttl);
        cache.insert(fid, entry);
        tracing::debug!("Cached social analysis for FID {}", fid);
    }

    /// Invalidate cached profile for a FID
    pub async fn invalidate_profile(&self, fid: i64) {
        let mut cache = self.profile_cache.write().await;
        cache.remove(&fid);
        debug!("Invalidated profile cache for FID {}", fid);
    }

    /// Invalidate cached social analysis for a FID
    pub async fn invalidate_social(&self, fid: i64) {
        let mut cache = self.social_cache.write().await;
        cache.remove(&fid);
        debug!("Invalidated social cache for FID {}", fid);
    }

    /// Invalidate all caches for a FID
    pub async fn invalidate_user(&self, fid: i64) {
        self.invalidate_profile(fid).await;
        self.invalidate_social(fid).await;
        debug!("Invalidated all caches for FID {}", fid);
    }

    /// Clear all cached profiles
    pub async fn clear_profiles(&self) {
        let mut cache = self.profile_cache.write().await;
        cache.clear();
        info!("Cleared all profile cache entries");
    }

    /// Clear all cached social analyses
    pub async fn clear_social(&self) {
        let mut cache = self.social_cache.write().await;
        cache.clear();
        info!("Cleared all social cache entries");
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        self.clear_profiles().await;
        self.clear_social().await;
        info!("Cleared all cache entries");
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            evictions: stats.evictions,
            expired_cleanups: stats.expired_cleanups,
        }
    }

    /// Get cache size information
    pub async fn get_cache_info(&self) -> CacheInfo {
        let profile_cache = self.profile_cache.read().await;
        let social_cache = self.social_cache.read().await;

        CacheInfo {
            profile_entries: profile_cache.len(),
            social_entries: social_cache.len(),
            total_entries: profile_cache.len() + social_cache.len(),
            max_entries: self.config.max_entries,
        }
    }

    /// Clean up expired entries from all caches
    pub async fn cleanup_expired(&self) {
        let mut profile_cache = self.profile_cache.write().await;
        let mut social_cache = self.social_cache.write().await;

        let mut profile_removed = 0;
        let mut social_removed = 0;

        // Clean up profile cache
        profile_cache.retain(|_, entry| {
            if entry.is_expired() {
                profile_removed += 1;
                false
            } else {
                true
            }
        });

        // Clean up social cache
        social_cache.retain(|_, entry| {
            if entry.is_expired() {
                social_removed += 1;
                false
            } else {
                true
            }
        });

        if profile_removed > 0 || social_removed > 0 {
            let mut stats = self.stats.write().await;
            stats.expired_cleanups += profile_removed + social_removed;
            debug!(
                "Cleaned up {} expired cache entries",
                profile_removed + social_removed
            );
        }
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) {
        let cache_service = Arc::new(self.clone());

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Cleanup every 5 minutes

            loop {
                interval.tick().await;
                cache_service.cleanup_expired().await;
            }
        });
    }

    // Private helper methods

    async fn increment_hit(&self) {
        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.hits += 1;
        }
    }

    async fn increment_miss(&self) {
        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.misses += 1;
        }
    }

    async fn evict_oldest_entries<T>(&self, cache: &mut HashMap<i64, CacheEntry<T>>) {
        // Simple eviction: remove 10% of entries
        let evict_count = (cache.len() / 10).max(1);
        let keys_to_remove: Vec<i64> = cache.keys().take(evict_count).copied().collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.evictions += evict_count as u64;
        }

        debug!("Evicted {} cache entries", evict_count);
    }
}

impl Clone for CacheService {
    fn clone(&self) -> Self {
        Self {
            profile_cache: self.profile_cache.clone(),
            social_cache: self.social_cache.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Cache information for monitoring
#[derive(Debug)]
pub struct CacheInfo {
    pub profile_entries: usize,
    pub social_entries: usize,
    pub total_entries: usize,
    pub max_entries: usize,
}

impl CacheInfo {
    pub fn usage_percentage(&self) -> f64 {
        if self.max_entries == 0 {
            0.0
        } else {
            self.total_entries as f64 / self.max_entries as f64 * 100.0
        }
    }
}
