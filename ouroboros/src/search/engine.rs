//! Unified search engine for Ouroboros
//!
//! This module provides a high-level interface for the search subsystem,
//! using BM25-based keyword search via Tantivy.

use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use tracing::{debug, info};

use super::keyword::KeywordSearch;
use super::types::{DocumentType, SearchDocument, SearchOptions, SearchResult};

/// Search engine mode
#[derive(Debug, Clone, Copy, Default)]
pub enum SearchMode {
    /// Keyword search only (BM25)
    #[default]
    KeywordOnly,
}

/// Unified search engine for Ouroboros
pub struct SearchEngine {
    keyword_search: KeywordSearch,
}

impl SearchEngine {
    /// Create a keyword-only search engine
    pub fn keyword_only(index_path: impl AsRef<Path>) -> Result<Self> {
        info!("Initializing keyword-only search engine");
        let keyword_search = KeywordSearch::new(index_path)?;

        Ok(Self { keyword_search })
    }

    /// Create a keyword-only search engine in reader mode (no write lock acquired)
    /// Use this for search-only operations to avoid lock conflicts with other processes
    pub fn keyword_reader_only(index_path: impl AsRef<Path>) -> Result<Self> {
        info!("Initializing keyword-only search engine (reader mode)");
        let keyword_search = KeywordSearch::new_reader_only(index_path)?;

        Ok(Self { keyword_search })
    }

    /// Get current search mode
    pub fn mode(&self) -> SearchMode {
        SearchMode::KeywordOnly
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

        self.index_document_internal(&doc).await
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

        self.index_document_internal(&doc).await
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

        self.index_document_internal(&doc).await
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

        self.index_document_internal(&doc).await
    }

    /// Internal document indexing
    async fn index_document_internal(&mut self, doc: &SearchDocument) -> Result<()> {
        self.keyword_search.index_document(doc)?;
        self.keyword_search.commit()?;
        debug!("Indexed document: {}", doc.id);
        Ok(())
    }

    /// Search for documents
    pub async fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        self.keyword_search.search(query, options)
    }

    /// Get document count
    pub async fn count(&self) -> Result<usize> {
        self.keyword_search.count()
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
