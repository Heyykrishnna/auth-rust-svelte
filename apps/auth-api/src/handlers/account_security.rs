use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{
    AccountRecoveryStatus, BackupCodesResponse, DeviceInfo, LoginHistoryEntry, PasskeyInfo,
    PasskeyRegistrationOptions, SecurityAlertInfo, StepUpChallengeRequest, StepUpTokenResponse,
    ThreatIntelligenceReport, TotpEnrollResponse, UpdateSecuritySettingsRequest,
    UserSecuritySettings, VerifyBackupCodeRequest, VerifyPasskeyRegistrationRequest,
    VerifyTotpRequest,
};
use crate::services::account_security as sec_service;
use crate::services::sessions as session_service;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecuritySummaryResponse {
    pub settings: UserSecuritySettings,
    pub passkeys: Vec<PasskeyInfo>,
    pub backup_codes_count: usize,
    pub threat_report: ThreatIntelligenceReport,
    pub recovery_status: AccountRecoveryStatus,
    pub active_alerts: Vec<SecurityAlertInfo>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

fn extract_client_info(headers: &HeaderMap) -> (Option<&str>, Option<&str>) {
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim())
        });
    (ua, ip)
}

pub async fn get_security_summary(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<SecuritySummaryResponse>, AppError> {
    let (ua, ip) = extract_client_info(&headers);

    let settings = sec_service::get_security_settings(&state.db, user.user_id).await?;
    let passkeys = sec_service::list_passkeys(&state.db, user.user_id).await?;
    let backup_codes_count = sec_service::get_backup_codes_status(&state.db, user.user_id).await?;
    let threat_report =
        sec_service::evaluate_ato_risk(&state.db, &state.redis, user.user_id, &user.email, ip, ua)
            .await?;
    let recovery_status =
        sec_service::get_recovery_status(&state.db, user.user_id, &user.email).await?;
    let active_alerts = sec_service::get_security_alerts(&state.db, user.user_id).await?;

    Ok(Json(SecuritySummaryResponse {
        settings,
        passkeys,
        backup_codes_count,
        threat_report,
        recovery_status,
        active_alerts,
    }))
}

pub async fn get_security_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserSecuritySettings>, AppError> {
    let settings = sec_service::get_security_settings(&state.db, user.user_id).await?;
    Ok(Json(settings))
}

pub async fn update_security_settings(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<UpdateSecuritySettingsRequest>,
) -> Result<Json<UserSecuritySettings>, AppError> {
    let updated = sec_service::update_security_settings(&state.db, user.user_id, payload).await?;
    Ok(Json(updated))
}

pub async fn enroll_totp(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<TotpEnrollResponse>, AppError> {
    let response = sec_service::enroll_totp(&state.db, user.user_id, &user.email).await?;
    Ok(Json(response))
}

pub async fn confirm_totp(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<VerifyTotpRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let success = sec_service::confirm_totp(&state.db, user.user_id, &payload.code).await?;
    if !success {
        return Err(AppError::Validation(
            "Invalid TOTP authentication code. Please check your authenticator clock.".to_string(),
        ));
    }

    let _ = crate::services::audit::log_event(
        &state.db,
        Some(user.user_id),
        crate::models::AuditEvent::MfaEnabled,
        None,
        None,
        serde_json::json!({ "method": "totp" }),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Two-factor authentication successfully enabled"
    })))
}

pub async fn disable_totp(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let token = headers
        .get("x-step-up-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let valid_step_up =
        sec_service::verify_step_up_token(&state.redis, user.user_id, token).await?;

    if !valid_step_up {
        return Err(AppError::Unauthorized(
            "Step-up re-authentication required to disable Two-Factor Authentication".to_string(),
        ));
    }

    sec_service::disable_totp(&state.db, user.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_passkey_registration_options(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<PasskeyRegistrationOptions>, AppError> {
    let options =
        sec_service::generate_passkey_options(user.user_id, &user.email, &user.display_name);
    Ok(Json(options))
}

pub async fn register_passkey(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<VerifyPasskeyRegistrationRequest>,
) -> Result<Json<PasskeyInfo>, AppError> {
    let passkey = sec_service::register_passkey(&state.db, user.user_id, payload).await?;

    let _ = crate::services::audit::log_event(
        &state.db,
        Some(user.user_id),
        crate::models::AuditEvent::MfaEnabled,
        None,
        None,
        serde_json::json!({ "method": "passkey", "name": passkey.name }),
    )
    .await;

    Ok(Json(passkey))
}

pub async fn list_passkeys(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<PasskeyInfo>>, AppError> {
    let list = sec_service::list_passkeys(&state.db, user.user_id).await?;
    Ok(Json(list))
}

pub async fn delete_passkey(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(passkey_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sec_service::delete_passkey(&state.db, user.user_id, passkey_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_backup_codes_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = sec_service::get_backup_codes_status(&state.db, user.user_id).await?;
    Ok(Json(serde_json::json!({ "remaining_count": count })))
}

pub async fn generate_backup_codes(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<BackupCodesResponse>, AppError> {
    let token = headers
        .get("x-step-up-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let valid_step_up =
        sec_service::verify_step_up_token(&state.redis, user.user_id, token).await?;

    if !valid_step_up {
        return Err(AppError::Unauthorized(
            "Step-up re-authentication required to generate and reveal backup codes".to_string(),
        ));
    }

    let response = sec_service::generate_and_store_backup_codes(&state.db, user.user_id).await?;
    Ok(Json(response))
}

pub async fn verify_backup_code(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<VerifyBackupCodeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let valid =
        sec_service::verify_and_consume_backup_code(&state.db, user.user_id, &payload.code).await?;

    if !valid {
        return Err(AppError::Validation(
            "Invalid or already used backup code".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({ "valid": true })))
}

pub async fn authenticate_step_up(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<StepUpChallengeRequest>,
) -> Result<Json<StepUpTokenResponse>, AppError> {
    let response =
        sec_service::authenticate_step_up(&state.db, &state.redis, user.user_id, payload).await?;
    Ok(Json(response))
}

pub async fn list_devices(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<Vec<DeviceInfo>>, AppError> {
    let (ua, ip) = extract_client_info(&headers);
    let _ = sec_service::register_device_activity(
        &state.db,
        user.user_id,
        ua,
        ip,
        Some("Current Location"),
    )
    .await;

    let devices = sec_service::list_devices(&state.db, user.user_id, ua, ip).await?;
    Ok(Json(devices))
}

pub async fn trust_device(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sec_service::trust_device(&state.db, user.user_id, device_id, true).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn untrust_device(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sec_service::trust_device(&state.db, user.user_id, device_id, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_device(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sec_service::remove_device(&state.db, user.user_id, device_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_security_alerts(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<SecurityAlertInfo>>, AppError> {
    let alerts = sec_service::get_security_alerts(&state.db, user.user_id).await?;
    Ok(Json(alerts))
}

pub async fn resolve_security_alert(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(alert_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    sec_service::resolve_security_alert(&state.db, user.user_id, alert_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_threat_intelligence(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<Json<ThreatIntelligenceReport>, AppError> {
    let (ua, ip) = extract_client_info(&headers);
    let report =
        sec_service::evaluate_ato_risk(&state.db, &state.redis, user.user_id, &user.email, ip, ua)
            .await?;
    Ok(Json(report))
}

pub async fn get_login_history(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<LoginHistoryEntry>>, AppError> {
    let limit = query.limit.unwrap_or(25).clamp(1, 100);
    let history = sec_service::get_login_history(&state.db, user.user_id, limit).await?;
    Ok(Json(history))
}

pub async fn get_recovery_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<AccountRecoveryStatus>, AppError> {
    let status = sec_service::get_recovery_status(&state.db, user.user_id, &user.email).await?;
    Ok(Json(status))
}

pub async fn revoke_other_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let token = headers
        .get("x-step-up-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let valid_step_up =
        sec_service::verify_step_up_token(&state.redis, user.user_id, token).await?;

    if !valid_step_up {
        return Err(AppError::Unauthorized(
            "Step-up re-authentication required to revoke all other active sessions".to_string(),
        ));
    }

    let sessions = session_service::list_user_sessions(&state.db, user.user_id, None).await?;
    for s in sessions {
        let _ = session_service::invalidate_session(&state.db, &state.redis, s.id).await;
    }

    Ok(StatusCode::NO_CONTENT)
}
