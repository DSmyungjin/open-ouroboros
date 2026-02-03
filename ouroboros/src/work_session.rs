//! Work session management for Ouroboros
//!
//! Manages execution sessions with git-style short hash identifiers.
//! Each session contains a complete execution state: DAG, context tree, tasks, results.

use std::path::{Path, PathBuf};
use anyhow::{Result, Context, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Work session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkSessionStatus {
    /// Session created but not started
    Pending,
    /// Tasks are being executed
    Running,
    /// All tasks completed successfully
    Completed,
    /// Some tasks failed
    Failed,
    /// Session was archived
    Archived,
}

/// Work session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    /// Short hash identifier (6-8 chars)
    pub id: String,
    /// Optional user-provided label
    pub label: Option<String>,
    /// Original goal/prompt
    pub goal: String,
    /// Current status
    pub status: WorkSessionStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
    /// Total number of tasks
    pub task_count: usize,
    /// Number of completed tasks
    pub completed_count: usize,
    /// Number of failed tasks
    pub failed_count: usize,
}

impl WorkSession {
    /// Create a new work session
    pub fn new(goal: impl Into<String>, label: Option<String>) -> Self {
        let id = generate_short_id();
        let full_id = match &label {
            Some(l) => format!("{}-{}", id, sanitize_label(l)),
            None => id,
        };

        Self {
            id: full_id,
            label,
            goal: goal.into(),
            status: WorkSessionStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            task_count: 0,
            completed_count: 0,
            failed_count: 0,
        }
    }

    /// Get the short ID (first 6 chars)
    pub fn short_id(&self) -> &str {
        if self.id.len() > 6 {
            &self.id[..6]
        } else {
            &self.id
        }
    }

    /// Mark session as running
    pub fn start(&mut self, task_count: usize) {
        self.status = WorkSessionStatus::Running;
        self.task_count = task_count;
    }

    /// Record task completion
    pub fn record_completion(&mut self, success: bool) {
        if success {
            self.completed_count += 1;
        } else {
            self.failed_count += 1;
        }

        // Check if all tasks are done
        if self.completed_count + self.failed_count >= self.task_count {
            self.completed_at = Some(Utc::now());
            if self.failed_count > 0 {
                self.status = WorkSessionStatus::Failed;
            } else {
                self.status = WorkSessionStatus::Completed;
            }
        }
    }

    /// Get display string for listing
    pub fn display_line(&self) -> String {
        let status_icon = match self.status {
            WorkSessionStatus::Pending => "○",
            WorkSessionStatus::Running => "◐",
            WorkSessionStatus::Completed => "●",
            WorkSessionStatus::Failed => "✗",
            WorkSessionStatus::Archived => "◌",
        };

        let label_str = self.label.as_ref()
            .map(|l| format!(" [{}]", l))
            .unwrap_or_default();

        let progress = format!("{}/{}", self.completed_count, self.task_count);
        let date = &self.created_at.format("%Y-%m-%d %H:%M").to_string();

        format!(
            "{} {}{}  {}  {}  {}",
            status_icon,
            self.short_id(),
            label_str,
            progress,
            date,
            truncate(&self.goal, 40)
        )
    }
}

/// Sessions index file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSessionsIndex {
    pub version: u32,
    pub current: Option<String>,
    pub sessions: Vec<WorkSessionSummary>,
}

impl Default for WorkSessionsIndex {
    fn default() -> Self {
        Self {
            version: 1,
            current: None,
            sessions: Vec::new(),
        }
    }
}

/// Summary entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSessionSummary {
    pub id: String,
    pub label: Option<String>,
    pub goal: String,
    pub status: WorkSessionStatus,
    pub created_at: DateTime<Utc>,
    pub task_count: usize,
    pub completed_count: usize,
}

impl From<&WorkSession> for WorkSessionSummary {
    fn from(session: &WorkSession) -> Self {
        Self {
            id: session.id.clone(),
            label: session.label.clone(),
            goal: session.goal.clone(),
            status: session.status.clone(),
            created_at: session.created_at,
            task_count: session.task_count,
            completed_count: session.completed_count,
        }
    }
}

/// Work session manager
pub struct WorkSessionManager {
    data_dir: PathBuf,
}

impl WorkSessionManager {
    /// Create a new work session manager
    pub fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        // Ensure sessions directory exists
        let sessions_dir = data_dir.join("sessions");
        if !sessions_dir.exists() {
            std::fs::create_dir_all(&sessions_dir)?;
        }

        Ok(Self { data_dir })
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    /// Get index file path
    pub fn index_path(&self) -> PathBuf {
        self.sessions_dir().join("index.json")
    }

    /// Get session directory
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(session_id)
    }

    /// Load sessions index
    pub fn load_index(&self) -> Result<WorkSessionsIndex> {
        let index_path = self.index_path();
        if !index_path.exists() {
            return Ok(WorkSessionsIndex::default());
        }

        let content = std::fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read: {:?}", index_path))?;

        serde_json::from_str(&content)
            .with_context(|| "Failed to parse sessions index")
    }

    /// Save sessions index
    pub fn save_index(&self, index: &WorkSessionsIndex) -> Result<()> {
        let content = serde_json::to_string_pretty(index)?;
        std::fs::write(self.index_path(), content)?;
        Ok(())
    }

    /// Create a new session
    pub fn create_session(&self, goal: &str, label: Option<String>) -> Result<WorkSession> {
        let session = WorkSession::new(goal, label);

        // Create session directory structure
        let session_dir = self.session_dir(&session.id);
        std::fs::create_dir_all(session_dir.join("tasks"))?;
        std::fs::create_dir_all(session_dir.join("results"))?;
        std::fs::create_dir_all(session_dir.join("contexts"))?;

        // Save session metadata
        self.save_session(&session)?;

        // Update index
        let mut index = self.load_index()?;
        index.sessions.push(WorkSessionSummary::from(&session));
        index.current = Some(session.id.clone());
        self.save_index(&index)?;

        // Update current symlink
        self.update_current_link(&session.id)?;

        Ok(session)
    }

    /// Save session metadata
    pub fn save_session(&self, session: &WorkSession) -> Result<()> {
        let session_file = self.session_dir(&session.id).join("session.json");
        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(session_file, content)?;
        Ok(())
    }

    /// Load session by ID
    pub fn load_session(&self, session_id: &str) -> Result<WorkSession> {
        // Try exact match first
        let session_file = self.session_dir(session_id).join("session.json");
        if session_file.exists() {
            let content = std::fs::read_to_string(&session_file)?;
            return serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse session: {}", session_id));
        }

        // Try prefix match
        let index = self.load_index()?;
        let matched = index.sessions.iter()
            .find(|s| s.id.starts_with(session_id))
            .ok_or_else(|| anyhow!("Session not found: {}", session_id))?;

        let session_file = self.session_dir(&matched.id).join("session.json");
        let content = std::fs::read_to_string(&session_file)?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse session: {}", matched.id))
    }

    /// Get current session ID
    pub fn current_session_id(&self) -> Result<Option<String>> {
        let index = self.load_index()?;
        Ok(index.current)
    }

    /// Get current session
    pub fn current_session(&self) -> Result<Option<WorkSession>> {
        match self.current_session_id()? {
            Some(id) => Ok(Some(self.load_session(&id)?)),
            None => Ok(None),
        }
    }

    /// Switch to a different session
    pub fn switch_session(&self, session_id: &str) -> Result<WorkSession> {
        let session = self.load_session(session_id)?;

        // Update index
        let mut index = self.load_index()?;
        index.current = Some(session.id.clone());
        self.save_index(&index)?;

        // Update symlink
        self.update_current_link(&session.id)?;

        Ok(session)
    }

    /// Update the 'current' symlink
    fn update_current_link(&self, session_id: &str) -> Result<()> {
        let current_link = self.data_dir.join("current");

        // Remove existing symlink if it exists
        if current_link.exists() || current_link.is_symlink() {
            std::fs::remove_file(&current_link).ok();
        }

        // Create new symlink
        #[cfg(unix)]
        {
            let target = PathBuf::from("sessions").join(session_id);
            std::os::unix::fs::symlink(&target, &current_link)
                .with_context(|| format!("Failed to create symlink: {:?}", current_link))?;
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, just write the session ID to a file
            std::fs::write(&current_link, session_id)?;
        }

        Ok(())
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<WorkSessionSummary>> {
        let index = self.load_index()?;
        Ok(index.sessions)
    }

    /// Update session in index
    pub fn update_session_in_index(&self, session: &WorkSession) -> Result<()> {
        let mut index = self.load_index()?;

        if let Some(entry) = index.sessions.iter_mut().find(|s| s.id == session.id) {
            *entry = WorkSessionSummary::from(session);
        }

        self.save_index(&index)?;
        self.save_session(session)?;

        Ok(())
    }

    /// Get the data directory for a session (or current if None)
    pub fn get_session_data_dir(&self, session_id: Option<&str>) -> Result<PathBuf> {
        match session_id {
            Some(id) => {
                let session = self.load_session(id)?;
                Ok(self.session_dir(&session.id))
            }
            None => {
                // Use current symlink
                let current = self.data_dir.join("current");
                if current.exists() {
                    Ok(std::fs::canonicalize(&current)?)
                } else {
                    Err(anyhow!("No current session. Run 'ouroboros plan' first."))
                }
            }
        }
    }
}

/// Generate a short ID (6 chars from UUID)
fn generate_short_id() -> String {
    let uuid = Uuid::new_v4();
    let hex = format!("{:x}", uuid);
    hex[..6].to_string()
}

/// Sanitize label for use in directory name
fn sanitize_label(label: &str) -> String {
    label.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_session() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkSessionManager::new(tmp.path()).unwrap();

        let session = mgr.create_session("Test goal", None).unwrap();
        assert_eq!(session.goal, "Test goal");
        assert_eq!(session.status, WorkSessionStatus::Pending);
        assert!(session.id.len() >= 6);

        // Verify directory structure
        let session_dir = mgr.session_dir(&session.id);
        assert!(session_dir.join("tasks").exists());
        assert!(session_dir.join("results").exists());
        assert!(session_dir.join("contexts").exists());
    }

    #[test]
    fn test_session_with_label() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkSessionManager::new(tmp.path()).unwrap();

        let session = mgr.create_session("API refactor", Some("api-refactor".to_string())).unwrap();
        assert!(session.id.contains("api-refactor"));
        assert_eq!(session.label, Some("api-refactor".to_string()));
    }

    #[test]
    fn test_session_lifecycle() {
        let mut session = WorkSession::new("Test", None);
        assert_eq!(session.status, WorkSessionStatus::Pending);

        session.start(3);
        assert_eq!(session.status, WorkSessionStatus::Running);
        assert_eq!(session.task_count, 3);

        session.record_completion(true);
        session.record_completion(true);
        assert_eq!(session.status, WorkSessionStatus::Running);

        session.record_completion(true);
        assert_eq!(session.status, WorkSessionStatus::Completed);
        assert!(session.completed_at.is_some());
    }

    #[test]
    fn test_session_failure() {
        let mut session = WorkSession::new("Test", None);
        session.start(2);

        session.record_completion(true);
        session.record_completion(false);

        assert_eq!(session.status, WorkSessionStatus::Failed);
        assert_eq!(session.failed_count, 1);
    }

    #[test]
    fn test_switch_session() {
        let tmp = TempDir::new().unwrap();
        let mgr = WorkSessionManager::new(tmp.path()).unwrap();

        let s1 = mgr.create_session("First", None).unwrap();
        let s2 = mgr.create_session("Second", None).unwrap();

        // Current should be s2
        assert_eq!(mgr.current_session_id().unwrap(), Some(s2.id.clone()));

        // Switch to s1
        mgr.switch_session(&s1.id).unwrap();
        assert_eq!(mgr.current_session_id().unwrap(), Some(s1.id.clone()));
    }
}
