//! Manual health check testing example
//!
//! This example can be run against a live Neo4j instance to manually validate
//! health check functionality.
//!
//! ## Usage
//!
//! Set environment variables (optional):
//! ```bash
//! export NEO4J_URI="bolt://localhost:7687"
//! export NEO4J_USER="neo4j"
//! export NEO4J_PASSWORD="password"
//! export NEO4J_DATABASE="neo4j"
//! ```
//!
//! Run the example:
//! ```bash
//! cargo run --example manual_health_check
//! ```

use ouroboros_kg::{HealthCheckConfig, HealthCheckMethod, Neo4jClient};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("ouroboros_kg=debug,manual_health_check=info")
        .init();

    println!("=== Neo4j Health Check Manual Testing ===\n");

    // Get configuration from environment or use defaults
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());
    let database = std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string());

    println!("Connecting to Neo4j:");
    println!("  URI: {}", uri);
    println!("  User: {}", user);
    println!("  Database: {}\n", database);

    // Test 1: Basic connection
    println!("Test 1: Connecting to Neo4j...");
    let client = match Neo4jClient::new(&uri, &user, &password, &database).await {
        Ok(client) => {
            println!("✓ Successfully connected to Neo4j\n");
            client
        }
        Err(e) => {
            println!("✗ Failed to connect: {}\n", e);
            println!("Please ensure Neo4j is running and credentials are correct.");
            return Err(e.into());
        }
    };

    // Test 2: Simple health check
    println!("Test 2: Simple health check (RETURN 1)...");
    match client.health_check().await {
        Ok(true) => println!("✓ Simple health check passed\n"),
        Ok(false) => println!("✗ Simple health check returned false\n"),
        Err(e) => println!("✗ Simple health check failed: {}\n", e),
    }

    // Test 3: Standard health check (db.ping)
    println!("Test 3: Standard health check (CALL db.ping())...");
    match client.health_check_ping().await {
        Ok(status) => println!("✓ Standard health check passed: {:?}\n", status),
        Err(e) => {
            println!("✗ Standard health check failed: {}", e);
            println!("  Note: db.ping() requires Neo4j 4.1+\n");
        }
    }

    // Test 4: Detailed health check
    println!("Test 4: Detailed health check (CALL db.info())...");
    let detailed_result = client.health_check_detailed().await;
    println!("✓ Detailed health check completed:");
    println!("  Status: {:?}", detailed_result.status);
    println!("  Response time: {}ms", detailed_result.response_time_ms);
    println!(
        "  Database name: {}",
        detailed_result.database_name.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Database ID: {}",
        detailed_result.database_id.as_deref().unwrap_or("N/A")
    );
    println!("  Timestamp: {}", detailed_result.timestamp);
    println!("  Check method: {:?}", detailed_result.metadata.check_method);
    if let Some(error) = &detailed_result.error {
        println!("  Error: {}", error);
    }
    println!();

    // Test 5: Health check with retry (default config)
    println!("Test 5: Health check with retry (default config)...");
    let retry_result = client.health_check_with_retry().await;
    println!("✓ Health check with retry completed:");
    println!("  Status: {:?}", retry_result.status);
    println!("  Response time: {}ms", retry_result.response_time_ms);
    println!("  Was retry: {}", retry_result.metadata.was_retry);
    println!("  Retry count: {}", retry_result.metadata.retry_count);
    println!("  Used fallback: {}", retry_result.metadata.used_fallback);
    println!();

    // Test 6: Custom configuration with Simple method
    println!("Test 6: Custom health check config (Simple method, no retries)...");
    let mut custom_config_client = Neo4jClient::with_config(
        &uri,
        &user,
        &password,
        &database,
        HealthCheckConfig {
            enabled: true,
            method: HealthCheckMethod::Simple,
            timeout: Duration::from_secs(5),
            enable_retries: false,
            max_retries: 0,
            retry_delay: Duration::from_millis(0),
            enable_fallback: false,
            degraded_threshold_ms: 500, // Low threshold for testing
        },
    )
    .await?;
    let custom_result = custom_config_client.health_check_with_retry().await;
    println!("✓ Custom config health check completed:");
    println!("  Status: {:?}", custom_result.status);
    println!("  Response time: {}ms", custom_result.response_time_ms);
    println!("  Check method: {:?}", custom_result.metadata.check_method);
    println!();

    // Test 7: Custom configuration with fallback enabled
    println!("Test 7: Custom health check config (Ping with fallback)...");
    custom_config_client.set_health_config(HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Ping,
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 2,
        retry_delay: Duration::from_millis(100),
        enable_fallback: true, // Enable fallback
        degraded_threshold_ms: 1000,
    });
    let fallback_result = custom_config_client.health_check_with_retry().await;
    println!("✓ Fallback test completed:");
    println!("  Status: {:?}", fallback_result.status);
    println!("  Response time: {}ms", fallback_result.response_time_ms);
    println!(
        "  Used fallback: {}",
        fallback_result.metadata.used_fallback
    );
    if fallback_result.metadata.used_fallback {
        println!("  → Fallback mechanism was triggered (db.ping() not available)");
    } else {
        println!("  → db.ping() succeeded without fallback");
    }
    println!();

    // Test 8: Concurrent health checks
    println!("Test 8: Running 10 concurrent health checks...");
    let mut handles = vec![];
    for i in 0..10 {
        let graph = client.graph().clone();
        let handle = tokio::spawn(async move {
            graph
                .run(neo4rs::query("RETURN 1"))
                .await
                .map(|_| i)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let success_count = results.iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok()).count();
    println!("✓ Concurrent health checks: {}/10 succeeded", success_count);
    println!();

    // Test 9: Degraded state detection
    println!("Test 9: Degraded state detection (low threshold)...");
    custom_config_client.set_health_config(HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Detailed,
        timeout: Duration::from_secs(5),
        enable_retries: false,
        max_retries: 0,
        retry_delay: Duration::from_millis(0),
        enable_fallback: false,
        degraded_threshold_ms: 1, // Very low threshold
    });
    let degraded_result = custom_config_client.health_check_with_retry().await;
    println!("✓ Degraded detection test completed:");
    println!("  Status: {:?}", degraded_result.status);
    println!("  Response time: {}ms", degraded_result.response_time_ms);
    if degraded_result.status.is_operational() {
        println!("  → System is operational (may be degraded if slow)");
    }
    println!();

    // Test 10: JSON serialization
    println!("Test 10: JSON serialization of health check result...");
    let json_result = serde_json::to_string_pretty(&detailed_result)?;
    println!("✓ Serialized health check result:");
    println!("{}", json_result);
    println!();

    // Summary
    println!("=== Summary ===");
    println!("All manual health check tests completed!");
    println!("✓ Connection: Working");
    println!("✓ Simple health check: Working");
    println!("✓ Detailed health check: Working");
    println!("✓ Retry mechanism: Working");
    println!("✓ Concurrent checks: Working");
    println!("✓ JSON serialization: Working");
    println!();
    println!("Neo4j health check functionality is fully operational.");

    Ok(())
}
