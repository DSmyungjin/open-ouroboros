//! Unified search engine for Ouroboros
//!
//! This module provides a high-level interface for the search subsystem,
//! integrating with the Orchestrator for automatic document indexing.

use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use tracing::{debug, info, warn};

use super::hybrid::{HybridConfig, HybridSearch};
use super::keyword::KeywordSearch;
use super::types::{DocumentType, SearchDocument, SearchOptions, SearchResult};

/// Search engine mode
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchMode {
    /// Keyword search only (no embedding model required)
    #[default]
    KeywordOnly,
    /// Vector search only (requires embeddings)
    VectorOnly,
    /// Hybrid search (requires embeddings)
    Hybrid,
}

/// Unified search engine for Ouroboros
pub struct SearchEngine {
    keyword_search: Option<KeywordSearch>,
    hybrid_search: Option<HybridSearch>,
    mode: SearchMode,
}

impl SearchEngine {
    /// Create a keyword-only search engine (no embedding model required)
    pub fn keyword_only(index_path: impl AsRef<Path>) -> Result<Self> {
        info!("Initializing keyword-only search engine");
        let keyword_search = KeywordSearch::new(index_path)?;

        Ok(Self {
            keyword_search: Some(keyword_search),
            hybrid_search: None,
            mode: SearchMode::KeywordOnly,
        })
    }

    /// Create a keyword-only search engine in reader mode (no write lock acquired)
    /// Use this for search-only operations to avoid lock conflicts with other processes
    pub fn keyword_reader_only(index_path: impl AsRef<Path>) -> Result<Self> {
        info!("Initializing keyword-only search engine (reader mode)");
        let keyword_search = KeywordSearch::new_reader_only(index_path)?;

        Ok(Self {
            keyword_search: Some(keyword_search),
            hybrid_search: None,
            mode: SearchMode::KeywordOnly,
        })
    }

    /// Create a hybrid search engine (requires embedding model)
    pub async fn hybrid(
        vector_db_path: impl AsRef<Path>,
        keyword_index_path: impl AsRef<Path>,
        config: Option<HybridConfig>,
    ) -> Result<Self> {
        info!("Initializing hybrid search engine");

        let hybrid_search = match config {
            Some(cfg) => {
                HybridSearch::with_config(vector_db_path, keyword_index_path, cfg).await?
            }
            None => HybridSearch::new(vector_db_path, keyword_index_path).await?,
        };

        Ok(Self {
            keyword_search: None,
            hybrid_search: Some(hybrid_search),
            mode: SearchMode::Hybrid,
        })
    }

    /// Get current search mode
    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    /// Index a task document
    pub async fn index_task(
        &mut self,
        task_id: &str,
        subject: &str,
        description: &str,
        session_id: Option<&str>,
    ) -> Result<()> {
        let doc = SearchDocument {
            id: format!("task:{}", task_id),
            doc_type: DocumentType::Task,
            title: subject.to_string(),
            content: description.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            task_id: Some(task_id.to_string()),
            created_at: Utc::now(),
            metadata: None,
        };

        self.index_document_internal(&doc, None).await
    }

    /// Index a task result document
    pub async fn index_task_result(
        &mut self,
        task_id: &str,
        result_content: &str,
        session_id: Option<&str>,
    ) -> Result<()> {
        let doc = SearchDocument {
            id: format!("result:{}", task_id),
            doc_type: DocumentType::TaskResult,
            title: format!("Result for {}", task_id),
            content: result_content.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            task_id: Some(task_id.to_string()),
            created_at: Utc::now(),
            metadata: None,
        };

        self.index_document_internal(&doc, None).await
    }

    /// Index a context document
    pub async fn index_context(
        &mut self,
        context_id: &str,
        title: &str,
        content: &str,
        session_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<()> {
        let doc = SearchDocument {
            id: format!("context:{}", context_id),
            doc_type: DocumentType::Context,
            title: title.to_string(),
            content: content.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            task_id: task_id.map(|s| s.to_string()),
            created_at: Utc::now(),
            metadata: None,
        };

        self.index_document_internal(&doc, None).await
    }

    /// Index a knowledge entry
    pub async fn index_knowledge(
        &mut self,
        knowledge_id: &str,
        title: &str,
        content: &str,
        session_id: Option<&str>,
    ) -> Result<()> {
        let doc = SearchDocument {
            id: format!("knowledge:{}", knowledge_id),
            doc_type: DocumentType::Knowledge,
            title: title.to_string(),
            content: content.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            task_id: None,
            created_at: Utc::now(),
            metadata: None,
        };

        self.index_document_internal(&doc, None).await
    }

    /// Internal document indexing
    async fn index_document_internal(
        &mut self,
        doc: &SearchDocument,
        embedding: Option<Vec<f32>>,
    ) -> Result<()> {
        match self.mode {
            SearchMode::KeywordOnly => {
                if let Some(ref mut ks) = self.keyword_search {
                    ks.index_document(doc)?;
                    ks.commit()?;
                }
            }
            SearchMode::Hybrid => {
                if let Some(ref mut hs) = self.hybrid_search {
                    // For hybrid mode, we need embeddings
                    // If not provided, fall back to keyword-only for this document
                    if let Some(emb) = embedding {
                        hs.index_document(doc, emb).await?;
                    } else {
                        warn!(
                            "No embedding provided for document {}, indexing keyword-only",
                            doc.id
                        );
                        hs.keyword_search_mut().index_document(doc)?;
                        hs.keyword_search_mut().commit()?;
                    }
                }
            }
            SearchMode::VectorOnly => {
                warn!("Vector-only mode requires embeddings, skipping document {}", doc.id);
            }
        }

        debug!("Indexed document: {}", doc.id);
        Ok(())
    }

    /// Search for documents
    pub async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        match self.mode {
            SearchMode::KeywordOnly => {
                if let Some(ref ks) = self.keyword_search {
                    ks.search(query, options)
                } else {
                    Ok(vec![])
                }
            }
            SearchMode::Hybrid | SearchMode::VectorOnly => {
                // For hybrid/vector mode without embedding, fall back to keyword
                warn!("Hybrid/Vector search requires query embedding, falling back to keyword search");
                if let Some(ref hs) = self.hybrid_search {
                    hs.keyword_search().search(query, options)
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    /// Search with embedding (for hybrid/vector modes)
    pub async fn search_with_embedding(
        &self,
        query: &str,
        query_embedding: Vec<f32>,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        match self.mode {
            SearchMode::KeywordOnly => {
                if let Some(ref ks) = self.keyword_search {
                    ks.search(query, options)
                } else {
                    Ok(vec![])
                }
            }
            SearchMode::Hybrid => {
                if let Some(ref hs) = self.hybrid_search {
                    hs.search(query, query_embedding, options).await
                } else {
                    Ok(vec![])
                }
            }
            SearchMode::VectorOnly => {
                if let Some(ref hs) = self.hybrid_search {
                    hs.vector_search().search(query_embedding, options).await
                } else {
                    Ok(vec![])
                }
            }
        }
    }

    /// Get document count
    pub async fn count(&self) -> Result<usize> {
        match self.mode {
            SearchMode::KeywordOnly => {
                if let Some(ref ks) = self.keyword_search {
                    ks.count()
                } else {
                    Ok(0)
                }
            }
            SearchMode::Hybrid | SearchMode::VectorOnly => {
                if let Some(ref hs) = self.hybrid_search {
                    let (_, keyword_count) = hs.count().await?;
                    Ok(keyword_count)
                } else {
                    Ok(0)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_keyword_only_engine() {
        let dir = tempdir().unwrap();
        let engine = SearchEngine::keyword_only(dir.path()).unwrap();
        assert!(matches!(engine.mode(), SearchMode::KeywordOnly));
    }
}
