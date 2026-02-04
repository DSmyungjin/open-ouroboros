//! Cache invalidation strategies
//!
//! Implements multiple invalidation approaches based on research:
//! - TTL-based: Automatic expiration after time-to-live
//! - Event-based: Trigger invalidation when source data changes
//! - Staleness detection: Monitor cache accuracy and invalidate on degradation
//! - Content-triggered: Clear entries referencing updated documents

use crate::cache::entry::CacheEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Reason for cache invalidation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvalidationReason {
    /// Entry expired based on TTL
    Expired,

    /// Manual invalidation by key
    Manual,

    /// Invalidated due to source document update
    SourceUpdated { document_id: String },

    /// Invalidated due to staleness detection
    Stale,

    /// Evicted due to cache size limits
    SizeLimit,

    /// Evicted by LRU policy
    LeastRecentlyUsed,

    /// Invalidated by tag match
    TagMatch { tag: String },

    /// Invalidated due to low accuracy/quality
    QualityDegradation,

    /// Cache coherence in distributed system
    Distributed,
}

impl std::fmt::Display for InvalidationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidationReason::Expired => write!(f, "TTL expired"),
            InvalidationReason::Manual => write!(f, "manual invalidation"),
            InvalidationReason::SourceUpdated { document_id } => {
                write!(f, "source document updated: {}", document_id)
            }
            InvalidationReason::Stale => write!(f, "staleness detected"),
            InvalidationReason::SizeLimit => write!(f, "cache size limit reached"),
            InvalidationReason::LeastRecentlyUsed => write!(f, "LRU eviction"),
            InvalidationReason::TagMatch { tag } => write!(f, "tag match: {}", tag),
            InvalidationReason::QualityDegradation => write!(f, "quality degradation"),
            InvalidationReason::Distributed => write!(f, "distributed cache coherence"),
        }
    }
}

/// Strategy for cache invalidation
#[derive(Debug, Clone)]
pub enum InvalidationStrategy {
    /// Time-based expiration
    TimeToLive,

    /// Event-driven invalidation
    EventBased {
        /// Document IDs that trigger invalidation
        watch_documents: HashSet<String>,
    },

    /// Staleness detection based on access patterns
    StalenessDetection {
        /// Threshold for considering an entry stale
        threshold: std::time::Duration,
    },

    /// Tag-based invalidation
    TagBased {
        /// Tags to match for invalidation
        tags: HashSet<String>,
    },

    /// Combined strategy
    Combined {
        strategies: Vec<InvalidationStrategy>,
    },
}

impl InvalidationStrategy {
    /// Create a TTL-based strategy
    pub fn ttl() -> Self {
        InvalidationStrategy::TimeToLive
    }

    /// Create an event-based strategy
    pub fn event_based(document_ids: Vec<String>) -> Self {
        InvalidationStrategy::EventBased {
            watch_documents: document_ids.into_iter().collect(),
        }
    }

    /// Create a staleness detection strategy
    pub fn staleness(threshold: std::time::Duration) -> Self {
        InvalidationStrategy::StalenessDetection { threshold }
    }

    /// Create a tag-based strategy
    pub fn tag_based(tags: Vec<String>) -> Self {
        InvalidationStrategy::TagBased {
            tags: tags.into_iter().collect(),
        }
    }

    /// Create a combined strategy
    pub fn combined(strategies: Vec<InvalidationStrategy>) -> Self {
        InvalidationStrategy::Combined { strategies }
    }

    /// Check if an entry should be invalidated according to this strategy
    pub fn should_invalidate(&self, entry: &CacheEntry) -> Option<InvalidationReason> {
        match self {
            InvalidationStrategy::TimeToLive => {
                if entry.is_expired() {
                    Some(InvalidationReason::Expired)
                } else {
                    None
                }
            }

            InvalidationStrategy::EventBased { watch_documents } => {
                // Check if any of the entry's tags match watched documents
                for tag in &entry.metadata.tags {
                    if tag.starts_with("doc:") {
                        let doc_id = &tag[4..];
                        if watch_documents.contains(doc_id) {
                            return Some(InvalidationReason::SourceUpdated {
                                document_id: doc_id.to_string(),
                            });
                        }
                    }
                }
                None
            }

            InvalidationStrategy::StalenessDetection { threshold } => {
                if entry.metadata.is_stale(*threshold) {
                    Some(InvalidationReason::Stale)
                } else {
                    None
                }
            }

            InvalidationStrategy::TagBased { tags } => {
                for tag in tags {
                    if entry.has_tag(tag) {
                        return Some(InvalidationReason::TagMatch { tag: tag.clone() });
                    }
                }
                None
            }

            InvalidationStrategy::Combined { strategies } => {
                // Check each strategy in order, return first match
                for strategy in strategies {
                    if let Some(reason) = strategy.should_invalidate(entry) {
                        return Some(reason);
                    }
                }
                None
            }
        }
    }
}

/// Invalidation policy that combines multiple strategies
#[derive(Debug, Clone)]
pub struct InvalidationPolicy {
    /// Primary invalidation strategy
    pub primary_strategy: InvalidationStrategy,

    /// Whether to use aggressive cleanup (remove expired entries immediately)
    pub aggressive_cleanup: bool,

    /// Grace period before invalidation (allows stale data temporarily)
    pub grace_period: Option<std::time::Duration>,
}

impl Default for InvalidationPolicy {
    fn default() -> Self {
        Self {
            primary_strategy: InvalidationStrategy::ttl(),
            aggressive_cleanup: false,
            grace_period: None,
        }
    }
}

impl InvalidationPolicy {
    /// Create a new invalidation policy
    pub fn new(strategy: InvalidationStrategy) -> Self {
        Self {
            primary_strategy: strategy,
            aggressive_cleanup: false,
            grace_period: None,
        }
    }

    /// Enable aggressive cleanup
    pub fn with_aggressive_cleanup(mut self) -> Self {
        self.aggressive_cleanup = true;
        self
    }

    /// Set grace period before invalidation
    pub fn with_grace_period(mut self, grace: std::time::Duration) -> Self {
        self.grace_period = Some(grace);
        self
    }

    /// Check if an entry should be invalidated
    pub fn should_invalidate(&self, entry: &CacheEntry) -> Option<InvalidationReason> {
        let reason = self.primary_strategy.should_invalidate(entry)?;

        // Apply grace period if configured
        if let Some(grace) = self.grace_period {
            match reason {
                InvalidationReason::Expired | InvalidationReason::Stale => {
                    // Check if we're still within grace period
                    if let Some(time_left) = entry.time_until_expiration() {
                        if time_left < grace {
                            return Some(reason);
                        }
                        return None;
                    }
                }
                _ => {}
            }
        }

        Some(reason)
    }
}

/// Event for cache invalidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationEvent {
    /// Reason for invalidation
    pub reason: InvalidationReason,

    /// When the invalidation occurred
    pub timestamp: DateTime<Utc>,

    /// Keys that were invalidated
    pub keys: Vec<String>,

    /// Additional context
    pub context: Option<String>,
}

impl InvalidationEvent {
    /// Create a new invalidation event
    pub fn new(reason: InvalidationReason, keys: Vec<String>) -> Self {
        Self {
            reason,
            timestamp: Utc::now(),
            keys,
            context: None,
        }
    }

    /// Add context to the event
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::types::CacheKey;
    use std::time::Duration;

    #[test]
    fn test_invalidation_reason_display() {
        let reason = InvalidationReason::Expired;
        assert_eq!(reason.to_string(), "TTL expired");

        let reason = InvalidationReason::SourceUpdated {
            document_id: "doc123".to_string(),
        };
        assert!(reason.to_string().contains("doc123"));
    }

    #[test]
    fn test_ttl_strategy() {
        let strategy = InvalidationStrategy::ttl();

        // Create an expired entry
        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_millis(1),
        );
        std::thread::sleep(Duration::from_millis(10));

        let reason = strategy.should_invalidate(&entry);
        assert!(matches!(reason, Some(InvalidationReason::Expired)));
    }

    #[test]
    fn test_event_based_strategy() {
        let strategy = InvalidationStrategy::event_based(vec!["doc123".to_string()]);

        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );
        entry.add_tag("doc:doc123".to_string());

        let reason = strategy.should_invalidate(&entry);
        assert!(matches!(
            reason,
            Some(InvalidationReason::SourceUpdated { .. })
        ));
    }

    #[test]
    fn test_staleness_strategy() {
        let strategy = InvalidationStrategy::staleness(Duration::from_millis(50));

        let entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        // Initially not stale
        assert!(strategy.should_invalidate(&entry).is_none());

        std::thread::sleep(Duration::from_millis(60));

        // Now should be stale
        let reason = strategy.should_invalidate(&entry);
        assert!(matches!(reason, Some(InvalidationReason::Stale)));
    }

    #[test]
    fn test_tag_based_strategy() {
        let strategy = InvalidationStrategy::tag_based(vec!["priority:low".to_string()]);

        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );
        entry.add_tag("priority:low".to_string());

        let reason = strategy.should_invalidate(&entry);
        assert!(matches!(reason, Some(InvalidationReason::TagMatch { .. })));
    }

    #[test]
    fn test_combined_strategy() {
        let strategy = InvalidationStrategy::combined(vec![
            InvalidationStrategy::ttl(),
            InvalidationStrategy::tag_based(vec!["test".to_string()]),
        ]);

        let mut entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_millis(1),
        );
        std::thread::sleep(Duration::from_millis(10));

        let reason = strategy.should_invalidate(&entry);
        assert!(reason.is_some());
    }

    #[test]
    fn test_invalidation_policy() {
        let policy = InvalidationPolicy::new(InvalidationStrategy::ttl())
            .with_aggressive_cleanup()
            .with_grace_period(Duration::from_secs(60));

        let entry = CacheEntry::new(
            "test".to_string(),
            "value".to_string(),
            Duration::from_secs(3600),
        );

        assert!(policy.should_invalidate(&entry).is_none());
    }

    #[test]
    fn test_invalidation_event() {
        let event = InvalidationEvent::new(
            InvalidationReason::Manual,
            vec!["key1".to_string(), "key2".to_string()],
        )
        .with_context("test context".to_string());

        assert_eq!(event.keys.len(), 2);
        assert_eq!(event.context, Some("test context".to_string()));
        assert!(matches!(event.reason, InvalidationReason::Manual));
    }
}
