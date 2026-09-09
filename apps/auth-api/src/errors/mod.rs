use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
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

    #[error("Token has expired")]
    TokenExpired,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Validation error: {0}")]
    Validation(String),

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

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", self.to_string()),
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                self.to_string(),
            ),
            AppError::EmailAlreadyExists => (
                StatusCode::CONFLICT,
                "EMAIL_ALREADY_EXISTS",
                self.to_string(),
            ),
            AppError::SessionNotFound => (
                StatusCode::UNAUTHORIZED,
                "SESSION_NOT_FOUND",
                self.to_string(),
            ),
            AppError::InvalidToken(msg) => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", msg.clone()),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED", self.to_string()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::TooManyRequests(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_REQUESTS",
                msg.clone(),
            ),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
            AppError::TokenCreation(err) => {
                tracing::error!(error = %err, "Token generation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "TOKEN_GENERATION_FAILED",
                    "Token generation failed".to_string(),
                )
            }
            AppError::PasswordHash(err) => {
                tracing::error!(error = %err, "Password operation failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "AUTHENTICATION_ERROR",
                    "Authentication error".to_string(),
                )
            }
            AppError::OidcError(msg) => {
                tracing::warn!(error = %msg, "OIDC provider error");
                (StatusCode::BAD_GATEWAY, "OIDC_ERROR", msg.clone())
            }
            AppError::Database(err) => {
                tracing::error!(error = %err, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "A database error occurred".to_string(),
                )
            }
            AppError::Redis(err) => {
                tracing::error!(error = %err, "Redis error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CACHE_ERROR",
                    "A cache error occurred".to_string(),
                )
            }
            AppError::Internal(err) => {
                tracing::error!(error = %err, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            },
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}
