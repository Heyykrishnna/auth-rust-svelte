use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, PRAGMA, USER_AGENT,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::SsoSessionId;
use crate::services::oidc_provider::{
    self, AuthorizeOutcome, AuthorizeQueryInput, PortalAccountView, PortalInteractionOutcome,
    PortalTransactionContext,
};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TokenEndpointRequest {
    pub grant_type: String,
    pub client_id: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeEndpointRequest {
    pub token: String,
    pub client_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LogoutQueryRequest {
    pub id_token_hint: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub state: Option<String>,
    pub global: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PortalLoginRequest {
    pub csrf_token: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalConsentRequest {
    pub csrf_token: String,
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct PortalSignUpRequest {
    pub csrf_token: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalVerifySignUpRequest {
    pub csrf_token: String,
    pub email: String,
    pub code: String,
}

fn extract_sso_cookie(headers: &HeaderMap, config: &AppConfig) -> Option<String> {
    let target_name = oidc_provider::sso_cookie_name(config);
    headers
        .get(COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            raw.split(';').find_map(|pair| {
                let (k, v) = pair.trim().split_once('=')?;
                if k.trim() == target_name {
                    Some(v.trim().to_string())
                } else {
                    None
                }
            })
        })
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

fn parse_form_or_json<T: for<'de> Deserialize<'de>>(
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<T, AppError> {
    let ct = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if ct.starts_with("application/x-www-form-urlencoded") {
        let map: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(body).into_owned().collect();
        let val = serde_json::to_value(map)
            .map_err(|e| AppError::Validation(format!("Invalid form body: {e}")))?;
        serde_json::from_value(val)
            .map_err(|e| AppError::Validation(format!("Invalid form fields: {e}")))
    } else {
        serde_json::from_slice(body)
            .map_err(|e| AppError::Validation(format!("Invalid JSON payload: {e}")))
    }
}

pub async fn openid_configuration(State(state): State<AppState>) -> Response {
    let doc = oidc_provider::build_discovery_document(&state.config);
    let mut res = (StatusCode::OK, Json(doc)).into_response();
    res.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    res
}

pub async fn oauth2_authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AuthorizeQueryInput>,
) -> Result<Response, AppError> {
    let sso_cookie = extract_sso_cookie(&headers, &state.config);
    let outcome =
        oidc_provider::handle_authorize(&state.db, &state.config, query, sso_cookie.as_deref())
            .await?;

    match outcome {
        AuthorizeOutcome::Redirect(location) => {
            let mut res = StatusCode::FOUND.into_response();
            let loc_val = HeaderValue::from_str(&location)
                .map_err(|_| AppError::Internal("Invalid redirect Location header".to_string()))?;
            res.headers_mut().insert(LOCATION, loc_val);
            res.headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            Ok(res)
        }
    }
}

pub async fn oauth2_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let req: TokenEndpointRequest = parse_form_or_json(&headers, &body)?;
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);

    let client_id = req.client_id.as_deref().unwrap_or("").trim();
    if client_id.is_empty() {
        return Err(AppError::Validation("Missing client_id".to_string()));
    }

    let token_res =
        match req.grant_type.trim() {
            "authorization_code" => {
                let code = req.code.as_deref().ok_or_else(|| {
                    AppError::Validation("Missing authorization code".to_string())
                })?;
                let redirect_uri = req
                    .redirect_uri
                    .as_deref()
                    .ok_or_else(|| AppError::Validation("Missing redirect_uri".to_string()))?;
                let code_verifier = req.code_verifier.as_deref().ok_or_else(|| {
                    AppError::Validation("Missing PKCE code_verifier".to_string())
                })?;

                oidc_provider::exchange_authorization_code(
                    &state.db,
                    &state.config,
                    client_id,
                    code,
                    redirect_uri,
                    code_verifier,
                    ua.as_deref(),
                    ip.as_deref(),
                )
                .await?
            }
            "refresh_token" => {
                let refresh_token = req
                    .refresh_token
                    .as_deref()
                    .ok_or_else(|| AppError::Validation("Missing refresh_token".to_string()))?;
                oidc_provider::exchange_refresh_token(
                    &state.db,
                    &state.config,
                    client_id,
                    refresh_token,
                    ua.as_deref(),
                    ip.as_deref(),
                )
                .await?
            }
            other => {
                return Err(AppError::Validation(format!(
                    "Unsupported grant_type '{other}'"
                )));
            }
        };

    let mut res = (StatusCode::OK, Json(token_res)).into_response();
    res.headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res.headers_mut()
        .insert(PRAGMA, HeaderValue::from_static("no-cache"));
    Ok(res)
}

pub async fn oauth2_userinfo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let bearer = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Unauthorized("Missing Bearer access token".to_string()))?;

    let profile = oidc_provider::get_userinfo(&state.db, &state.config, bearer).await?;
    Ok(Json(profile))
}

pub async fn oauth2_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let req: RevokeEndpointRequest = parse_form_or_json(&headers, &body)?;
    oidc_provider::revoke_oidc_token(&state.db, &req.token, req.client_id.as_deref()).await?;
    Ok(StatusCode::OK)
}

pub async fn oauth2_logout_get(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(query): Query<LogoutQueryRequest>,
) -> Result<(CookieJar, Response), AppError> {
    execute_logout(&state, jar, &headers, query).await
}

pub async fn oauth2_logout_post(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(CookieJar, Response), AppError> {
    let req: LogoutQueryRequest = if body.is_empty() {
        LogoutQueryRequest::default()
    } else {
        parse_form_or_json(&headers, &body)?
    };
    execute_logout(&state, jar, &headers, req).await
}

async fn execute_logout(
    state: &AppState,
    mut jar: CookieJar,
    headers: &HeaderMap,
    req: LogoutQueryRequest,
) -> Result<(CookieJar, Response), AppError> {
    let sso_cookie = extract_sso_cookie(headers, &state.config);
    let global = req.global.unwrap_or(false);
    let outcome = oidc_provider::handle_oidc_logout(
        &state.db,
        &state.config,
        req.id_token_hint.as_deref(),
        req.post_logout_redirect_uri.as_deref(),
        req.state.as_deref(),
        global,
        sso_cookie.as_deref(),
    )
    .await?;

    if outcome.clear_sso_cookie {
        jar = jar.add(oidc_provider::build_clear_sso_cookie(&state.config));
    }

    if let Some(redirect) = outcome.redirect_url {
        let mut res = StatusCode::FOUND.into_response();
        let loc_val = HeaderValue::from_str(&redirect)
            .map_err(|_| AppError::Internal("Invalid logout redirect URL".to_string()))?;
        res.headers_mut().insert(LOCATION, loc_val);
        return Ok((jar, res));
    }

    Ok((
        jar,
        (
            StatusCode::OK,
            Json(json!({
                "logged_out": true,
                "global_logout": outcome.global_logout,
                "revoked_app_sessions": outcome.revoked_app_sessions
            })),
        )
            .into_response(),
    ))
}

pub async fn portal_get_transaction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tx_id): Path<Uuid>,
) -> Result<Json<PortalTransactionContext>, AppError> {
    let sso_cookie = extract_sso_cookie(&headers, &state.config);
    let ctx = oidc_provider::get_portal_transaction_context(
        &state.db,
        &state.config,
        tx_id,
        sso_cookie.as_deref(),
    )
    .await?;
    Ok(Json(ctx))
}

pub async fn portal_login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(tx_id): Path<Uuid>,
    Json(body): Json<PortalLoginRequest>,
) -> Result<(CookieJar, Json<PortalInteractionOutcome>), AppError> {
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);
    let (outcome, raw_sso_token) = oidc_provider::portal_login_for_transaction(
        &state.db,
        &state.redis,
        &state.config,
        tx_id,
        &body.csrf_token,
        &body.email,
        &body.password,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;

    let updated_jar = jar.add(oidc_provider::build_sso_session_cookie(
        &state.config,
        raw_sso_token,
    ));
    Ok((updated_jar, Json(outcome)))
}

pub async fn portal_consent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tx_id): Path<Uuid>,
    Json(body): Json<PortalConsentRequest>,
) -> Result<Json<PortalInteractionOutcome>, AppError> {
    let sso_cookie = extract_sso_cookie(&headers, &state.config);
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);
    let outcome = oidc_provider::portal_consent_for_transaction(
        &state.db,
        tx_id,
        &body.csrf_token,
        body.approved,
        sso_cookie.as_deref(),
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(outcome))
}

pub async fn portal_sign_up(
    State(state): State<AppState>,
    Path(tx_id): Path<Uuid>,
    Json(body): Json<PortalSignUpRequest>,
) -> Result<
    (
        StatusCode,
        Json<crate::services::client_auth::ClientSignUpInitiatedResponse>,
    ),
    AppError,
> {
    let res = oidc_provider::portal_initiate_sign_up_for_transaction(
        &state.db,
        &state.config,
        tx_id,
        &body.csrf_token,
        &body.email,
        &body.password,
        &body.display_name,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(res)))
}

pub async fn portal_verify_sign_up(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(tx_id): Path<Uuid>,
    Json(body): Json<PortalVerifySignUpRequest>,
) -> Result<(CookieJar, Json<PortalInteractionOutcome>), AppError> {
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);
    let (outcome, raw_sso_token) = oidc_provider::portal_verify_sign_up_for_transaction(
        &state.db,
        &state.config,
        tx_id,
        &body.csrf_token,
        &body.email,
        &body.code,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;
    let updated_jar = jar.add(oidc_provider::build_sso_session_cookie(
        &state.config,
        raw_sso_token,
    ));
    Ok((updated_jar, Json(outcome)))
}

pub async fn portal_get_account(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PortalAccountView>, AppError> {
    let sso_cookie = extract_sso_cookie(&headers, &state.config);
    let view = oidc_provider::get_portal_account_view(&state.db, sso_cookie.as_deref()).await?;
    Ok(Json(view))
}

pub async fn portal_revoke_sso_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(sso_id): Path<SsoSessionId>,
) -> Result<StatusCode, AppError> {
    let sso_cookie = extract_sso_cookie(&headers, &state.config);
    oidc_provider::revoke_portal_sso_session(&state.db, sso_cookie.as_deref(), sso_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
