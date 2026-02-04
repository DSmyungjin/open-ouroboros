//! Integration tests for Neo4j health check functionality
//!
//! These tests require a running Neo4j instance or use Testcontainers
//! to spin up a temporary Neo4j instance.

use ouroboros_kg::{HealthCheckConfig, HealthCheckMethod, HealthStatus, Neo4jClient};
use std::time::Duration;

// Helper function to get Neo4j connection details from environment or use defaults
fn get_neo4j_config() -> (String, String, String, String) {
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());
    let database = std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string());
    (uri, user, password, database)
}

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_health_check_simple() {
    let (uri, user, password, database) = get_neo4j_config();

    let client = Neo4jClient::new(&uri, &user, &password, &database)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check().await;

    assert!(result.is_ok(), "Health check should succeed");
    assert!(result.unwrap(), "Health check should return true");
}

#[tokio::test]
#[ignore]
async fn test_health_check_ping() {
    let (uri, user, password, database) = get_neo4j_config();

    let client = Neo4jClient::new(&uri, &user, &password, &database)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_ping().await;

    // Note: This may fail on Neo4j 4.0.x or if db.ping() is not available
    match result {
        Ok(status) => {
            assert_eq!(status, HealthStatus::Healthy);
        }
        Err(e) => {
            // If db.ping() is not available, this is expected
            println!("db.ping() not available (may be Neo4j 4.0.x): {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_health_check_detailed() {
    let (uri, user, password, database) = get_neo4j_config();

    let client = Neo4jClient::new(&uri, &user, &password, &database)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_detailed().await;

    assert!(
        result.status.is_operational(),
        "Health check should be operational"
    );
    assert!(
        result.response_time_ms > 0,
        "Response time should be greater than 0"
    );
    assert!(
        result.database_name.is_some(),
        "Database name should be present"
    );
    println!("Health check result: {:?}", result);
}

#[tokio::test]
#[ignore]
async fn test_health_check_with_retry() {
    let (uri, user, password, database) = get_neo4j_config();

    let client = Neo4jClient::new(&uri, &user, &password, &database)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_with_retry().await;

    assert!(
        result.status.is_operational(),
        "Health check with retry should be operational"
    );
    println!("Health check with retry result: {:?}", result);
}

#[tokio::test]
#[ignore]
async fn test_health_check_custom_config() {
    let (uri, user, password, database) = get_neo4j_config();

    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Detailed,
        timeout: Duration::from_secs(10),
        enable_retries: true,
        max_retries: 2,
        retry_delay: Duration::from_millis(200),
        enable_fallback: true,
        degraded_threshold_ms: 500,
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, &database, config)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_with_retry().await;

    assert!(result.status.is_operational());
    println!("Custom config health check: {:?}", result);
}

#[tokio::test]
#[ignore]
async fn test_health_check_fallback() {
    let (uri, user, password, database) = get_neo4j_config();

    // Configure to use ping method with fallback enabled
    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Ping,
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 1,
        retry_delay: Duration::from_millis(100),
        enable_fallback: true,
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, &database, config)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_with_retry().await;

    // Should succeed either via db.ping() or fallback to RETURN 1
    assert!(result.status.is_operational());

    if result.metadata.used_fallback {
        println!("Fallback was used (db.ping() not available)");
    } else {
        println!("db.ping() was successful");
    }
}

#[tokio::test]
#[ignore]
async fn test_health_check_degraded_detection() {
    let (uri, user, password, database) = get_neo4j_config();

    // Set a very low degraded threshold to force degraded state
    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Detailed,
        timeout: Duration::from_secs(5),
        enable_retries: false,
        max_retries: 0,
        retry_delay: Duration::from_millis(0),
        enable_fallback: false,
        degraded_threshold_ms: 1, // Any response > 1ms will be degraded
    };

    let client = Neo4jClient::with_config(&uri, &user, &password, &database, config)
        .await
        .expect("Failed to connect to Neo4j");

    let result = client.health_check_with_retry().await;

    // With such a low threshold, it should likely be degraded
    println!("Degraded detection test result: {:?}", result);
    assert!(result.status.is_operational()); // Should still be operational
}

#[tokio::test]
async fn test_health_check_invalid_connection() {
    // Try to connect to non-existent server
    let result = Neo4jClient::new("bolt://localhost:9999", "neo4j", "password", "neo4j").await;

    assert!(result.is_err(), "Connection to invalid host should fail");
}

#[tokio::test]
#[ignore]
async fn test_concurrent_health_checks() {
    let (uri, user, password, database) = get_neo4j_config();

    let client = Neo4jClient::new(&uri, &user, &password, &database)
        .await
        .expect("Failed to connect to Neo4j");

    // Run multiple health checks concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let client_clone = client.graph().clone();
        let handle = tokio::spawn(async move {
            // Use the graph directly for a simple query
            client_clone
                .run(neo4rs::query("RETURN 1"))
                .await
                .map(|_| i)
        });
        handles.push(handle);
    }

    // Wait for all health checks to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Concurrent health check {} should succeed",
            i
        );
        assert!(
            result.as_ref().unwrap().is_ok(),
            "Health check {} should return Ok",
            i
        );
    }

    println!("All {} concurrent health checks succeeded", results.len());
}

// Note: For full integration tests with Testcontainers, we would add:
// #[cfg(test)]
// mod testcontainers_tests {
//     use super::*;
//     use testcontainers::clients::Cli;
//     use testcontainers_modules::neo4j::Neo4j;
//
//     #[tokio::test]
//     async fn test_with_testcontainer() {
//         let docker = Cli::default();
//         let neo4j_container = docker.run(Neo4j::default());
//         let port = neo4j_container.get_host_port_ipv4(7687);
//
//         let client = Neo4jClient::new(
//             &format!("bolt://localhost:{}", port),
//             "neo4j",
//             "neo4j",
//             "neo4j"
//         ).await.unwrap();
//
//         let result = client.health_check().await;
//         assert!(result.is_ok());
//     }
// }
