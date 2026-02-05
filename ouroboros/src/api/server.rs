//! API server for Ouroboros

use anyhow::Result;
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::search::SearchEngine;

use super::middleware::{auth_middleware, AuthState};
use super::routes::{health_check, login, search, AppState};

/// Configuration for the API server
pub struct ApiServerConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub data_dir: PathBuf,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "default_secret_change_in_production".to_string()),
            data_dir: PathBuf::from("./data"),
        }
    }
}

/// API server
pub struct ApiServer {
    config: ApiServerConfig,
}

impl ApiServer {
    /// Create a new API server with configuration
    pub fn new(config: ApiServerConfig) -> Self {
        Self { config }
    }

    /// Create a new API server with default configuration
    pub fn with_defaults() -> Self {
        Self {
            config: ApiServerConfig::default(),
        }
    }

    /// Start the API server
    pub async fn start(self) -> Result<()> {
        // Initialize search engine
        let search_index_path = self.config.data_dir.join("search_index");
        let search_engine = if search_index_path.exists() {
            Some(SearchEngine::keyword_reader_only(&search_index_path)?)
        } else {
            info!("Search index not found at {:?}, search will be unavailable", search_index_path);
            None
        };

        // Create application state
        let app_state = Arc::new(AppState {
            search_engine: Arc::new(RwLock::new(search_engine)),
            data_dir: self.config.data_dir.clone(),
        });

        // Create authentication state
        let auth_state = AuthState::new(&self.config.jwt_secret);

        // Build router
        let app = Router::new()
            // Public routes
            .route("/health", get(health_check))
            .route("/login", post(login))
            .with_state(auth_state.clone())
            // Protected routes
            .route(
                "/api/search",
                get(search).route_layer(from_fn_with_state(auth_state.clone(), auth_middleware)),
            )
            .with_state(app_state)
            // Add CORS layer
            .layer(CorsLayer::permissive());

        // Start server
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting API server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
