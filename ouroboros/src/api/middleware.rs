//! Authentication middleware for Axum

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use super::auth::JwtAuth;

/// Authentication state shared across requests
#[derive(Clone)]
pub struct AuthState {
    pub jwt_auth: Arc<JwtAuth>,
}

impl AuthState {
    pub fn new(secret: &str) -> Self {
        Self {
            jwt_auth: Arc::new(JwtAuth::new(secret)),
        }
    }
}

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract bearer token
    let token = JwtAuth::extract_bearer_token(auth_header)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = state.jwt_auth
        .validate_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Store user_id in request extensions for later use
    request.extensions_mut().insert(claims.sub.clone());

    Ok(next.run(request).await)
}

/// Extract user ID from request extensions (set by auth middleware)
pub fn get_user_id(request: &Request) -> Option<String> {
    request.extensions().get::<String>().cloned()
}
