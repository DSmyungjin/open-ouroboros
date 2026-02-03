use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Task type distinguishing context preparation from actual work
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
    /// Context fill task - prepares documents for a context tree node
    ContextFill {
        /// The context node this task fills
        target_node: String,
    },
    /// Worker task - actual implementation work
    #[default]
    Worker,
}

/// Context captured from a failed attempt for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttemptContext {
    pub attempt_num: u32,
    pub output: String,
    pub feedback: String,
    pub severity: String,
    pub timestamp: DateTime<Utc>,
}

impl AttemptContext {
    pub fn new(attempt_num: u32, output: String, feedback: String, severity: String) -> Self {
        Self {
            attempt_num,
            output,
            feedback,
            severity,
            timestamp: Utc::now(),
        }
    }

    /// Format as context for next attempt
    pub fn as_context(&self) -> String {
        format!(
            "## Previous Attempt #{} ({})\n\n### Output:\n{}\n\n### Feedback ({}):\n{}\n",
            self.attempt_num,
            self.timestamp.format("%Y-%m-%d %H:%M"),
            self.output,
            self.severity,
            self.feedback
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: TaskStatus,
    /// Task type: ContextFill or Worker
    #[serde(default)]
    pub task_type: TaskType,
    /// Context node to reference for this task's context
    /// Worker tasks use this to load the right documents
    pub context_ref: Option<String>,
    pub session_id: Option<String>,
    pub result_doc: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Previous attempt contexts for retry learning
    #[serde(default)]
    pub attempts: Vec<AttemptContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
}

impl Task {
    pub fn new(subject: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: format!("task-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            subject: subject.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            task_type: TaskType::Worker,
            context_ref: None,
            session_id: None,
            result_doc: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            attempts: vec![],
        }
    }

    /// Create a context fill task
    pub fn new_context_fill(
        subject: impl Into<String>,
        description: impl Into<String>,
        target_node: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("ctx-fill-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            subject: subject.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            task_type: TaskType::ContextFill { target_node: target_node.into() },
            context_ref: None,
            session_id: None,
            result_doc: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            attempts: vec![],
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set context reference (which context node's docs to load)
    pub fn with_context_ref(mut self, context_ref: impl Into<String>) -> Self {
        self.context_ref = Some(context_ref.into());
        self
    }

    /// Check if this is a context fill task
    pub fn is_context_fill(&self) -> bool {
        matches!(self.task_type, TaskType::ContextFill { .. })
    }

    /// Get target context node (for context fill tasks)
    pub fn target_context_node(&self) -> Option<&str> {
        match &self.task_type {
            TaskType::ContextFill { target_node } => Some(target_node),
            TaskType::Worker => None,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.status, TaskStatus::Pending)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed { .. })
    }

    pub fn is_done(&self) -> bool {
        self.is_completed() || self.is_failed()
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self, result_doc: Option<PathBuf>) {
        self.status = TaskStatus::Completed;
        self.result_doc = result_doc;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = TaskStatus::Failed { reason: reason.into() };
        self.completed_at = Some(Utc::now());
    }

    /// Record a failed attempt for retry learning
    pub fn record_attempt(&mut self, output: String, feedback: String, severity: String) {
        let attempt_num = self.attempts.len() as u32 + 1;
        self.attempts.push(AttemptContext::new(attempt_num, output, feedback, severity));
    }

    /// Get current attempt number (1-based)
    pub fn current_attempt(&self) -> u32 {
        self.attempts.len() as u32 + 1
    }

    /// Get combined context from all previous attempts
    pub fn previous_attempts_context(&self) -> Option<String> {
        if self.attempts.is_empty() {
            return None;
        }

        let context = self.attempts
            .iter()
            .map(|a| a.as_context())
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        Some(format!(
            "# Previous Attempts ({} total)\n\nLearn from these failures and avoid repeating the same mistakes.\n\n{}\n",
            self.attempts.len(),
            context
        ))
    }

    /// Reset for retry (keeps attempt history)
    pub fn reset_for_retry(&mut self) {
        self.status = TaskStatus::Pending;
        self.started_at = None;
        self.completed_at = None;
        self.result_doc = None;
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("Test Task", "Do something");
        assert!(task.is_pending());

        task.start();
        assert!(matches!(task.status, TaskStatus::InProgress));

        task.complete(None);
        assert!(task.is_completed());
        assert!(task.is_done());
    }
}
