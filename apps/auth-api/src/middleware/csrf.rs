use crate::errors::ApiErrorEnvelope;
use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use rand_core::{OsRng, RngCore};
use subtle::ConstantTimeEq;

static CSRF_COOKIE: &str = "csrf_token";
static CSRF_HEADER: &str = "x-csrf-token";

static MUTATING_METHODS: &[Method] = &[Method::POST, Method::PUT, Method::PATCH, Method::DELETE];

pub async fn csrf_protection_middleware(req: Request, next: Next) -> Response {
    if !MUTATING_METHODS.contains(req.method()) {
        return next.run(req).await;
    }

    let has_cookie = has_session_cookie(req.headers());
    let uses_bearer = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer "))
        .unwrap_or(false);

    if uses_bearer && !has_cookie {
        return next.run(req).await;
    }

    let path = req.uri().path();
    let is_public_auth_route = path.ends_with("/register")
        || path.ends_with("/login")
        || path.ends_with("/forgot-password")
        || path.ends_with("/reset-password")
        || path.ends_with("/verify-code")
        || path.ends_with("/refresh")
        || path.ends_with("/csrf")
        || path.contains("/oidc/");

    if is_public_auth_route {
        return next.run(req).await;
    }

    let cookie_token = extract_csrf_cookie(req.headers());
    let header_token = req
        .headers()
        .get(CSRF_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if constant_time_eq(&cookie, &header) => next.run(req).await,
        _ => csrf_rejection(request_id),
    }
}

fn has_session_cookie(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .map(|cookie_str| {
            cookie_str.split(';').any(|s| {
                if let Some((name, _)) = s.trim().split_once('=') {
                    let trimmed = name.trim();
                    trimmed == "session_token" || trimmed == "access_token"
                } else {
                    false
                }
            })
        })
        .unwrap_or(false)
}

fn extract_csrf_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str.split(';').find_map(|s| {
                let (name, val) = s.trim().split_once('=')?;
                if name.trim() == CSRF_COOKIE {
                    Some(val.trim().to_string())
                } else {
                    None
                }
            })
        })
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    a_bytes.ct_eq(b_bytes).into()
}

fn csrf_rejection(request_id: Option<String>) -> Response {
    let body = ApiErrorEnvelope::new(
        StatusCode::FORBIDDEN,
        "CSRF_TOKEN_INVALID",
        "Missing or invalid CSRF token",
        request_id,
        None,
    );
    (StatusCode::FORBIDDEN, axum::Json(body)).into_response()
}

pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
