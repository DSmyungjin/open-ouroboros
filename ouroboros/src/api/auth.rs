//! JWT authentication module for Ouroboros API
//!
//! This module provides JWT token generation and validation functionality.

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user identifier)
    pub sub: String,
    /// Issued at (timestamp)
    pub iat: i64,
    /// Expiration time (timestamp)
    pub exp: i64,
}

/// JWT authentication handler
pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtAuth {
    /// Create a new JWT authentication handler with a secret key
    pub fn new(secret: &str) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            validation,
        }
    }

    /// Generate a JWT token for a user
    ///
    /// # Arguments
    /// * `user_id` - The user identifier
    /// * `expires_in_hours` - Token expiration time in hours (default: 24)
    pub fn generate_token(&self, user_id: &str, expires_in_hours: Option<i64>) -> Result<String> {
        let now = Utc::now();
        let expires_in = expires_in_hours.unwrap_or(24);
        let exp = now + Duration::hours(expires_in);

        let claims = Claims {
            sub: user_id.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Failed to generate token: {}", e))?;

        Ok(token)
    }

    /// Validate a JWT token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| anyhow!("Invalid token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Extract token from Authorization header (Bearer token)
    pub fn extract_bearer_token(auth_header: &str) -> Result<String> {
        if !auth_header.starts_with("Bearer ") {
            return Err(anyhow!("Invalid authorization header format"));
        }

        let token = auth_header.trim_start_matches("Bearer ").trim();
        if token.is_empty() {
            return Err(anyhow!("Empty token"));
        }

        Ok(token.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let auth = JwtAuth::new("test_secret_key_12345");
        let user_id = "user123";

        // Generate token
        let token = auth.generate_token(user_id, Some(1)).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = auth.validate_token(&token).unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_invalid_token() {
        let auth = JwtAuth::new("test_secret_key_12345");
        let result = auth.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bearer_token() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtAuth::extract_bearer_token(header).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_extract_bearer_token_invalid() {
        let header = "InvalidHeader token";
        let result = JwtAuth::extract_bearer_token(header);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bearer_token_empty() {
        let header = "Bearer ";
        let result = JwtAuth::extract_bearer_token(header);
        assert!(result.is_err());
    }
}
