use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH, USER_AGENT};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::middleware::tenant::PUBLISHABLE_KEY_HEADER;
use crate::middleware::{AuthenticatedClientUser, PublishableKeyTenant};
use crate::models::AppSessionId;
use crate::services::client_auth::{
    self, ClientAuthResponse, ClientSessionView, ClientSignUpInitiatedResponse,
    ClientTokenRefreshResponse, ClientUserView, SecondaryEmailVerificationResponse,
};
use crate::services::signing_keys;
use crate::services::social_oauth::{self, SocialOAuthStartResponse};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ClientSignUpRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientVerifySignUpRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientEmailOnlyRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientSignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientRefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientVerifyEmailRequest {
    pub email: String,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClientResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientUpdateProfileRequest {
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SocialOAuthStartQuery {
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SocialOAuthCallbackRequest {
    pub code: String,
    pub state: String,
}

fn extract_publishable_key_str(headers: &HeaderMap) -> &str {
    headers
        .get(PUBLISHABLE_KEY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .unwrap_or("")
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
}

pub async fn jwks_endpoint(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    crate::services::metrics::record_jwks_request();
    let (jwks, etag) = signing_keys::build_jwks_with_etag(&state.db, &state.config).await?;

    if let Some(if_none_match) = headers.get(IF_NONE_MATCH).and_then(|v| v.to_str().ok()) {
        if if_none_match.trim() == etag {
            let mut res = StatusCode::NOT_MODIFIED.into_response();
            res.headers_mut().insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=300"),
            );
            if let Ok(etag_val) = HeaderValue::from_str(&etag) {
                res.headers_mut().insert(ETAG, etag_val);
            }
            return Ok(res);
        }
    }

    let mut res = (StatusCode::OK, Json(jwks)).into_response();
    res.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        res.headers_mut().insert(ETAG, etag_val);
    }
    Ok(res)
}

pub async fn client_sign_up(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientSignUpRequest>,
) -> Result<(StatusCode, Json<ClientSignUpInitiatedResponse>), AppError> {
    let (res, _) = client_auth::initiate_sign_up(
        &state.db,
        &state.config,
        tenant.environment_id,
        &body.email,
        &body.password,
        &body.display_name,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(res)))
}

pub async fn client_verify_sign_up(
    State(state): State<AppState>,
    headers: HeaderMap,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientVerifySignUpRequest>,
) -> Result<(StatusCode, Json<ClientAuthResponse>), AppError> {
    let azp = extract_publishable_key_str(&headers);
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);

    let res = client_auth::verify_sign_up_otp(
        &state.db,
        &state.config,
        tenant.environment_id,
        azp,
        &body.email,
        &body.code,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(res)))
}

pub async fn client_resend_sign_up_otp(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientEmailOnlyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = client_auth::resend_sign_up_otp(
        &state.db,
        &state.config,
        tenant.environment_id,
        &body.email,
    )
    .await?;
    Ok(Json(json!({ "message": "Verification code resent" })))
}

pub async fn client_sign_in(
    State(state): State<AppState>,
    headers: HeaderMap,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientSignInRequest>,
) -> Result<Json<ClientAuthResponse>, AppError> {
    let azp = extract_publishable_key_str(&headers);
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);

    let res = client_auth::sign_in(
        &state.db,
        &state.redis,
        &state.config,
        tenant.environment_id,
        azp,
        &body.email,
        &body.password,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;

    Ok(Json(res))
}

pub async fn client_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientRefreshRequest>,
) -> Result<Json<ClientTokenRefreshResponse>, AppError> {
    let azp = extract_publishable_key_str(&headers);
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);

    let res = client_auth::refresh_tokens(
        &state.db,
        &state.config,
        tenant.environment_id,
        azp,
        &body.refresh_token,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;

    Ok(Json(res))
}

pub async fn client_sign_out(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<StatusCode, AppError> {
    client_auth::sign_out(
        &state.db,
        &state.redis,
        user.tenant.environment_id,
        user.user_id,
        user.session_id,
        &user.jti,
        state.config.jwt_access_expiry_secs,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_verify_email(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<ClientVerifyEmailRequest>,
) -> Result<(StatusCode, Json<SecondaryEmailVerificationResponse>), AppError> {
    user.require_permission("profile.write")?;
    if let Some(code) = body
        .code
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        let res = client_auth::confirm_secondary_email_verification(
            &state.db,
            user.tenant.environment_id,
            user.user_id,
            &body.email,
            code,
        )
        .await?;
        Ok((StatusCode::OK, Json(res)))
    } else {
        let (res, _) = client_auth::request_secondary_email_verification(
            &state.db,
            &state.config,
            user.tenant.environment_id,
            user.user_id,
            &body.email,
        )
        .await?;
        Ok((StatusCode::ACCEPTED, Json(res)))
    }
}

pub async fn client_forgot_password(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientEmailOnlyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = client_auth::request_password_reset(
        &state.db,
        &state.config,
        tenant.environment_id,
        &body.email,
    )
    .await?;
    Ok(Json(json!({
        "message": "If an account exists for that email, a password reset link has been sent."
    })))
}

pub async fn client_reset_password(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Json(body): Json<ClientResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    client_auth::complete_password_reset(
        &state.db,
        tenant.environment_id,
        &body.token,
        &body.new_password,
    )
    .await?;
    Ok(Json(json!({
        "message": "Password reset successfully; all active sessions have been revoked."
    })))
}

pub async fn client_change_password(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<ClientChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    client_auth::change_password(
        &state.db,
        user.tenant.environment_id,
        user.user_id,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok(Json(json!({ "message": "Password changed successfully" })))
}

pub async fn client_oauth_start(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Path(provider): Path<String>,
    Query(query): Query<SocialOAuthStartQuery>,
) -> Result<Json<SocialOAuthStartResponse>, AppError> {
    let res = social_oauth::start_social_oauth(
        &state.db,
        &state.config,
        tenant.environment_id,
        &provider,
        query.redirect_uri.as_deref(),
    )
    .await?;
    Ok(Json(res))
}

pub async fn client_oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
    Path(provider): Path<String>,
    Json(body): Json<SocialOAuthCallbackRequest>,
) -> Result<Json<ClientAuthResponse>, AppError> {
    let azp = extract_publishable_key_str(&headers);
    let ua = extract_user_agent(&headers);
    let ip = extract_client_ip(&headers);

    let res = social_oauth::handle_social_oauth_callback(
        &state.db,
        &state.config,
        tenant.environment_id,
        azp,
        &provider,
        &body.code,
        &body.state,
        ua.as_deref(),
        ip.as_deref(),
    )
    .await?;
    Ok(Json(res))
}

pub async fn client_get_me(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<ClientUserView>, AppError> {
    user.require_permission("profile.read")?;
    let profile =
        client_auth::get_client_user_profile(&state.db, user.tenant.environment_id, user.user_id)
            .await?;
    Ok(Json(profile))
}

pub async fn client_update_me(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<ClientUpdateProfileRequest>,
) -> Result<Json<ClientUserView>, AppError> {
    user.require_permission("profile.write")?;
    let profile = client_auth::update_client_user_profile(
        &state.db,
        user.tenant.environment_id,
        user.user_id,
        &body.display_name,
        body.avatar_url.as_deref(),
    )
    .await?;
    Ok(Json(profile))
}

pub async fn client_list_sessions(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<Vec<ClientSessionView>>, AppError> {
    user.require_permission("sessions.read")?;
    let sessions = client_auth::list_user_environment_sessions(
        &state.db,
        user.tenant.environment_id,
        user.app_user_id,
        user.session_id,
    )
    .await?;
    Ok(Json(sessions))
}

pub async fn client_revoke_session(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(session_id): Path<AppSessionId>,
) -> Result<StatusCode, AppError> {
    user.require_permission("sessions.delete")?;
    client_auth::revoke_single_session(
        &state.db,
        user.tenant.environment_id,
        user.app_user_id,
        user.user_id,
        session_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_revoke_all_sessions(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<StatusCode, AppError> {
    user.require_permission("sessions.delete")?;
    client_auth::revoke_all_environment_sessions(
        &state.db,
        user.tenant.environment_id,
        user.app_user_id,
        user.user_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
