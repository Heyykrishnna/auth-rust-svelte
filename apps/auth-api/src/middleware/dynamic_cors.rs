use axum::extract::{Request, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE, ORIGIN, VARY,
};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::middleware::tenant::PUBLISHABLE_KEY_HEADER;
use crate::repositories::tenants as tenant_repo;
use crate::services::domains as domain_service;
use crate::AppState;

const ALLOWED_METHODS_HEADER: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";
const ALLOWED_HEADERS_VALUE: &str =
    "authorization, accept, content-type, origin, x-csrf-token, x-request-id, x-publishable-key";

pub async fn dynamic_cors_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let origin_header = req
        .headers()
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().trim_end_matches('/').to_string());

    let Some(origin) = origin_header else {
        return next.run(req).await;
    };

    let publishable_key = req
        .headers()
        .get(PUBLISHABLE_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    let is_preflight = req.method() == Method::OPTIONS;
    let origin_allowed =
        is_origin_permitted(&state, &origin, publishable_key.as_deref(), is_preflight).await;

    if is_preflight {
        if origin_allowed {
            let mut res = StatusCode::NO_CONTENT.into_response();
            attach_cors_headers(res.headers_mut(), &origin);
            res.headers_mut()
                .insert(ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("600"));
            return res;
        }
        return StatusCode::FORBIDDEN.into_response();
    }

    let mut res = next.run(req).await;
    if origin_allowed {
        attach_cors_headers(res.headers_mut(), &origin);
    } else {
        res.headers_mut()
            .append(VARY, HeaderValue::from_static("Origin"));
    }
    res
}

async fn is_origin_permitted(
    state: &AppState,
    origin: &str,
    publishable_key: Option<&str>,
    is_preflight: bool,
) -> bool {
    if state
        .config
        .cors_origins
        .iter()
        .any(|allowed| allowed.trim_end_matches('/') == origin)
    {
        return true;
    }

    if let Some(pk) = publishable_key {
        if let Ok(Some(env)) =
            tenant_repo::find_active_environment_by_publishable_key(&state.db, pk).await
        {
            if let Ok(origins) =
                domain_service::get_cached_allowed_origins(&state.db, &state.redis, env.id).await
            {
                return origins.iter().any(|allowed| allowed == origin);
            }
        }
        return false;
    }

    if is_preflight {
        return tenant_repo::is_origin_allowed_in_any_active_environment(&state.db, origin)
            .await
            .unwrap_or(false);
    }

    false
}

fn attach_cors_headers(headers: &mut axum::http::HeaderMap, origin: &str) {
    if let Ok(origin_val) = HeaderValue::from_str(origin) {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
        headers.insert(
            ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );
        headers.insert(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOWED_METHODS_HEADER),
        );
        headers.insert(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOWED_HEADERS_VALUE),
        );
        headers.append(VARY, HeaderValue::from_static("Origin"));
    }
}
