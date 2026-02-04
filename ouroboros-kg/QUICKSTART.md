# Quick Start Guide

Get up and running with `ouroboros-kg` health checks in 5 minutes!

## Prerequisites

- Rust 1.70+ installed
- Neo4j 4.1+ running (or use Docker)

## 1. Start Neo4j (Docker)

```bash
docker run -d --name neo4j \
  -p 7474:7474 -p 7687:7687 \
  -e NEO4J_AUTH=neo4j/password \
  neo4j:latest

# Wait for Neo4j to start (check logs)
docker logs -f neo4j
# Press Ctrl+C when you see "Started."
```

## 2. Add Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
ouroboros-kg = { path = "../ouroboros-kg" }  # or version from crates.io
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

## 3. Basic Health Check

Create `src/main.rs`:

```rust
use ouroboros_kg::Neo4jClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to Neo4j
    let client = Neo4jClient::new(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j"
    ).await?;

    // Simple health check
    if client.health_check().await? {
        println!("✓ Database is healthy!");
    }

    Ok(())
}
```

Run it:

```bash
cargo run
```

## 4. Detailed Health Check

For more information:

```rust
use ouroboros_kg::Neo4jClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Neo4jClient::new(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j"
    ).await?;

    // Detailed health check with diagnostics
    let result = client.health_check_detailed().await;

    println!("Status: {:?}", result.status);
    println!("Response time: {}ms", result.response_time_ms);

    if let Some(db_name) = result.database_name {
        println!("Database: {}", db_name);
    }

    Ok(())
}
```

## 5. Production-Ready Health Check

With retry logic and fallback:

```rust
use ouroboros_kg::{Neo4jClient, HealthCheckConfig, HealthCheckMethod};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Custom configuration
    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Ping,
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 3,
        retry_delay: Duration::from_millis(500),
        enable_fallback: true,
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j",
        config
    ).await?;

    // Health check with automatic retry and fallback
    let result = client.health_check_with_retry().await;

    if result.status.is_operational() {
        println!("✓ Database is operational");
        println!("  Response time: {}ms", result.response_time_ms);

        if result.metadata.used_fallback {
            println!("  ⚠ Used fallback method (db.ping() not available)");
        }

        if result.metadata.retry_count > 0 {
            println!("  ⚠ Recovered after {} retries", result.metadata.retry_count);
        }
    } else {
        println!("✗ Database is unhealthy");
        if let Some(error) = result.error {
            println!("  Error: {}", error);
        }
    }

    Ok(())
}
```

## 6. HTTP Server Integration (Axum)

Add to `Cargo.toml`:

```toml
[dependencies]
axum = "0.7"
```

Create `src/main.rs`:

```rust
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use ouroboros_kg::Neo4jClient;
use std::sync::Arc;

struct AppState {
    neo4j: Neo4jClient,
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.neo4j.health_check().await {
        Ok(_) => (StatusCode::OK, "healthy"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "unhealthy"),
    }
}

async fn health_check_detailed(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = state.neo4j.health_check_detailed().await;

    let status_code = match result.status {
        ouroboros_kg::HealthStatus::Healthy => StatusCode::OK,
        ouroboros_kg::HealthStatus::Degraded => StatusCode::OK,
        ouroboros_kg::HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    (status_code, Json(result))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to Neo4j
    let client = Neo4jClient::new(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j"
    ).await?;

    let state = Arc::new(AppState { neo4j: client });

    // Create router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/health/detailed", get(health_check_detailed))
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://0.0.0.0:3000");
    println!("Try: curl http://localhost:3000/health");
    println!("Try: curl http://localhost:3000/health/detailed");

    axum::serve(listener, app).await?;

    Ok(())
}
```

Run the server:

```bash
cargo run
```

Test it:

```bash
# Simple health check
curl http://localhost:3000/health

# Detailed health check with JSON
curl http://localhost:3000/health/detailed | jq .
```

## 7. Run the Demo

The library includes a comprehensive demo:

```bash
cargo run --example health_check_demo
```

## 8. Run Tests

### Unit tests (no Neo4j required):

```bash
cargo test --lib
```

### Integration tests (requires Neo4j):

```bash
# Start Neo4j first
docker start neo4j

# Run integration tests
cargo test --ignored
```

## Common Use Cases

### Load Balancer Health Check

```rust
// Fast, minimal overhead
if client.health_check().await? {
    // Route traffic here
}
```

### Kubernetes Liveness Probe

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3000
  initialDelaySeconds: 30
  periodSeconds: 10
```

### Kubernetes Readiness Probe

```yaml
readinessProbe:
  httpGet:
    path: /health/detailed
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 5
```

### Monitoring Dashboard

```rust
// Get detailed metrics
let result = client.health_check_detailed().await;

// Export to monitoring system
let json = serde_json::to_string(&result)?;
// Send to monitoring system (Prometheus, Datadog, etc.)
```

## Environment Variables

Set these for easy configuration:

```bash
export NEO4J_URI=bolt://localhost:7687
export NEO4J_USER=neo4j
export NEO4J_PASSWORD=password
export NEO4J_DATABASE=neo4j

# Run your application
cargo run
```

## Troubleshooting

### Connection Failed

```
Error: Connection error: Failed to connect
```

**Solution:** Ensure Neo4j is running and accessible:

```bash
docker ps | grep neo4j
curl http://localhost:7474  # Should show Neo4j browser
```

### db.ping() Not Available

```
Error: Query error: Procedure db.ping() not found
```

**Solution:** You're using Neo4j 4.0.x. Either:
1. Upgrade to Neo4j 4.1+ (recommended)
2. Use `HealthCheckMethod::Simple` instead
3. Enable fallback (it will automatically use `RETURN 1`)

### Authentication Failed

```
Error: Authentication error: Invalid credentials
```

**Solution:** Check your username and password:

```bash
# Default credentials for fresh Neo4j install
# Username: neo4j
# Password: neo4j (must be changed on first login)

# Or check your environment variables
echo $NEO4J_PASSWORD
```

## Next Steps

- Read the [README.md](README.md) for comprehensive documentation
- Check the [examples/](examples/) directory for more examples
- Review [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for technical details
- See the [tests/](tests/) directory for usage examples

## Help & Support

- GitHub Issues: Report bugs or request features
- Documentation: See README.md and code documentation
- Examples: Check examples/ directory

## License

MIT License - See LICENSE file for details
