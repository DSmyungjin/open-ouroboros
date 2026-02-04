//! Error types for Neo4j operations
//!
//! This module defines custom error types for the ouroboros-kg library,
//! providing detailed error information for health checks and database operations.

use thiserror::Error;

/// Main error type for Neo4j operations
#[derive(Error, Debug)]
pub enum Neo4jError {
    /// Connection error - network or connection pool issues
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Authentication error - invalid credentials or permissions
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Query execution error
    #[error("Query error: {0}")]
    QueryError(String),

    /// Operation timeout
    #[error("Operation timed out after {timeout_seconds}s: {context}")]
    TimeoutError {
        timeout_seconds: u64,
        context: String,
    },

    /// Connection pool exhausted - no available connections
    #[error("Connection pool exhausted: all {max_connections} connections are in use")]
    PoolExhaustedError { max_connections: usize },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/Deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Neo4rs driver error (wrapper)
    #[error("Neo4rs driver error: {0}")]
    DriverError(#[from] neo4rs::Error),

    /// Generic error with context
    #[error("Error: {0}")]
    Other(String),
}

/// Result type alias for Neo4j operations
pub type Result<T> = std::result::Result<T, Neo4jError>;

impl From<String> for Neo4jError {
    fn from(s: String) -> Self {
        Neo4jError::Other(s)
    }
}

impl From<&str> for Neo4jError {
    fn from(s: &str) -> Self {
        Neo4jError::Other(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Neo4jError::ConnectionError("Failed to connect".to_string());
        assert_eq!(error.to_string(), "Connection error: Failed to connect");

        let timeout_error = Neo4jError::TimeoutError {
            timeout_seconds: 5,
            context: "health check".to_string(),
        };
        assert!(timeout_error.to_string().contains("timed out after 5s"));

        let pool_error = Neo4jError::PoolExhaustedError {
            max_connections: 16,
        };
        assert!(pool_error.to_string().contains("16 connections"));
    }

    #[test]
    fn test_error_conversion() {
        let error: Neo4jError = "test error".into();
        assert!(matches!(error, Neo4jError::Other(_)));

        let error: Neo4jError = "test error".to_string().into();
        assert!(matches!(error, Neo4jError::Other(_)));
    }
}
