# Ouroboros Knowledge Graph (ouroboros-kg)

A Neo4j client library for Rust with comprehensive health check support.

## Features

- **Three-Tier Health Check System**
  - Simple: `RETURN 1` (fastest, minimal overhead)
  - Standard: `CALL db.ping()` (official Neo4j procedure, requires 4.1+)
  - Detailed: `CALL db.info()` (comprehensive diagnostics with timing)

- **Async-First Design**
  - Built on tokio runtime
  - Non-blocking operations
  - Connection pooling with neo4rs

- **Robust Error Handling**
  - Custom error types with detailed context
  - Automatic retry logic with configurable delays
  - Fallback strategies (e.g., from `db.ping()` to `RETURN 1`)

- **Production-Ready**
  - Degraded state detection based on response time
  - Structured logging with tracing
  - JSON serialization support for monitoring integration
  - Comprehensive test coverage

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ouroboros-kg = "0.1.0"
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

## Quick Start

### Simple Health Check

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

    let is_healthy = client.health_check().await?;
    println!("Database healthy: {}", is_healthy);

    Ok(())
}
```

### Detailed Health Check with Full Diagnostics

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

    let result = client.health_check_detailed().await;

    println!("Status: {:?}", result.status);
    println!("Response time: {}ms", result.response_time_ms);

    if let Some(db_name) = result.database_name {
        println!("Database: {}", db_name);
    }

    if result.status.is_operational() {
        println!("✓ Database is operational");
    }

    Ok(())
}
```

### Health Check with Automatic Retry and Fallback

```rust
use ouroboros_kg::{Neo4jClient, HealthCheckConfig, HealthCheckMethod};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = HealthCheckConfig {
        enabled: true,
        method: HealthCheckMethod::Ping,
        timeout: Duration::from_secs(5),
        enable_retries: true,
        max_retries: 3,
        retry_delay: Duration::from_millis(500),
        enable_fallback: true,  // Falls back to RETURN 1 if db.ping() fails
        degraded_threshold_ms: 1000,
    };

    let client = Neo4jClient::with_config(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j",
        config
    ).await?;

    let result = client.health_check_with_retry().await;

    println!("Status: {:?}", result.status);
    println!("Used fallback: {}", result.metadata.used_fallback);
    println!("Retry count: {}", result.metadata.retry_count);

    Ok(())
}
```

## Health Check Methods

### 1. Simple Health Check (`health_check()`)

**Query:** `RETURN 1`

**Performance:** ~1-5ms
**Use Case:** Load balancers, Kubernetes liveness probes
**Returns:** `Result<bool, Neo4jError>`

**Characteristics:**
- Fastest method with minimal overhead
- Works on all Neo4j versions
- Only validates basic connectivity

### 2. Standard Health Check (`health_check_ping()`)

**Query:** `CALL db.ping()`

**Performance:** ~5-10ms
**Use Case:** Kubernetes readiness probes
**Returns:** `Result<HealthStatus, Neo4jError>`

**Characteristics:**
- Official Neo4j health check procedure
- Requires Neo4j 4.1+
- More explicit health check intent
- Automatic fallback available if procedure not found

### 3. Detailed Health Check (`health_check_detailed()`)

**Query:** `CALL db.info()`

**Performance:** ~10-20ms
**Use Case:** Monitoring dashboards, diagnostics
**Returns:** `HealthCheckResult` (never panics)

**Characteristics:**
- Comprehensive diagnostics
- Includes database name, ID, and timing
- Captures errors internally (never fails)
- Supports degraded state detection

### 4. Health Check with Retry (`health_check_with_retry()`)

**Combines:** Configured method + retry logic + fallback

**Returns:** `HealthCheckResult` (never panics)

**Characteristics:**
- Implements automatic retry with exponential backoff
- Supports fallback from `db.ping()` to `RETURN 1`
- Detects degraded state based on response time threshold
- Tracks retry attempts and fallback usage

## Health Status Types

### `HealthStatus` Enum

```rust
pub enum HealthStatus {
    Healthy,    // Database is healthy and responsive
    Degraded,   // Database is responsive but slow
    Unhealthy,  // Database is not responsive or erroring
}
```

### `HealthCheckResult` Struct

```rust
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub database_name: Option<String>,
    pub database_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
    pub metadata: HealthCheckMetadata,
}
```

**JSON Example:**

```json
{
  "status": "Healthy",
  "response_time_ms": 15,
  "database_name": "neo4j",
  "database_id": "01234567-89AB-CDEF-0123-456789ABCDEF",
  "timestamp": "2026-02-03T12:34:56Z",
  "error": null,
  "metadata": {
    "check_method": "Detailed",
    "was_retry": false,
    "retry_count": 0,
    "used_fallback": false
  }
}
```

## Configuration

### Environment Variables

```bash
# Neo4j connection
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=password
NEO4J_DATABASE=neo4j

# Health check configuration
NEO4J_HEALTH_CHECK_ENABLED=true
NEO4J_HEALTH_CHECK_METHOD=ping  # simple, ping, detailed
NEO4J_HEALTH_CHECK_TIMEOUT=5
NEO4J_HEALTH_CHECK_ENABLE_RETRIES=true
NEO4J_HEALTH_CHECK_MAX_RETRIES=3
NEO4J_HEALTH_CHECK_RETRY_DELAY_MS=500
NEO4J_HEALTH_CHECK_ENABLE_FALLBACK=true
NEO4J_HEALTH_CHECK_DEGRADED_THRESHOLD_MS=1000
```

### `HealthCheckConfig` Struct

```rust
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub method: HealthCheckMethod,
    pub timeout: Duration,
    pub enable_retries: bool,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub enable_fallback: bool,
    pub degraded_threshold_ms: u64,
}
```

## HTTP Integration Examples

### Axum Example

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
    let client = Neo4jClient::new(
        "bolt://localhost:7687",
        "neo4j",
        "password",
        "neo4j"
    ).await?;

    let state = Arc::new(AppState { neo4j: client });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/health/detailed", get(health_check_detailed))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Recommended Endpoints

| Path | Method | Response | Use Case |
|------|--------|----------|----------|
| `/health` | GET | Plain text | Load balancer / K8s liveness |
| `/health/ready` | GET | Plain text | K8s readiness probe |
| `/health/detailed` | GET | JSON | Monitoring/diagnostics |
| `/healthz` | GET | Plain text | Alternative convention |

## Testing

### Unit Tests

```bash
# Run unit tests
cargo test
```

### Integration Tests

Requires a running Neo4j instance:

```bash
# Start Neo4j (Docker)
docker run -d --name neo4j \
  -p 7474:7474 -p 7687:7687 \
  -e NEO4J_AUTH=neo4j/password \
  neo4j:latest

# Run integration tests
cargo test --ignored

# Run all tests
cargo test --all
```

### Examples

```bash
# Run health check demo
cargo run --example health_check_demo

# With custom connection
NEO4J_URI=bolt://localhost:7687 \
NEO4J_PASSWORD=mypassword \
cargo run --example health_check_demo
```

## Best Practices

1. **Choose the Right Method**
   - Load balancers → Simple (`health_check()`)
   - Kubernetes probes → Standard (`health_check_ping()`)
   - Monitoring dashboards → Detailed (`health_check_detailed()`)

2. **Enable Retry Logic**
   - Handle transient network failures
   - Configure appropriate retry delays
   - Set reasonable max retry limits

3. **Use Fallback Strategies**
   - Enable fallback from `db.ping()` to `RETURN 1`
   - Supports older Neo4j versions (4.0.x)
   - Ensures health checks always work

4. **Configure Degraded Thresholds**
   - Set appropriate response time thresholds
   - Alert on degraded state (slow but operational)
   - Distinguish from complete failures

5. **Monitor Response Times**
   - Track health check latency over time
   - Identify performance degradation early
   - Correlate with database load

6. **Implement Structured Logging**
   - Use tracing for observability
   - Include context in error messages
   - Enable debugging in production

## Error Handling

The library provides detailed error types:

```rust
pub enum Neo4jError {
    ConnectionError(String),
    AuthenticationError(String),
    QueryError(String),
    TimeoutError { timeout_seconds: u64, context: String },
    PoolExhaustedError { max_connections: usize },
    ConfigError(String),
    SerializationError(String),
    DriverError(neo4rs::Error),
    Other(String),
}
```

All health check methods handle errors gracefully:
- `health_check()` and `health_check_ping()` return `Result<T, Neo4jError>`
- `health_check_detailed()` and `health_check_with_retry()` never panic, capturing errors internally

## Performance Characteristics

| Method | Typical Response Time | Overhead | Recommended For |
|--------|----------------------|----------|-----------------|
| Simple | 1-5ms | Minimal | Frequent checks |
| Standard | 5-10ms | Low | Readiness probes |
| Detailed | 10-20ms | Moderate | Diagnostics |
| With Retry | Varies | Configurable | Production use |

## Contributing

Contributions are welcome! Please ensure:
- All tests pass (`cargo test --all`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Documentation is updated

## License

This project is licensed under the MIT License.

## Acknowledgments

- Built with [neo4rs](https://github.com/neo4j-labs/neo4rs) - Official Neo4j Labs Rust driver
- Inspired by health check best practices from Spring Boot, ASP.NET Core, and Python communities
