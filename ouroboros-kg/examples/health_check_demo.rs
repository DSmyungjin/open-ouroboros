//! Health Check Demo Application
//!
//! Demonstrates all three health check methods and advanced features.
//!
//! Usage:
//!   cargo run --example health_check_demo
//!
//! Environment variables:
//!   NEO4J_URI      - Neo4j connection URI (default: bolt://localhost:7687)
//!   NEO4J_USER     - Neo4j username (default: neo4j)
//!   NEO4J_PASSWORD - Neo4j password (default: password)
//!   NEO4J_DATABASE - Neo4j database name (default: neo4j)

use ouroboros_kg::{HealthCheckConfig, HealthCheckMethod, Neo4jClient};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Neo4j Health Check Demo ===");

    // Get connection details from environment or use defaults
    let uri = std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string());
    let user = std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".to_string());
    let password = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "password".to_string());
    let database = std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string());

    info!("Connecting to Neo4j at {} (database: {})", uri, database);

    // Create client with default configuration
    let client = Neo4jClient::new(&uri, &user, &password, &database).await?;

    info!("\n--- Simple Health Check (RETURN 1) ---");
    match client.health_check().await {
        Ok(healthy) => {
            info!("✓ Simple health check passed: {}", healthy);
        }
        Err(e) => {
            info!("✗ Simple health check failed: {}", e);
        }
    }

    info!("\n--- Standard Health Check (CALL db.ping()) ---");
    match client.health_check_ping().await {
        Ok(status) => {
            info!("✓ Standard health check passed: {:?}", status);
        }
        Err(e) => {
            info!("✗ Standard health check failed: {}", e);
            info!("  (This is expected on Neo4j 4.0.x - db.ping() was added in 4.1)");
        }
    }

    info!("\n--- Detailed Health Check (CALL db.info()) ---");
    let detailed = client.health_check_detailed().await;
    info!("Status: {:?}", detailed.status);
    info!("Response time: {}ms", detailed.response_time_ms);
    if let Some(db_name) = &detailed.database_name {
        info!("Database name: {}", db_name);
    }
    if let Some(db_id) = &detailed.database_id {
        info!("Database ID: {}", db_id);
    }
    if let Some(error) = &detailed.error {
        info!("Error: {}", error);
    }

    info!("\n--- Health Check with Retry ---");
    let retry_result = client.health_check_with_retry().await;
    info!("Status: {:?}", retry_result.status);
    info!("Response time: {}ms", retry_result.response_time_ms);
    info!("Check method: {:?}", retry_result.metadata.check_method);
    info!("Retry count: {}", retry_result.metadata.retry_count);
    info!("Used fallback: {}", retry_result.metadata.used_fallback);

    info!("\n--- Custom Configuration Demo ---");
    let custom_config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Detailed,
        timeout: Duration::from_secs(10),
        enable_retries: true,
        max_retries: 2,
        retry_delay: Duration::from_millis(200),
        enable_fallback: true,
        degraded_threshold_ms: 500,
    };

    let custom_client = Neo4jClient::with_config(&uri, &user, &password, &database, custom_config)
        .await?;

    let custom_result = custom_client.health_check_with_retry().await;
    info!("Custom config result:");
    info!("  Status: {:?}", custom_result.status);
    info!("  Response time: {}ms", custom_result.response_time_ms);
    info!("  Operational: {}", custom_result.status.is_operational());
    info!(
        "  HTTP status code: {}",
        custom_result.status.to_http_status_code()
    );

    info!("\n--- Degraded State Demo ---");
    // Set a very low threshold to demonstrate degraded detection
    let degraded_config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Detailed,
        timeout: Duration::from_secs(5),
        enable_retries: false,
        max_retries: 0,
        retry_delay: Duration::from_millis(0),
        enable_fallback: false,
        degraded_threshold_ms: 5, // Very low threshold
    };

    let degraded_client =
        Neo4jClient::with_config(&uri, &user, &password, &database, degraded_config).await?;

    let degraded_result = degraded_client.health_check_with_retry().await;
    info!("Degraded threshold test:");
    info!("  Status: {:?}", degraded_result.status);
    info!("  Response time: {}ms", degraded_result.response_time_ms);
    info!("  Threshold: 5ms");

    if degraded_result.response_time_ms > 5 {
        info!("  → Database is responding but slower than threshold (degraded)");
    } else {
        info!("  → Database is responding within threshold (healthy)");
    }

    info!("\n--- JSON Serialization Demo ---");
    let json = serde_json::to_string_pretty(&detailed)?;
    info!("HealthCheckResult as JSON:\n{}", json);

    info!("\n=== Demo Complete ===");

    Ok(())
}
