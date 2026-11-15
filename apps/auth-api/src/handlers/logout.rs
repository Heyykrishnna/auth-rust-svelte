use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::cookie::CookieJar;

use crate::errors::AppError;
use crate::services::authentication::{self, AuthContext};
use crate::services::cookies::clear_auth_cookies;
use crate::AppState;

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(CookieJar, StatusCode), AppError> {
    let access_token = jar
        .get("session_token")
        .or_else(|| jar.get("access_token"))
        .map(|c| c.value().to_string())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|val| val.to_str().ok())
                .and_then(|val| val.strip_prefix("Bearer ").map(String::from))
        });

    let refresh_token = jar.get("refresh_token").map(|c| c.value().to_string());

    let ctx = AuthContext {
        config: &state.config,
        db: &state.db,
        redis: &state.redis,
    };

    let _ = authentication::logout(ctx, access_token.as_deref(), refresh_token.as_deref()).await;

    let jar = clear_auth_cookies(jar, &state.config);

    Ok((jar, StatusCode::NO_CONTENT))
}
