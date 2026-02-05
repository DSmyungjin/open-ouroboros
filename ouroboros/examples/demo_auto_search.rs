//! Demo: Auto Search Feature
//!
//! This example demonstrates how the Auto Search feature works in Ouroboros.
//!
//! Run with:
//! ```
//! cargo run --example demo_auto_search
//! ```

use anyhow::Result;
use ouroboros::search::{SearchEngine, SearchOptions};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║   Ouroboros Auto Search Demo            ║");
    println!("╚══════════════════════════════════════════╝\n");

    // Setup
    let temp_dir = TempDir::new()?;
    let search_path = temp_dir.path().join("search_index");
    let mut search = SearchEngine::keyword_only(&search_path)?;

    println!("📚 Step 1: Indexing sample documents...\n");

    // Index some knowledge documents
    search.index_knowledge(
        "kb-001",
        "Auto Search Architecture",
        "The Auto Search feature automatically discovers relevant documents before task execution. \
         It uses keyword extraction from task descriptions, BM25 scoring, and configurable relevance thresholds.",
        Some("demo-session"),
    ).await?;

    search.index_knowledge(
        "kb-002",
        "Keyword Extraction Algorithm",
        "Keywords are extracted by tokenizing task subject and description, filtering stopwords, \
         and selecting the top 10 most relevant terms. Supports both English and Korean.",
        Some("demo-session"),
    ).await?;

    search.index_knowledge(
        "kb-003",
        "Search Configuration Options",
        "Auto Search can be configured with: auto_search_enabled (on/off), \
         auto_search_max_results (number of results), and auto_search_min_score (relevance threshold).",
        Some("demo-session"),
    ).await?;

    // Index some task results
    search.index_task_result(
        "task-001",
        "Successfully implemented the keyword extraction feature. \
         Used Rust's split function with Unicode support for multilingual tokenization.",
        Some("demo-session"),
    ).await?;

    search.index_task_result(
        "task-002",
        "Configured BM25 scoring parameters for optimal relevance ranking. \
         Tested with various document types and query patterns.",
        Some("demo-session"),
    ).await?;

    println!("✓ Indexed 5 documents\n");
    println!("─────────────────────────────────────────\n");

    // Simulate Auto Search scenarios
    demo_scenario(
        &search,
        "Scenario 1: Finding Architecture Documentation",
        "auto search implementation details",
    ).await?;

    demo_scenario(
        &search,
        "Scenario 2: Looking for Configuration Help",
        "how to configure search settings",
    ).await?;

    demo_scenario(
        &search,
        "Scenario 3: Keyword Extraction Information",
        "keyword extraction algorithm",
    ).await?;

    demo_scenario(
        &search,
        "Scenario 4: Korean Query (한글 검색)",
        "검색 설정 옵션",
    ).await?;

    println!("\n╔══════════════════════════════════════════╗");
    println!("║   Demo Complete! ✓                      ║");
    println!("╚══════════════════════════════════════════╝\n");

    println!("💡 Key Takeaways:\n");
    println!("  1. Auto Search automatically finds relevant docs");
    println!("  2. Works with both English and Korean");
    println!("  3. Uses BM25 scoring for relevance ranking");
    println!("  4. Configurable thresholds and result limits");
    println!("  5. Injected into task context before execution\n");

    Ok(())
}

async fn demo_scenario(
    search: &SearchEngine,
    title: &str,
    query: &str,
) -> Result<()> {
    println!("🔍 {}", title);
    println!("   Query: \"{}\"", query);
    println!();

    let options = SearchOptions::new()
        .with_limit(5)
        .with_min_score(0.3);

    let results = search.search(query, &options).await?;

    if results.is_empty() {
        println!("   ⚠️  No results found above threshold\n");
    } else {
        println!("   Found {} relevant document(s):\n", results.len());

        for (i, result) in results.iter().enumerate() {
            println!("   {}. {}", i + 1, result.title);
            println!("      Type: {:?}", result.doc_type);
            println!("      Relevance: {:.0}%", result.score * 100.0);

            // Show content preview
            let preview = if result.content.len() > 100 {
                format!("{}...", &result.content[..100])
            } else {
                result.content.clone()
            };
            println!("      Preview: {}", preview);
            println!();
        }
    }

    println!("─────────────────────────────────────────\n");

    Ok(())
}
