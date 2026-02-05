//! Common types for search module

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Document to be indexed for search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    /// Unique document ID
    pub id: String,
    /// Document type (task, result, context, etc.)
    pub doc_type: DocumentType,
    /// Title or subject
    pub title: String,
    /// Main content body
    pub content: String,
    /// Associated session ID
    pub session_id: Option<String>,
    /// Associated task ID
    pub task_id: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Additional metadata as JSON
    pub metadata: Option<serde_json::Value>,
}

/// Types of documents that can be searched
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Task,
    TaskResult,
    Context,
    Plan,
    ValidationReport,
    Knowledge,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::TaskResult => "task_result",
            Self::Context => "context",
            Self::Plan => "plan",
            Self::ValidationReport => "validation_report",
            Self::Knowledge => "knowledge",
        }
    }
}

/// Search result with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID
    pub id: String,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
    /// Document type
    pub doc_type: DocumentType,
    /// Title
    pub title: String,
    /// Content snippet or full content
    pub content: String,
    /// Search method that found this result
    pub source: SearchSource,
}

/// Source of search result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSource {
    Vector,
    Keyword,
    Hybrid,
}

/// Search query options
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Maximum number of results
    pub limit: usize,
    /// Filter by document type
    pub doc_type: Option<DocumentType>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Minimum score threshold
    pub min_score: Option<f32>,
    /// Filter by date range - start date (inclusive)
    pub date_from: Option<DateTime<Utc>>,
    /// Filter by date range - end date (inclusive)
    pub date_to: Option<DateTime<Utc>>,
}

impl SearchOptions {
    pub fn new() -> Self {
        Self {
            limit: 10,
            doc_type: None,
            session_id: None,
            min_score: None,
            date_from: None,
            date_to: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_doc_type(mut self, doc_type: DocumentType) -> Self {
        self.doc_type = Some(doc_type);
        self
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    pub fn with_date_from(mut self, date_from: DateTime<Utc>) -> Self {
        self.date_from = Some(date_from);
        self
    }

    pub fn with_date_to(mut self, date_to: DateTime<Utc>) -> Self {
        self.date_to = Some(date_to);
        self
    }

    pub fn with_date_range(mut self, date_from: DateTime<Utc>, date_to: DateTime<Utc>) -> Self {
        self.date_from = Some(date_from);
        self.date_to = Some(date_to);
        self
    }
}
