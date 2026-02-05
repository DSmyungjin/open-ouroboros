//! Hybrid search combining vector and keyword search with fusion

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::{debug, info};

use super::keyword::KeywordSearch;
use super::types::{SearchDocument, SearchOptions, SearchResult, SearchSource};
use super::vector::VectorSearch;

/// Fusion strategy for combining search results
#[derive(Debug, Clone, Copy, Default)]
pub enum FusionStrategy {
    /// Reciprocal Rank Fusion (default)
    #[default]
    RRF,
    /// Distribution-Based Score Fusion
    DBSF,
    /// Simple score averaging
    Average,
}

/// Configuration for hybrid search
#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Weight for vector search results (0.0 - 1.0)
    pub vector_weight: f32,
    /// Weight for keyword search results (0.0 - 1.0)
    pub keyword_weight: f32,
    /// Fusion strategy
    pub fusion: FusionStrategy,
    /// RRF k parameter (default: 60)
    pub rrf_k: f32,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.5,
            keyword_weight: 0.5,
            fusion: FusionStrategy::RRF,
            rrf_k: 60.0,
        }
    }
}

impl HybridConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_weights(mut self, vector: f32, keyword: f32) -> Self {
        self.vector_weight = vector;
        self.keyword_weight = keyword;
        self
    }

    pub fn with_fusion(mut self, fusion: FusionStrategy) -> Self {
        self.fusion = fusion;
        self
    }

    pub fn with_rrf_k(mut self, k: f32) -> Self {
        self.rrf_k = k;
        self
    }
}

/// Hybrid search engine combining vector and keyword search
pub struct HybridSearch {
    vector_search: VectorSearch,
    keyword_search: KeywordSearch,
    config: HybridConfig,
}

impl HybridSearch {
    /// Create a new hybrid search instance
    pub async fn new(
        vector_db_path: impl AsRef<Path>,
        keyword_index_path: impl AsRef<Path>,
    ) -> Result<Self> {
        info!("Initializing hybrid search engine");

        let vector_search = VectorSearch::new(vector_db_path).await?;
        let keyword_search = KeywordSearch::new(keyword_index_path)?;

        Ok(Self {
            vector_search,
            keyword_search,
            config: HybridConfig::default(),
        })
    }

    /// Create with custom configuration
    pub async fn with_config(
        vector_db_path: impl AsRef<Path>,
        keyword_index_path: impl AsRef<Path>,
        config: HybridConfig,
    ) -> Result<Self> {
        let mut search = Self::new(vector_db_path, keyword_index_path).await?;
        search.config = config;
        Ok(search)
    }

    /// Set configuration
    pub fn set_config(&mut self, config: HybridConfig) {
        self.config = config;
    }

    /// Index a document in both vector and keyword indices
    pub async fn index_document(&mut self, doc: &SearchDocument, embedding: Vec<f32>) -> Result<()> {
        // Index in vector store
        self.vector_search.index_document(doc, embedding).await?;

        // Index in keyword store
        self.keyword_search.index_document(doc)?;
        self.keyword_search.commit()?;

        debug!("Indexed document in hybrid search: {}", doc.id);
        Ok(())
    }

    /// Perform hybrid search
    pub async fn search(
        &self,
        query: &str,
        query_embedding: Vec<f32>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // Get results from both search engines
        let vector_results = self.vector_search.search(query_embedding, options).await?;
        let keyword_results = self.keyword_search.search(query, options)?;

        debug!(
            "Vector results: {}, Keyword results: {}",
            vector_results.len(),
            keyword_results.len()
        );

        // Fuse results based on strategy
        let fused = match self.config.fusion {
            FusionStrategy::RRF => {
                self.fuse_rrf(&vector_results, &keyword_results, options.limit)
            }
            FusionStrategy::DBSF => {
                self.fuse_dbsf(&vector_results, &keyword_results, options.limit)
            }
            FusionStrategy::Average => {
                self.fuse_average(&vector_results, &keyword_results, options.limit)
            }
        };

        Ok(fused)
    }

    /// Reciprocal Rank Fusion
    fn fuse_rrf(
        &self,
        vector_results: &[SearchResult],
        keyword_results: &[SearchResult],
        limit: usize,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, (f32, Option<SearchResult>)> = HashMap::new();
        let k = self.config.rrf_k;

        // Process vector results
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = self.config.vector_weight / (k + rank as f32 + 1.0);
            scores
                .entry(result.id.clone())
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, Some(result.clone())));
        }

        // Process keyword results
        for (rank, result) in keyword_results.iter().enumerate() {
            let rrf_score = self.config.keyword_weight / (k + rank as f32 + 1.0);
            scores
                .entry(result.id.clone())
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, Some(result.clone())));
        }

        // Sort by fused score
        let mut results: Vec<_> = scores
            .into_iter()
            .filter_map(|(_, (score, result))| {
                result.map(|mut r| {
                    r.score = score;
                    r.source = SearchSource::Hybrid;
                    r
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Distribution-Based Score Fusion (normalize scores to same distribution)
    fn fuse_dbsf(
        &self,
        vector_results: &[SearchResult],
        keyword_results: &[SearchResult],
        limit: usize,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, (f32, Option<SearchResult>)> = HashMap::new();

        // Normalize vector scores
        let (v_min, v_max) = self.get_score_range(vector_results);
        for result in vector_results {
            let normalized = if v_max > v_min {
                (result.score - v_min) / (v_max - v_min)
            } else {
                result.score
            };
            let weighted = normalized * self.config.vector_weight;
            scores
                .entry(result.id.clone())
                .and_modify(|(s, _)| *s += weighted)
                .or_insert((weighted, Some(result.clone())));
        }

        // Normalize keyword scores
        let (k_min, k_max) = self.get_score_range(keyword_results);
        for result in keyword_results {
            let normalized = if k_max > k_min {
                (result.score - k_min) / (k_max - k_min)
            } else {
                result.score
            };
            let weighted = normalized * self.config.keyword_weight;
            scores
                .entry(result.id.clone())
                .and_modify(|(s, _)| *s += weighted)
                .or_insert((weighted, Some(result.clone())));
        }

        // Sort and return
        let mut results: Vec<_> = scores
            .into_iter()
            .filter_map(|(_, (score, result))| {
                result.map(|mut r| {
                    r.score = score;
                    r.source = SearchSource::Hybrid;
                    r
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Simple score averaging
    fn fuse_average(
        &self,
        vector_results: &[SearchResult],
        keyword_results: &[SearchResult],
        limit: usize,
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, (f32, usize, Option<SearchResult>)> = HashMap::new();

        // Add vector scores
        for result in vector_results {
            let weighted = result.score * self.config.vector_weight;
            scores
                .entry(result.id.clone())
                .and_modify(|(s, c, _)| {
                    *s += weighted;
                    *c += 1;
                })
                .or_insert((weighted, 1, Some(result.clone())));
        }

        // Add keyword scores
        for result in keyword_results {
            let weighted = result.score * self.config.keyword_weight;
            scores
                .entry(result.id.clone())
                .and_modify(|(s, c, _)| {
                    *s += weighted;
                    *c += 1;
                })
                .or_insert((weighted, 1, Some(result.clone())));
        }

        // Average and sort
        let mut results: Vec<_> = scores
            .into_iter()
            .filter_map(|(_, (score, count, result))| {
                result.map(|mut r| {
                    r.score = score / count as f32;
                    r.source = SearchSource::Hybrid;
                    r
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    fn get_score_range(&self, results: &[SearchResult]) -> (f32, f32) {
        if results.is_empty() {
            return (0.0, 1.0);
        }
        let min = results.iter().map(|r| r.score).fold(f32::INFINITY, f32::min);
        let max = results.iter().map(|r| r.score).fold(f32::NEG_INFINITY, f32::max);
        (min, max)
    }

    /// Delete a document from both indices
    pub async fn delete_document(&mut self, doc_id: &str) -> Result<()> {
        self.vector_search.delete_document(doc_id).await?;
        self.keyword_search.delete_document(doc_id)?;
        self.keyword_search.commit()?;
        Ok(())
    }

    /// Get total document count
    pub async fn count(&self) -> Result<(usize, usize)> {
        let vector_count = self.vector_search.count().await?;
        let keyword_count = self.keyword_search.count()?;
        Ok((vector_count, keyword_count))
    }

    /// Get reference to vector search
    pub fn vector_search(&self) -> &VectorSearch {
        &self.vector_search
    }

    /// Get mutable reference to vector search
    pub fn vector_search_mut(&mut self) -> &mut VectorSearch {
        &mut self.vector_search
    }

    /// Get reference to keyword search
    pub fn keyword_search(&self) -> &KeywordSearch {
        &self.keyword_search
    }

    /// Get mutable reference to keyword search
    pub fn keyword_search_mut(&mut self) -> &mut KeywordSearch {
        &mut self.keyword_search
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_config_default() {
        let config = HybridConfig::default();
        assert_eq!(config.vector_weight, 0.5);
        assert_eq!(config.keyword_weight, 0.5);
    }

    #[test]
    fn test_hybrid_config_custom() {
        let config = HybridConfig::new()
            .with_weights(0.7, 0.3)
            .with_fusion(FusionStrategy::DBSF)
            .with_rrf_k(50.0);

        assert_eq!(config.vector_weight, 0.7);
        assert_eq!(config.keyword_weight, 0.3);
        assert_eq!(config.rrf_k, 50.0);
    }
}
