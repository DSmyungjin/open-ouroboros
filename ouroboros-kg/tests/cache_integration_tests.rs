//! Integration tests for the cache module
//!
//! These tests verify the complete cache functionality including:
//! - Basic cache operations
//! - TTL expiration
//! - LRU eviction
//! - Invalidation strategies
//! - Knowledge graph integration
//! - Performance characteristics

use ouroboros_kg::cache::{
    CachedContextChunk, CachedQueryResult, CacheConfig, CacheKeyBuilder, ContextCache,
    ContextType, InvalidationPolicy, InvalidationStrategy, KnowledgeGraphCache,
};
use std::collections::HashMap;
use std::time::Duration;

#[tokio::test]
async fn test_basic_cache_operations() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(60))
        .max_entries(100)
        .enable_metrics(true)
        .build();

    let cache = ContextCache::new(config);

    // Test insert and get
    cache
        .insert("key1".to_string(), "value1".to_string())
        .await
        .unwrap();

    let value = cache.get("key1").await.unwrap();
    assert_eq!(value, Some("value1".to_string()));

    // Test cache hit tracking
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_ttl_expiration() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_millis(100))
        .ttl_jitter(0.0) // No jitter for predictable tests
        .build();

    let cache = ContextCache::new(config);

    cache
        .insert("expiring_key".to_string(), "expiring_value".to_string())
        .await
        .unwrap();

    // Should be available immediately
    let value = cache.get("expiring_key").await.unwrap();
    assert!(value.is_some());

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should be expired now
    let value = cache.get("expiring_key").await.unwrap();
    assert!(value.is_none());

    let stats = cache.stats().await;
    assert!(stats.evictions_ttl > 0);
}

#[tokio::test]
async fn test_lru_eviction() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(300))
        .max_entries(3)
        .enable_lru_eviction(true)
        .build();

    let cache = ContextCache::new(config);

    // Fill cache to capacity
    cache.insert("key1".to_string(), "value1".to_string()).await.unwrap();
    cache.insert("key2".to_string(), "value2".to_string()).await.unwrap();
    cache.insert("key3".to_string(), "value3".to_string()).await.unwrap();

    // Access key2 and key3 to make them more recent
    cache.get("key2").await.unwrap();
    cache.get("key3").await.unwrap();

    // Insert new entry, should evict key1 (least recently used)
    cache.insert("key4".to_string(), "value4".to_string()).await.unwrap();

    // Verify key1 was evicted
    assert!(cache.get("key1").await.unwrap().is_none());

    // Others should still be present
    assert!(cache.get("key2").await.unwrap().is_some());
    assert!(cache.get("key3").await.unwrap().is_some());
    assert!(cache.get("key4").await.unwrap().is_some());
}

#[tokio::test]
async fn test_size_based_eviction() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(300))
        .max_entries(1000)
        .max_size_bytes(500) // Very small size limit
        .enable_lru_eviction(true)
        .build();

    let cache = ContextCache::new(config);

    // Insert entries until size limit is reached
    cache.insert("k1".to_string(), "v".repeat(100)).await.unwrap();
    cache.insert("k2".to_string(), "v".repeat(100)).await.unwrap();

    // This should trigger eviction due to size limit
    cache.insert("k3".to_string(), "v".repeat(100)).await.unwrap();

    let stats = cache.stats().await;
    assert!(stats.evictions_size > 0 || stats.entries < 3);
}

#[tokio::test]
async fn test_invalidation_strategies() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(60))
        .build();

    let strategy = InvalidationStrategy::ttl();
    let policy = InvalidationPolicy::new(strategy);

    let cache = ContextCache::with_invalidation_policy(config, policy);

    cache.insert("key1".to_string(), "value1".to_string()).await.unwrap();

    let value = cache.get("key1").await.unwrap();
    assert!(value.is_some());
}

#[tokio::test]
async fn test_cleanup_expired() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_millis(50))
        .ttl_jitter(0.0)
        .build();

    let cache = ContextCache::new(config);

    // Insert multiple entries
    cache.insert("k1".to_string(), "v1".to_string()).await.unwrap();
    cache.insert("k2".to_string(), "v2".to_string()).await.unwrap();
    cache.insert("k3".to_string(), "v3".to_string()).await.unwrap();

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run cleanup
    let events = cache.cleanup_expired().await.unwrap();
    assert!(!events.is_empty());

    // Cache should be empty
    assert_eq!(cache.len().await, 0);
}

#[tokio::test]
async fn test_cache_stats() {
    let config = CacheConfig::builder()
        .enable_metrics(true)
        .build();

    let cache = ContextCache::new(config);

    // Generate some cache activity
    cache.insert("k1".to_string(), "v1".to_string()).await.unwrap();
    cache.insert("k2".to_string(), "v2".to_string()).await.unwrap();

    cache.get("k1").await.unwrap(); // Hit
    cache.get("k1").await.unwrap(); // Hit
    cache.get("k3").await.unwrap(); // Miss

    let stats = cache.stats().await;
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.entries, 2);
    assert!(stats.hit_rate() > 0.0);
    assert!(stats.size_bytes > 0);
}

#[tokio::test]
async fn test_knowledge_graph_cache_query_caching() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(60))
        .build();

    let kg_cache = KnowledgeGraphCache::new(config);

    let query = "MATCH (n:Person) WHERE n.age > 30 RETURN n.name, n.age";
    let result = CachedQueryResult::new(
        query.to_string(),
        r#"[{"name": "Alice", "age": 35}, {"name": "Bob", "age": 42}]"#.to_string(),
        2,
        150,
    );

    // Cache the query result
    kg_cache.cache_query_result(query, result.clone()).await.unwrap();

    // Retrieve from cache
    let cached = kg_cache.get_query_result(query).await.unwrap();
    assert!(cached.is_some());

    let cached_result = cached.unwrap();
    assert_eq!(cached_result.query, query);
    assert_eq!(cached_result.record_count, 2);
    assert_eq!(cached_result.execution_time_ms, 150);
}

#[tokio::test]
async fn test_knowledge_graph_cache_node_caching() {
    let config = CacheConfig::default();
    let kg_cache = KnowledgeGraphCache::new(config);

    let node_id = "person:123";
    let node_data = r#"{"id": "person:123", "name": "Alice", "age": 35}"#;

    // Cache node
    kg_cache.cache_node(node_id, node_data.to_string()).await.unwrap();

    // Retrieve node
    let cached = kg_cache.get_node(node_id).await.unwrap();
    assert!(cached.is_some());
    assert!(cached.unwrap().contains("Alice"));

    // Check stats
    let stats = kg_cache.stats().await;
    assert_eq!(stats.entries, 1);
}

#[tokio::test]
async fn test_knowledge_graph_cache_relationship_caching() {
    let config = CacheConfig::default();
    let kg_cache = KnowledgeGraphCache::new(config);

    let rel_id = "knows:456";
    let rel_data = r#"{"id": "knows:456", "from": "person:123", "to": "person:789", "since": 2020}"#;

    // Cache relationship
    kg_cache.cache_relationship(rel_id, rel_data.to_string()).await.unwrap();

    // Retrieve relationship
    let cached = kg_cache.get_relationship(rel_id).await.unwrap();
    assert!(cached.is_some());
    assert!(cached.unwrap().contains("knows:456"));
}

#[tokio::test]
async fn test_knowledge_graph_cache_chunk_caching() {
    let config = CacheConfig::default();
    let kg_cache = KnowledgeGraphCache::new(config);

    let chunk = CachedContextChunk {
        chunk_id: "chunk_001".to_string(),
        content: "This is a sample text chunk for RAG retrieval.".to_string(),
        document_id: "doc_123".to_string(),
        position: 0,
        relevance_score: Some(0.95),
        metadata: HashMap::from([
            ("source".to_string(), "manual".to_string()),
            ("language".to_string(), "en".to_string()),
        ]),
    };

    // Cache chunk
    kg_cache.cache_chunk(chunk.clone()).await.unwrap();

    // Retrieve chunk
    let cached = kg_cache.get_chunk("chunk_001", "doc_123").await.unwrap();
    assert!(cached.is_some());

    let cached_chunk = cached.unwrap();
    assert_eq!(cached_chunk.chunk_id, "chunk_001");
    assert_eq!(cached_chunk.document_id, "doc_123");
    assert_eq!(cached_chunk.relevance_score, Some(0.95));
}

#[tokio::test]
async fn test_knowledge_graph_cache_document_invalidation() {
    let config = CacheConfig::default();
    let kg_cache = KnowledgeGraphCache::new(config);

    // Cache multiple chunks from the same document
    for i in 0..5 {
        let chunk = CachedContextChunk {
            chunk_id: format!("chunk_{:03}", i),
            content: format!("Content of chunk {}", i),
            document_id: "doc_999".to_string(),
            position: i,
            relevance_score: Some(0.9),
            metadata: HashMap::new(),
        };
        kg_cache.cache_chunk(chunk).await.unwrap();
    }

    // Verify chunks are cached
    let stats_before = kg_cache.stats().await;
    assert_eq!(stats_before.entries, 5);

    // Invalidate all chunks for the document
    let invalidated = kg_cache.invalidate_document("doc_999").await.unwrap();
    assert_eq!(invalidated, 5);

    // Verify chunks are removed
    let stats_after = kg_cache.stats().await;
    assert_eq!(stats_after.entries, 0);
}

#[tokio::test]
async fn test_cache_key_builder() {
    let key1 = CacheKeyBuilder::new(ContextType::QueryResult)
        .identifier("query_hash_123")
        .build();
    assert_eq!(key1, "query_result:query_hash_123");

    let key2 = CacheKeyBuilder::new(ContextType::TextChunk)
        .identifier("chunk_456")
        .param("doc", "doc_789")
        .param("position", "0")
        .build();
    assert!(key2.starts_with("text_chunk:chunk_456"));
    assert!(key2.contains("doc=doc_789"));
    assert!(key2.contains("position=0"));

    let key3 = CacheKeyBuilder::new(ContextType::Custom("embedding".to_string()))
        .identifier("embed_vector_001")
        .build();
    assert!(key3.starts_with("custom:embedding:embed_vector_001"));
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    use tokio::task;

    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(60))
        .max_entries(1000)
        .build();

    let cache = std::sync::Arc::new(ContextCache::new(config));

    // Spawn multiple concurrent tasks
    let mut handles = vec![];

    for i in 0..10 {
        let cache_clone = cache.clone();
        let handle = task::spawn(async move {
            for j in 0..10 {
                let key = format!("key_{}_{}", i, j);
                let value = format!("value_{}_{}", i, j);
                cache_clone.insert(key.clone(), value.clone()).await.unwrap();
                let retrieved = cache_clone.get(&key).await.unwrap();
                assert_eq!(retrieved, Some(value));
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all entries were inserted
    let stats = cache.stats().await;
    assert_eq!(stats.entries, 100);
    assert_eq!(stats.hits, 100);
}

#[tokio::test]
async fn test_preset_configurations() {
    // Test realtime config
    let realtime = ContextCache::new(CacheConfig::realtime());
    assert_eq!(realtime.len().await, 0);

    // Test daily config
    let daily = ContextCache::new(CacheConfig::daily());
    assert_eq!(daily.len().await, 0);

    // Test static content config
    let static_content = ContextCache::new(CacheConfig::static_content());
    assert_eq!(static_content.len().await, 0);

    // Test small config
    let small = ContextCache::new(CacheConfig::small());
    assert_eq!(small.len().await, 0);

    // Test large config
    let large = ContextCache::new(CacheConfig::large());
    assert_eq!(large.len().await, 0);
}

#[tokio::test]
async fn test_cache_performance_characteristics() {
    let config = CacheConfig::builder()
        .default_ttl(Duration::from_secs(60))
        .max_entries(10_000)
        .enable_metrics(true)
        .build();

    let cache = ContextCache::new(config);

    // Insert many entries
    let start = std::time::Instant::now();
    for i in 0..1000 {
        cache.insert(format!("key_{}", i), format!("value_{}", i)).await.unwrap();
    }
    let insert_duration = start.elapsed();

    // Read many entries
    let start = std::time::Instant::now();
    for i in 0..1000 {
        cache.get(&format!("key_{}", i)).await.unwrap();
    }
    let read_duration = start.elapsed();

    println!("Insert 1000 entries: {:?}", insert_duration);
    println!("Read 1000 entries: {:?}", read_duration);

    // Verify performance is reasonable (should be well under 1 second for each)
    assert!(insert_duration.as_millis() < 5000);
    assert!(read_duration.as_millis() < 5000);

    let stats = cache.stats().await;
    assert_eq!(stats.entries, 1000);
    assert_eq!(stats.hits, 1000);
}
