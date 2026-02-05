//! Test script to populate search index with diverse test documents

use anyhow::Result;
use ouroboros::search::SearchEngine;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize search engine for current session
    let index_path = PathBuf::from("./data/sessions/0db373-search-cli-evaluation/search_index");
    let mut engine = SearchEngine::keyword_only(&index_path)?;

    println!("Populating search index with test documents...\n");

    // 1. English task documents
    engine.index_task(
        "test-001",
        "API Design Task",
        "Design a REST API for user management with authentication and authorization",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[1] Indexed: test-001 - API Design Task");

    engine.index_task_result(
        "test-001",
        "Completed REST API design with 5 endpoints: login, logout, register, profile, and update",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[2] Indexed: result:test-001 - API Design Result");

    // 2. Korean task documents
    engine.index_task(
        "test-003",
        "데이터베이스 설계",
        "사용자 관리를 위한 데이터베이스 스키마를 설계합니다. 테이블은 users, roles, permissions를 포함해야 합니다.",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[3] Indexed: test-003 - 데이터베이스 설계");

    engine.index_context(
        "ctx-004",
        "프로젝트 컨텍스트",
        "이 프로젝트는 마이크로서비스 아키텍처를 사용합니다. Rust로 작성되며 Docker로 배포됩니다.",
        Some("0db373-search-cli-evaluation"),
        None,
    ).await?;
    println!("[4] Indexed: context:ctx-004 - 프로젝트 컨텍스트");

    // 3. Mixed language documents
    engine.index_knowledge(
        "know-005",
        "REST API 디자인 패턴",
        "RESTful API design follows HTTP methods: GET for retrieval, POST for creation, PUT for updates, DELETE for removal. 한국어로는 조회, 생성, 수정, 삭제라고 합니다.",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[5] Indexed: knowledge:know-005 - REST API 디자인 패턴");

    // 4. Technical documents
    engine.index_context(
        "ctx-006",
        "Authentication Implementation Plan",
        "Implement JWT-based authentication with refresh tokens. Use bcrypt for password hashing with a cost factor of 12.",
        Some("0db373-search-cli-evaluation"),
        None,
    ).await?;
    println!("[6] Indexed: context:ctx-006 - Authentication Implementation Plan");

    // 5. Document with special characters
    engine.index_task(
        "test-007",
        "테스트: Special Characters #@!$%",
        "Testing special characters: !@#$%^&*()_+-=[]{}|;:',.<>?/~ and emojis 🚀 🔍 ✅ ❌",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[7] Indexed: test-007 - Special Characters Test");

    // 6. Long content document
    engine.index_task_result(
        "test-008",
        "The microservices architecture consists of multiple independent services. \
         Each service has its own database and communicates via REST APIs. \
         The API gateway handles routing and authentication. \
         Services include: user service for authentication, product service for catalog, \
         order service for transactions, payment service for billing, \
         notification service for emails and SMS. All services are containerized with Docker \
         and orchestrated with Kubernetes. Monitoring is done with Prometheus and Grafana. \
         Logging uses ELK stack (Elasticsearch, Logstash, Kibana). \
         CI/CD pipeline is implemented with GitHub Actions.",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[8] Indexed: result:test-008 - Comprehensive System Architecture");

    // 7. Different session document
    engine.index_task(
        "test-009",
        "Another Session Task",
        "This document belongs to a different session for testing session filtering",
        Some("other-session-id"),
    ).await?;
    println!("[9] Indexed: test-009 - Another Session Task");

    // 8. Search query test documents
    engine.index_task(
        "test-010",
        "Search Engine Implementation",
        "Implement full-text search using Tantivy with Korean morphological analysis. Support BM25 ranking algorithm.",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[10] Indexed: test-010 - Search Engine Implementation");

    engine.index_knowledge(
        "know-011",
        "검색 최적화 전략",
        "검색 성능을 향상시키기 위해 인덱싱, 캐싱, 샤딩 전략을 사용합니다. 형태소 분석기로 한국어를 처리합니다.",
        Some("0db373-search-cli-evaluation"),
    ).await?;
    println!("[11] Indexed: knowledge:know-011 - 검색 최적화 전략");

    println!("\n✅ Successfully indexed 11 test documents");
    println!("Search index ready at: {:?}", index_path);

    Ok(())
}
