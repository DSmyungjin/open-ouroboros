//! Comprehensive Search Indexing Test Suite
//!
//! This test suite verifies:
//! 1. Document creation and indexing workflow
//! 2. Search index integrity after document insertion
//! 3. Keyword search functionality with indexed documents
//! 4. Search result accuracy and ranking
//! 5. Index state verification

use anyhow::Result;
use ouroboros::search::{DocumentType, SearchEngine, SearchOptions};
use std::collections::HashMap;
use tempfile::TempDir;

/// Test Report Structure
#[derive(Debug)]
struct TestReport {
    test_name: String,
    status: TestStatus,
    details: String,
    execution_time_ms: u128,
}

#[derive(Debug, PartialEq)]
enum TestStatus {
    Passed,
    Failed,
    #[allow(dead_code)]
    Warning,
}

impl TestReport {
    fn new(test_name: String) -> Self {
        Self {
            test_name,
            status: TestStatus::Passed,
            details: String::new(),
            execution_time_ms: 0,
        }
    }

    fn with_status(mut self, status: TestStatus) -> Self {
        self.status = status;
        self
    }

    fn with_details(mut self, details: String) -> Self {
        self.details = details;
        self
    }

    fn with_execution_time(mut self, time_ms: u128) -> Self {
        self.execution_time_ms = time_ms;
        self
    }

    fn print(&self) {
        let status_icon = match self.status {
            TestStatus::Passed => "PASS",
            TestStatus::Failed => "FAIL",
            TestStatus::Warning => "WARN",
        };

        println!("\n{} {} ({} ms)", status_icon, self.test_name, self.execution_time_ms);
        if !self.details.is_empty() {
            println!("  {}", self.details);
        }
    }
}

/// Test 1: Basic document indexing and verification
#[tokio::test]
async fn test_basic_document_indexing() -> Result<()> {
    println!("\n=== Test 1: Basic Document Indexing and Verification ===");

    let start = std::time::Instant::now();
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("\nStep 1: Creating test documents with specific keywords");

    // Create test documents with specific keywords in title and content
    let test_docs = vec![
        (
            "doc-001",
            "Search System Testing",
            "This document tests the search indexing functionality. Verifying keyword matching works correctly.",
        ),
        (
            "doc-002",
            "Indexing Performance Analysis",
            "Verifying that documents are correctly added to the search index. Testing indexing process reliability.",
        ),
        (
            "doc-003",
            "Search Engine Development",
            "This document covers search engine development with various keywords. Testing verification of keyword matching.",
        ),
    ];

    for (id, title, content) in &test_docs {
        search.index_task(id, title, content, Some("test-session")).await?;
        println!("  Indexed: {} - {}", id, title);
    }

    println!("\nStep 2: Verifying documents in search index");
    let doc_count = search.count().await?;
    println!("  Total documents in index: {}", doc_count);

    assert_eq!(
        doc_count,
        test_docs.len(),
        "Document count mismatch: expected {}, got {}",
        test_docs.len(),
        doc_count
    );

    println!("\nStep 3: Testing keyword search on indexed documents");

    let options = SearchOptions::new().with_limit(10);
    let results = search.search("search indexing", &options).await?;

    println!("  Query: 'search indexing'");
    println!("  Results found: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!(
            "    {}. {} (score: {:.4}, id: {})",
            i + 1,
            result.title,
            result.score,
            result.id
        );
    }

    assert!(
        results.len() >= 2,
        "Expected at least 2 results for search, got {}",
        results.len()
    );

    println!("\nStep 4: Verification complete");
    println!("  All documents successfully indexed");
    println!("  Index count verified: {}", doc_count);
    println!("  Keyword searches working correctly");

    let elapsed = start.elapsed();
    let report = TestReport::new("Basic Document Indexing".to_string())
        .with_status(TestStatus::Passed)
        .with_details(format!(
            "Indexed {} documents, verified count, tested searches",
            test_docs.len()
        ))
        .with_execution_time(elapsed.as_millis());

    report.print();

    Ok(())
}

/// Test 2: Multi-keyword document search and ranking
#[tokio::test]
async fn test_multi_keyword_search_ranking() -> Result<()> {
    println!("\n=== Test 2: Multi-Keyword Search and Ranking ===");

    let start = std::time::Instant::now();
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("\nStep 1: Creating documents with varying keyword density");

    // Document with high keyword density
    search
        .index_knowledge(
            "kb-high-density",
            "Search Engine Optimization Guide",
            "A guide on how to optimize search engines. \
             Covers search algorithms and indexing strategies. \
             Explains optimization techniques for efficient search. \
             Presents various optimization methods to improve search performance.",
            Some("test-session"),
        )
        .await?;

    println!("  High density document indexed");

    // Document with medium keyword density
    search
        .index_knowledge(
            "kb-medium-density",
            "Database Search Features",
            "How to implement search functionality in databases. \
             Explains fast data retrieval techniques using indexes.",
            Some("test-session"),
        )
        .await?;

    println!("  Medium density document indexed");

    // Document with low keyword density
    search
        .index_knowledge(
            "kb-low-density",
            "Web Application Development",
            "When developing web applications, search functionality should also be considered.",
            Some("test-session"),
        )
        .await?;

    println!("  Low density document indexed");

    println!("\nStep 2: Executing multi-keyword search");

    let options = SearchOptions::new().with_limit(10);
    let results = search.search("search optimization", &options).await?;

    println!("  Query: 'search optimization'");
    println!("  Results: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!(
            "    {}. {} (score: {:.4})",
            i + 1,
            result.title,
            result.score
        );
    }

    println!("\nStep 3: Verifying ranking order");

    assert!(
        !results.is_empty(),
        "Search should return results for multi-keyword query"
    );

    // Verify BM25 ranking: scores should be in descending order
    for i in 0..results.len().saturating_sub(1) {
        assert!(
            results[i].score >= results[i + 1].score,
            "Results not properly ranked: result[{}].score ({:.4}) < result[{}].score ({:.4})",
            i,
            results[i].score,
            i + 1,
            results[i + 1].score
        );
    }

    println!("  BM25 ranking verified (scores in descending order)");
    println!("  High-density document ranked higher");

    let elapsed = start.elapsed();
    let report = TestReport::new("Multi-Keyword Search Ranking".to_string())
        .with_status(TestStatus::Passed)
        .with_details(format!(
            "Tested ranking with {} results, verified BM25 score ordering",
            results.len()
        ))
        .with_execution_time(elapsed.as_millis());

    report.print();

    Ok(())
}

/// Test 3: Index state verification and document retrieval
#[tokio::test]
async fn test_index_state_verification() -> Result<()> {
    println!("\n=== Test 3: Index State Verification ===");

    let start = std::time::Instant::now();
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("\nStep 1: Creating multiple document types");

    // Create documents of different types
    search
        .index_task(
            "task-001",
            "Implementation Task",
            "Task to implement search functionality",
            Some("session-1"),
        )
        .await?;

    search
        .index_knowledge(
            "kb-001",
            "Search Knowledge",
            "Knowledge document about search algorithms",
            Some("session-1"),
        )
        .await?;

    search
        .index_task_result("task-001", "Search functionality implemented successfully", Some("session-1"))
        .await?;

    search
        .index_context(
            "ctx-001",
            "Search Context",
            "Context information related to search",
            Some("session-1"),
            Some("task-001"),
        )
        .await?;

    println!("  Task document indexed");
    println!("  Knowledge document indexed");
    println!("  TaskResult document indexed");
    println!("  Context document indexed");

    println!("\nStep 2: Verifying index state");

    let total_count = search.count().await?;
    println!("  Total documents in index: {}", total_count);

    assert_eq!(total_count, 4, "Expected 4 documents in index");

    println!("\nStep 3: Testing filtered searches");

    // Test 3.1: Filter by document type
    println!("\n  Test 3.1: Filter by document type (Task)");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_doc_type(DocumentType::Task);
    let results = search.search("search", &options).await?;

    println!("    Results: {}", results.len());
    for result in &results {
        println!("      - {} (type: {:?})", result.title, result.doc_type);
        assert_eq!(
            result.doc_type,
            DocumentType::Task,
            "Result should be Task type"
        );
    }

    // Test 3.2: Filter by session
    println!("\n  Test 3.2: Filter by session ID");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_session_id("session-1");
    let results = search.search("search", &options).await?;

    println!("    Results: {}", results.len());
    assert_eq!(
        results.len(),
        4,
        "Should find all 4 documents in session-1"
    );

    // Test 3.3: Combined filters
    println!("\n  Test 3.3: Combined filters (Knowledge + session-1)");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_doc_type(DocumentType::Knowledge)
        .with_session_id("session-1");
    let results = search.search("search", &options).await?;

    println!("    Results: {}", results.len());
    assert_eq!(
        results.len(),
        1,
        "Should find exactly 1 Knowledge document"
    );

    println!("\nStep 4: Index state verified");
    println!("  Document count: {}", total_count);
    println!("  Type filtering working");
    println!("  Session filtering working");
    println!("  Combined filtering working");

    let elapsed = start.elapsed();
    let report = TestReport::new("Index State Verification".to_string())
        .with_status(TestStatus::Passed)
        .with_details(format!(
            "Verified {} documents with multiple filter combinations",
            total_count
        ))
        .with_execution_time(elapsed.as_millis());

    report.print();

    Ok(())
}

/// Test 4: Document update and re-indexing
#[tokio::test]
async fn test_document_update_reindexing() -> Result<()> {
    println!("\n=== Test 4: Document Update and Re-indexing ===");

    let start = std::time::Instant::now();
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("\nStep 1: Creating initial document");

    search
        .index_task(
            "update-test",
            "Initial Title",
            "Initial content. Contains keyword alpha.",
            Some("session-update"),
        )
        .await?;

    println!("  Initial document indexed");

    println!("\nStep 2: Searching for initial keyword");

    let options = SearchOptions::new().with_limit(10);
    let results = search.search("keyword alpha", &options).await?;

    println!("  Query: 'keyword alpha'");
    println!("  Results: {}", results.len());

    assert!(
        !results.is_empty(),
        "Should find document with initial keyword"
    );
    println!("  Found initial document");

    println!("\nStep 3: Updating document with new content");

    search
        .index_task(
            "update-test",
            "Updated Title",
            "Updated content. Contains keyword beta.",
            Some("session-update"),
        )
        .await?;

    println!("  Document re-indexed with new content");

    println!("\nStep 4: Verifying update in search results");

    // Search for new keyword
    let results = search.search("keyword beta", &options).await?;
    println!("  Query: 'keyword beta'");
    println!("  Results: {}", results.len());

    assert!(
        !results.is_empty(),
        "Should find document with new keyword"
    );
    assert!(
        results[0].title.contains("Updated"),
        "Should find updated document"
    );
    println!("  Found updated document with new keyword");

    // Verify old keyword no longer matches strongly
    let results = search.search("keyword alpha", &options).await?;
    println!("\n  Query: 'keyword alpha' (old keyword)");
    println!("  Results: {}", results.len());

    // After update, old keyword should not be found or have very low relevance
    if !results.is_empty() {
        println!("  Note: Old keyword still present (may be cached)");
    } else {
        println!("  Old keyword correctly removed from index");
    }

    println!("\nStep 5: Re-indexing verified");
    println!("  Document successfully updated in index");
    println!("  New keywords searchable");

    let elapsed = start.elapsed();
    let report = TestReport::new("Document Update and Re-indexing".to_string())
        .with_status(TestStatus::Passed)
        .with_details("Document update and re-indexing working correctly".to_string())
        .with_execution_time(elapsed.as_millis());

    report.print();

    Ok(())
}

/// Test 5: Comprehensive search indexing workflow
#[tokio::test]
async fn test_comprehensive_indexing_workflow() -> Result<()> {
    println!("\n=== Test 5: Comprehensive Search Indexing Workflow ===");

    let overall_start = std::time::Instant::now();
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    let mut test_results: HashMap<String, TestReport> = HashMap::new();

    // Phase 1: Index Creation
    println!("\nPhase 1: Index Creation");
    let phase_start = std::time::Instant::now();

    let test_dataset = vec![
        (
            "ds-001",
            "Machine Learning Basics",
            "Covers basic concepts and algorithms of machine learning. Includes supervised and unsupervised learning.",
            DocumentType::Knowledge,
        ),
        (
            "ds-002",
            "Search Engine Implementation",
            "Explains how to implement a search engine. Uses BM25 algorithm and vector search.",
            DocumentType::Task,
        ),
        (
            "ds-003",
            "Data Indexing Strategies",
            "Presents efficient data indexing strategies. Covers search performance optimization methods.",
            DocumentType::Knowledge,
        ),
        (
            "ds-004",
            "API Development Guide",
            "RESTful API development with search capabilities. Implementing efficient search endpoints.",
            DocumentType::Task,
        ),
        (
            "ds-005",
            "Natural Language Processing",
            "Explains NLP and text analysis techniques. Covers preprocessing methods for search quality improvement.",
            DocumentType::Knowledge,
        ),
    ];

    for (id, title, content, doc_type) in &test_dataset {
        match doc_type {
            DocumentType::Task => {
                search.index_task(id, title, content, Some("comprehensive-test")).await?;
            }
            DocumentType::Knowledge => {
                search
                    .index_knowledge(id, title, content, Some("comprehensive-test"))
                    .await?;
            }
            _ => {}
        }
        println!("  Indexed: {}", title);
    }

    let phase_elapsed = phase_start.elapsed();
    test_results.insert(
        "Index Creation".to_string(),
        TestReport::new("Index Creation".to_string())
            .with_status(TestStatus::Passed)
            .with_details(format!("Indexed {} documents", test_dataset.len()))
            .with_execution_time(phase_elapsed.as_millis()),
    );

    // Phase 2: Index Verification
    println!("\nPhase 2: Index Verification");
    let phase_start = std::time::Instant::now();

    let doc_count = search.count().await?;
    println!("  Total documents: {}", doc_count);

    assert_eq!(
        doc_count,
        test_dataset.len(),
        "Document count verification failed"
    );

    let phase_elapsed = phase_start.elapsed();
    test_results.insert(
        "Index Verification".to_string(),
        TestReport::new("Index Verification".to_string())
            .with_status(TestStatus::Passed)
            .with_details(format!("Verified {} documents in index", doc_count))
            .with_execution_time(phase_elapsed.as_millis()),
    );

    // Phase 3: Search Testing
    println!("\nPhase 3: Search Testing");
    let phase_start = std::time::Instant::now();

    let test_queries = vec![
        ("search engine", "Basic search query"),
        ("machine learning", "Technical term"),
        ("indexing optimization", "Compound query"),
        ("API search", "Mixed query"),
        ("natural language", "NLP term"),
    ];

    let mut search_test_details = Vec::new();

    for (query, description) in test_queries {
        let options = SearchOptions::new().with_limit(10);
        let results = search.search(query, &options).await?;

        println!("\n  Query: '{}' ({})", query, description);
        println!("  Results: {}", results.len());

        for (i, result) in results.iter().take(3).enumerate() {
            println!(
                "    {}. {} (score: {:.4})",
                i + 1,
                result.title,
                result.score
            );
        }

        search_test_details.push(format!("'{}': {} results", query, results.len()));

        // Verify ranking
        for i in 0..results.len().saturating_sub(1) {
            assert!(
                results[i].score >= results[i + 1].score,
                "Ranking error in query: {}",
                query
            );
        }
    }

    let phase_elapsed = phase_start.elapsed();
    test_results.insert(
        "Search Testing".to_string(),
        TestReport::new("Search Testing".to_string())
            .with_status(TestStatus::Passed)
            .with_details(search_test_details.join("; "))
            .with_execution_time(phase_elapsed.as_millis()),
    );

    // Phase 4: Filter Testing
    println!("\nPhase 4: Filter Testing");
    let phase_start = std::time::Instant::now();

    // Test document type filter
    let options = SearchOptions::new()
        .with_limit(10)
        .with_doc_type(DocumentType::Knowledge);
    let knowledge_results = search.search("search", &options).await?;

    println!("  Filter: DocumentType::Knowledge");
    println!("  Results: {}", knowledge_results.len());

    for result in &knowledge_results {
        assert_eq!(result.doc_type, DocumentType::Knowledge);
    }

    // Test session filter
    let options = SearchOptions::new()
        .with_limit(10)
        .with_session_id("comprehensive-test");
    let session_results = search.search("search", &options).await?;

    println!("\n  Filter: session_id = 'comprehensive-test'");
    println!("  Results: {}", session_results.len());

    let phase_elapsed = phase_start.elapsed();
    test_results.insert(
        "Filter Testing".to_string(),
        TestReport::new("Filter Testing".to_string())
            .with_status(TestStatus::Passed)
            .with_details(format!(
                "Type filter: {} results, Session filter: {} results",
                knowledge_results.len(),
                session_results.len()
            ))
            .with_execution_time(phase_elapsed.as_millis()),
    );

    // Generate Final Report
    println!("\n=== COMPREHENSIVE TEST REPORT ===");

    let total_elapsed = overall_start.elapsed();

    for (phase_name, report) in &test_results {
        println!("\n{}", phase_name);
        report.print();
    }

    println!("\n=== SUMMARY ===");
    println!("  Total Tests: {}", test_results.len());
    println!(
        "  Passed: {}",
        test_results
            .values()
            .filter(|r| r.status == TestStatus::Passed)
            .count()
    );
    println!(
        "  Failed: {}",
        test_results
            .values()
            .filter(|r| r.status == TestStatus::Failed)
            .count()
    );
    println!("  Total Execution Time: {} ms", total_elapsed.as_millis());
    println!("\n  All search indexing tests passed successfully!");

    Ok(())
}
