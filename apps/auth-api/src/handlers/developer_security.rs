use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::services::developer_security::{
    devices, factors, passkeys, recovery, risk, verification,
};
use crate::AppState;

#[derive(Deserialize)]
pub struct VerifyPassword {
    password: String,
    second_factor_code: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfirmCode {
    code: String,
}

#[derive(Deserialize)]
pub struct DeviceLabel {
    label: String,
}

#[derive(Deserialize)]
pub struct PasskeyRegistration {
    label: String,
    credential: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
pub struct RecoverAccount {
    email: String,
}

#[derive(Deserialize)]
pub struct ResetPassword {
    token: String,
    new_password: String,
}

#[derive(Serialize)]
pub struct Proof {
    verified_until: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct BackupCodes {
    codes: Vec<String>,
}

pub async fn verify_password(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<VerifyPassword>,
) -> Result<Json<Proof>, AppError> {
    let verified_until = verification::verify_password(
        &state,
        &dev,
        body.password,
        body.second_factor_code.as_deref(),
    )
    .await?;
    Ok(Json(Proof { verified_until }))
}

pub async fn begin_totp(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<factors::Enrollment>, AppError> {
    Ok(Json(factors::begin_enrollment(&state, &dev).await?))
}

pub async fn confirm_totp(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<ConfirmCode>,
) -> Result<Json<BackupCodes>, AppError> {
    let codes = factors::confirm_enrollment(&state, &dev, &body.code).await?;
    Ok(Json(BackupCodes { codes }))
}

pub async fn remove_totp(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<StatusCode, AppError> {
    factors::remove_authenticator(&state, &dev).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rotate_backup_codes(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<BackupCodes>, AppError> {
    let codes = factors::rotate_backup_codes(&state, &dev).await?;
    Ok(Json(BackupCodes { codes }))
}

pub async fn begin_passkey(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<CreationChallengeResponse>, AppError> {
    Ok(Json(passkeys::begin_registration(&state, &dev).await?))
}

pub async fn finish_passkey(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<PasskeyRegistration>,
) -> Result<StatusCode, AppError> {
    passkeys::finish_registration(&state, &dev, &body.label, &body.credential).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_passkey(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    passkeys::remove(&state, &dev, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn begin_passkey_verification(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<RequestChallengeResponse>, AppError> {
    Ok(Json(passkeys::begin_verification(&state, &dev).await?))
}

pub async fn finish_passkey_verification(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(credential): Json<PublicKeyCredential>,
) -> Result<Json<Proof>, AppError> {
    let verified_until = passkeys::finish_verification(&state, &dev, &credential).await?;
    Ok(Json(Proof { verified_until }))
}

pub async fn trust_browser(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    jar: CookieJar,
    Json(body): Json<DeviceLabel>,
) -> Result<(CookieJar, StatusCode), AppError> {
    let cookie = devices::trust(&state, &dev, &body.label).await?;
    Ok((jar.add(cookie), StatusCode::NO_CONTENT))
}

pub async fn forget_browser(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<(CookieJar, StatusCode), AppError> {
    let current = devices::forget(&state, &dev, id).await?;
    let jar = if current {
        jar.add(devices::clear_cookie(&state.config))
    } else {
        jar
    };
    Ok((jar, StatusCode::NO_CONTENT))
}

pub async fn set_notifications(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(preferences): Json<risk::NotificationPreferences>,
) -> Result<StatusCode, AppError> {
    risk::set_preferences(&state, &dev, preferences).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn request_recovery(
    State(state): State<AppState>,
    Json(body): Json<RecoverAccount>,
) -> Result<StatusCode, AppError> {
    recovery::request(&state, &body.email).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn reset_password(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<ResetPassword>,
) -> Result<(CookieJar, StatusCode), AppError> {
    recovery::finish(&state, &body.token, body.new_password).await?;
    let jar = crate::services::control_auth::clear_dev_auth_cookies(jar, &state.config)
        .add(devices::clear_cookie(&state.config));
    Ok((jar, StatusCode::NO_CONTENT))
}
