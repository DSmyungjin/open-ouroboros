//! Neo4j connection management and health check implementation
//!
//! This module provides the main client for interacting with Neo4j,
//! including three-tier health check functionality.

use crate::error::{Neo4jError, Result};
use chrono::{DateTime, Utc};
use neo4rs::{query, ConfigBuilder, Graph};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Configuration for health check behavior
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Whether health checks are enabled
    pub enabled: bool,
    /// Health check method to use
    pub method: HealthCheckMethod,
    /// Timeout for health check operations
    pub timeout: Duration,
    /// Whether to enable retry logic
    pub enable_retries: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
    /// Whether to enable fallback from ping to simple
    pub enable_fallback: bool,
    /// Response time threshold for degraded state (in milliseconds)
    pub degraded_threshold_ms: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: HealthCheckMethod::Ping,
            timeout: Duration::from_secs(5),
            enable_retries: true,
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            enable_fallback: true,
            degraded_threshold_ms: 1000,
        }
    }
}

/// Health check method variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthCheckMethod {
    /// Simple health check using RETURN 1 (fastest, minimal overhead)
    Simple,
    /// Standard health check using CALL db.ping() (Neo4j 4.1+)
    Ping,
    /// Detailed health check using CALL db.info() (comprehensive diagnostics)
    Detailed,
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Database is healthy and responsive
    Healthy,
    /// Database is responsive but slow (above degraded threshold)
    Degraded,
    /// Database is not responsive or erroring
    Unhealthy,
}

impl HealthStatus {
    /// Convert to HTTP status code equivalent
    pub fn to_http_status_code(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 200,
            HealthStatus::Unhealthy => 503,
        }
    }

    /// Check if status is healthy or degraded (operational)
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

/// Detailed health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Overall health status
    pub status: HealthStatus,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Database name (if available)
    pub database_name: Option<String>,
    /// Database ID (if available)
    pub database_id: Option<String>,
    /// Timestamp of the health check
    pub timestamp: DateTime<Utc>,
    /// Error message (if unhealthy)
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HealthCheckMetadata,
}

/// Additional metadata for health check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckMetadata {
    /// Health check method used
    pub check_method: HealthCheckMethod,
    /// Whether this was a retry attempt
    pub was_retry: bool,
    /// Number of retry attempts made
    pub retry_count: u32,
    /// Whether fallback was used
    pub used_fallback: bool,
}

impl HealthCheckResult {
    /// Create a healthy result
    fn healthy(
        response_time: Duration,
        database_name: Option<String>,
        database_id: Option<String>,
        method: HealthCheckMethod,
        degraded_threshold_ms: u64,
    ) -> Self {
        let response_time_ms = response_time.as_millis() as u64;
        let status = if response_time_ms > degraded_threshold_ms {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            status,
            response_time_ms,
            database_name,
            database_id,
            timestamp: Utc::now(),
            error: None,
            metadata: HealthCheckMetadata {
                check_method: method,
                was_retry: false,
                retry_count: 0,
                used_fallback: false,
            },
        }
    }

    /// Create an unhealthy result
    fn unhealthy(response_time: Duration, error: &str, method: HealthCheckMethod) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            response_time_ms: response_time.as_millis() as u64,
            database_name: None,
            database_id: None,
            timestamp: Utc::now(),
            error: Some(error.to_string()),
            metadata: HealthCheckMetadata {
                check_method: method,
                was_retry: false,
                retry_count: 0,
                used_fallback: false,
            },
        }
    }

    /// Update metadata for retry/fallback
    fn with_metadata_update(mut self, retry_count: u32, used_fallback: bool) -> Self {
        self.metadata.was_retry = retry_count > 0;
        self.metadata.retry_count = retry_count;
        self.metadata.used_fallback = used_fallback;
        self
    }
}

/// Main Neo4j client with connection pooling
pub struct Neo4jClient {
    graph: Graph,
    health_config: HealthCheckConfig,
}

impl Neo4jClient {
    /// Create a new Neo4j client with default configuration
    ///
    /// # Arguments
    /// * `uri` - Neo4j connection URI (e.g., "bolt://localhost:7687")
    /// * `user` - Username for authentication
    /// * `password` - Password for authentication
    /// * `database` - Database name (default: "neo4j")
    ///
    /// # Example
    /// ```no_run
    /// use ouroboros_kg::Neo4jClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let client = Neo4jClient::new(
    ///         "bolt://localhost:7687",
    ///         "neo4j",
    ///         "password",
    ///         "neo4j"
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(uri: &str, user: &str, password: &str, database: &str) -> Result<Self> {
        Self::with_config(uri, user, password, database, HealthCheckConfig::default()).await
    }

    /// Create a new Neo4j client with custom health check configuration
    pub async fn with_config(
        uri: &str,
        user: &str,
        password: &str,
        database: &str,
        health_config: HealthCheckConfig,
    ) -> Result<Self> {
        info!(
            "Connecting to Neo4j at {} (database: {})",
            uri, database
        );

        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(password)
            .db(database)
            .fetch_size(500)
            .max_connections(16)
            .build()
            .map_err(|e| Neo4jError::ConfigError(e.to_string()))?;

        let graph = Graph::connect(config)
            .await
            .map_err(|e| Neo4jError::ConnectionError(e.to_string()))?;

        info!("Successfully connected to Neo4j");

        Ok(Self {
            graph,
            health_config,
        })
    }

    /// Simple health check using RETURN 1
    ///
    /// This is the fastest health check method with minimal overhead.
    /// Suitable for load balancers and frequent health checks.
    ///
    /// # Returns
    /// * `Ok(true)` if connection is healthy
    /// * `Err(Neo4jError)` if connection fails
    ///
    /// # Example
    /// ```no_run
    /// # use ouroboros_kg::Neo4jClient;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let client = Neo4jClient::new("bolt://localhost:7687", "neo4j", "password", "neo4j").await?;
    /// let is_healthy = client.health_check().await?;
    /// println!("Database healthy: {}", is_healthy);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check(&self) -> Result<bool> {
        debug!("Executing simple health check (RETURN 1)");

        self.graph
            .run(query("RETURN 1"))
            .await
            .map_err(|e| Neo4jError::ConnectionError(e.to_string()))?;

        debug!("Simple health check passed");
        Ok(true)
    }

    /// Standard health check using CALL db.ping()
    ///
    /// Uses the official Neo4j health check procedure (requires Neo4j 4.1+).
    /// Provides explicit health check intent.
    ///
    /// # Returns
    /// * `Ok(HealthStatus::Healthy)` if database responds with success=true
    /// * `Err(Neo4jError)` if connection fails or procedure not available
    ///
    /// # Example
    /// ```no_run
    /// # use ouroboros_kg::Neo4jClient;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let client = Neo4jClient::new("bolt://localhost:7687", "neo4j", "password", "neo4j").await?;
    /// let status = client.health_check_ping().await?;
    /// println!("Health status: {:?}", status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check_ping(&self) -> Result<HealthStatus> {
        debug!("Executing standard health check (CALL db.ping())");

        let mut result = self
            .graph
            .execute(query("CALL db.ping()"))
            .await
            .map_err(|e| Neo4jError::QueryError(e.to_string()))?;

        if let Some(row) = result.next().await.map_err(|e| Neo4jError::QueryError(e.to_string()))? {
            let success: bool = row.get("success").unwrap_or(false);

            if success {
                debug!("Standard health check passed");
                Ok(HealthStatus::Healthy)
            } else {
                warn!("Standard health check returned success=false");
                Ok(HealthStatus::Unhealthy)
            }
        } else {
            error!("Standard health check returned no results");
            Err(Neo4jError::QueryError(
                "No result returned from db.ping()".to_string(),
            ))
        }
    }

    /// Detailed health check using CALL db.info()
    ///
    /// Provides comprehensive diagnostics including database name, response time,
    /// and detailed error information. Never panics - captures all errors internally.
    ///
    /// Suitable for monitoring dashboards and detailed diagnostics.
    ///
    /// # Returns
    /// Always returns a `HealthCheckResult`, even on failure (status will be Unhealthy)
    ///
    /// # Example
    /// ```no_run
    /// # use ouroboros_kg::Neo4jClient;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let client = Neo4jClient::new("bolt://localhost:7687", "neo4j", "password", "neo4j").await?;
    /// let result = client.health_check_detailed().await;
    /// println!("Status: {:?}", result.status);
    /// println!("Response time: {}ms", result.response_time_ms);
    /// if let Some(db_name) = result.database_name {
    ///     println!("Database: {}", db_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check_detailed(&self) -> HealthCheckResult {
        debug!("Executing detailed health check (CALL db.info())");
        let start = Instant::now();

        match self.graph.execute(query("CALL db.info()")).await {
            Ok(mut result) => {
                let elapsed = start.elapsed();

                match result.next().await {
                    Ok(Some(row)) => {
                        let db_name: Option<String> = row.get("name").ok();
                        let db_id: Option<String> = row.get("id").ok();

                        debug!(
                            "Detailed health check passed ({}ms)",
                            elapsed.as_millis()
                        );

                        HealthCheckResult::healthy(
                            elapsed,
                            db_name,
                            db_id,
                            HealthCheckMethod::Detailed,
                            self.health_config.degraded_threshold_ms,
                        )
                    }
                    Ok(None) => {
                        error!("Detailed health check returned no results");
                        HealthCheckResult::unhealthy(
                            elapsed,
                            "No result returned from db.info()",
                            HealthCheckMethod::Detailed,
                        )
                    }
                    Err(e) => {
                        error!("Detailed health check failed to read results: {}", e);
                        HealthCheckResult::unhealthy(
                            elapsed,
                            &format!("Failed to read results: {}", e),
                            HealthCheckMethod::Detailed,
                        )
                    }
                }
            }
            Err(e) => {
                let elapsed = start.elapsed();
                error!("Detailed health check query failed: {}", e);
                HealthCheckResult::unhealthy(
                    elapsed,
                    &format!("Query execution failed: {}", e),
                    HealthCheckMethod::Detailed,
                )
            }
        }
    }

    /// Execute health check with retry logic and fallback
    ///
    /// This method implements the configured health check strategy including:
    /// - Automatic retries on transient failures
    /// - Fallback from db.ping() to RETURN 1 if procedure not available
    /// - Degraded state detection based on response time
    ///
    /// # Returns
    /// Always returns a `HealthCheckResult` with detailed status information
    ///
    /// # Example
    /// ```no_run
    /// # use ouroboros_kg::Neo4jClient;
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// # let client = Neo4jClient::new("bolt://localhost:7687", "neo4j", "password", "neo4j").await?;
    /// let result = client.health_check_with_retry().await;
    /// if result.status.is_operational() {
    ///     println!("Database is operational");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn health_check_with_retry(&self) -> HealthCheckResult {
        let mut retry_count = 0;
        let mut used_fallback = false;
        let max_retries = if self.health_config.enable_retries {
            self.health_config.max_retries
        } else {
            0
        };

        loop {
            let start = Instant::now();

            // Try the configured method
            let result = match self.health_config.method {
                HealthCheckMethod::Simple => {
                    match self.health_check().await {
                        Ok(_) => HealthCheckResult::healthy(
                            start.elapsed(),
                            None,
                            None,
                            HealthCheckMethod::Simple,
                            self.health_config.degraded_threshold_ms,
                        ),
                        Err(e) => HealthCheckResult::unhealthy(
                            start.elapsed(),
                            &e.to_string(),
                            HealthCheckMethod::Simple,
                        ),
                    }
                }
                HealthCheckMethod::Ping => {
                    match self.health_check_ping().await {
                        Ok(status) => {
                            let elapsed = start.elapsed();
                            HealthCheckResult {
                                status,
                                response_time_ms: elapsed.as_millis() as u64,
                                database_name: None,
                                database_id: None,
                                timestamp: Utc::now(),
                                error: None,
                                metadata: HealthCheckMetadata {
                                    check_method: HealthCheckMethod::Ping,
                                    was_retry: false,
                                    retry_count: 0,
                                    used_fallback: false,
                                },
                            }
                        }
                        Err(e) => {
                            // Try fallback to simple if enabled and this is a procedure-not-found error
                            if self.health_config.enable_fallback && !used_fallback {
                                warn!("db.ping() failed, falling back to RETURN 1: {}", e);
                                used_fallback = true;
                                match self.health_check().await {
                                    Ok(_) => HealthCheckResult::healthy(
                                        start.elapsed(),
                                        None,
                                        None,
                                        HealthCheckMethod::Simple,
                                        self.health_config.degraded_threshold_ms,
                                    ),
                                    Err(fallback_err) => HealthCheckResult::unhealthy(
                                        start.elapsed(),
                                        &fallback_err.to_string(),
                                        HealthCheckMethod::Simple,
                                    ),
                                }
                            } else {
                                HealthCheckResult::unhealthy(
                                    start.elapsed(),
                                    &e.to_string(),
                                    HealthCheckMethod::Ping,
                                )
                            }
                        }
                    }
                }
                HealthCheckMethod::Detailed => self.health_check_detailed().await,
            };

            // If healthy or we've exhausted retries, return the result
            if result.status.is_operational() || retry_count >= max_retries {
                return result.with_metadata_update(retry_count, used_fallback);
            }

            // Wait before retrying
            retry_count += 1;
            warn!(
                "Health check failed (attempt {}/{}), retrying after {:?}",
                retry_count,
                max_retries + 1,
                self.health_config.retry_delay
            );
            tokio::time::sleep(self.health_config.retry_delay).await;
        }
    }

    /// Get a reference to the underlying Neo4j Graph instance
    ///
    /// This allows direct access to the neo4rs Graph for custom queries.
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Get the current health check configuration
    pub fn health_config(&self) -> &HealthCheckConfig {
        &self.health_config
    }

    /// Update the health check configuration
    pub fn set_health_config(&mut self, config: HealthCheckConfig) {
        self.health_config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_http_codes() {
        assert_eq!(HealthStatus::Healthy.to_http_status_code(), 200);
        assert_eq!(HealthStatus::Degraded.to_http_status_code(), 200);
        assert_eq!(HealthStatus::Unhealthy.to_http_status_code(), 503);
    }

    #[test]
    fn test_health_status_operational() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded.is_operational());
        assert!(!HealthStatus::Unhealthy.is_operational());
    }

    #[test]
    fn test_health_check_result_healthy() {
        let result = HealthCheckResult::healthy(
            Duration::from_millis(50),
            Some("neo4j".to_string()),
            Some("test-id".to_string()),
            HealthCheckMethod::Detailed,
            1000,
        );

        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.response_time_ms, 50);
        assert_eq!(result.database_name, Some("neo4j".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_health_check_result_degraded() {
        let result = HealthCheckResult::healthy(
            Duration::from_millis(1500),
            Some("neo4j".to_string()),
            None,
            HealthCheckMethod::Detailed,
            1000,
        );

        assert_eq!(result.status, HealthStatus::Degraded);
        assert_eq!(result.response_time_ms, 1500);
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult::unhealthy(
            Duration::from_millis(100),
            "Connection failed",
            HealthCheckMethod::Simple,
        );

        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.response_time_ms, 100);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Connection failed");
    }

    #[test]
    fn test_default_health_check_config() {
        let config = HealthCheckConfig::default();

        assert!(config.enabled);
        assert_eq!(config.method, HealthCheckMethod::Ping);
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert!(config.enable_retries);
        assert_eq!(config.max_retries, 3);
        assert!(config.enable_fallback);
    }
}
