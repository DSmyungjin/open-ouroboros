//! Core type definitions for the cache system

use serde::{Deserialize, Serialize};
use std::fmt;

/// Cache key type - currently string-based, can be extended to support embeddings
pub type CacheKey = String;

/// Cache value type - stores serialized context chunks
pub type CacheValue = String;

/// Hash type for content-based addressing
pub type ContentHash = [u8; 32];

/// Statistics and metrics for cache performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,

    /// Total number of cache misses
    pub misses: u64,

    /// Number of entries currently in cache
    pub entries: usize,

    /// Total size of cached data in bytes
    pub size_bytes: usize,

    /// Number of evictions due to size limits
    pub evictions_size: u64,

    /// Number of evictions due to TTL expiration
    pub evictions_ttl: u64,

    /// Number of manual invalidations
    pub invalidations: u64,

    /// Average cache entry size in bytes
    pub avg_entry_size: usize,
}

impl CacheStats {
    /// Calculate cache hit rate as a percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }

    /// Calculate miss rate as a percentage
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }

    /// Calculate total evictions
    pub fn total_evictions(&self) -> u64 {
        self.evictions_size + self.evictions_ttl
    }

    /// Calculate cache efficiency score (0-100)
    /// Based on hit rate and eviction rate
    pub fn efficiency_score(&self) -> f64 {
        let hit_rate = self.hit_rate();
        let eviction_penalty = if self.entries > 0 {
            (self.total_evictions() as f64 / (self.entries as f64 + self.total_evictions() as f64)) * 20.0
        } else {
            0.0
        };
        (hit_rate - eviction_penalty).max(0.0)
    }
}

impl fmt::Display for CacheStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CacheStats {{ hits: {}, misses: {}, hit_rate: {:.2}%, entries: {}, size: {} bytes, evictions: {} }}",
            self.hits,
            self.misses,
            self.hit_rate(),
            self.entries,
            self.size_bytes,
            self.total_evictions()
        )
    }
}

/// Cache layer type for multi-layer caching architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheLayer {
    /// Layer 1: Exact string match cache
    ExactMatch,

    /// Layer 2: Semantic similarity cache (embedding-based)
    Semantic,

    /// Layer 3: Provider-level cache integration
    Provider,
}

impl fmt::Display for CacheLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheLayer::ExactMatch => write!(f, "exact_match"),
            CacheLayer::Semantic => write!(f, "semantic"),
            CacheLayer::Provider => write!(f, "provider"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 80;
        stats.misses = 20;

        assert_eq!(stats.hit_rate(), 80.0);
        assert_eq!(stats.miss_rate(), 20.0);
    }

    #[test]
    fn test_cache_stats_zero_requests() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.miss_rate(), 100.0);
    }

    #[test]
    fn test_cache_stats_efficiency_score() {
        let mut stats = CacheStats::default();
        stats.hits = 90;
        stats.misses = 10;
        stats.entries = 100;
        stats.evictions_size = 5;
        stats.evictions_ttl = 5;

        let score = stats.efficiency_score();
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_cache_stats_display() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            entries: 75,
            size_bytes: 1024,
            evictions_size: 10,
            evictions_ttl: 5,
            invalidations: 3,
            avg_entry_size: 13,
        };

        let display = format!("{}", stats);
        assert!(display.contains("hits: 100"));
        assert!(display.contains("misses: 50"));
    }

    #[test]
    fn test_cache_layer_display() {
        assert_eq!(format!("{}", CacheLayer::ExactMatch), "exact_match");
        assert_eq!(format!("{}", CacheLayer::Semantic), "semantic");
        assert_eq!(format!("{}", CacheLayer::Provider), "provider");
    }
}
