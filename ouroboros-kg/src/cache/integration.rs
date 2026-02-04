//! Integration points for knowledge graph and LLM context caching
//!
//! This module provides high-level integration between the cache system
//! and knowledge graph operations, including:
//! - Query result caching
//! - Context chunk caching
//! - Embedding caching (future)
//! - RAG retrieval caching

use crate::cache::{
    config::CacheConfig,
    store::ContextCache,
    types::{CacheKey, CacheValue},
};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Context type for categorizing cache entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContextType {
    /// Cypher query result
    QueryResult,

    /// Knowledge graph node
    GraphNode,

    /// Knowledge graph relationship
    GraphRelationship,

    /// Text chunk for RAG
    TextChunk,

    /// Embedding vector (future)
    Embedding,

    /// LLM response
    LlmResponse,

    /// Custom context type
    Custom(String),
}

impl std::fmt::Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextType::QueryResult => write!(f, "query_result"),
            ContextType::GraphNode => write!(f, "graph_node"),
            ContextType::GraphRelationship => write!(f, "graph_relationship"),
            ContextType::TextChunk => write!(f, "text_chunk"),
            ContextType::Embedding => write!(f, "embedding"),
            ContextType::LlmResponse => write!(f, "llm_response"),
            ContextType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// Cache key builder for knowledge graph contexts
pub struct CacheKeyBuilder {
    context_type: ContextType,
    identifier: String,
    params: Vec<(String, String)>,
}

impl CacheKeyBuilder {
    /// Create a new cache key builder
    pub fn new(context_type: ContextType) -> Self {
        Self {
            context_type,
            identifier: String::new(),
            params: Vec::new(),
        }
    }

    /// Set the primary identifier
    pub fn identifier(mut self, id: impl Into<String>) -> Self {
        self.identifier = id.into();
        self
    }

    /// Add a parameter to the key
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    /// Build the cache key
    pub fn build(self) -> CacheKey {
        let mut key = format!("{}:{}", self.context_type, self.identifier);

        if !self.params.is_empty() {
            let params_str: Vec<String> = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            key.push_str(&format!("?{}", params_str.join("&")));
        }

        key
    }
}

/// Cached query result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedQueryResult {
    /// The original query
    pub query: String,

    /// Serialized result data
    pub result_json: String,

    /// Number of records in result
    pub record_count: usize,

    /// Query execution time (for metrics)
    pub execution_time_ms: u64,
}

impl CachedQueryResult {
    /// Create a new cached query result
    pub fn new(query: String, result_json: String, record_count: usize, execution_time_ms: u64) -> Self {
        Self {
            query,
            result_json,
            record_count,
            execution_time_ms,
        }
    }

    /// Serialize to cache value
    pub fn to_cache_value(&self) -> Result<CacheValue> {
        serde_json::to_string(self)
            .map_err(|e| crate::error::Neo4jError::SerializationError(e.to_string()))
    }

    /// Deserialize from cache value
    pub fn from_cache_value(value: &str) -> Result<Self> {
        serde_json::from_str(value)
            .map_err(|e| crate::error::Neo4jError::SerializationError(e.to_string()))
    }
}

/// Cached context chunk for RAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedContextChunk {
    /// Unique chunk identifier
    pub chunk_id: String,

    /// The text content
    pub content: String,

    /// Source document ID
    pub document_id: String,

    /// Chunk position in document
    pub position: usize,

    /// Relevance score (if from retrieval)
    pub relevance_score: Option<f64>,

    /// Metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl CachedContextChunk {
    /// Create a cache key for this chunk
    pub fn cache_key(&self) -> CacheKey {
        CacheKeyBuilder::new(ContextType::TextChunk)
            .identifier(&self.chunk_id)
            .param("doc", &self.document_id)
            .build()
    }

    /// Serialize to cache value
    pub fn to_cache_value(&self) -> Result<CacheValue> {
        serde_json::to_string(self)
            .map_err(|e| crate::error::Neo4jError::SerializationError(e.to_string()))
    }

    /// Deserialize from cache value
    pub fn from_cache_value(value: &str) -> Result<Self> {
        serde_json::from_str(value)
            .map_err(|e| crate::error::Neo4jError::SerializationError(e.to_string()))
    }
}

/// Knowledge graph cache wrapper
///
/// Provides high-level caching operations for knowledge graph contexts
pub struct KnowledgeGraphCache {
    cache: Arc<ContextCache>,
}

impl KnowledgeGraphCache {
    /// Create a new knowledge graph cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(ContextCache::new(config)),
        }
    }

    /// Create from existing cache instance
    pub fn from_cache(cache: Arc<ContextCache>) -> Self {
        Self { cache }
    }

    /// Get the underlying cache instance
    pub fn inner(&self) -> Arc<ContextCache> {
        self.cache.clone()
    }

    /// Cache a query result
    pub async fn cache_query_result(
        &self,
        query: &str,
        result: CachedQueryResult,
    ) -> Result<()> {
        let key = CacheKeyBuilder::new(ContextType::QueryResult)
            .identifier(Self::hash_query(query))
            .build();

        let value = result.to_cache_value()?;
        self.cache.insert(key, value).await
    }

    /// Get cached query result
    pub async fn get_query_result(&self, query: &str) -> Result<Option<CachedQueryResult>> {
        let key = CacheKeyBuilder::new(ContextType::QueryResult)
            .identifier(Self::hash_query(query))
            .build();

        if let Some(value) = self.cache.get(&key).await? {
            Ok(Some(CachedQueryResult::from_cache_value(&value)?))
        } else {
            Ok(None)
        }
    }

    /// Cache a context chunk
    pub async fn cache_chunk(&self, chunk: CachedContextChunk) -> Result<()> {
        let key = chunk.cache_key();
        let value = chunk.to_cache_value()?;

        // Add document tag for invalidation
        let tags = vec![format!("doc:{}", chunk.document_id)];

        self.cache.insert_with_tags(key, value, tags).await
    }

    /// Get cached context chunk
    pub async fn get_chunk(&self, chunk_id: &str, document_id: &str) -> Result<Option<CachedContextChunk>> {
        let key = CacheKeyBuilder::new(ContextType::TextChunk)
            .identifier(chunk_id)
            .param("doc", document_id)
            .build();

        if let Some(value) = self.cache.get(&key).await? {
            Ok(Some(CachedContextChunk::from_cache_value(&value)?))
        } else {
            Ok(None)
        }
    }

    /// Invalidate all chunks for a document
    pub async fn invalidate_document(&self, document_id: &str) -> Result<usize> {
        let tag = format!("doc:{}", document_id);
        self.cache.invalidate_by_tag(&tag).await
    }

    /// Cache a graph node
    pub async fn cache_node(&self, node_id: &str, node_data: String) -> Result<()> {
        let key = CacheKeyBuilder::new(ContextType::GraphNode)
            .identifier(node_id)
            .build();

        self.cache.insert(key, node_data).await
    }

    /// Get cached graph node
    pub async fn get_node(&self, node_id: &str) -> Result<Option<String>> {
        let key = CacheKeyBuilder::new(ContextType::GraphNode)
            .identifier(node_id)
            .build();

        self.cache.get(&key).await
    }

    /// Cache a graph relationship
    pub async fn cache_relationship(&self, rel_id: &str, rel_data: String) -> Result<()> {
        let key = CacheKeyBuilder::new(ContextType::GraphRelationship)
            .identifier(rel_id)
            .build();

        self.cache.insert(key, rel_data).await
    }

    /// Get cached graph relationship
    pub async fn get_relationship(&self, rel_id: &str) -> Result<Option<String>> {
        let key = CacheKeyBuilder::new(ContextType::GraphRelationship)
            .identifier(rel_id)
            .build();

        self.cache.get(&key).await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> crate::cache::types::CacheStats {
        self.cache.stats().await
    }

    /// Clear the entire cache
    pub async fn clear(&self) -> Result<()> {
        self.cache.clear().await
    }

    /// Internal: Generate a hash for a query string
    fn hash_query(query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_context_type_display() {
        assert_eq!(format!("{}", ContextType::QueryResult), "query_result");
        assert_eq!(format!("{}", ContextType::GraphNode), "graph_node");
        assert_eq!(format!("{}", ContextType::TextChunk), "text_chunk");
        assert_eq!(
            format!("{}", ContextType::Custom("test".to_string())),
            "custom:test"
        );
    }

    #[test]
    fn test_cache_key_builder() {
        let key = CacheKeyBuilder::new(ContextType::QueryResult)
            .identifier("query123")
            .build();
        assert_eq!(key, "query_result:query123");

        let key = CacheKeyBuilder::new(ContextType::TextChunk)
            .identifier("chunk456")
            .param("doc", "doc789")
            .param("pos", "0")
            .build();
        assert!(key.starts_with("text_chunk:chunk456"));
        assert!(key.contains("doc=doc789"));
    }

    #[test]
    fn test_cached_query_result_serialization() {
        let result = CachedQueryResult::new(
            "MATCH (n) RETURN n".to_string(),
            r#"{"nodes": []}"#.to_string(),
            0,
            100,
        );

        let value = result.to_cache_value().unwrap();
        let deserialized = CachedQueryResult::from_cache_value(&value).unwrap();

        assert_eq!(result.query, deserialized.query);
        assert_eq!(result.record_count, deserialized.record_count);
    }

    #[test]
    fn test_cached_context_chunk() {
        let chunk = CachedContextChunk {
            chunk_id: "chunk1".to_string(),
            content: "test content".to_string(),
            document_id: "doc1".to_string(),
            position: 0,
            relevance_score: Some(0.95),
            metadata: std::collections::HashMap::new(),
        };

        let key = chunk.cache_key();
        assert!(key.contains("text_chunk:chunk1"));
        assert!(key.contains("doc=doc1"));

        let value = chunk.to_cache_value().unwrap();
        let deserialized = CachedContextChunk::from_cache_value(&value).unwrap();
        assert_eq!(chunk.chunk_id, deserialized.chunk_id);
    }

    #[tokio::test]
    async fn test_knowledge_graph_cache_query() {
        let config = CacheConfig::builder()
            .default_ttl(Duration::from_secs(60))
            .build();
        let kg_cache = KnowledgeGraphCache::new(config);

        let query = "MATCH (n:Person) RETURN n.name";
        let result = CachedQueryResult::new(
            query.to_string(),
            r#"[{"name": "Alice"}]"#.to_string(),
            1,
            50,
        );

        kg_cache.cache_query_result(query, result.clone()).await.unwrap();

        let cached = kg_cache.get_query_result(query).await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().record_count, 1);
    }

    #[tokio::test]
    async fn test_knowledge_graph_cache_node() {
        let config = CacheConfig::default();
        let kg_cache = KnowledgeGraphCache::new(config);

        kg_cache.cache_node("node123", r#"{"name": "Alice"}"#.to_string())
            .await
            .unwrap();

        let cached = kg_cache.get_node("node123").await.unwrap();
        assert!(cached.is_some());
        assert!(cached.unwrap().contains("Alice"));
    }

    #[tokio::test]
    async fn test_knowledge_graph_cache_chunk() {
        let config = CacheConfig::default();
        let kg_cache = KnowledgeGraphCache::new(config);

        let chunk = CachedContextChunk {
            chunk_id: "chunk1".to_string(),
            content: "test content".to_string(),
            document_id: "doc1".to_string(),
            position: 0,
            relevance_score: Some(0.95),
            metadata: std::collections::HashMap::new(),
        };

        kg_cache.cache_chunk(chunk.clone()).await.unwrap();

        let cached = kg_cache.get_chunk("chunk1", "doc1").await.unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "test content");
    }
}
