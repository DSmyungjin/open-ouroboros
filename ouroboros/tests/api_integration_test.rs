//! Integration tests for API server with JWT authentication

use reqwest::{Client, StatusCode};
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

use ouroboros::api::server::{ApiServer, ApiServerConfig};
use ouroboros::search::{DocumentType, SearchEngine};

/// Test helper to start the API server in the background
async fn start_test_server(data_dir: PathBuf, port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let config = ApiServerConfig {
            host: "127.0.0.1".to_string(),
            port,
            jwt_secret: "test_secret_key_12345".to_string(),
            data_dir,
        };

        let server = ApiServer::new(config);
        let _ = server.start().await;
    })
}

#[tokio::test]
async fn test_health_check() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8081;

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    // Test health check endpoint
    let client = Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
async fn test_login_success() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8082;

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    // Test login endpoint
    let client = Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/login", port))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["token"].is_string());
    assert_eq!(body["expires_in_hours"], 24);
}

#[tokio::test]
async fn test_login_empty_credentials() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8083;

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    // Test login with empty credentials
    let client = Client::new();
    let response = client
        .post(format!("http://127.0.0.1:{}/login", port))
        .json(&json!({
            "username": "",
            "password": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_search_unauthorized() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8084;

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    // Test search without authorization
    let client = Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/api/search?query=test", port))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_search_with_invalid_token() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8085;

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    // Test search with invalid token
    let client = Client::new();
    let response = client
        .get(format!("http://127.0.0.1:{}/api/search?query=test", port))
        .header("Authorization", "Bearer invalid_token_here")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_search_with_valid_token() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8086;

    // Create search index with test data
    let search_index_path = temp_dir.path().join("search_index");
    {
        let mut engine = SearchEngine::keyword_only(&search_index_path).unwrap();
        engine
            .index_task("task-001", "Test Task", "This is a test task description", None)
            .await
            .unwrap();
    }

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    let client = Client::new();

    // Login to get token
    let login_response = client
        .post(format!("http://127.0.0.1:{}/login", port))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let token = login_body["token"].as_str().unwrap();

    // Test search with valid token
    let search_response = client
        .get(format!("http://127.0.0.1:{}/api/search?query=test", port))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body: serde_json::Value = search_response.json().await.unwrap();
    assert!(search_body["results"].is_array());
    assert!(search_body["total"].is_number());
}

#[tokio::test]
async fn test_search_with_filters() {
    let temp_dir = TempDir::new().unwrap();
    let port = 8087;

    // Create search index with test data
    let search_index_path = temp_dir.path().join("search_index");
    {
        let mut engine = SearchEngine::keyword_only(&search_index_path).unwrap();
        engine
            .index_task("task-001", "Authentication Task", "Implement JWT authentication", None)
            .await
            .unwrap();
        engine
            .index_task_result("task-001", "JWT authentication implemented successfully", None)
            .await
            .unwrap();
    }

    // Start server
    let _server_handle = start_test_server(temp_dir.path().to_path_buf(), port).await;
    sleep(Duration::from_secs(1)).await;

    let client = Client::new();

    // Login to get token
    let login_response = client
        .post(format!("http://127.0.0.1:{}/login", port))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await
        .unwrap();

    let login_body: serde_json::Value = login_response.json().await.unwrap();
    let token = login_body["token"].as_str().unwrap();

    // Test search with type filter
    let search_response = client
        .get(format!("http://127.0.0.1:{}/api/search?query=authentication&type=task&limit=5", port))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body: serde_json::Value = search_response.json().await.unwrap();
    let results = search_body["results"].as_array().unwrap();

    // Should find the task
    assert!(!results.is_empty());
    assert_eq!(search_body["total"].as_u64().unwrap(), results.len() as u64);
}
