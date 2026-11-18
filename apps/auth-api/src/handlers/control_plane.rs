use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::{AuthenticatedDeveloper, PublishableKeyTenant, SecretKeyAuth};
use crate::models::{
    Application, ApplicationDomain, ApplicationEnvironment, ApplicationId, CredentialType,
    EnrollmentPolicy, EnvironmentKind, WorkspaceId, WorkspaceRole,
};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::tenants::{self as tenant_repo, WorkspaceWithRole};
use crate::services::control_auth::{
    self, attach_dev_auth_cookies, clear_dev_auth_cookies, DeveloperProfileResponse,
};
use crate::services::control_plane::{
    self, ApplicationWithEnvironments, CreatedApplicationOutput, CreatedCredentialOutput,
    CredentialSummary, UpdateEnvironmentInput,
};
use crate::services::domains as domain_service;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct DeveloperSignUpRequest {
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub workspace_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeveloperSignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct DeveloperAuthResponse {
    pub developer: DeveloperProfileResponse,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ListApplicationsQuery {
    pub workspace_id: Option<WorkspaceId>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApplicationRequest {
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub slug: String,
    pub initial_dev_redirect_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCredentialRequest {
    pub credential_type: CredentialType,
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RotateCredentialRequest {
    pub grace_window_secs: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDomainRequest {
    pub domain: String,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub post_logout_redirect_uris: Vec<String>,
}

fn parse_env_kind(raw: &str) -> Result<EnvironmentKind, AppError> {
    EnvironmentKind::from_db_str(raw.trim().to_lowercase().as_str()).ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid environment kind '{raw}'; expected 'development' or 'production'"
        ))
    })
}

pub async fn developer_sign_up(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<DeveloperSignUpRequest>,
) -> Result<(StatusCode, CookieJar, Json<DeveloperAuthResponse>), AppError> {
    let (developer, token) = control_auth::register_developer(
        &state.db,
        &state.config,
        &body.email,
        &body.password,
        &body.display_name,
        body.workspace_name.as_deref(),
    )
    .await?;

    let jar = attach_dev_auth_cookies(jar, &state.config, token.clone());
    Ok((
        StatusCode::CREATED,
        jar,
        Json(DeveloperAuthResponse { developer, token }),
    ))
}

pub async fn developer_sign_in(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<DeveloperSignInRequest>,
) -> Result<(CookieJar, Json<DeveloperAuthResponse>), AppError> {
    let (developer, token) = control_auth::login_developer(
        &state.db,
        &state.redis,
        &state.config,
        &body.email,
        &body.password,
    )
    .await?;

    let jar = attach_dev_auth_cookies(jar, &state.config, token.clone());
    Ok((jar, Json(DeveloperAuthResponse { developer, token })))
}

pub async fn developer_sign_out(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<serde_json::Value>), AppError> {
    let jar = clear_dev_auth_cookies(jar, &state.config);
    Ok((jar, Json(json!({ "message": "Signed out" }))))
}

pub async fn developer_me(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<DeveloperProfileResponse>, AppError> {
    let developer = tenant_repo::find_developer_by_id(&state.db, dev.developer_id)
        .await?
        .ok_or(AppError::UserNotFound)?;
    Ok(Json(DeveloperProfileResponse::from(developer)))
}

pub async fn list_workspaces(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
) -> Result<Json<Vec<WorkspaceWithRole>>, AppError> {
    let workspaces = tenant_repo::list_developer_workspaces(&state.db, dev.developer_id).await?;
    Ok(Json(workspaces))
}

pub async fn create_workspace(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceWithRole>), AppError> {
    let clean_name = validate_workspace_name(&body.name)?;
    let clean_slug = domain_service::validate_slug(&body.slug)?;

    let mut tx = begin_tx(&state.db).await?;
    let ws = tenant_repo::create_workspace(&mut *tx, &clean_slug, clean_name)
        .await
        .map_err(|err| match err {
            AppError::Database(sqlx::Error::Database(db_err))
                if db_err.code().as_deref() == Some("23505") =>
            {
                AppError::Validation(format!("Workspace slug '{clean_slug}' is already taken"))
            }
            other => other,
        })?;
    tenant_repo::add_workspace_member(&mut *tx, ws.id, dev.developer_id, WorkspaceRole::Owner)
        .await?;
    commit_tx(tx).await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkspaceWithRole {
            id: ws.id,
            slug: ws.slug,
            name: ws.name,
            status: ws.status,
            role: WorkspaceRole::Owner.as_str().to_string(),
            created_at: ws.created_at,
        }),
    ))
}

fn validate_workspace_name(name: &str) -> Result<&str, AppError> {
    let name = name.trim();
    if !(2..=120).contains(&name.chars().count()) || name.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "Use 2 to 120 characters and no control characters in the workspace name".to_string(),
        ));
    }
    Ok(name)
}

pub async fn update_workspace(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(workspace_id): Path<WorkspaceId>,
    Json(body): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceWithRole>, AppError> {
    let name = validate_workspace_name(&body.name)?;
    let mut tx = begin_tx(&state.db).await?;
    let updated = tenant_repo::rename_workspace(&mut *tx, workspace_id, dev.developer_id, name)
        .await?
        .ok_or_else(|| AppError::Forbidden("You cannot edit this workspace".to_string()))?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        dev.developer_id,
        "WORKSPACE_RENAMED",
        "developer_workspace",
        Some(workspace_id.into_inner()),
        json!({}),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(Json(updated))
}

pub async fn create_application(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Json(body): Json<CreateApplicationRequest>,
) -> Result<(StatusCode, Json<CreatedApplicationOutput>), AppError> {
    let output = control_plane::create_application(
        &state.db,
        dev.developer_id,
        body.workspace_id,
        &body.name,
        &body.slug,
        body.initial_dev_redirect_uri.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(output)))
}

pub async fn list_applications(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Query(query): Query<ListApplicationsQuery>,
) -> Result<Json<Vec<ApplicationWithEnvironments>>, AppError> {
    let apps =
        tenant_repo::list_developer_applications(&state.db, dev.developer_id, query.workspace_id)
            .await?;

    let mut results = Vec::with_capacity(apps.len());
    for app in apps {
        let envs = tenant_repo::list_application_environments(&state.db, app.id).await?;
        results.push(ApplicationWithEnvironments {
            application: app,
            environments: envs,
        });
    }

    Ok(Json(results))
}

pub async fn get_application(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(app_id): Path<ApplicationId>,
) -> Result<Json<ApplicationWithEnvironments>, AppError> {
    let app = control_plane::require_application_access(
        &state.db,
        dev.developer_id,
        app_id,
        WorkspaceRole::Viewer,
    )
    .await?;
    let environments = tenant_repo::list_application_environments(&state.db, app.id).await?;
    Ok(Json(ApplicationWithEnvironments {
        application: app,
        environments,
    }))
}

pub async fn update_application(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(app_id): Path<ApplicationId>,
    Json(body): Json<UpdateApplicationRequest>,
) -> Result<Json<Application>, AppError> {
    let updated = control_plane::update_application(
        &state.db,
        &state.redis,
        dev.developer_id,
        app_id,
        body.name.as_deref(),
        body.status.as_deref(),
        body.metadata,
    )
    .await?;
    Ok(Json(updated))
}

pub async fn delete_application(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path(app_id): Path<ApplicationId>,
) -> Result<StatusCode, AppError> {
    control_plane::delete_application(&state.db, &state.redis, dev.developer_id, app_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_environment(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<ApplicationEnvironment>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    Ok(Json(env))
}

pub async fn update_environment(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<UpdateEnvironmentInput>,
) -> Result<Json<ApplicationEnvironment>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let updated =
        control_plane::update_environment_settings(&state.db, dev.developer_id, app_id, kind, body)
            .await?;
    Ok(Json(updated))
}

pub async fn list_credentials(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<CredentialSummary>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let creds = tenant_repo::list_environment_credentials(&state.db, env.id).await?;
    Ok(Json(
        creds.into_iter().map(CredentialSummary::from).collect(),
    ))
}

pub async fn create_credential(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreateCredentialRequest>,
) -> Result<(StatusCode, Json<CreatedCredentialOutput>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let created = control_plane::create_credential(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        body.credential_type,
        &body.name,
        body.scopes,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn rotate_credential(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, cred_id)): Path<(ApplicationId, String, Uuid)>,
    payload: Option<Json<RotateCredentialRequest>>,
) -> Result<Json<CreatedCredentialOutput>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let grace = payload.and_then(|Json(p)| p.grace_window_secs);
    let rotated = control_plane::rotate_secret_credential(
        &state.db,
        &state.redis,
        dev.developer_id,
        app_id,
        kind,
        cred_id,
        grace,
    )
    .await?;
    Ok(Json(rotated))
}

pub async fn revoke_credential(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, cred_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    control_plane::revoke_credential(
        &state.db,
        &state.redis,
        dev.developer_id,
        app_id,
        kind,
        cred_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_domains(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<ApplicationDomain>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let domains = tenant_repo::list_environment_domains(&state.db, env.id).await?;
    Ok(Json(domains))
}

pub async fn create_domain(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreateDomainRequest>,
) -> Result<(StatusCode, Json<ApplicationDomain>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let created = control_plane::register_domain(
        &state.db,
        &state.redis,
        dev.developer_id,
        app_id,
        kind,
        &body.domain,
        &body.allowed_origins,
        &body.redirect_uris,
        &body.post_logout_redirect_uris,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete_domain(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, domain_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    control_plane::delete_domain(
        &state.db,
        &state.redis,
        dev.developer_id,
        app_id,
        kind,
        domain_id,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_environment_config(
    State(state): State<AppState>,
    PublishableKeyTenant(tenant): PublishableKeyTenant,
) -> Result<Json<serde_json::Value>, AppError> {
    let env = tenant_repo::find_environment_by_id(&state.db, tenant.environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let policy = match env.enrollment_policy.as_str() {
        "auto_join" => EnrollmentPolicy::AutoJoin,
        "invite_only" => EnrollmentPolicy::InviteOnly,
        _ => EnrollmentPolicy::ConsentRequired,
    };

    Ok(Json(json!({
        "application_id": env.application_id,
        "environment_id": env.id,
        "kind": env.kind,
        "enrollment_policy": policy,
        "require_email_verification": env.require_email_verification,
        "branding": env.branding,
    })))
}

pub async fn backend_ping(auth: SecretKeyAuth) -> Result<Json<serde_json::Value>, AppError> {
    auth.require_scope("users:read")?;
    Ok(Json(json!({
        "status": "ok",
        "application_id": auth.tenant.application_id,
        "environment_id": auth.tenant.environment_id,
        "environment_kind": auth.tenant.environment_kind,
        "scopes": auth.scopes,
    })))
}
