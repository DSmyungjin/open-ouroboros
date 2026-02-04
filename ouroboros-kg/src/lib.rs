//! # Ouroboros Knowledge Graph (ouroboros-kg)
//!
//! A Neo4j client library for Rust with comprehensive health check support.
//!
//! ## Features
//!
//! - Three-tier health check system (simple, standard, detailed)
//! - Async-first design using tokio
//! - Connection pooling
//! - Automatic retry logic with fallback strategies
//! - Detailed error handling
//! - Degraded state detection
//!
//! ## Health Check Methods
//!
//! ### Simple Health Check
//! Uses `RETURN 1` - fastest method with minimal overhead, suitable for load balancers.
//!
//! ```no_run
//! use ouroboros_kg::Neo4jClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Neo4jClient::new(
//!         "bolt://localhost:7687",
//!         "neo4j",
//!         "password",
//!         "neo4j"
//!     ).await?;
//!
//!     let is_healthy = client.health_check().await?;
//!     println!("Database healthy: {}", is_healthy);
//!     Ok(())
//! }
//! ```
//!
//! ### Standard Health Check
//! Uses `CALL db.ping()` - official Neo4j procedure (requires Neo4j 4.1+).
//!
//! ```no_run
//! use ouroboros_kg::Neo4jClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Neo4jClient::new(
//!         "bolt://localhost:7687",
//!         "neo4j",
//!         "password",
//!         "neo4j"
//!     ).await?;
//!
//!     let status = client.health_check_ping().await?;
//!     println!("Health status: {:?}", status);
//!     Ok(())
//! }
//! ```
//!
//! ### Detailed Health Check
//! Uses `CALL db.info()` - provides comprehensive diagnostics with timing.
//!
//! ```no_run
//! use ouroboros_kg::Neo4jClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Neo4jClient::new(
//!         "bolt://localhost:7687",
//!         "neo4j",
//!         "password",
//!         "neo4j"
//!     ).await?;
//!
//!     let result = client.health_check_detailed().await;
//!     println!("Status: {:?}", result.status);
//!     println!("Response time: {}ms", result.response_time_ms);
//!     Ok(())
//! }
//! ```
//!
//! ### Health Check with Retry
//! Implements automatic retries and fallback strategies.
//!
//! ```no_run
//! use ouroboros_kg::Neo4jClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Neo4jClient::new(
//!         "bolt://localhost:7687",
//!         "neo4j",
//!         "password",
//!         "neo4j"
//!     ).await?;
//!
//!     let result = client.health_check_with_retry().await;
//!     if result.status.is_operational() {
//!         println!("Database is operational");
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Knowledge Graph Schema
//!
//! The library provides a schema for storing task execution results in Neo4j:
//!
//! ```no_run
//! use ouroboros_kg::{Neo4jClient, schema::{Task, TaskStatus, create_task, create_dependency}};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = Neo4jClient::new(
//!         "bolt://localhost:7687",
//!         "neo4j",
//!         "password",
//!         "neo4j"
//!     ).await?;
//!
//!     // Create tasks
//!     let task1 = Task::new(
//!         "task-001".to_string(),
//!         "Setup database".to_string(),
//!         "Initialize Neo4j database".to_string()
//!     );
//!     create_task(client.graph(), &task1).await?;
//!
//!     let task2 = Task::new(
//!         "task-002".to_string(),
//!         "Load data".to_string(),
//!         "Load initial data".to_string()
//!     );
//!     create_task(client.graph(), &task2).await?;
//!
//!     // Create dependency: task-002 depends on task-001
//!     create_dependency(client.graph(), "task-002", "task-001").await?;
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod connection;
pub mod error;
pub mod schema;

// Re-export main types for convenience
pub use cache::{
    CachedContextChunk, CachedQueryResult, CacheConfig, CacheConfigBuilder, CacheEntry, CacheKey,
    CacheKeyBuilder, CacheMetadata, CacheStats, CacheValue, ContextCache, ContextType,
    InvalidationReason, InvalidationStrategy, KnowledgeGraphCache,
};
pub use connection::{
    HealthCheckConfig, HealthCheckMetadata, HealthCheckMethod, HealthCheckResult, HealthStatus,
    Neo4jClient,
};
pub use error::{Neo4jError, Result};
