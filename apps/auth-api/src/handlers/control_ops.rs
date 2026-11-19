use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::{
    AppSessionId, AppUserId, ApplicationId, DeveloperId, EnvironmentKind, WorkspaceId,
    WorkspaceRole,
};
use crate::services::backend_sync::{
    BackendSessionStatusView, BackendUserSyncItem, UpdateBackendUserMetadataInput,
};
use crate::services::control_ops::{
    self, EnvironmentOverviewResponse, EnvironmentUserDetailResponse, ListAuditLogsFilter,
    ListEnvironmentUsersFilter, OAuthProviderConfigView, PaginatedAuditLogsResponse,
    PaginatedEnvironmentUsersResponse, UpsertOAuthProviderInput, WorkspaceMemberView,
};
use crate::AppState;

fn parse_env_kind(raw: &str) -> Result<EnvironmentKind, AppError> {
    EnvironmentKind::from_db_str(raw.trim().to_lowercase().as_str()).ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid environment kind '{raw}'; expected 'development' or 'production'"
        ))
    })
}

fn parse_workspace_role(raw: &str) -> Result<WorkspaceRole, AppError> {
    match raw.trim().to_lowercase().as_str() {
        "owner" => Ok(WorkspaceRole::Owner),
        "admin" => Ok(WorkspaceRole::Admin),
        "developer" => Ok(WorkspaceRole::Developer),
        "viewer" => Ok(WorkspaceRole::Viewer),
        other => Err(AppError::Validation(format!(
            "Invalid workspace role '{other}'; expected owner, admin, developer, or viewer"
        ))),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ListUsersQuery {
    pub q: Option<String>,
    pub status: Option<String>,
    pub email_verified: Option<bool>,
    pub role: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateUserMetadataRequest {
    pub external_id: Option<Option<String>>,
    pub public_metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: String,
    pub action: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct SuspendUserRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListAuditLogsQuery {
    pub event: Option<String>,
    pub actor_type: Option<String>,
    pub resource_type: Option<String>,
    pub q: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AddWorkspaceMemberRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceMemberRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeDeveloperPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

pub async fn get_overview(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<EnvironmentOverviewResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let overview = control_ops::get_environment_overview(
        &state.db,
        &state.config,
        dev.developer_id,
        app_id,
        kind,
    )
    .await?;
    Ok(Json(overview))
}

pub async fn list_oauth_providers(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<OAuthProviderConfigView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let providers =
        control_ops::list_oauth_providers(&state.db, &state.config, dev.developer_id, app_id, kind)
            .await?;
    Ok(Json(providers))
}

pub async fn upsert_oauth_provider(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, provider)): Path<(ApplicationId, String, String)>,
    Json(body): Json<UpsertOAuthProviderInput>,
) -> Result<Json<OAuthProviderConfigView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let updated = control_ops::upsert_oauth_provider(
        &state.db,
        &state.config,
        dev.developer_id,
        app_id,
        kind,
        &provider,
        body,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn list_users(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<PaginatedEnvironmentUsersResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let response = control_ops::list_environment_users(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        ListEnvironmentUsersFilter {
            query: query.q,
            status: query.status,
            email_verified: query.email_verified,
            role: query.role,
            limit: query.limit,
            offset: query.offset,
        },
    )
    .await?;
    Ok(Json(response))
}

pub async fn get_user_detail(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
) -> Result<Json<EnvironmentUserDetailResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let detail = control_ops::get_environment_user_detail(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
    )
    .await?;
    Ok(Json(detail))
}

pub async fn update_user_metadata(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
    Json(body): Json<UpdateUserMetadataRequest>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let updated = control_ops::update_environment_user_metadata(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
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

pub async fn update_user_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
    Json(body): Json<UpdateUserRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let roles = control_ops::update_environment_user_role(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
        &body.role,
        &body.action,
    )
    .await?;
    Ok(Json(json!({ "roles": roles })))
}

pub async fn suspend_user(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
    body: Option<Json<SuspendUserRequest>>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let reason = body.and_then(|Json(b)| b.reason);
    let updated = control_ops::suspend_environment_user(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
        reason,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn restore_user(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
) -> Result<Json<BackendUserSyncItem>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let updated = control_ops::restore_environment_user(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn trigger_password_reset(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let email = control_ops::trigger_user_password_reset(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
    )
    .await?;
    Ok(Json(json!({
        "status": "dispatched",
        "email": email
    })))
}

pub async fn revoke_all_user_sessions(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, app_user_id)): Path<(ApplicationId, String, AppUserId)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let count = control_ops::revoke_all_environment_user_sessions(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        app_user_id,
    )
    .await?;
    Ok(Json(json!({ "revoked_sessions": count })))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, session_id)): Path<(ApplicationId, String, AppSessionId)>,
) -> Result<Json<BackendSessionStatusView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let view = control_ops::revoke_environment_session(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        session_id,
    )
    .await?;
    Ok(Json(view))
}

pub async fn list_audit_logs(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<PaginatedAuditLogsResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let response = control_ops::list_environment_audit_logs(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        ListAuditLogsFilter {
            event: query.event,
            actor_type: query.actor_type,
            resource_type: query.resource_type,
            query: query.q,
            from: query.from,
            to: query.to,
            limit: query.limit,
            offset: query.offset,
        },
    )
    .await?;
    Ok(Json(response))
}

pub async fn list_workspace_members(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(workspace_id): Path<WorkspaceId>,
) -> Result<Json<Vec<WorkspaceMemberView>>, AppError> {
    let members =
        control_ops::list_workspace_members(&state.db, dev.developer_id, workspace_id).await?;
    Ok(Json(members))
}

pub async fn add_workspace_member(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(workspace_id): Path<WorkspaceId>,
    Json(body): Json<AddWorkspaceMemberRequest>,
) -> Result<(StatusCode, Json<WorkspaceMemberView>), AppError> {
    let role = parse_workspace_role(&body.role)?;
    let added = control_ops::add_workspace_member_by_email(
        &state.db,
        dev.developer_id,
        workspace_id,
        &body.email,
        role,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(added)))
}

pub async fn update_workspace_member_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((workspace_id, member_id)): Path<(WorkspaceId, DeveloperId)>,
    Json(body): Json<UpdateWorkspaceMemberRoleRequest>,
) -> Result<Json<WorkspaceMemberView>, AppError> {
    let role = parse_workspace_role(&body.role)?;
    let updated = control_ops::update_workspace_member_role(
        &state.db,
        dev.developer_id,
        workspace_id,
        member_id,
        role,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn remove_workspace_member(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((workspace_id, member_id)): Path<(WorkspaceId, DeveloperId)>,
) -> Result<StatusCode, AppError> {
    control_ops::remove_workspace_member(&state.db, dev.developer_id, workspace_id, member_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_developer_password(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<ChangeDeveloperPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    control_ops::change_developer_password(
        &state.db,
        &dev,
        &body.current_password,
        &body.new_password,
    )
    .await?;
    Ok(Json(json!({ "message": "Password updated successfully" })))
}
