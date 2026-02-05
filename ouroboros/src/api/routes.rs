//! API routes for Ouroboros server

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::search::{DocumentType, SearchEngine, SearchOptions};

/// Application state
pub struct AppState {
    pub search_engine: Arc<RwLock<Option<SearchEngine>>>,
    pub data_dir: PathBuf,
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Login request
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in_hours: i64,
}

/// Search query parameters
#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
    pub session: Option<String>,
    pub limit: Option<usize>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Search result response
#[derive(Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResultItem>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct SearchResultItem {
    pub doc_type: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub session_id: Option<String>,
    pub task_id: Option<String>,
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Login endpoint (simple demo - in production, validate against a database)
pub async fn login(
    State(auth_state): State<super::middleware::AuthState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Simple validation (in production, check against database)
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // For demo purposes, accept any non-empty credentials
    // In production, validate against a user database
    let expires_in = 24;
    let token = auth_state
        .jwt_auth
        .generate_token(&payload.username, Some(expires_in))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        expires_in_hours: expires_in,
    }))
}

/// Protected search endpoint
pub async fn search(
    State(app_state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    // Get search engine
    let search_engine_guard = app_state.search_engine.read().await;
    let search_engine = search_engine_guard
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Build search options
    let mut options = SearchOptions::new().with_limit(params.limit.unwrap_or(10));

    if let Some(ref dtype) = params.doc_type {
        let dt = match dtype.as_str() {
            "task" => DocumentType::Task,
            "task_result" | "result" => DocumentType::TaskResult,
            "context" => DocumentType::Context,
            "knowledge" => DocumentType::Knowledge,
            "plan" => DocumentType::Plan,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        options = options.with_doc_type(dt);
    }

    // Perform search
    let results = search_engine
        .search(&params.query, &options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert to response format
    let items: Vec<SearchResultItem> = results
        .into_iter()
        .map(|r| SearchResultItem {
            doc_type: format!("{:?}", r.doc_type),
            title: r.title,
            content: r.content,
            score: r.score,
            session_id: None,  // Not available in SearchResult
            task_id: None,     // Not available in SearchResult
        })
        .collect();

    let total = items.len();

    Ok(Json(SearchResponse {
        results: items,
        total,
    }))
}
