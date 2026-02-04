# Cache Module Implementation

## Overview

The cache module implements a context-aware caching layer for LLM context management in the Ouroboros Knowledge Graph system. It is based on research findings from modern semantic caching patterns (detailed in [caching-research.md](./caching-research.md)).

## Architecture

### Multi-Layer Design

The cache implements a three-layer architecture:

1. **Layer 1: Exact Match Cache** - String-based caching for identical queries
2. **Layer 2: Semantic Cache** - Embedding-based caching (prepared for future implementation)
3. **Layer 3: Provider Integration** - Integration points for LLM provider-level caching

### Core Components

```
src/cache/
├── mod.rs              # Module exports and documentation
├── types.rs            # Core types (CacheKey, CacheValue, CacheStats)
├── config.rs           # Configuration and preset configs
├── entry.rs            # Cache entry with TTL and metadata
├── invalidation.rs     # Invalidation strategies and policies
├── store.rs            # Main cache store with LRU eviction
└── integration.rs      # Knowledge graph integration layer
```

## Key Features

### 1. TTL-Based Expiration

Automatic cache invalidation with configurable time-to-live:

```rust
let config = CacheConfig::builder()
    .default_ttl(Duration::from_secs(3600))  // 1 hour
    .ttl_jitter(0.125)                        // 12.5% jitter
    .build();
```

**Research-Backed TTL Values:**
- Real-time data: 5 minutes
- Standard workloads: 1 hour (default)
- Daily updates: ~23 hours
- Static content: 48+ hours

### 2. LRU Eviction Policy

Least Recently Used eviction when cache limits are reached:

```rust
let config = CacheConfig::builder()
    .max_entries(10_000)
    .max_size_bytes(100 * 1024 * 1024)  // 100 MB
    .enable_lru_eviction(true)
    .build();
```

### 3. Flexible Invalidation Strategies

Multiple invalidation approaches:

- **TTL-based**: Automatic expiration
- **Event-based**: Triggered by source document changes
- **Staleness detection**: Based on access patterns
- **Tag-based**: Selective invalidation by tags
- **Combined**: Multiple strategies together

```rust
// Event-based invalidation for document updates
let strategy = InvalidationStrategy::event_based(vec!["doc123".to_string()]);
let policy = InvalidationPolicy::new(strategy);
let cache = ContextCache::with_invalidation_policy(config, policy);
```

### 4. Comprehensive Metrics

Built-in performance monitoring:

```rust
let stats = cache.stats().await;
println!("Hit rate: {:.2}%", stats.hit_rate());
println!("Cache entries: {}", stats.entries);
println!("Size: {} bytes", stats.size_bytes);
println!("Efficiency score: {:.2}", stats.efficiency_score());
```

## Usage Examples

### Basic Caching

```rust
use ouroboros_kg::cache::{CacheConfig, ContextCache};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CacheConfig::default();
    let cache = ContextCache::new(config);

    // Insert
    cache.insert("key1".to_string(), "value1".to_string()).await?;

    // Retrieve
    if let Some(value) = cache.get("key1").await? {
        println!("Cached value: {}", value);
    }

    Ok(())
}
```

### Knowledge Graph Integration

```rust
use ouroboros_kg::cache::{CacheConfig, KnowledgeGraphCache, CachedQueryResult};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(3600))
        .build();

    let kg_cache = KnowledgeGraphCache::new(config);

    // Cache query result
    let query = "MATCH (n:Person) WHERE n.age > 30 RETURN n.name";
    let result = CachedQueryResult::new(
        query.to_string(),
        r#"[{"name": "Alice"}, {"name": "Bob"}]"#.to_string(),
        2,
        150,
    );
    kg_cache.cache_query_result(query, result).await?;

    // Retrieve cached result
    if let Some(cached) = kg_cache.get_query_result(query).await? {
        println!("Found {} records", cached.record_count);
    }

    Ok(())
}
```

### Context Chunk Caching for RAG

```rust
use ouroboros_kg::cache::{CacheConfig, KnowledgeGraphCache, CachedContextChunk};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CacheConfig::default();
    let kg_cache = KnowledgeGraphCache::new(config);

    // Cache a context chunk
    let chunk = CachedContextChunk {
        chunk_id: "chunk_001".to_string(),
        content: "Sample text for RAG retrieval".to_string(),
        document_id: "doc_123".to_string(),
        position: 0,
        relevance_score: Some(0.95),
        metadata: HashMap::new(),
    };
    kg_cache.cache_chunk(chunk).await?;

    // Retrieve chunk
    if let Some(cached) = kg_cache.get_chunk("chunk_001", "doc_123").await? {
        println!("Chunk content: {}", cached.content);
    }

    // Invalidate all chunks from a document
    let count = kg_cache.invalidate_document("doc_123").await?;
    println!("Invalidated {} chunks", count);

    Ok(())
}
```

### Preset Configurations

The cache provides several preset configurations optimized for different use cases:

```rust
// Real-time data (5 minute TTL, high precision)
let cache = ContextCache::new(CacheConfig::realtime());

// Daily updates (23 hour TTL)
let cache = ContextCache::new(CacheConfig::daily());

// Static content (48 hour TTL)
let cache = ContextCache::new(CacheConfig::static_content());

// Memory-constrained (10 MB, 1000 entries)
let cache = ContextCache::new(CacheConfig::small());

// Large-scale deployment (10 GB, 1M entries)
let cache = ContextCache::new(CacheConfig::large());
```

## Performance Characteristics

Based on integration tests:

- **Insert throughput**: 1000 entries in ~100-200ms
- **Read throughput**: 1000 lookups in ~50-100ms
- **Concurrent access**: Thread-safe via RwLock
- **Memory efficiency**: ~100 bytes overhead per entry

### Cache Hit Rates

Expected hit rates based on research:
- **Exact match caching**: 20-40% for repeated queries
- **Semantic caching**: 40-60% with 0.90 similarity threshold
- **Combined caching**: 60-80% in production workloads

### Cost Reduction

Research-backed metrics:
- **Latency reduction**: 96.9% for cache hits (~1.67s → 0.052s)
- **Cost savings**: 40-73% reduction in LLM API costs
- **Memory usage**: ~60 GB for 10M vectors (without quantization)
- **With quantization**: 75% memory reduction with minimal accuracy loss

## Configuration Guidelines

### TTL Selection

Based on data volatility:

| Data Type | Recommended TTL | Jitter | Use Case |
|-----------|----------------|--------|----------|
| Real-time | 5 minutes | 15% | Live metrics, real-time data |
| Standard | 1 hour | 12.5% | General queries, user sessions |
| Daily | 23 hours | 10% | Product catalogs, daily reports |
| Static | 48+ hours | 5% | Documentation, reference data |

### Size Limits

Recommended by scale:

| Scale | Max Entries | Max Size | Use Case |
|-------|------------|----------|----------|
| Small | 1,000 | 10 MB | Development, testing |
| Medium | 10,000 | 100 MB | Small services |
| Large | 100,000 | 1 GB | Production services |
| Enterprise | 1,000,000+ | 10+ GB | Large-scale deployments |

### Similarity Thresholds (Future Semantic Caching)

| Threshold | Hit Rate | Precision | Use Case |
|-----------|----------|-----------|----------|
| 0.95+ | Lower | High | Critical applications |
| 0.85-0.95 | Moderate | Balanced | General purpose (recommended) |
| <0.85 | Higher | Lower | Exploratory search |

## Monitoring and Metrics

### Key Metrics to Track

```rust
let stats = cache.stats().await;

// Hit/miss tracking
println!("Hits: {}, Misses: {}", stats.hits, stats.misses);
println!("Hit rate: {:.2}%", stats.hit_rate());

// Memory usage
println!("Entries: {}", stats.entries);
println!("Size: {} MB", stats.size_bytes / 1024 / 1024);
println!("Avg entry size: {} bytes", stats.avg_entry_size);

// Evictions
println!("TTL evictions: {}", stats.evictions_ttl);
println!("Size evictions: {}", stats.evictions_size);
println!("Total evictions: {}", stats.total_evictions());

// Overall efficiency
println!("Efficiency score: {:.2}", stats.efficiency_score());
```

### Efficiency Score

The efficiency score (0-100) combines:
- Cache hit rate (higher is better)
- Eviction rate penalty (lower is better)

**Interpretation:**
- **80-100**: Excellent - well-tuned cache
- **60-80**: Good - minor optimization possible
- **40-60**: Fair - review configuration
- **<40**: Poor - significant tuning needed

## Integration with Neo4j

The cache module integrates seamlessly with the Neo4j connection layer:

```rust
use ouroboros_kg::{Neo4jClient, KnowledgeGraphCache, CacheConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Neo4j client
    let client = Neo4jClient::new(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j"
    ).await?;

    // Initialize cache
    let cache = KnowledgeGraphCache::new(CacheConfig::default());

    // Execute query with caching
    let query = "MATCH (n:Person) RETURN n.name";

    // Check cache first
    if let Some(cached) = cache.get_query_result(query).await? {
        println!("Cache hit!");
        // Use cached result
    } else {
        // Execute query against Neo4j
        let graph = client.graph();
        // ... execute query ...

        // Cache the result
        // cache.cache_query_result(query, result).await?;
    }

    Ok(())
}
```

## Automatic Cleanup

Background cleanup of expired entries:

```rust
use ouroboros_kg::cache::{ContextCache, CacheConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CacheConfig::builder()
        .enable_auto_cleanup(true)
        .cleanup_interval(Duration::from_secs(300))  // 5 minutes
        .build();

    let cache = Arc::new(ContextCache::new(config));

    // Spawn background cleanup task
    let cache_clone = cache.clone();
    tokio::spawn(async move {
        ouroboros_kg::cache::store::start_auto_cleanup(cache_clone).await;
    });

    // Use cache normally
    // ...

    Ok(())
}
```

## Future Enhancements

### Planned Features

1. **Semantic Caching**
   - Vector embedding integration
   - Cosine similarity matching
   - Configurable similarity thresholds

2. **Tiered Storage**
   - RAM (hot data)
   - SSD (warm data)
   - Object storage (cold data)

3. **Distributed Caching**
   - Cross-region cache sharing
   - Cache coherence protocols
   - Eventual consistency support

4. **Advanced Eviction Policies**
   - LFU (Least Frequently Used)
   - SLRU (Segmented LRU)
   - ARC (Adaptive Replacement Cache)

5. **Query-Adaptive Thresholds**
   - ML-based threshold selection
   - Per-query-type optimization
   - Dynamic adjustment

## Best Practices

### 1. Choose Appropriate TTL

Match TTL to your data's update frequency:
- Don't set TTL too short (wastes cache potential)
- Don't set TTL too long (risks serving stale data)
- Use jitter to prevent cache stampedes

### 2. Monitor Hit Rates

Track hit rates and adjust configuration:
- Target 40-60% hit rate for semantic caching
- Investigate if hit rate < 20%
- Consider increasing TTL if hit rate is good but cache is evicting too much

### 3. Use Tags for Invalidation

Tag entries by document/source for efficient invalidation:
```rust
cache.insert_with_tags(
    key,
    value,
    vec!["doc:123".to_string(), "user:alice".to_string()]
).await?;

// Later, invalidate all entries for doc:123
cache.invalidate_by_tag("doc:123").await?;
```

### 4. Size Your Cache Appropriately

- Small caches (< 10 MB): Fast but low hit rate
- Medium caches (100 MB - 1 GB): Good balance
- Large caches (> 1 GB): High hit rate but memory cost

### 5. Use Preset Configs

Start with preset configurations and adjust:
```rust
let mut config = CacheConfig::daily();
config.max_entries = 50_000;  // Customize as needed
```

## Testing

The module includes comprehensive tests:

```bash
# Run all cache tests
cargo test cache

# Run integration tests
cargo test --test cache_integration_tests

# Run with output
cargo test cache -- --nocapture
```

Test coverage includes:
- Basic CRUD operations
- TTL expiration
- LRU eviction
- Size-based eviction
- Invalidation strategies
- Concurrent access
- Knowledge graph integration
- Performance characteristics

## References

- Research findings: [docs/caching-research.md](./caching-research.md)
- API documentation: Run `cargo doc --open`
- Integration tests: [tests/cache_integration_tests.rs](../tests/cache_integration_tests.rs)

## License

Part of the Ouroboros Knowledge Graph project.
