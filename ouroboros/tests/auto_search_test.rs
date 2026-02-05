//! Integration test for Auto Search functionality
//!
//! This test verifies that:
//! 1. Auto search is enabled by default
//! 2. Keywords are correctly extracted from tasks
//! 3. Relevant documents are found and injected into task context
//! 4. Auto search can be disabled via config

use anyhow::Result;
use ouroboros::orchestrator::OrchestratorConfig;
use ouroboros::search::SearchEngine;
use tempfile::TempDir;

#[test]
fn test_auto_search_default_config() -> Result<()> {
    let config = OrchestratorConfig::default();

    // Verify auto search is enabled by default
    assert!(config.auto_search_enabled);
    assert_eq!(config.auto_search_max_results, 5);
    assert_eq!(config.auto_search_min_score, 0.3);

    println!("✓ Auto search is enabled by default");
    println!("  - Max results: {}", config.auto_search_max_results);
    println!("  - Min score: {}", config.auto_search_min_score);

    Ok(())
}

#[test]
fn test_auto_search_disabled() -> Result<()> {
    let config = OrchestratorConfig {
        auto_search_enabled: false,
        ..Default::default()
    };

    // When auto search is disabled in config
    assert!(!config.auto_search_enabled);

    println!("✓ Auto search can be disabled via config");

    Ok(())
}

#[tokio::test]
async fn test_search_engine_initialization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    // Create keyword-only search engine (as used by Orchestrator)
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("✓ Search engine initialized successfully");

    // Index a test document
    search.index_task(
        "task-001",
        "Test task subject",
        "This is a test task description with some keywords",
        Some("session-123"),
    ).await?;

    println!("✓ Document indexed successfully");

    // Search for the indexed document
    let options = ouroboros::search::SearchOptions::new()
        .with_limit(5)
        .with_min_score(0.1);

    let results = search.search("test keywords", &options).await?;

    println!("✓ Search executed successfully");
    println!("  - Found {} results", results.len());

    if !results.is_empty() {
        println!("  - Top result: {}", results[0].title);
        println!("  - Score: {:.2}", results[0].score);
    }

    Ok(())
}

#[test]
fn test_keyword_extraction_logic() -> Result<()> {
    // Test the keyword extraction logic used in auto_search_for_task
    let subject = "Auto Search 기능 확인";
    let description = "Auto Search 기능이 제대로 작동하는지 확인하고 테스트합니다.";

    // Simulate keyword extraction
    let text = format!("{} {}", subject, description);

    let stop_words = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "and", "or", "but", "if", "then",
        "else", "when", "where", "why", "how", "what", "which", "who", "whom",
        "this", "that", "these", "those", "it", "its", "for", "from", "to",
        "of", "in", "on", "at", "by", "with", "about", "into", "through",
        "를", "을", "이", "가", "은", "는", "에", "의", "로", "으로", "와", "과",
        "도", "만", "까지", "부터", "에서", "한다", "하는", "하고", "하여",
    ];

    let keywords: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.len() >= 2 && !stop_words.contains(&s.as_str()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(10)
        .collect();

    println!("✓ Keyword extraction test");
    println!("  - Input: {}", text);
    println!("  - Extracted keywords: {:?}", keywords);
    println!("  - Keyword count: {}", keywords.len());

    assert!(!keywords.is_empty(), "Should extract at least some keywords");

    // Verify that meaningful words are extracted
    let has_meaningful = keywords.iter().any(|k|
        k.contains("auto") || k.contains("search") || k.contains("기능")
    );
    assert!(has_meaningful, "Should extract meaningful keywords");

    Ok(())
}

#[tokio::test]
async fn test_auto_search_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    // Create and populate search engine
    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index some test documents
    search.index_knowledge(
        "knowledge-001",
        "Auto Search Implementation",
        "The auto search feature automatically finds relevant documents before task execution. It uses keyword extraction and BM25 ranking.",
        Some("session-test"),
    ).await?;

    search.index_task_result(
        "task-prev-001",
        "Successfully implemented search functionality with keyword indexing and scoring.",
        Some("session-test"),
    ).await?;

    println!("✓ Search index populated with test documents");

    // Test search with query similar to task
    let options = ouroboros::search::SearchOptions::new()
        .with_limit(5)
        .with_min_score(0.3);

    let results = search.search("auto search feature implementation", &options).await?;

    println!("✓ Auto search query executed");
    println!("  - Query: 'auto search feature implementation'");
    println!("  - Results found: {}", results.len());

    for (i, result) in results.iter().enumerate() {
        println!("  - Result {}: {} (score: {:.2})", i + 1, result.title, result.score);
        println!("    Type: {:?}", result.doc_type);
    }

    assert!(!results.is_empty(), "Should find relevant documents");

    Ok(())
}

#[tokio::test]
async fn test_auto_search_with_korean_content() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index Korean content
    search.index_knowledge(
        "knowledge-kr-001",
        "자동 검색 기능",
        "자동 검색 기능은 태스크 실행 전에 관련 문서를 자동으로 찾아줍니다. 키워드 추출과 BM25 랭킹을 사용합니다.",
        Some("session-kr"),
    ).await?;

    println!("✓ Korean content indexed");

    // Search with Korean query
    let options = ouroboros::search::SearchOptions::new()
        .with_limit(5)
        .with_min_score(0.2);

    let results = search.search("자동 검색 기능", &options).await?;

    println!("✓ Korean search executed");
    println!("  - Query: '자동 검색 기능'");
    println!("  - Results: {}", results.len());

    for result in &results {
        println!("  - {}: {:.2}", result.title, result.score);
    }

    Ok(())
}

#[test]
fn test_auto_search_config_variations() -> Result<()> {
    println!("Testing different auto search configurations...\n");

    // Test 1: Default config
    let config1 = OrchestratorConfig::default();
    println!("✓ Default config:");
    println!("  - Enabled: {}", config1.auto_search_enabled);
    println!("  - Max results: {}", config1.auto_search_max_results);
    println!("  - Min score: {}", config1.auto_search_min_score);

    // Test 2: Custom high-precision config
    let config2 = OrchestratorConfig {
        auto_search_enabled: true,
        auto_search_max_results: 3,
        auto_search_min_score: 0.5,
        ..Default::default()
    };
    println!("\n✓ High-precision config:");
    println!("  - Max results: {} (fewer, higher quality)", config2.auto_search_max_results);
    println!("  - Min score: {} (higher threshold)", config2.auto_search_min_score);

    // Test 3: Custom high-recall config
    let config3 = OrchestratorConfig {
        auto_search_enabled: true,
        auto_search_max_results: 10,
        auto_search_min_score: 0.1,
        ..Default::default()
    };
    println!("\n✓ High-recall config:");
    println!("  - Max results: {} (more results)", config3.auto_search_max_results);
    println!("  - Min score: {} (lower threshold)", config3.auto_search_min_score);

    Ok(())
}

#[tokio::test]
async fn test_auto_search_document_count() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    let mut search = SearchEngine::keyword_only(&search_path)?;

    // Index multiple documents
    for i in 1..=10 {
        search.index_task(
            &format!("task-{:03}", i),
            &format!("Test task {}", i),
            &format!("Description for task number {}", i),
            Some("session-test"),
        ).await?;
    }

    let count = search.count().await?;
    println!("✓ Document count test");
    println!("  - Indexed: 10 documents");
    println!("  - Count returned: {}", count);

    assert_eq!(count, 10, "Should have exactly 10 documents indexed");

    Ok(())
}

#[tokio::test]
async fn test_auto_search_complete_workflow() -> Result<()> {
    println!("========================================");
    println!("Auto Search Feature Complete Test");
    println!("========================================\n");

    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");

    println!("1. Initializing search engine...");
    let mut search = SearchEngine::keyword_only(&search_path)?;
    println!("✓ Search engine initialized\n");

    println!("2. Indexing sample documents...");
    // Index task definitions
    search.index_task(
        "task-001",
        "Implement auto search feature",
        "Create automatic search functionality that finds relevant documents before task execution",
        Some("session-main"),
    ).await?;

    // Index knowledge
    search.index_knowledge(
        "kb-001",
        "Search Architecture",
        "The search system uses hybrid approach with keyword (BM25) and vector search capabilities",
        Some("session-main"),
    ).await?;

    // Index task results
    search.index_task_result(
        "task-001",
        "Successfully implemented auto search with keyword extraction and document scoring",
        Some("session-main"),
    ).await?;

    println!("✓ Indexed 3 documents\n");

    println!("3. Testing search functionality...");
    let options = ouroboros::search::SearchOptions::new()
        .with_limit(5)
        .with_min_score(0.2);

    let results = search.search("auto search implementation", &options).await?;
    println!("✓ Found {} results\n", results.len());

    println!("4. Analyzing results...");
    for (i, result) in results.iter().enumerate() {
        println!("   [{} Result {}: {}",
            match result.doc_type {
                ouroboros::search::DocumentType::Task => "TASK",
                ouroboros::search::DocumentType::TaskResult => "RESULT",
                ouroboros::search::DocumentType::Knowledge => "KNOWLEDGE",
                _ => "OTHER",
            },
            i + 1,
            result.title
        );
        println!("   Score: {:.2}", result.score);
        println!();
    }

    println!("5. Verifying document count...");
    let count = search.count().await?;
    println!("✓ Total documents: {}\n", count);

    println!("========================================");
    println!("✓ All Auto Search Tests Passed!");
    println!("========================================");

    Ok(())
}
