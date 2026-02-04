//! Main cache store implementation with LRU eviction and memory management

use crate::cache::{
    config::CacheConfig,
    entry::CacheEntry,
    invalidation::{InvalidationEvent, InvalidationPolicy, InvalidationReason},
    types::{CacheKey, CacheStats, CacheValue},
};
use crate::error::{Neo4jError, Result};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Context-aware cache with TTL support and LRU eviction
///
/// This implementation provides:
/// - Thread-safe async access via RwLock
/// - Automatic TTL-based expiration
/// - LRU eviction when size limits are reached
/// - Comprehensive metrics collection
/// - Flexible invalidation strategies
pub struct ContextCache {
    /// Cache configuration
    pub(crate) config: CacheConfig,

    /// Internal storage
    store: Arc<RwLock<CacheStore>>,

    /// Invalidation policy
    invalidation_policy: InvalidationPolicy,
}

/// Internal cache storage
struct CacheStore {
    /// Main storage: key -> entry
    entries: HashMap<CacheKey, CacheEntry>,

    /// LRU tracking: maintains access order
    lru_queue: VecDeque<CacheKey>,

    /// Current cache statistics
    stats: CacheStats,

    /// Total size of cached data in bytes
    current_size_bytes: usize,
}

impl ContextCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        info!("Initializing context cache with config: {:?}", config);

        let store = CacheStore {
            entries: HashMap::new(),
            lru_queue: VecDeque::new(),
            stats: CacheStats::default(),
            current_size_bytes: 0,
        };

        Self {
            config,
            store: Arc::new(RwLock::new(store)),
            invalidation_policy: InvalidationPolicy::default(),
        }
    }

    /// Create a cache with custom invalidation policy
    pub fn with_invalidation_policy(config: CacheConfig, policy: InvalidationPolicy) -> Self {
        let mut cache = Self::new(config);
        cache.invalidation_policy = policy;
        cache
    }

    /// Insert a value into the cache
    pub async fn insert(&self, key: CacheKey, value: CacheValue) -> Result<()> {
        self.insert_with_tags(key, value, Vec::new()).await
    }

    /// Insert a value into the cache with tags
    pub async fn insert_with_tags(&self, key: CacheKey, value: CacheValue, tags: Vec<String>) -> Result<()> {
        let ttl = self.config.ttl_with_jitter();
        let mut entry = CacheEntry::new(key.clone(), value, ttl);
        entry.metadata.size_bytes = entry.calculate_size();

        // Add tags
        for tag in tags {
            entry.add_tag(tag);
        }

        let mut store = self.store.write().await;

        // Check if we need to evict entries
        self.evict_if_needed(&mut store, entry.metadata.size_bytes).await?;

        // Update or insert entry
        if let Some(existing) = store.entries.get_mut(&key) {
            debug!("Updating existing cache entry: {}", key);
            *existing = entry;
            // Move to end of LRU queue (most recently used)
            store.lru_queue.retain(|k| k != &key);
            store.lru_queue.push_back(key);
        } else {
            debug!("Inserting new cache entry: {}", key);
            let size = entry.metadata.size_bytes;
            store.entries.insert(key.clone(), entry);
            store.lru_queue.push_back(key.clone());
            store.current_size_bytes += size;
            store.stats.entries += 1;
        }

        self.update_stats(&mut store);

        Ok(())
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &str) -> Result<Option<CacheValue>> {
        let mut store = self.store.write().await;

        // Try to find the entry
        if let Some(entry) = store.entries.get(key) {
            // Check if expired
            if entry.is_expired() {
                debug!("Cache entry expired: {}", key);
                store.stats.misses += 1;
                store.stats.evictions_ttl += 1;
                self.remove_entry(&mut store, key);
                return Ok(None);
            }

            // Check invalidation policy
            if let Some(reason) = self.invalidation_policy.should_invalidate(entry) {
                debug!("Cache entry invalidated ({}): {}", reason, key);
                store.stats.misses += 1;
                store.stats.invalidations += 1;
                self.remove_entry(&mut store, key);
                return Ok(None);
            }

            // Cache hit - clone value before updating metadata
            let value = entry.value.clone();

            // Update access metadata
            if let Some(entry) = store.entries.get_mut(key) {
                entry.mark_accessed();
            }
            store.stats.hits += 1;

            // Update LRU queue (move to end)
            if self.config.enable_lru_eviction {
                store.lru_queue.retain(|k| k != key);
                store.lru_queue.push_back(key.to_string());
            }

            debug!("Cache hit: {}", key);
            Ok(Some(value))
        } else {
            // Cache miss
            debug!("Cache miss: {}", key);
            store.stats.misses += 1;
            Ok(None)
        }
    }

    /// Check if a key exists in the cache (without updating access time)
    pub async fn contains_key(&self, key: &str) -> bool {
        let store = self.store.read().await;
        store.entries.contains_key(key)
    }

    /// Remove a specific entry from the cache
    pub async fn remove(&self, key: &str) -> Result<Option<CacheValue>> {
        let mut store = self.store.write().await;

        if let Some(entry) = store.entries.remove(key) {
            store.lru_queue.retain(|k| k != key);
            store.current_size_bytes = store.current_size_bytes.saturating_sub(entry.metadata.size_bytes);
            store.stats.entries = store.stats.entries.saturating_sub(1);
            store.stats.invalidations += 1;

            debug!("Removed cache entry: {}", key);
            Ok(Some(entry.value))
        } else {
            Ok(None)
        }
    }

    /// Clear all entries from the cache
    pub async fn clear(&self) -> Result<()> {
        let mut store = self.store.write().await;

        let count = store.entries.len();
        store.entries.clear();
        store.lru_queue.clear();
        store.current_size_bytes = 0;
        store.stats.entries = 0;
        store.stats.invalidations += count as u64;

        info!("Cleared {} entries from cache", count);
        Ok(())
    }

    /// Remove all expired entries
    pub async fn cleanup_expired(&self) -> Result<Vec<InvalidationEvent>> {
        let mut store = self.store.write().await;
        let mut events = Vec::new();
        let mut expired_keys = Vec::new();

        // Find all expired entries
        for (key, entry) in &store.entries {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        if !expired_keys.is_empty() {
            for key in &expired_keys {
                self.remove_entry(&mut store, key);
            }

            store.stats.evictions_ttl += expired_keys.len() as u64;

            let event = InvalidationEvent::new(InvalidationReason::Expired, expired_keys.clone())
                .with_context(format!("Cleaned up {} expired entries", expired_keys.len()));
            events.push(event);

            debug!("Cleaned up {} expired entries", expired_keys.len());
        }

        Ok(events)
    }

    /// Invalidate entries by tag
    pub async fn invalidate_by_tag(&self, tag: &str) -> Result<usize> {
        let mut store = self.store.write().await;
        let mut removed = 0;

        let keys_to_remove: Vec<CacheKey> = store
            .entries
            .iter()
            .filter(|(_, entry)| entry.has_tag(tag))
            .map(|(key, _)| key.clone())
            .collect();

        for key in keys_to_remove {
            self.remove_entry(&mut store, &key);
            removed += 1;
        }

        store.stats.invalidations += removed as u64;
        info!("Invalidated {} entries with tag: {}", removed, tag);

        Ok(removed)
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let store = self.store.read().await;
        store.stats.clone()
    }

    /// Get current cache size in bytes
    pub async fn size_bytes(&self) -> usize {
        let store = self.store.read().await;
        store.current_size_bytes
    }

    /// Get number of entries in cache
    pub async fn len(&self) -> usize {
        let store = self.store.read().await;
        store.entries.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        let store = self.store.read().await;
        store.entries.is_empty()
    }

    /// Internal: Remove an entry from the store
    fn remove_entry(&self, store: &mut CacheStore, key: &str) {
        if let Some(entry) = store.entries.remove(key) {
            store.lru_queue.retain(|k| k != key);
            store.current_size_bytes = store.current_size_bytes.saturating_sub(entry.metadata.size_bytes);
            store.stats.entries = store.stats.entries.saturating_sub(1);
        }
    }

    /// Internal: Evict entries if needed to make room for new entry
    async fn evict_if_needed(&self, store: &mut CacheStore, needed_size: usize) -> Result<()> {
        // Check entry count limit
        while store.entries.len() >= self.config.max_entries {
            if let Some(key) = store.lru_queue.pop_front() {
                debug!("Evicting entry due to max_entries limit: {}", key);
                self.remove_entry(store, &key);
                store.stats.evictions_size += 1;
            } else {
                break;
            }
        }

        // Check size limit
        while store.current_size_bytes + needed_size > self.config.max_size_bytes {
            if let Some(key) = store.lru_queue.pop_front() {
                debug!("Evicting entry due to size limit: {}", key);
                self.remove_entry(store, &key);
                store.stats.evictions_size += 1;
            } else {
                // No more entries to evict, but still over limit
                warn!("Cannot evict more entries, cache size limit exceeded");
                return Err(Neo4jError::Other(
                    "Cache size limit exceeded".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Internal: Update cache statistics
    fn update_stats(&self, store: &mut CacheStore) {
        if self.config.enable_metrics {
            let total_size: usize = store.entries.values()
                .map(|e| e.metadata.size_bytes)
                .sum();

            store.stats.size_bytes = total_size;
            store.stats.entries = store.entries.len();

            if store.entries.len() > 0 {
                store.stats.avg_entry_size = total_size / store.entries.len();
            }
        }
    }
}

/// Background task for automatic cache cleanup
pub async fn start_auto_cleanup(cache: Arc<ContextCache>) {
    let interval = cache.config.cleanup_interval;

    info!("Starting automatic cache cleanup task (interval: {:?})", interval);

    loop {
        tokio::time::sleep(interval).await;

        match cache.cleanup_expired().await {
            Ok(events) => {
                if !events.is_empty() {
                    debug!("Auto cleanup: {} events", events.len());
                }
            }
            Err(e) => {
                warn!("Auto cleanup failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_insert_and_get() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(60))
            .max_entries(100)
            .build();

        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();

        let value = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);

        let value = cache.get("nonexistent").await.unwrap();
        assert_eq!(value, None);

        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_millis(100))
            .ttl_jitter(0.0)
            .build();

        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();

        // Should be available immediately
        let value = cache.get("key1").await.unwrap();
        assert!(value.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        let value = cache.get("key1").await.unwrap();
        assert!(value.is_none());

        let stats = cache.stats().await;
        assert_eq!(stats.evictions_ttl, 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(60))
            .max_entries(3)
            .enable_lru_eviction(true)
            .build();

        let cache = ContextCache::new(config);

        // Insert 3 entries (fill the cache)
        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();
        cache.insert("key2".to_string(), "value2".to_string())
            .await
            .unwrap();
        cache.insert("key3".to_string(), "value3".to_string())
            .await
            .unwrap();

        // Insert 4th entry, should evict key1 (least recently used)
        cache.insert("key4".to_string(), "value4".to_string())
            .await
            .unwrap();

        // key1 should be evicted
        let value = cache.get("key1").await.unwrap();
        assert!(value.is_none());

        // Others should still be there
        assert!(cache.get("key2").await.unwrap().is_some());
        assert!(cache.get("key3").await.unwrap().is_some());
        assert!(cache.get("key4").await.unwrap().is_some());

        let stats = cache.stats().await;
        assert!(stats.evictions_size > 0);
    }

    #[tokio::test]
    async fn test_remove() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();

        let removed = cache.remove("key1").await.unwrap();
        assert_eq!(removed, Some("value1".to_string()));

        let value = cache.get("key1").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();
        cache.insert("key2".to_string(), "value2".to_string())
            .await
            .unwrap();

        cache.clear().await.unwrap();

        assert_eq!(cache.len().await, 0);
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_millis(50))
            .ttl_jitter(0.0)
            .build();

        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();
        cache.insert("key2".to_string(), "value2".to_string())
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        let events = cache.cleanup_expired().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_invalidate_by_tag() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);

        // Insert entry with tag
        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();

        // We need to access the entry to add a tag
        // For now, let's test the invalidation mechanism
        let count = cache.invalidate_by_tag("test_tag").await.unwrap();
        // Should be 0 since we didn't add tags
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let config = CacheConfig::builder()
            .enable_metrics(true)
            .build();

        let cache = ContextCache::new(config);

        cache.insert("key1".to_string(), "value1".to_string())
            .await
            .unwrap();
        cache.get("key1").await.unwrap();
        cache.get("nonexistent").await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
        assert!(stats.size_bytes > 0);
    }
}
