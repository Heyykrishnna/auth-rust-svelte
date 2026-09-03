use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

/// Domain-level errors that map to HTTP responses.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("User not found")]
    UserNotFound,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Email already registered")]
    EmailAlreadyExists,

    #[error("Session not found or expired")]
    SessionNotFound,

    #[error("Invalid or expired token")]
    InvalidToken(String),

    #[error("Token creation failed: {0}")]
    TokenCreation(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("OIDC error: {0}")]
    OidcError(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for DomainError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            DomainError::UserNotFound         => (StatusCode::NOT_FOUND, self.to_string()),
            DomainError::InvalidCredentials   => (StatusCode::UNAUTHORIZED, self.to_string()),
            DomainError::EmailAlreadyExists   => (StatusCode::CONFLICT, self.to_string()),
            DomainError::SessionNotFound      => (StatusCode::UNAUTHORIZED, self.to_string()),
            DomainError::InvalidToken(_)      => (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()),
            DomainError::TokenCreation(_)     => (StatusCode::INTERNAL_SERVER_ERROR, "Token generation failed".to_string()),
            DomainError::PasswordHash(_)      => (StatusCode::INTERNAL_SERVER_ERROR, "Authentication error".to_string()),
            DomainError::OidcError(msg)       => (StatusCode::BAD_GATEWAY, msg.clone()),
            DomainError::Database(_)          => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
            DomainError::Redis(_)             => (StatusCode::INTERNAL_SERVER_ERROR, "Cache error".to_string()),
            DomainError::Internal(msg)        => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}
