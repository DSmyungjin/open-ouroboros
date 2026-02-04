//! Type definitions for knowledge graph nodes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Task status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is pending execution
    Pending,
    /// Task is currently in progress
    InProgress,
    /// Task has been completed
    Completed,
    /// Task has failed
    Failed,
}

impl TaskStatus {
    /// Convert status to string for Neo4j storage
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
        }
    }

    /// Parse status from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" | "inprogress" => Some(TaskStatus::InProgress),
            "completed" => Some(TaskStatus::Completed),
            "failed" => Some(TaskStatus::Failed),
            _ => None,
        }
    }
}

/// Task node representing a unit of work in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task
    pub id: String,
    /// Subject/title of the task
    pub subject: String,
    /// Detailed description of the task
    pub description: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// Timestamp when the task was created
    pub created_at: DateTime<Utc>,
}

impl Task {
    /// Create a new task with the given parameters
    pub fn new(id: String, subject: String, description: String) -> Self {
        Self {
            id,
            subject,
            description,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
        }
    }

    /// Create a new task with explicit status and timestamp
    pub fn with_status(
        id: String,
        subject: String,
        description: String,
        status: TaskStatus,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject,
            description,
            status,
            created_at,
        }
    }
}

/// Result node representing the output of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    /// Unique identifier for the result
    pub id: String,
    /// Content of the result (can be text, JSON, etc.)
    pub content: String,
    /// Timestamp when the result was created
    pub created_at: DateTime<Utc>,
}

impl Result {
    /// Create a new result with the given content
    pub fn new(id: String, content: String) -> Self {
        Self {
            id,
            content,
            created_at: Utc::now(),
        }
    }

    /// Create a new result with explicit timestamp
    pub fn with_timestamp(id: String, content: String, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            content,
            created_at,
        }
    }
}

/// Context node representing contextual information or resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Unique identifier for the context
    pub id: String,
    /// Name/title of the context
    pub name: String,
    /// Additional metadata as JSON
    pub metadata: JsonValue,
}

impl Context {
    /// Create a new context with the given name
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            metadata: JsonValue::Object(serde_json::Map::new()),
        }
    }

    /// Create a new context with metadata
    pub fn with_metadata(id: String, name: String, metadata: JsonValue) -> Self {
        Self { id, name, metadata }
    }
}

/// Knowledge type enum for categorizing accumulated knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    /// A decision made (e.g., technology selection)
    Decision,
    /// A failure record with lessons learned
    Failure,
    /// An experiment or hypothesis test
    Experiment,
    /// A learning or insight gained
    Learning,
    /// An abandoned approach
    Abandoned,
}

impl KnowledgeType {
    /// Convert to string for Neo4j storage
    pub fn as_str(&self) -> &'static str {
        match self {
            KnowledgeType::Decision => "decision",
            KnowledgeType::Failure => "failure",
            KnowledgeType::Experiment => "experiment",
            KnowledgeType::Learning => "learning",
            KnowledgeType::Abandoned => "abandoned",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "decision" => Some(KnowledgeType::Decision),
            "failure" => Some(KnowledgeType::Failure),
            "experiment" => Some(KnowledgeType::Experiment),
            "learning" => Some(KnowledgeType::Learning),
            "abandoned" => Some(KnowledgeType::Abandoned),
            _ => None,
        }
    }
}

/// Knowledge node representing accumulated project knowledge
///
/// This is a flexible node type that can represent:
/// - Decisions: Technology choices, architectural decisions
/// - Failures: Implementation failures with causes
/// - Experiments: Hypothesis tests and their results
/// - Learnings: Insights and patterns discovered
/// - Abandoned: Approaches that were tried and discarded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    /// Unique identifier
    pub id: String,
    /// Type of knowledge
    pub knowledge_type: KnowledgeType,
    /// Title/summary of the knowledge
    pub title: String,
    /// Detailed content
    pub content: String,
    /// Additional structured metadata (reason, alternatives, metrics, etc.)
    pub metadata: JsonValue,
    /// Timestamp when this knowledge was recorded
    pub created_at: DateTime<Utc>,
    /// Reference to the source task that generated this knowledge
    pub source_task_id: Option<String>,
}

impl Knowledge {
    /// Create a new knowledge node
    pub fn new(
        id: String,
        knowledge_type: KnowledgeType,
        title: String,
        content: String,
    ) -> Self {
        Self {
            id,
            knowledge_type,
            title,
            content,
            metadata: JsonValue::Object(serde_json::Map::new()),
            created_at: Utc::now(),
            source_task_id: None,
        }
    }

    /// Create a decision knowledge node
    pub fn decision(id: String, title: String, choice: &str, alternatives: Vec<&str>, reason: &str) -> Self {
        let mut metadata = serde_json::Map::new();
        metadata.insert("choice".to_string(), JsonValue::String(choice.to_string()));
        metadata.insert("alternatives".to_string(), JsonValue::Array(
            alternatives.iter().map(|a| JsonValue::String(a.to_string())).collect()
        ));
        metadata.insert("reason".to_string(), JsonValue::String(reason.to_string()));

        Self {
            id,
            knowledge_type: KnowledgeType::Decision,
            title,
            content: format!("Selected {} over {:?}. Reason: {}", choice, alternatives, reason),
            metadata: JsonValue::Object(metadata),
            created_at: Utc::now(),
            source_task_id: None,
        }
    }

    /// Create a failure knowledge node
    pub fn failure(id: String, title: String, approach: &str, symptom: &str, cause: &str) -> Self {
        let mut metadata = serde_json::Map::new();
        metadata.insert("approach".to_string(), JsonValue::String(approach.to_string()));
        metadata.insert("symptom".to_string(), JsonValue::String(symptom.to_string()));
        metadata.insert("cause".to_string(), JsonValue::String(cause.to_string()));

        Self {
            id,
            knowledge_type: KnowledgeType::Failure,
            title,
            content: format!("Approach '{}' failed. Symptom: {}. Cause: {}", approach, symptom, cause),
            metadata: JsonValue::Object(metadata),
            created_at: Utc::now(),
            source_task_id: None,
        }
    }

    /// Create an experiment knowledge node
    pub fn experiment(id: String, title: String, hypothesis: &str, result: &str, confirmed: bool) -> Self {
        let mut metadata = serde_json::Map::new();
        metadata.insert("hypothesis".to_string(), JsonValue::String(hypothesis.to_string()));
        metadata.insert("result".to_string(), JsonValue::String(result.to_string()));
        metadata.insert("confirmed".to_string(), JsonValue::Bool(confirmed));

        Self {
            id,
            knowledge_type: KnowledgeType::Experiment,
            title,
            content: format!("Hypothesis: {}. Result: {}. {}",
                hypothesis, result, if confirmed { "CONFIRMED" } else { "REFUTED" }),
            metadata: JsonValue::Object(metadata),
            created_at: Utc::now(),
            source_task_id: None,
        }
    }

    /// Set the source task ID
    pub fn with_source_task(mut self, task_id: String) -> Self {
        self.source_task_id = Some(task_id);
        self
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_conversion() {
        assert_eq!(TaskStatus::Pending.as_str(), "pending");
        assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
        assert_eq!(TaskStatus::Completed.as_str(), "completed");
        assert_eq!(TaskStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_task_status_parsing() {
        assert_eq!(TaskStatus::from_str("pending"), Some(TaskStatus::Pending));
        assert_eq!(
            TaskStatus::from_str("in_progress"),
            Some(TaskStatus::InProgress)
        );
        assert_eq!(
            TaskStatus::from_str("completed"),
            Some(TaskStatus::Completed)
        );
        assert_eq!(TaskStatus::from_str("failed"), Some(TaskStatus::Failed));
        assert_eq!(TaskStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_task_creation() {
        let task = Task::new(
            "task-001".to_string(),
            "Test Task".to_string(),
            "Description".to_string(),
        );

        assert_eq!(task.id, "task-001");
        assert_eq!(task.subject, "Test Task");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_result_creation() {
        let result = Result::new("result-001".to_string(), "Test content".to_string());

        assert_eq!(result.id, "result-001");
        assert_eq!(result.content, "Test content");
    }

    #[test]
    fn test_context_creation() {
        let context = Context::new("ctx-001".to_string(), "Test Context".to_string());

        assert_eq!(context.id, "ctx-001");
        assert_eq!(context.name, "Test Context");
        assert!(context.metadata.is_object());
    }
}
