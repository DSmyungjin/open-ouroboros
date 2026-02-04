//! # Context-Aware Caching Layer
//!
//! This module implements a multi-layer caching system for LLM context management,
//! based on research findings from modern semantic caching patterns.
//!
//! ## Features
//!
//! - **TTL-Based Expiration**: Automatic cache invalidation with configurable time-to-live
//! - **Multi-Layer Caching**: Support for exact match, semantic, and embedding caches
//! - **LRU Eviction**: Least Recently Used eviction policy for memory management
//! - **Flexible Invalidation**: Event-based, TTL-based, and manual invalidation strategies
//! - **Memory Management**: Configurable size limits and tiered storage support
//! - **Context-Aware**: Position-independent chunk hashing for RAG systems
//!
//! ## Architecture
//!
//! The cache implements a three-layer approach:
//! - Layer 1: Exact match cache (string-based)
//! - Layer 2: Semantic cache (embedding-based, future)
//! - Layer 3: Provider cache integration points
//!
//! ## Example
//!
//! ```rust
//! use ouroboros_kg::cache::{CacheConfig, ContextCache};
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = CacheConfig::builder()
//!     .default_ttl(Duration::from_secs(3600)) // 1 hour
//!     .max_entries(10_000)
//!     .max_size_bytes(100 * 1024 * 1024) // 100 MB
//!     .build();
//!
//! let mut cache = ContextCache::new(config);
//!
//! // Store a context chunk
//! cache.insert("query:123".to_string(), "cached response".to_string()).await?;
//!
//! // Retrieve cached value
//! if let Some(value) = cache.get("query:123").await? {
//!     println!("Cache hit: {}", value);
//! }
//!
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod entry;
pub mod integration;
pub mod invalidation;
pub mod store;
pub mod types;

pub use config::{CacheConfig, CacheConfigBuilder};
pub use entry::{CacheEntry, CacheMetadata};
pub use integration::{
    CachedContextChunk, CachedQueryResult, CacheKeyBuilder, ContextType, KnowledgeGraphCache,
};
pub use invalidation::{InvalidationPolicy, InvalidationReason, InvalidationStrategy};
pub use store::ContextCache;
pub use types::{CacheKey, CacheStats, CacheValue};
