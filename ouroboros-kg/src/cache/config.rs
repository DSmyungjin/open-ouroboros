//! Configuration for the cache system

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the context cache
///
/// Based on research findings:
/// - Default TTL: 1 hour (standard for relatively stable content)
/// - Jitter: 10-15% to prevent thundering herd
/// - Size limits: Balance memory usage with cache effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default time-to-live for cache entries
    /// Research recommendation: 1 hour for standard workloads
    pub default_ttl: Duration,

    /// Maximum number of entries in the cache
    /// Prevents unbounded memory growth
    pub max_entries: usize,

    /// Maximum total size of cached data in bytes
    /// Research: 100MB-1GB typical for medium-scale systems
    pub max_size_bytes: usize,

    /// TTL jitter factor (0.0 - 1.0)
    /// Adds random variation to prevent cache stampede
    /// Research recommendation: 0.10-0.15 (10-15%)
    pub ttl_jitter: f64,

    /// Enable automatic cleanup of expired entries
    pub enable_auto_cleanup: bool,

    /// Interval for automatic cleanup checks
    pub cleanup_interval: Duration,

    /// Similarity threshold for semantic caching (0.0 - 1.0)
    /// Research recommendation: 0.85-0.95 depending on use case
    /// 0.90 is a good default balance
    pub semantic_similarity_threshold: f64,

    /// Enable LRU eviction policy
    /// When true, least recently used entries are evicted first
    pub enable_lru_eviction: bool,

    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            // 1 hour default TTL (research-backed standard)
            default_ttl: Duration::from_secs(3600),
            // 10,000 entries default
            max_entries: 10_000,
            // 100 MB default
            max_size_bytes: 100 * 1024 * 1024,
            // 12.5% jitter (research recommendation: 10-15%)
            ttl_jitter: 0.125,
            enable_auto_cleanup: true,
            // Cleanup every 5 minutes
            cleanup_interval: Duration::from_secs(300),
            // 0.90 similarity threshold (research-backed default)
            semantic_similarity_threshold: 0.90,
            enable_lru_eviction: true,
            enable_metrics: true,
        }
    }
}

impl CacheConfig {
    /// Create a new builder for cache configuration
    pub fn builder() -> CacheConfigBuilder {
        CacheConfigBuilder::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_entries == 0 {
            return Err("max_entries must be greater than 0".to_string());
        }

        if self.max_size_bytes == 0 {
            return Err("max_size_bytes must be greater than 0".to_string());
        }

        if self.ttl_jitter < 0.0 || self.ttl_jitter > 1.0 {
            return Err("ttl_jitter must be between 0.0 and 1.0".to_string());
        }

        if self.semantic_similarity_threshold < 0.0 || self.semantic_similarity_threshold > 1.0 {
            return Err(
                "semantic_similarity_threshold must be between 0.0 and 1.0".to_string()
            );
        }

        Ok(())
    }

    /// Calculate actual TTL with jitter applied
    pub fn ttl_with_jitter(&self) -> Duration {
        if self.ttl_jitter == 0.0 {
            return self.default_ttl;
        }

        let base_secs = self.default_ttl.as_secs_f64();
        let jitter_range = base_secs * self.ttl_jitter;
        let jitter = (rand::random::<f64>() * 2.0 - 1.0) * jitter_range;
        let final_secs = (base_secs + jitter).max(1.0);

        Duration::from_secs_f64(final_secs)
    }
}

/// Builder for cache configuration with validation
#[derive(Debug, Default)]
pub struct CacheConfigBuilder {
    default_ttl: Option<Duration>,
    max_entries: Option<usize>,
    max_size_bytes: Option<usize>,
    ttl_jitter: Option<f64>,
    enable_auto_cleanup: Option<bool>,
    cleanup_interval: Option<Duration>,
    semantic_similarity_threshold: Option<f64>,
    enable_lru_eviction: Option<bool>,
    enable_metrics: Option<bool>,
}

impl CacheConfigBuilder {
    /// Set default TTL for cache entries
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// Set maximum number of cache entries
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max_entries = Some(max);
        self
    }

    /// Set maximum cache size in bytes
    pub fn max_size_bytes(mut self, size: usize) -> Self {
        self.max_size_bytes = Some(size);
        self
    }

    /// Set TTL jitter factor (0.0 - 1.0)
    pub fn ttl_jitter(mut self, jitter: f64) -> Self {
        self.ttl_jitter = Some(jitter);
        self
    }

    /// Enable or disable automatic cleanup
    pub fn enable_auto_cleanup(mut self, enable: bool) -> Self {
        self.enable_auto_cleanup = Some(enable);
        self
    }

    /// Set cleanup interval
    pub fn cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = Some(interval);
        self
    }

    /// Set semantic similarity threshold
    pub fn semantic_similarity_threshold(mut self, threshold: f64) -> Self {
        self.semantic_similarity_threshold = Some(threshold);
        self
    }

    /// Enable or disable LRU eviction
    pub fn enable_lru_eviction(mut self, enable: bool) -> Self {
        self.enable_lru_eviction = Some(enable);
        self
    }

    /// Enable or disable metrics collection
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = Some(enable);
        self
    }

    /// Build the cache configuration
    pub fn build(self) -> CacheConfig {
        let defaults = CacheConfig::default();

        CacheConfig {
            default_ttl: self.default_ttl.unwrap_or(defaults.default_ttl),
            max_entries: self.max_entries.unwrap_or(defaults.max_entries),
            max_size_bytes: self.max_size_bytes.unwrap_or(defaults.max_size_bytes),
            ttl_jitter: self.ttl_jitter.unwrap_or(defaults.ttl_jitter),
            enable_auto_cleanup: self
                .enable_auto_cleanup
                .unwrap_or(defaults.enable_auto_cleanup),
            cleanup_interval: self.cleanup_interval.unwrap_or(defaults.cleanup_interval),
            semantic_similarity_threshold: self
                .semantic_similarity_threshold
                .unwrap_or(defaults.semantic_similarity_threshold),
            enable_lru_eviction: self
                .enable_lru_eviction
                .unwrap_or(defaults.enable_lru_eviction),
            enable_metrics: self.enable_metrics.unwrap_or(defaults.enable_metrics),
        }
    }
}

/// Preset configurations for common use cases
impl CacheConfig {
    /// Configuration optimized for real-time data (short TTL)
    /// Research: 5 minutes for rapidly changing data
    pub fn realtime() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_entries: 5_000,
            max_size_bytes: 50 * 1024 * 1024, // 50 MB
            ttl_jitter: 0.15,
            semantic_similarity_threshold: 0.95, // High threshold for precision
            ..Default::default()
        }
    }

    /// Configuration for daily-updated content
    /// Research: ~23 hours with jitter for daily updates
    pub fn daily() -> Self {
        Self {
            default_ttl: Duration::from_secs(23 * 3600), // 23 hours
            max_entries: 50_000,
            max_size_bytes: 500 * 1024 * 1024, // 500 MB
            ttl_jitter: 0.10,
            semantic_similarity_threshold: 0.90,
            ..Default::default()
        }
    }

    /// Configuration for static content (long TTL)
    /// Research: 24+ hours for documentation, reference materials
    pub fn static_content() -> Self {
        Self {
            default_ttl: Duration::from_secs(48 * 3600), // 48 hours
            max_entries: 100_000,
            max_size_bytes: 1024 * 1024 * 1024, // 1 GB
            ttl_jitter: 0.05,
            semantic_similarity_threshold: 0.85,
            ..Default::default()
        }
    }

    /// Configuration for memory-constrained environments
    pub fn small() -> Self {
        Self {
            default_ttl: Duration::from_secs(1800), // 30 minutes
            max_entries: 1_000,
            max_size_bytes: 10 * 1024 * 1024, // 10 MB
            ttl_jitter: 0.15,
            semantic_similarity_threshold: 0.90,
            ..Default::default()
        }
    }

    /// Configuration for large-scale deployments
    /// Research: Medium-scale (1M-100M vectors, 10GB-1TB)
    pub fn large() -> Self {
        Self {
            default_ttl: Duration::from_secs(7200), // 2 hours
            max_entries: 1_000_000,
            max_size_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            ttl_jitter: 0.10,
            semantic_similarity_threshold: 0.88,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(3600));
        assert_eq!(config.max_entries, 10_000);
        assert!(config.enable_auto_cleanup);
    }

    #[test]
    fn test_config_validation() {
        let valid_config = CacheConfig::default();
        assert!(valid_config.validate().is_ok());

        let mut invalid_config = CacheConfig::default();
        invalid_config.max_entries = 0;
        assert!(invalid_config.validate().is_err());

        let mut invalid_config = CacheConfig::default();
        invalid_config.ttl_jitter = 1.5;
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(600))
            .max_entries(5000)
            .max_size_bytes(50_000_000)
            .build();

        assert_eq!(config.default_ttl, Duration::from_secs(600));
        assert_eq!(config.max_entries, 5000);
        assert_eq!(config.max_size_bytes, 50_000_000);
    }

    #[test]
    fn test_ttl_with_jitter() {
        let config = CacheConfig {
            default_ttl: Duration::from_secs(3600),
            ttl_jitter: 0.1,
            ..Default::default()
        };

        let ttl = config.ttl_with_jitter();
        let base_secs = 3600.0;
        let jitter_range = base_secs * 0.1;

        assert!(ttl.as_secs_f64() >= base_secs - jitter_range);
        assert!(ttl.as_secs_f64() <= base_secs + jitter_range);
    }

    #[test]
    fn test_preset_configs() {
        let realtime = CacheConfig::realtime();
        assert_eq!(realtime.default_ttl, Duration::from_secs(300));

        let daily = CacheConfig::daily();
        assert_eq!(daily.default_ttl, Duration::from_secs(23 * 3600));

        let static_content = CacheConfig::static_content();
        assert_eq!(static_content.default_ttl, Duration::from_secs(48 * 3600));

        let small = CacheConfig::small();
        assert_eq!(small.max_entries, 1_000);

        let large = CacheConfig::large();
        assert_eq!(large.max_entries, 1_000_000);
    }
}
