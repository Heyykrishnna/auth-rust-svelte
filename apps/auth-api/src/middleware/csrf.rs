use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

static CSRF_COOKIE: &str = "csrf_token";
static CSRF_HEADER: &str = "x-csrf-token";

static MUTATING_METHODS: &[Method] = &[
    Method::POST,
    Method::PUT,
    Method::PATCH,
    Method::DELETE,
];

pub async fn csrf_protection_middleware(req: Request, next: Next) -> Response {
    if !MUTATING_METHODS.contains(req.method()) {
        return next.run(req).await;
    }

    let uses_bearer = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer "))
        .unwrap_or(false);

    if uses_bearer {
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

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if constant_time_eq(&cookie, &header) => {
            next.run(req).await
        }
        _ => csrf_rejection(),
    }
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
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn csrf_rejection() -> Response {
    let body = json!({
        "error": {
            "code": "CSRF_TOKEN_INVALID",
            "message": "Missing or invalid CSRF token"
        },
        "status": 403
    });
    (StatusCode::FORBIDDEN, axum::Json(body)).into_response()
}

pub fn generate_csrf_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    hex::encode(bytes)
}
