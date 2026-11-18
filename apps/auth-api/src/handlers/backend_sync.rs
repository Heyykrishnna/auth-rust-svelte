use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::SecretKeyAuth;
use crate::models::{AppSessionId, AppUserId, UserId};
use crate::services::backend_sync::{
    self, BackendSessionStatusView, BackendUserSyncItem, PaginatedBackendEventsResponse,
    PaginatedBackendUsersResponse, UpdateBackendUserMetadataInput,
};
use crate::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct BackendListUsersQuery {
    pub updated_since: Option<DateTime<Utc>>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct BackendListEventsQuery {
    pub after_event_id: Option<Uuid>,
    pub types: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BackendUpdateMetadataRequest {
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub external_id: Option<Option<String>>,
    pub public_metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize, Default)]
pub struct BackendSuspendUserRequest {
    pub reason: Option<String>,
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Query(query): Query<BackendListUsersQuery>,
) -> Result<Json<PaginatedBackendUsersResponse>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("users:read")?;
    }
    let res = backend_sync::list_users_for_sync(
        &state.db,
        auth.tenant.environment_id,
        query.updated_since,
        query.cursor.as_deref(),
        query.limit,
    )
    .await?;
    Ok(Json(res))
}

pub async fn get_user_by_app_user_id(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(app_user_id): Path<AppUserId>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    auth.require_scope("users:read")?;
    let user =
        backend_sync::get_user_by_app_user_id(&state.db, auth.tenant.environment_id, app_user_id)
            .await?;
    Ok(Json(user))
}

pub async fn get_user_by_sub(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(sub): Path<UserId>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    auth.require_scope("users:read")?;
    let user = backend_sync::get_user_by_sub(&state.db, auth.tenant.environment_id, sub).await?;
    Ok(Json(user))
}

pub async fn update_user_metadata(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(app_user_id): Path<AppUserId>,
    Json(body): Json<BackendUpdateMetadataRequest>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    auth.require_scope("users:write")?;
    let updated = backend_sync::update_user_metadata(
        &state.db,
        auth.tenant.environment_id,
        app_user_id,
        UpdateBackendUserMetadataInput {
            external_id: body.external_id,
            public_metadata: body.public_metadata,
            private_metadata: body.private_metadata,
        },
    )
    .await?;
    Ok(Json(updated))
}

pub async fn suspend_user(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(app_user_id): Path<AppUserId>,
    body: Option<Json<BackendSuspendUserRequest>>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    auth.require_scope("users:write")?;
    let reason = body.and_then(|Json(b)| b.reason);
    let updated = backend_sync::suspend_application_user(
        &state.db,
        auth.tenant.environment_id,
        app_user_id,
        reason.as_deref(),
    )
    .await?;
    Ok(Json(updated))
}

pub async fn restore_user(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(app_user_id): Path<AppUserId>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    auth.require_scope("users:write")?;
    let updated =
        backend_sync::restore_application_user(&state.db, auth.tenant.environment_id, app_user_id)
            .await?;
    Ok(Json(updated))
}

pub async fn list_events(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Query(query): Query<BackendListEventsQuery>,
) -> Result<Json<PaginatedBackendEventsResponse>, AppError> {
    auth.require_scope("sync:read")?;
    let parsed_types: Option<Vec<String>> = query.types.as_deref().map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect()
    });
    let res = backend_sync::list_events_catchup(
        &state.db,
        auth.tenant.environment_id,
        query.after_event_id,
        parsed_types.as_deref(),
        query.limit,
    )
    .await?;
    Ok(Json(res))
}

pub async fn get_session_status(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(session_id): Path<AppSessionId>,
) -> Result<Json<BackendSessionStatusView>, AppError> {
    auth.require_scope("sessions:read")?;
    let res =
        backend_sync::get_session_status(&state.db, auth.tenant.environment_id, session_id).await?;
    Ok(Json(res))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(session_id): Path<AppSessionId>,
) -> Result<Json<BackendSessionStatusView>, AppError> {
    auth.require_scope("sessions:revoke")?;
    let res =
        backend_sync::revoke_session(&state.db, auth.tenant.environment_id, session_id).await?;
    Ok(Json(res))
}

pub async fn revoke_all_user_sessions(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(app_user_id): Path<AppUserId>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_scope("sessions:revoke")?;
    let revoked =
        backend_sync::revoke_all_user_sessions(&state.db, auth.tenant.environment_id, app_user_id)
            .await?;
    Ok(Json(json!({ "revoked_sessions": revoked })))
}
