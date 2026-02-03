//! Claude Code session management
//!
//! Provides access to Claude Code's session data stored in ~/.claude/projects/

use std::path::{Path, PathBuf};
use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};

/// Claude Code sessions index
#[derive(Debug, Deserialize)]
pub struct SessionsIndex {
    #[allow(dead_code)]
    pub version: u32,
    pub entries: Vec<SessionEntry>,
    #[serde(rename = "originalPath")]
    pub original_path: String,
}

/// Individual session entry
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionEntry {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "firstPrompt")]
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "messageCount")]
    pub message_count: u32,
    pub created: String,
    pub modified: String,
    #[serde(rename = "gitBranch")]
    pub git_branch: String,
    #[serde(rename = "projectPath")]
    pub project_path: String,
    #[serde(rename = "isSidechain")]
    pub is_sidechain: bool,
}

impl SessionEntry {
    /// Check if this session matches a task tag (metadata only)
    pub fn matches_tag(&self, tag: &str) -> bool {
        self.first_prompt.as_ref()
            .map(|p| p.contains(tag))
            .unwrap_or(false)
        || self.summary.as_ref()
            .map(|s| s.contains(tag))
            .unwrap_or(false)
    }

    /// Check if session conversation contains the tag (reads .jsonl file)
    pub fn conversation_contains(&self, tag: &str) -> bool {
        if let Ok(content) = std::fs::read_to_string(&self.full_path) {
            content.contains(tag)
        } else {
            false
        }
    }

    /// Full search: metadata + conversation
    pub fn matches_tag_full(&self, tag: &str) -> bool {
        self.matches_tag(tag) || self.conversation_contains(tag)
    }

    /// Generate resume command
    pub fn resume_command(&self) -> String {
        format!("claude --resume {}", self.session_id)
    }

    /// Generate fork command
    pub fn fork_command(&self) -> String {
        format!("claude --resume {} --fork-session", self.session_id)
    }

    /// Generate fork command with prompt
    pub fn fork_command_with_prompt(&self, prompt: &str) -> String {
        format!("claude --resume {} --fork-session -p \"{}\"", self.session_id, prompt)
    }

    /// Generate task-tagged fork command
    pub fn fork_command_for_task(&self, task_id: &str, task_name: &str) -> String {
        let tag = format!("[TASK:{}] {}", &task_id[..8.min(task_id.len())], task_name);
        self.fork_command_with_prompt(&tag)
    }
}

/// Session manager for Claude Code sessions
pub struct SessionManager {
    project_path: PathBuf,
    claude_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager for a project
    pub fn new(project_path: impl AsRef<Path>) -> Result<Self> {
        let project_path = std::fs::canonicalize(project_path.as_ref())
            .with_context(|| format!("Could not resolve path: {:?}", project_path.as_ref()))?;

        let claude_dir = get_claude_projects_dir()?;

        Ok(Self {
            project_path,
            claude_dir,
        })
    }

    /// Create session manager for current directory
    pub fn for_current_dir() -> Result<Self> {
        Self::new(std::env::current_dir()?)
    }

    /// Get the encoded project path used by Claude
    pub fn encoded_path(&self) -> String {
        encode_project_path(self.project_path.to_str().unwrap_or(""))
    }

    /// Get the sessions index file path
    pub fn index_path(&self) -> PathBuf {
        self.claude_dir
            .join(self.encoded_path())
            .join("sessions-index.json")
    }

    /// Check if sessions exist for this project
    pub fn has_sessions(&self) -> bool {
        self.index_path().exists()
    }

    /// Load the sessions index
    pub fn load_index(&self) -> Result<SessionsIndex> {
        let index_path = self.index_path();

        if !index_path.exists() {
            return Err(anyhow!(
                "No sessions found for project: {:?}\nExpected: {:?}",
                self.project_path,
                index_path
            ));
        }

        let content = std::fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read: {:?}", index_path))?;

        let index: SessionsIndex = serde_json::from_str(&content)
            .with_context(|| "Failed to parse sessions-index.json")?;

        Ok(index)
    }

    /// List all sessions, sorted by modified date (newest first)
    pub fn list_sessions(&self) -> Result<Vec<SessionEntry>> {
        let index = self.load_index()?;
        let mut entries = index.entries;
        entries.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(entries)
    }

    /// Find session by ID prefix
    pub fn find_by_id(&self, id_prefix: &str) -> Result<Option<SessionEntry>> {
        let index = self.load_index()?;
        Ok(index.entries.into_iter()
            .find(|e| e.session_id.starts_with(id_prefix)))
    }

    /// Find sessions matching a tag/keyword (metadata only - fast)
    pub fn find_by_tag(&self, tag: &str) -> Result<Vec<SessionEntry>> {
        let index = self.load_index()?;
        Ok(index.entries.into_iter()
            .filter(|e| e.matches_tag(tag))
            .collect())
    }

    /// Find sessions matching a tag/keyword (includes conversation - slower)
    pub fn find_by_tag_full(&self, tag: &str) -> Result<Vec<SessionEntry>> {
        let index = self.load_index()?;
        Ok(index.entries.into_iter()
            .filter(|e| e.matches_tag_full(tag))
            .collect())
    }

    /// Find session for a task (by [TASK:id] tag) - searches conversations
    pub fn find_for_task(&self, task_id: &str) -> Result<Option<SessionEntry>> {
        let tag = format!("[TASK:{}]", &task_id[..8.min(task_id.len())]);
        let matches = self.find_by_tag_full(&tag)?;
        Ok(matches.into_iter().next())
    }

    /// Get forked sessions (sidechains)
    pub fn list_forks(&self) -> Result<Vec<SessionEntry>> {
        let index = self.load_index()?;
        Ok(index.entries.into_iter()
            .filter(|e| e.is_sidechain)
            .collect())
    }

    /// Get root sessions (not sidechains)
    pub fn list_roots(&self) -> Result<Vec<SessionEntry>> {
        let index = self.load_index()?;
        Ok(index.entries.into_iter()
            .filter(|e| !e.is_sidechain)
            .collect())
    }
}

/// Get Claude's projects directory (~/.claude/projects)
pub fn get_claude_projects_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not find home directory"))?;
    Ok(home.join(".claude").join("projects"))
}

/// Encode project path to Claude's format
/// `/path/to/project` -> `-path-to-project`
pub fn encode_project_path(path: &str) -> String {
    path.replace('/', "-").replace('_', "-")
}

/// Decode Claude's path format back to original
/// `-path-to-project` -> `/path/to/project`
pub fn decode_project_path(encoded: &str) -> String {
    // Note: This is lossy - underscores become slashes
    encoded.replacen('-', "/", 1).replace('-', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_project_path() {
        assert_eq!(
            encode_project_path("/Users/test/my_project"),
            "-Users-test-my-project"
        );
    }

    #[test]
    fn test_session_entry_matches_tag() {
        let entry = SessionEntry {
            session_id: "test-123".to_string(),
            full_path: "/test".to_string(),
            first_prompt: Some("[TASK:abc123] Do something".to_string()),
            summary: None,
            message_count: 5,
            created: "2025-01-01".to_string(),
            modified: "2025-01-01".to_string(),
            git_branch: "main".to_string(),
            project_path: "/test".to_string(),
            is_sidechain: false,
        };

        assert!(entry.matches_tag("[TASK:abc123]"));
        assert!(entry.matches_tag("Do something"));
        assert!(!entry.matches_tag("[TASK:xyz]"));
    }

    #[test]
    fn test_fork_command() {
        let entry = SessionEntry {
            session_id: "abc-123-def".to_string(),
            full_path: "/test".to_string(),
            first_prompt: None,
            summary: None,
            message_count: 0,
            created: "2025-01-01".to_string(),
            modified: "2025-01-01".to_string(),
            git_branch: "main".to_string(),
            project_path: "/test".to_string(),
            is_sidechain: false,
        };

        assert_eq!(
            entry.fork_command_for_task("task-12345678", "Do stuff"),
            "claude --resume abc-123-def --fork-session -p \"[TASK:task-123] Do stuff\""
        );
    }
}
