//! Cache entry management with TTL support

use crate::cache::types::{CacheKey, CacheValue};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A cache entry with TTL and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cache key
    pub key: CacheKey,

    /// The cached value
    pub value: CacheValue,

    /// Entry metadata
    pub metadata: CacheMetadata,
}

impl CacheEntry {
    /// Create a new cache entry with default TTL
    pub fn new(key: CacheKey, value: CacheValue, ttl: Duration) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::seconds(3600));

        Self {
            key,
            value,
            metadata: CacheMetadata {
                created_at: now,
                accessed_at: now,
                expires_at,
                access_count: 0,
                size_bytes: 0, // Will be calculated on insert
                version: 1,
                tags: Vec::new(),
            },
        }
    }

    /// Create a new cache entry with custom expiration time
    pub fn with_expiration(key: CacheKey, value: CacheValue, expires_at: DateTime<Utc>) -> Self {
        let now = Utc::now();

        Self {
            key,
            value,
            metadata: CacheMetadata {
                created_at: now,
                accessed_at: now,
                expires_at,
                access_count: 0,
                size_bytes: 0,
                version: 1,
                tags: Vec::new(),
            },
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.metadata.expires_at
    }

    /// Get time until expiration
    pub fn time_until_expiration(&self) -> Option<Duration> {
        let now = Utc::now();
        if now > self.metadata.expires_at {
            None
        } else {
            let duration = self.metadata.expires_at - now;
            duration.to_std().ok()
        }
    }

    /// Mark the entry as accessed (updates access time and count)
    pub fn mark_accessed(&mut self) {
        self.metadata.accessed_at = Utc::now();
        self.metadata.access_count += 1;
    }

    /// Get the age of the entry
    pub fn age(&self) -> Duration {
        let now = Utc::now();
        (now - self.metadata.created_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0))
    }

    /// Get time since last access
    pub fn time_since_access(&self) -> Duration {
        let now = Utc::now();
        (now - self.metadata.accessed_at)
            .to_std()
            .unwrap_or(Duration::from_secs(0))
    }

    /// Extend the TTL by a given duration
    pub fn extend_ttl(&mut self, extension: Duration) {
        self.metadata.expires_at = self.metadata.expires_at
            + chrono::Duration::from_std(extension).unwrap_or(chrono::Duration::seconds(0));
    }

    /// Update the value and reset expiration
    pub fn update_value(&mut self, new_value: CacheValue, ttl: Duration) {
        self.value = new_value;
        self.metadata.expires_at = Utc::now()
            + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::seconds(3600));
        self.metadata.version += 1;
    }

    /// Calculate the size of this entry in bytes
    pub fn calculate_size(&self) -> usize {
        // Approximate size: key + value + metadata overhead
        self.key.len() + self.value.len() + std::mem::size_of::<CacheMetadata>()
    }

    /// Add a tag to the entry for categorization
    pub fn add_tag(&mut self, tag: String) {
        if !self.metadata.tags.contains(&tag) {
            self.metadata.tags.push(tag);
        }
    }

    /// Check if entry has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.metadata.tags.iter().any(|t| t == tag)
    }

    /// Remove a tag from the entry
    pub fn remove_tag(&mut self, tag: &str) {
        self.metadata.tags.retain(|t| t != tag);
    }
}

/// Metadata associated with a cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// When the entry was created
    pub created_at: DateTime<Utc>,

    /// Last access time (for LRU tracking)
    pub accessed_at: DateTime<Utc>,

    /// When the entry expires
    pub expires_at: DateTime<Utc>,

    /// Number of times this entry has been accessed
    pub access_count: u64,

    /// Size of the entry in bytes
    pub size_bytes: usize,

    /// Version number (incremented on updates)
    pub version: u64,

    /// Tags for categorization and selective invalidation
    pub tags: Vec<String>,
}

impl CacheMetadata {
    /// Check if metadata indicates a hot entry (frequently accessed)
    pub fn is_hot(&self, threshold: u64) -> bool {
        self.access_count >= threshold
    }

    /// Calculate hotness score (access_count / age_in_hours)
    pub fn hotness_score(&self) -> f64 {
        let age_hours = (Utc::now() - self.created_at)
            .num_seconds()
            .max(1) as f64
            / 3600.0;
        self.access_count as f64 / age_hours
    }

    /// Check if entry is stale (not accessed for a while)
    pub fn is_stale(&self, threshold: Duration) -> bool {
        let time_since_access = Utc::now() - self.accessed_at;
        time_since_access.to_std().unwrap_or(Duration::from_secs(0)) > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_cache_entry_creation() {
        let entry = CacheEntry::new(
            "test_key".to_string(),
            "test_value".to_string(),
            Duration::from_secs(3600),
        );

        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, "test_value");
        assert!(!entry.is_expired());
        assert_eq!(entry.metadata.version, 1);
    }

    #[test]
    fn test_entry_expiration() {
        let entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_millis(100),
        );

        assert!(!entry.is_expired());
        sleep(Duration::from_millis(150));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_mark_accessed() {
        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        let initial_count = entry.metadata.access_count;
        let initial_time = entry.metadata.accessed_at;

        sleep(Duration::from_millis(10));
        entry.mark_accessed();

        assert_eq!(entry.metadata.access_count, initial_count + 1);
        assert!(entry.metadata.accessed_at > initial_time);
    }

    #[test]
    fn test_extend_ttl() {
        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(10),
        );

        let original_expiry = entry.metadata.expires_at;
        entry.extend_ttl(Duration::from_secs(10));

        assert!(entry.metadata.expires_at > original_expiry);
    }

    #[test]
    fn test_update_value() {
        let mut entry = CacheEntry::new(
            "test".to_string(),
            "old_value".to_string(),
            Duration::from_secs(3600),
        );

        let original_version = entry.metadata.version;
        entry.update_value("new_value".to_string(), Duration::from_secs(7200));

        assert_eq!(entry.value, "new_value");
        assert_eq!(entry.metadata.version, original_version + 1);
    }

    #[test]
    fn test_calculate_size() {
        let entry = CacheEntry::new(
            "key".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        let size = entry.calculate_size();
        assert!(size > 0);
        assert!(size >= "key".len() + "value".len());
    }

    #[test]
    fn test_tags() {
        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        entry.add_tag("category:llm".to_string());
        entry.add_tag("priority:high".to_string());

        assert!(entry.has_tag("category:llm"));
        assert!(entry.has_tag("priority:high"));
        assert!(!entry.has_tag("nonexistent"));

        entry.remove_tag("priority:high");
        assert!(!entry.has_tag("priority:high"));
    }

    #[test]
    fn test_metadata_hotness() {
        let mut metadata = CacheMetadata {
            created_at: Utc::now() - chrono::Duration::hours(2),
            accessed_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            access_count: 100,
            size_bytes: 1024,
            version: 1,
            tags: Vec::new(),
        };

        assert!(metadata.is_hot(50));
        assert!(!metadata.is_hot(200));

        let score = metadata.hotness_score();
        assert!(score > 0.0);
    }

    #[test]
    fn test_metadata_staleness() {
        let metadata = CacheMetadata {
            created_at: Utc::now() - chrono::Duration::hours(2),
            accessed_at: Utc::now() - chrono::Duration::hours(1),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            access_count: 10,
            size_bytes: 1024,
            version: 1,
            tags: Vec::new(),
        };

        assert!(metadata.is_stale(Duration::from_secs(1800))); // 30 minutes
        assert!(!metadata.is_stale(Duration::from_secs(7200))); // 2 hours
    }

    #[test]
    fn test_time_until_expiration() {
        let entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        let time_left = entry.time_until_expiration();
        assert!(time_left.is_some());
        assert!(time_left.unwrap() <= Duration::from_secs(3600));
    }

    #[test]
    fn test_age() {
        let entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        sleep(Duration::from_millis(10));
        let age = entry.age();
        assert!(age >= Duration::from_millis(10));
    }
}
