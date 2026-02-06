//! Knowledge module for Ouroboros
//!
//! This module provides:
//! - Knowledge types (KnowledgeCategory, KnowledgeEntry)
//! - Knowledge extraction from task execution results
//! - Integration with SearchEngine for cross-session retrieval

mod extractor;

pub use extractor::KnowledgeExtractor;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Category of knowledge extracted from task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCategory {
    /// Reusable implementation patterns
    Pattern,
    /// API/library usage examples
    ApiUsage,
    /// Configuration/setup steps
    Configuration,
    /// Architecture decisions and rationale
    Architecture,
    /// Failed approaches to avoid
    AntiPattern,
    /// Error resolution methods
    ErrorResolution,
    /// Useful commands/tools discovered
    ToolUsage,
}

impl KnowledgeCategory {
    /// Get all categories for success extraction
    pub fn success_categories() -> &'static [KnowledgeCategory] {
        &[
            KnowledgeCategory::Pattern,
            KnowledgeCategory::ApiUsage,
            KnowledgeCategory::Configuration,
            KnowledgeCategory::Architecture,
            KnowledgeCategory::ToolUsage,
        ]
    }

    /// Get all categories for failure extraction
    pub fn failure_categories() -> &'static [KnowledgeCategory] {
        &[
            KnowledgeCategory::AntiPattern,
            KnowledgeCategory::ErrorResolution,
            KnowledgeCategory::Configuration,
        ]
    }

    /// Convert to display string
    pub fn as_str(&self) -> &'static str {
        match self {
            KnowledgeCategory::Pattern => "pattern",
            KnowledgeCategory::ApiUsage => "api_usage",
            KnowledgeCategory::Configuration => "configuration",
            KnowledgeCategory::Architecture => "architecture",
            KnowledgeCategory::AntiPattern => "anti_pattern",
            KnowledgeCategory::ErrorResolution => "error_resolution",
            KnowledgeCategory::ToolUsage => "tool_usage",
        }
    }
}

/// A single knowledge entry extracted from task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Unique identifier for this knowledge entry
    pub id: String,
    /// Category of the knowledge
    pub category: KnowledgeCategory,
    /// Short descriptive title
    pub title: String,
    /// Full content/explanation
    pub content: String,
    /// ID of the task this was extracted from
    pub source_task_id: String,
    /// Session ID where this was extracted (for cross-session search)
    pub source_session_id: Option<String>,
    /// Tags for better searchability
    pub tags: Vec<String>,
    /// When this knowledge was created
    pub created_at: DateTime<Utc>,
}

impl KnowledgeEntry {
    /// Create a new knowledge entry
    pub fn new(
        category: KnowledgeCategory,
        title: impl Into<String>,
        content: impl Into<String>,
        source_task_id: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("knowledge-{}", uuid::Uuid::new_v4()),
            category,
            title: title.into(),
            content: content.into(),
            source_task_id: source_task_id.into(),
            source_session_id: None,
            tags: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Set source session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.source_session_id = Some(session_id.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Format for indexing in search engine
    pub fn to_search_content(&self) -> String {
        let tags_str = if self.tags.is_empty() {
            String::new()
        } else {
            format!("\n\nTags: {}", self.tags.join(", "))
        };

        format!(
            "[{}] {}\n\n{}{}",
            self.category.as_str(),
            self.title,
            self.content,
            tags_str
        )
    }
}
