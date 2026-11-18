use axum::extract::State;
use axum::http::header::CACHE_CONTROL;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::{json, Value};

use crate::middleware::generate_csrf_token;
use crate::services::cookies::build_csrf_cookie;
use crate::AppState;

pub async fn csrf_token(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (
    CookieJar,
    [(axum::http::HeaderName, &'static str); 1],
    Json<Value>,
) {
    let token = generate_csrf_token();
    let cookie = build_csrf_cookie(&state.config, token.clone());
    (
        jar.add(cookie),
        [(CACHE_CONTROL, "no-store")],
        Json(json!({ "csrf_token": token })),
    )
}
