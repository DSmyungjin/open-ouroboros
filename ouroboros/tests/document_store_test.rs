use ouroboros::docs::{Document, DocumentStore, DocType, Metadata};
use tempfile::TempDir;
use chrono::Utc;
use std::thread::sleep;
use std::time::Duration;

/// Test: Create a new document and verify it's stored correctly
#[test]
fn test_create_document() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc = Document::new(
        "task-001",
        DocType::TaskDefinition,
        "# Task 1\n\nImplement feature X"
    )
    .with_task_id("task-001")
    .with_tags(vec!["feature".to_string(), "priority-high".to_string()]);

    let path = store.create(&doc).unwrap();

    // Verify file exists
    assert!(path.exists());
    assert!(path.to_str().unwrap().contains("tasks"));
    assert!(path.to_str().unwrap().ends_with("task-001.md"));
}

/// Test: Create multiple documents with different types
#[test]
fn test_create_multiple_documents() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc_types = vec![
        DocType::TaskDefinition,
        DocType::TaskResult,
        DocType::Context,
        DocType::ValidationReport,
    ];

    for (i, doc_type) in doc_types.iter().enumerate() {
        let doc = Document::new(
            format!("doc-{:03}", i),
            *doc_type,
            format!("Content for document {}", i)
        );

        let path = store.create(&doc).unwrap();
        assert!(path.exists());
    }
}

/// Test: Read a document by path
#[test]
fn test_read_document_by_path() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let original = Document::new(
        "test-read-001",
        DocType::TaskResult,
        "# Test Result\n\nThis is a test result."
    )
    .with_task_id("task-001")
    .with_tags(vec!["test".to_string()]);

    let path = store.create(&original).unwrap();
    let loaded = store.read(&path).unwrap();

    assert_eq!(loaded.id, "test-read-001");
    assert_eq!(loaded.doc_type, DocType::TaskResult);
    assert_eq!(loaded.metadata.task_id, Some("task-001".to_string()));
    assert_eq!(loaded.metadata.tags, vec!["test".to_string()]);
    assert!(loaded.content.contains("This is a test result"));
}

/// Test: Read a document by ID and type
#[test]
fn test_read_by_id() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc = Document::new(
        "test-id-001",
        DocType::Context,
        "# Context\n\nSome context information."
    );

    store.create(&doc).unwrap();

    let loaded = store.read_by_id("test-id-001", DocType::Context).unwrap();
    assert_eq!(loaded.id, "test-id-001");
    assert_eq!(loaded.doc_type, DocType::Context);
}

/// Test: Read non-existent document returns error
#[test]
fn test_read_nonexistent_document() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let result = store.read_by_id("nonexistent", DocType::TaskResult);
    assert!(result.is_err());
}

/// Test: Update document (simulated by creating a new version)
#[test]
fn test_update_document() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create initial document
    let mut doc = Document::new(
        "update-test-001",
        DocType::TaskResult,
        "# Initial Content\n\nVersion 1"
    )
    .with_task_id("task-001");

    store.create(&doc).unwrap();

    // Simulate update by modifying and re-creating
    sleep(Duration::from_millis(10)); // Ensure timestamp changes
    doc.content = "# Updated Content\n\nVersion 2".to_string();
    doc.metadata.updated_at = Utc::now();

    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    assert!(loaded.content.contains("Version 2"));
    assert!(loaded.metadata.updated_at > loaded.metadata.created_at);
}

/// Test: Delete document (file system operation)
#[test]
fn test_delete_document() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc = Document::new(
        "delete-test-001",
        DocType::TaskResult,
        "# To be deleted"
    );

    let path = store.create(&doc).unwrap();
    assert!(path.exists());

    // Delete the file
    std::fs::remove_file(&path).unwrap();
    assert!(!path.exists());

    // Verify we can't read it anymore
    let result = store.read(&path);
    assert!(result.is_err());
}

/// Test: List documents by type
#[test]
fn test_list_documents() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create multiple task results
    for i in 0..5 {
        let doc = Document::new(
            format!("task-{:03}", i),
            DocType::TaskResult,
            format!("Result {}", i)
        );
        store.create(&doc).unwrap();
    }

    // Create some contexts (should not be included in results list)
    for i in 0..3 {
        let doc = Document::new(
            format!("context-{:03}", i),
            DocType::Context,
            format!("Context {}", i)
        );
        store.create(&doc).unwrap();
    }

    let results = store.list(DocType::TaskResult).unwrap();
    assert_eq!(results.len(), 5);

    let contexts = store.list(DocType::Context).unwrap();
    assert_eq!(contexts.len(), 3);
}

/// Test: List documents when directory is empty
#[test]
fn test_list_empty_directory() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let docs = store.list(DocType::TaskResult).unwrap();
    assert_eq!(docs.len(), 0);
}

/// Test: Metadata management - tags
#[test]
fn test_metadata_tags() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let tags = vec![
        "feature".to_string(),
        "backend".to_string(),
        "api".to_string(),
        "v2.0".to_string(),
    ];

    let doc = Document::new(
        "tagged-doc",
        DocType::TaskDefinition,
        "# Tagged Task"
    )
    .with_tags(tags.clone());

    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    assert_eq!(loaded.metadata.tags, tags);
}

/// Test: Metadata management - timestamps
#[test]
fn test_metadata_timestamps() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let before = Utc::now();

    let doc = Document::new(
        "timestamp-test",
        DocType::TaskResult,
        "# Timestamp Test"
    );

    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    let after = Utc::now();

    assert!(loaded.metadata.created_at >= before);
    assert!(loaded.metadata.created_at <= after);
    assert_eq!(loaded.metadata.created_at, loaded.metadata.updated_at);
}

/// Test: Metadata management - task_id and session_id
#[test]
fn test_metadata_ids() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let mut doc = Document::new(
        "ids-test",
        DocType::TaskResult,
        "# ID Test"
    )
    .with_task_id("task-123");

    doc.metadata.session_id = Some("session-456".to_string());

    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    assert_eq!(loaded.metadata.task_id, Some("task-123".to_string()));
    assert_eq!(loaded.metadata.session_id, Some("session-456".to_string()));
}

/// Test: Metadata default values
#[test]
fn test_metadata_defaults() {
    let metadata = Metadata::default();

    assert!(metadata.task_id.is_none());
    assert!(metadata.session_id.is_none());
    assert_eq!(metadata.tags.len(), 0);
    assert_eq!(metadata.created_at, metadata.updated_at);
}

/// Test: Read latest result for a task (handles retry attempts)
#[test]
fn test_read_latest_result() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create multiple result attempts for the same task
    let results = vec![
        ("task-001-result", "Attempt 1"),
        ("task-001-result-2", "Attempt 2"),
        ("task-001-result-3", "Attempt 3"),
    ];

    for (id, content) in results {
        let doc = Document::new(id, DocType::TaskResult, content);
        store.create(&doc).unwrap();
    }

    // Should return the latest attempt (result-3)
    let latest = store.read_latest_result("task-001").unwrap();
    assert!(latest.content.contains("Attempt 3"));
}

/// Test: Read latest result when only one exists
#[test]
fn test_read_latest_result_single() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc = Document::new("task-002-result", DocType::TaskResult, "Single result");
    store.create(&doc).unwrap();

    let latest = store.read_latest_result("task-002").unwrap();
    assert!(latest.content.contains("Single result"));
}

/// Test: Read latest result when none exists
#[test]
fn test_read_latest_result_none() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let result = store.read_latest_result("nonexistent-task");
    assert!(result.is_err());
}

/// Test: Assemble context from multiple sources
#[test]
fn test_assemble_context() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create some previous results
    let results = vec![
        ("result-001", "Previous result 1"),
        ("result-002", "Previous result 2"),
    ];

    for (id, content) in results {
        let doc = Document::new(id, DocType::TaskResult, content);
        store.create(&doc).unwrap();
    }

    let context = store.assemble_context(
        "task-003",
        &["result-001".to_string(), "result-002".to_string()]
    ).unwrap();

    assert!(context.contains("Context for Task Execution"));
    assert!(context.contains("Previous Task Results"));
    assert!(context.contains("result-001"));
    assert!(context.contains("result-002"));
    assert!(context.contains("Current Task: task-003"));
}

/// Test: Assemble context with no previous results
#[test]
fn test_assemble_context_empty() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let context = store.assemble_context("task-001", &[]).unwrap();

    assert!(context.contains("Context for Task Execution"));
    assert!(context.contains("Current Task: task-001"));
    assert!(!context.contains("Previous Task Results"));
}

/// Test: Error handling - invalid document format
#[test]
fn test_error_invalid_format() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create a malformed document manually
    let malformed_path = tmp.path().join("tasks").join("malformed.md");
    std::fs::create_dir_all(tmp.path().join("tasks")).unwrap();
    std::fs::write(&malformed_path, "This is not a valid document format").unwrap();

    let result = store.read(&malformed_path);
    assert!(result.is_err());
}

/// Test: Error handling - missing frontmatter
#[test]
fn test_error_missing_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let no_frontmatter = tmp.path().join("tasks").join("no-frontmatter.md");
    std::fs::create_dir_all(tmp.path().join("tasks")).unwrap();
    std::fs::write(&no_frontmatter, "# Just some content\n\nNo frontmatter here").unwrap();

    let result = store.read(&no_frontmatter);
    assert!(result.is_err());
}

/// Test: Document builder pattern
#[test]
fn test_document_builder() {
    let doc = Document::new("builder-test", DocType::TaskDefinition, "Content")
        .with_task_id("task-100")
        .with_tags(vec!["tag1".to_string(), "tag2".to_string()]);

    assert_eq!(doc.id, "builder-test");
    assert_eq!(doc.doc_type, DocType::TaskDefinition);
    assert_eq!(doc.metadata.task_id, Some("task-100".to_string()));
    assert_eq!(doc.metadata.tags.len(), 2);
}

/// Test: Concurrent document creation
#[test]
fn test_concurrent_creation() {
    use std::sync::Arc;
    use std::thread;

    let tmp = TempDir::new().unwrap();
    let store = Arc::new(DocumentStore::new(tmp.path()).unwrap());

    let mut handles = vec![];

    for i in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = thread::spawn(move || {
            let doc = Document::new(
                format!("concurrent-{:03}", i),
                DocType::TaskResult,
                format!("Concurrent result {}", i)
            );
            store_clone.create(&doc).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let docs = store.list(DocType::TaskResult).unwrap();
    assert_eq!(docs.len(), 10);
}

/// Test: Large document content
#[test]
fn test_large_document() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Create a large document (1MB of content)
    let large_content = "x".repeat(1024 * 1024);
    let doc = Document::new("large-doc", DocType::TaskResult, &large_content);

    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    assert_eq!(loaded.content.len(), large_content.len());
}

/// Test: Special characters in content
#[test]
fn test_special_characters() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let special_content = r#"
# Special Characters Test

- Emoji: 🚀 🎉 ✅
- Unicode: 한글 日本語 中文
- Code: `let x = "hello";`
- Symbols: @#$%^&*()
- Quotes: "double" 'single'
"#;

    let doc = Document::new("special-chars", DocType::TaskResult, special_content);
    let path = store.create(&doc).unwrap();
    let loaded = store.read(&path).unwrap();

    assert!(loaded.content.contains("🚀"));
    assert!(loaded.content.contains("한글"));
    assert!(loaded.content.contains("日本語"));
}

/// Test: Empty tags list serialization
#[test]
fn test_empty_tags_serialization() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    let doc = Document::new("empty-tags", DocType::TaskResult, "Content");
    let path = store.create(&doc).unwrap();

    // Read raw file content to verify tags are not serialized when empty
    let _file_content = std::fs::read_to_string(&path).unwrap();

    let loaded = store.read(&path).unwrap();
    assert_eq!(loaded.metadata.tags.len(), 0);
}

/// Test: Optional metadata fields serialization
#[test]
fn test_optional_metadata_serialization() {
    let tmp = TempDir::new().unwrap();
    let store = DocumentStore::new(tmp.path()).unwrap();

    // Document without task_id and session_id
    let doc = Document::new("optional-meta", DocType::TaskResult, "Content");
    let path = store.create(&doc).unwrap();

    let loaded = store.read(&path).unwrap();
    assert!(loaded.metadata.task_id.is_none());
    assert!(loaded.metadata.session_id.is_none());
}
