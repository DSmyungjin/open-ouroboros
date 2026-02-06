use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    #[serde(skip)]
    pub path: PathBuf,
    pub doc_type: DocType,
    pub metadata: Metadata,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    TaskDefinition,
    TaskResult,
    Context,
    ValidationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            task_id: None,
            session_id: None,
            tags: vec![],
        }
    }
}

pub struct DocumentStore {
    base_dir: PathBuf,
}

impl DocumentStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        // Don't create subdirectories here - they're created lazily in create()
        // or explicitly by WorkSessionManager when creating a session
        Ok(Self { base_dir })
    }

    /// Create a new document
    pub fn create(&self, doc: &Document) -> Result<PathBuf> {
        let subdir = match doc.doc_type {
            DocType::TaskDefinition => "tasks",
            DocType::TaskResult => "results",
            DocType::Context => "contexts",
            DocType::ValidationReport => "results",
        };

        let subdir_path = self.base_dir.join(subdir);
        // Create subdirectory lazily if it doesn't exist
        fs::create_dir_all(&subdir_path)?;

        let filename = format!("{}.md", doc.id);
        let path = subdir_path.join(&filename);

        let content = self.serialize_document(doc)?;
        fs::write(&path, content)?;

        Ok(path)
    }

    /// Read a document from path
    pub fn read(&self, path: &Path) -> Result<Document> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read document: {:?}", path))?;

        self.parse_document(&content, path)
    }

    /// Read a document by ID and type
    pub fn read_by_id(&self, id: &str, doc_type: DocType) -> Result<Document> {
        let subdir = match doc_type {
            DocType::TaskDefinition => "tasks",
            DocType::TaskResult => "results",
            DocType::Context => "contexts",
            DocType::ValidationReport => "results",
        };

        let path = self.base_dir.join(subdir).join(format!("{}.md", id));
        self.read(&path)
    }

    /// Read the latest result for a task (handles retry attempts)
    pub fn read_latest_result(&self, task_id: &str) -> Result<Document> {
        let results_dir = self.base_dir.join("results");

        // Find all result files for this task
        let mut candidates: Vec<(PathBuf, u32)> = vec![];

        if results_dir.exists() {
            for entry in fs::read_dir(&results_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    // Match patterns: task-XXX-result or task-XXX-result-N
                    if name.starts_with(&format!("{}-result", task_id)) {
                        let attempt_num = if name == format!("{}-result", task_id) {
                            1
                        } else if let Some(suffix) = name.strip_prefix(&format!("{}-result-", task_id)) {
                            suffix.parse().unwrap_or(0)
                        } else {
                            0
                        };
                        if attempt_num > 0 {
                            candidates.push((path, attempt_num));
                        }
                    }
                }
            }
        }

        // Sort by attempt number and get the latest
        candidates.sort_by_key(|(_, num)| std::cmp::Reverse(*num));

        if let Some((path, _)) = candidates.first() {
            self.read(path)
        } else {
            Err(anyhow::anyhow!("No result found for task: {}", task_id))
        }
    }

    /// List all documents of a type
    pub fn list(&self, doc_type: DocType) -> Result<Vec<Document>> {
        let subdir = match doc_type {
            DocType::TaskDefinition => "tasks",
            DocType::TaskResult => "results",
            DocType::Context => "contexts",
            DocType::ValidationReport => "results",
        };

        let dir = self.base_dir.join(subdir);
        let mut docs = vec![];

        if dir.exists() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    if let Ok(doc) = self.read(&path) {
                        docs.push(doc);
                    }
                }
            }
        }

        Ok(docs)
    }

    /// Assemble context for a task from previous results and KG
    pub fn assemble_context(&self, task_id: &str, previous_results: &[String]) -> Result<String> {
        let mut context = String::new();

        context.push_str("# Context for Task Execution\n\n");

        if !previous_results.is_empty() {
            context.push_str("## Previous Task Results\n\n");
            for result_id in previous_results {
                if let Ok(doc) = self.read_by_id(result_id, DocType::TaskResult) {
                    context.push_str(&format!("### {}\n\n", result_id));
                    context.push_str(&doc.content);
                    context.push_str("\n\n");
                }
            }
        }

        context.push_str(&format!("## Current Task: {}\n\n", task_id));

        Ok(context)
    }

    fn serialize_document(&self, doc: &Document) -> Result<String> {
        // YAML frontmatter + Markdown content
        let frontmatter = serde_yaml::to_string(&DocumentFrontmatter {
            id: doc.id.clone(),
            doc_type: doc.doc_type,
            metadata: doc.metadata.clone(),
        })?;

        Ok(format!("---\n{}---\n\n{}", frontmatter, doc.content))
    }

    fn parse_document(&self, content: &str, path: &Path) -> Result<Document> {
        // Parse YAML frontmatter
        let parts: Vec<&str> = content.splitn(3, "---").collect();

        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid document format: missing frontmatter"));
        }

        let frontmatter: DocumentFrontmatter = serde_yaml::from_str(parts[1].trim())?;
        let body = parts[2].trim().to_string();

        Ok(Document {
            id: frontmatter.id,
            path: path.to_path_buf(),
            doc_type: frontmatter.doc_type,
            metadata: frontmatter.metadata,
            content: body,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct DocumentFrontmatter {
    id: String,
    #[serde(rename = "type")]
    doc_type: DocType,
    #[serde(flatten)]
    metadata: Metadata,
}

impl Document {
    pub fn new(id: impl Into<String>, doc_type: DocType, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            path: PathBuf::new(),
            doc_type,
            metadata: Metadata::default(),
            content: content.into(),
        }
    }

    pub fn with_task_id(mut self, task_id: impl Into<String>) -> Self {
        self.metadata.task_id = Some(task_id.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags = tags;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_document_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = DocumentStore::new(tmp.path()).unwrap();

        let doc = Document::new("test-001", DocType::TaskResult, "# Test Result\n\nThis is a test.")
            .with_task_id("task-001")
            .with_tags(vec!["test".to_string()]);

        let path = store.create(&doc).unwrap();
        let loaded = store.read(&path).unwrap();

        assert_eq!(loaded.id, "test-001");
        assert_eq!(loaded.doc_type, DocType::TaskResult);
        assert_eq!(loaded.metadata.task_id, Some("task-001".to_string()));
        assert!(loaded.content.contains("This is a test"));
    }
}
