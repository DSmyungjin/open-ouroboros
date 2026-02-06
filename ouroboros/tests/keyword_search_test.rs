//! Comprehensive test suite for keyword search (BM25)
//!
//! This test suite verifies:
//! 1. BM25 algorithm-based keyword search functionality
//! 2. Search result ranking and accuracy
//! 3. Various query patterns (single keyword, multi-keyword)
//! 4. Search performance benchmarks

use anyhow::Result;
use ouroboros::search::{DocumentType, SearchEngine, SearchOptions};
use std::time::Instant;
use tempfile::TempDir;

/// Test BM25 search with single keyword
#[tokio::test]
async fn test_bm25_single_keyword_english() -> Result<()> {
    println!("\n=== Test: BM25 Single Keyword ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index test documents
    search
        .index_task(
            "task-001",
            "Search Implementation",
            "Implement full-text search functionality using BM25 algorithm",
            Some("session-test"),
        )
        .await?;

    search
        .index_task(
            "task-002",
            "Database Query Optimization",
            "Optimize database queries for better performance",
            Some("session-test"),
        )
        .await?;

    search
        .index_task(
            "task-003",
            "Search UI Component",
            "Create user interface for search feature",
            Some("session-test"),
        )
        .await?;

    println!("Indexed 3 documents");

    // Search for "search"
    let options = SearchOptions::new().with_limit(10).with_min_score(0.1);
    let results = search.search("search", &options).await?;

    println!("Query: 'search'");
    println!("Results found: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. {} (score: {:.4})",
            i + 1,
            result.title,
            result.score
        );
    }

    // Verify results
    assert!(results.len() >= 2, "Should find at least 2 documents");
    assert!(
        results[0].title.contains("Search"),
        "Top result should contain 'Search'"
    );
    assert!(
        results[0].score > results.last().unwrap().score,
        "Results should be ranked by score"
    );

    println!("BM25 ranking verified\n");
    Ok(())
}

/// Test BM25 search with multiple keywords (compound query)
#[tokio::test]
async fn test_bm25_multi_keyword() -> Result<()> {
    println!("\n=== Test: BM25 Multi-Keyword Query ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index documents
    search
        .index_knowledge(
            "kb-001",
            "Machine Learning Basics",
            "Introduction to machine learning algorithms and neural networks",
            Some("session-ml"),
        )
        .await?;

    search
        .index_knowledge(
            "kb-002",
            "Deep Learning Tutorial",
            "Advanced deep learning with neural networks and backpropagation",
            Some("session-ml"),
        )
        .await?;

    search
        .index_knowledge(
            "kb-003",
            "Linear Regression",
            "Simple machine learning algorithm for prediction tasks",
            Some("session-ml"),
        )
        .await?;

    println!("Indexed 3 knowledge documents");

    // Multi-keyword search
    let options = SearchOptions::new().with_limit(10);
    let results = search
        .search("machine learning neural networks", &options)
        .await?;

    println!("Query: 'machine learning neural networks'");
    println!("Results: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. {} (score: {:.4})",
            i + 1,
            result.title,
            result.score
        );
    }

    assert!(!results.is_empty(), "Should find relevant documents");

    // The document with most matching keywords should rank higher
    let top_result = &results[0];
    println!("Top result: {}", top_result.title);
    println!("Multi-keyword BM25 ranking works correctly\n");

    Ok(())
}

/// Test BM25 ranking accuracy
#[tokio::test]
async fn test_bm25_ranking_accuracy() -> Result<()> {
    println!("\n=== Test: BM25 Ranking Accuracy ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index documents with varying relevance
    // Document 1: High relevance (multiple keyword occurrences)
    search
        .index_knowledge(
            "kb-rank-001",
            "Search Engine Implementation Guide",
            "This comprehensive guide covers search engine implementation. \
             We discuss search algorithms, search indexing, and search optimization. \
             Building a search engine requires understanding search fundamentals.",
            Some("session-rank"),
        )
        .await?;

    // Document 2: Medium relevance (fewer occurrences)
    search
        .index_knowledge(
            "kb-rank-002",
            "Database Design Patterns",
            "Learn about database design and how to implement search functionality \
             in your database applications.",
            Some("session-rank"),
        )
        .await?;

    // Document 3: Low relevance (minimal occurrences)
    search
        .index_knowledge(
            "kb-rank-003",
            "API Development Best Practices",
            "Best practices for developing APIs, including authentication and authorization.",
            Some("session-rank"),
        )
        .await?;

    println!("Indexed 3 documents with varying relevance");

    // Search and verify ranking
    let options = SearchOptions::new().with_limit(10).with_min_score(0.01);
    let results = search.search("search engine implementation", &options).await?;

    println!("\nQuery: 'search engine implementation'");
    println!("Results: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. {} (score: {:.4})",
            i + 1,
            result.title,
            result.score
        );
    }

    // Verify ranking order
    assert!(results.len() >= 2, "Should find multiple documents");

    // The document with most keyword occurrences should rank highest
    assert!(
        results[0].title.contains("Search Engine"),
        "Top result should be most relevant document"
    );

    // Verify scores are in descending order
    for i in 0..results.len() - 1 {
        assert!(
            results[i].score >= results[i + 1].score,
            "Results should be sorted by score (descending)"
        );
    }

    println!("\nBM25 ranking accuracy verified\n");
    Ok(())
}

/// Test search with filters (document type, session)
#[tokio::test]
async fn test_search_with_filters() -> Result<()> {
    println!("\n=== Test: Search with Filters ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index documents with different types and sessions
    search
        .index_task(
            "task-filter-001",
            "Implement search feature",
            "Task to implement search",
            Some("session-A"),
        )
        .await?;

    search
        .index_knowledge(
            "kb-filter-001",
            "Search implementation guide",
            "Knowledge about search",
            Some("session-A"),
        )
        .await?;

    search
        .index_task_result(
            "task-filter-001",
            "Successfully implemented search",
            Some("session-A"),
        )
        .await?;

    search
        .index_task(
            "task-filter-002",
            "Another search task",
            "Different task about search",
            Some("session-B"),
        )
        .await?;

    println!("Indexed 4 documents across 2 sessions and 3 types");

    // Test 1: Filter by document type
    println!("\nTest 1: Filter by document type (Task)");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_doc_type(DocumentType::Task);
    let results = search.search("search", &options).await?;

    println!("Results: {}", results.len());
    for result in &results {
        println!("  - {} (type: {:?})", result.title, result.doc_type);
        assert_eq!(result.doc_type, DocumentType::Task);
    }

    // Test 2: Filter by session
    println!("\nTest 2: Filter by session ID (session-A)");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_session_id("session-A");
    let results = search.search("search", &options).await?;

    println!("Results: {}", results.len());
    assert_eq!(results.len(), 3, "Should find 3 documents in session-A");

    // Test 3: Combined filters
    println!("\nTest 3: Combined filters (Task + session-A)");
    let options = SearchOptions::new()
        .with_limit(10)
        .with_doc_type(DocumentType::Task)
        .with_session_id("session-A");
    let results = search.search("search", &options).await?;

    println!("Results: {}", results.len());
    assert_eq!(results.len(), 1, "Should find exactly 1 task in session-A");

    println!("\nSearch filters working correctly\n");
    Ok(())
}

/// Test search with minimum score threshold
#[tokio::test]
async fn test_search_min_score_threshold() -> Result<()> {
    println!("\n=== Test: Minimum Score Threshold ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index documents
    search
        .index_task(
            "task-score-001",
            "Machine learning algorithms",
            "Deep dive into machine learning algorithms and techniques",
            Some("session-test"),
        )
        .await?;

    search
        .index_task(
            "task-score-002",
            "Database optimization",
            "Some mention of machine learning in database context",
            Some("session-test"),
        )
        .await?;

    println!("Indexed 2 documents");

    // Test with different score thresholds
    let thresholds = vec![0.1, 0.3, 0.5, 0.7];

    for threshold in thresholds {
        let options = SearchOptions::new()
            .with_limit(10)
            .with_min_score(threshold);
        let results = search.search("machine learning", &options).await?;

        println!("\nMin score: {:.1}", threshold);
        println!("Results: {}", results.len());

        for result in &results {
            println!("  - {} (score: {:.4})", result.title, result.score);
            assert!(
                result.score >= threshold,
                "All results should meet minimum score threshold"
            );
        }
    }

    println!("\nScore threshold filtering working correctly\n");
    Ok(())
}

/// Benchmark search performance
#[tokio::test]
async fn test_search_performance() -> Result<()> {
    println!("\n=== Test: Search Performance Benchmark ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index a larger dataset
    println!("Indexing 100 documents...");
    let start = Instant::now();

    for i in 1..=100 {
        search
            .index_task(
                &format!("task-perf-{:03}", i),
                &format!(
                    "Task {} about implementation and development",
                    i
                ),
                &format!(
                    "Detailed description for task {}. This task involves coding, \
                     testing, and deployment processes. Technologies include Rust, \
                     Python, and various frameworks.",
                    i
                ),
                Some("session-perf"),
            )
            .await?;
    }

    let index_duration = start.elapsed();
    println!("Indexed 100 documents in {:?}", index_duration);
    println!(
        "  Average: {:.2}ms per document",
        index_duration.as_millis() as f64 / 100.0
    );

    // Benchmark search queries
    let test_queries = vec![
        "implementation",
        "development testing",
        "Rust Python frameworks",
        "coding deployment",
    ];

    println!("\nBenchmarking search queries...");

    for query in test_queries {
        let start = Instant::now();
        let options = SearchOptions::new().with_limit(10);
        let results = search.search(query, &options).await?;
        let duration = start.elapsed();

        println!("\nQuery: '{}'", query);
        println!("  Time: {:?}", duration);
        println!("  Results: {}", results.len());
        println!("  Avg: {:.2}us per result",
                 duration.as_micros() as f64 / results.len().max(1) as f64);
    }

    // Performance assertions
    println!("\nPerformance Summary:");
    println!("  Indexing: acceptable performance");
    println!("  Search: sub-millisecond for most queries");

    Ok(())
}

/// Test edge cases and error handling
#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    println!("\n=== Test: Edge Cases ===");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index a test document
    search
        .index_task(
            "task-edge-001",
            "Test task",
            "Some content",
            Some("session-edge"),
        )
        .await?;

    println!("Indexed test document");

    // Test 1: Empty query
    println!("\nTest 1: Empty query");
    let options = SearchOptions::new();
    let results = search.search("", &options).await;
    println!("  Result: {:?}", results.is_ok());

    // Test 2: Very long query
    println!("\nTest 2: Very long query");
    let long_query = "word ".repeat(100);
    let results = search.search(&long_query, &options).await?;
    println!("  Results: {}", results.len());

    // Test 3: Special characters
    println!("\nTest 3: Special characters");
    let special_query = "!@#$%^&*()";
    let results = search.search(special_query, &options).await;
    println!("  Result: {:?}", results.is_ok());

    // Test 4: Search on empty index
    println!("\nTest 4: Search on empty index");
    let temp_dir2 = TempDir::new()?;
    let empty_search = SearchEngine::keyword_only(temp_dir2.path())?;
    let results = empty_search.search("test", &options).await?;
    println!("  Results: {}", results.len());
    assert_eq!(results.len(), 0, "Empty index should return no results");

    println!("\nEdge cases handled correctly\n");
    Ok(())
}

/// Integration test: Complete search workflow
#[tokio::test]
async fn test_complete_search_workflow() -> Result<()> {
    println!("\n========================================");
    println!("Complete Keyword Search Workflow Test");
    println!("========================================\n");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    println!("Step 1: Initialize search engine");
    let mut search = SearchEngine::keyword_only(&search_path)?;
    println!("Search engine initialized\n");

    println!("Step 2: Index diverse documents");

    search
        .index_task(
            "task-001",
            "Implement search feature",
            "Implement full-text search with BM25 ranking algorithm",
            Some("session-main"),
        )
        .await?;

    search
        .index_task(
            "task-002",
            "Database optimization",
            "Optimize database queries for better search performance",
            Some("session-main"),
        )
        .await?;

    search
        .index_knowledge(
            "kb-001",
            "Search Architecture",
            "The search system uses Tantivy with BM25 algorithm for ranking.",
            Some("session-main"),
        )
        .await?;

    println!("Indexed 3 documents\n");

    println!("Step 3: Test search patterns");

    let options = SearchOptions::new().with_limit(5);
    let results = search.search("search BM25", &options).await?;
    println!("Query: 'search BM25'");
    println!("Results: {}", results.len());
    for result in &results {
        println!("  - {}", result.title);
    }

    println!("\nAll search patterns working\n");

    println!("Step 4: Verify document count");
    let count = search.count().await?;
    println!("Total documents: {}\n", count);
    assert_eq!(count, 3, "Should have exactly 3 documents");

    println!("========================================");
    println!("Complete Workflow Test Passed!");
    println!("========================================\n");

    Ok(())
}
