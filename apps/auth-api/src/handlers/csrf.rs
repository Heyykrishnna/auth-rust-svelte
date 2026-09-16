use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde_json::{json, Value};

use crate::middleware::generate_csrf_token;
use crate::services::cookies::build_csrf_cookie;
use crate::AppState;

pub async fn csrf_token(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (StatusCode, CookieJar, Json<Value>) {
    let token = generate_csrf_token();
    let cookie = build_csrf_cookie(&state.config, token.clone());
    (
        StatusCode::OK,
        jar.add(cookie),
        Json(json!({ "csrf_token": token })),
    )
}
