//! Integration tests for Neo4j health check failure scenarios
//!
//! These tests cover connection failures, query failures, timeouts,
//! and other error conditions.

use ouroboros_kg::{HealthCheckConfig, HealthCheckMethod, HealthStatus, Neo4jClient};
use std::time::Duration;

#[tokio::test]
async fn test_connection_failure_invalid_host() {
    // Try to connect to non-existent host
    // Note: neo4rs uses lazy connection, so we need to attempt a query
    let result = Neo4jClient::new(
        "bolt://nonexistent-host-12345.invalid:7687",
        "neo4j",
        "password",
        "neo4j",
    )
    .await;

    // Connection may succeed initially (lazy connection)
    // The actual failure happens on first query
    if let Ok(client) = result {
        let health_check = client.health_check().await;
        assert!(
            health_check.is_err(),
            "Health check should fail with connection error"
        );
        if let Err(error) = health_check {
            println!("Expected connection error during health check: {}", error);
            assert!(error.to_string().contains("onnection") || error.to_string().contains("error"));
        }
    } else if let Err(error) = result {
        // If connection fails immediately (stricter implementation)
        println!("Expected connection error: {}", error);
        assert!(error.to_string().contains("onnection"));
    }
}

#[tokio::test]
async fn test_connection_failure_invalid_port() {
    // Try to connect to invalid port
    let result = Neo4jClient::new("bolt://localhost:9999", "neo4j", "password", "neo4j").await;

    // Connection may succeed initially (lazy connection), test with health check
    if let Ok(client) = result {
        let health_check = client.health_check().await;
        assert!(
            health_check.is_err(),
            "Health check should fail with connection error"
        );
        if let Err(error) = health_check {
            println!("Expected connection error during health check: {}", error);
        }
    } else if let Err(error) = result {
        println!("Expected connection error: {}", error);
    }
}

#[tokio::test]
async fn test_connection_failure_wrong_scheme() {
    // Try to connect with wrong URI scheme
    let result = Neo4jClient::new("http://localhost:7687", "neo4j", "password", "neo4j").await;

    assert!(
        result.is_err(),
        "Connection with wrong scheme should fail with error"
    );

    if let Err(error) = result {
        println!("Expected config/connection error: {}", error);
    }
}

#[tokio::test]
#[ignore] // Requires running Neo4j instance
async fn test_authentication_failure() {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());

    // Try to connect with invalid credentials
    let result = Neo4jClient::new(&uri, "invalid_user", "invalid_password", "neo4j").await;

    // Connection may succeed initially, but operations will fail
    // This depends on Neo4j authentication configuration
    if let Err(e) = result {
        println!("Expected authentication error: {}", e);
        assert!(
            e.to_string().to_lowercase().contains("auth")
                || e.to_string().to_lowercase().contains("credential")
                || e.to_string().to_lowercase().contains("unauthorized")
        );
    } else {
        // If connection succeeds (some configs), health check should fail
        let client = result.unwrap();
        let health_result = client.health_check().await;
        assert!(
            health_result.is_err(),
            "Health check with invalid credentials should fail"
        );
    }
}

#[tokio::test]
#[ignore] // Requires running Neo4j instance with specific database
async fn test_invalid_database_name() {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

    // Try to connect to non-existent database
    let result = Neo4jClient::new(&uri, &user, &password, "nonexistent_database_12345").await;

    // Neo4j may allow connection but queries will fail
    if result.is_ok() {
        let client = result.unwrap();
        let health_result = client.health_check().await;

        // Health check may fail or succeed depending on Neo4j config
        println!("Health check with invalid database: {:?}", health_result);
    } else if let Err(e) = result {
        println!("Connection with invalid database failed: {}", e);
    }
}

#[tokio::test]
async fn test_health_check_result_serialization() {
    // Test that HealthCheckResult can be serialized/deserialized
    use ouroboros_kg::HealthCheckResult;
    use serde_json;

    let json_str = r#"{
        "status": "Healthy",
        "response_time_ms": 42,
        "database_name": "neo4j",
        "database_id": "test-id-123",
        "timestamp": "2026-02-03T12:00:00Z",
        "error": null,
        "metadata": {
            "check_method": "ping",
            "was_retry": false,
            "retry_count": 0,
            "used_fallback": false
        }
    }"#;

    let result: Result<HealthCheckResult, _> = serde_json::from_str(json_str);
    assert!(result.is_ok(), "Should deserialize valid JSON");

    let health_check = result.unwrap();
    assert_eq!(health_check.status, HealthStatus::Healthy);
    assert_eq!(health_check.response_time_ms, 42);
    assert_eq!(health_check.database_name, Some("neo4j".to_string()));

    // Test serialization
    let serialized = serde_json::to_string(&health_check).unwrap();
    assert!(serialized.contains("Healthy"));
    assert!(serialized.contains("\"response_time_ms\":42"));
}

#[tokio::test]
#[ignore] // Requires running Neo4j instance
async fn test_retry_logic_eventual_success() {
    // This test simulates transient failures by using a very short timeout
    // and relying on retries to succeed

    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Simple,
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 3,
        retry_delay: Duration::from_millis(100),
        enable_fallback: false,
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, "neo4j", config)
        .await
        .expect("Failed to connect");

    let result = client.health_check_with_retry().await;

    // With retries enabled, should eventually succeed
    assert!(result.status.is_operational());
    println!("Retry test result: {:?}", result);
}

#[tokio::test]
#[ignore] // Requires running Neo4j instance
async fn test_no_retry_on_failure() {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Simple,
        timeout: Duration::from_secs(5),
        enable_retries: false, // Disabled retries
        max_retries: 0,
        retry_delay: Duration::from_millis(0),
        enable_fallback: false,
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, "neo4j", config)
        .await
        .expect("Failed to connect");

    let result = client.health_check_with_retry().await;

    // Should succeed on first try (no retries needed for healthy connection)
    assert!(result.status.is_operational());
    assert_eq!(result.metadata.retry_count, 0);
    assert!(!result.metadata.was_retry);
}

#[tokio::test]
#[ignore] // Requires running Neo4j 4.0.x or instance without db.ping()
async fn test_fallback_from_ping_to_simple() {
    // This test requires Neo4j 4.0.x or an instance where db.ping() is not available
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Ping, // Try ping first
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 1,
        retry_delay: Duration::from_millis(100),
        enable_fallback: true, // Enable fallback
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, "neo4j", config)
        .await
        .expect("Failed to connect");

    let result = client.health_check_with_retry().await;

    println!("Fallback test result: {:?}", result);

    // Should succeed via fallback if db.ping() not available
    assert!(result.status.is_operational());

    if result.metadata.used_fallback {
        println!("✓ Fallback mechanism was successfully used");
        assert_eq!(result.metadata.check_method, HealthCheckMethod::Simple);
    } else {
        println!("✓ db.ping() was available and succeeded");
        assert_eq!(result.metadata.check_method, HealthCheckMethod::Ping);
    }
}

#[tokio::test]
#[ignore] // Requires running Neo4j instance
async fn test_health_status_http_codes() {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());

    let client = Neo4jClient::new(&uri, &user, &password, "neo4j")
        .await
        .expect("Failed to connect");

    let result = client.health_check_detailed().await;

    // Check HTTP status code mapping
    let http_code = result.status.to_http_status_code();
    println!(
        "Health status: {:?}, HTTP code: {}",
        result.status, http_code
    );

    match result.status {
        HealthStatus::Healthy | HealthStatus::Degraded => assert_eq!(http_code, 200),
        HealthStatus::Unhealthy => assert_eq!(http_code, 503),
    }
}

#[tokio::test]
async fn test_error_types() {
    use ouroboros_kg::Neo4jError;

    // Test error type creation and display
    let conn_err = Neo4jError::ConnectionError("test connection error".to_string());
    assert!(conn_err.to_string().contains("Connection error"));

    let query_err = Neo4jError::QueryError("test query error".to_string());
    assert!(query_err.to_string().contains("Query error"));

    let timeout_err = Neo4jError::TimeoutError {
        timeout_seconds: 5,
        context: "health check".to_string(),
    };
    assert!(timeout_err.to_string().contains("timed out after 5s"));

    let pool_err = Neo4jError::PoolExhaustedError {
        max_connections: 16,
    };
    assert!(pool_err.to_string().contains("16 connections"));
}
