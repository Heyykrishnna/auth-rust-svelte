use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::{AuthenticatedClientUser, AuthenticatedDeveloper, SecretKeyAuth};
use crate::models::{AppUserId, ApplicationId, EnvironmentKind, OrganizationId, WorkspaceRole};
use crate::services::control_plane;
use crate::services::email::{self, EmailBrand};
use crate::services::identity_advanced::{
    self, IdentityDataExportResponse, LinkedOAuthAccountView, UserEmailView,
};
use crate::services::organizations::{
    self, ActiveOrganizationSwitchResponse, CreateEnvironmentRoleInput, CreateInvitationInput,
    CreateOrganizationInput, CreatedInvitationResponse, EnvironmentPermissionView,
    EnvironmentRoleView, OrganizationInvitationView, OrganizationMemberView, OrganizationView,
    PaginatedOrganizationsResponse, SwitchActiveOrganizationInput, UpdateOrganizationInput,
    UserOrganizationMembershipView,
};
use crate::AppState;

fn parse_env_kind(raw: &str) -> Result<EnvironmentKind, AppError> {
    EnvironmentKind::from_db_str(raw.trim().to_lowercase().as_str()).ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid environment kind '{raw}'; expected 'development' or 'production'"
        ))
    })
}

fn deserialize_double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize, Default)]
pub struct ListOrganizationsQuery {
    pub updated_since: Option<DateTime<Utc>>,
    pub cursor: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub creator_app_user_id: Option<AppUserId>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub logo_url: Option<Option<String>>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddOrgMemberRequest {
    pub application_user_id: AppUserId,
    pub role_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgMemberRoleRequest {
    pub role_slug: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgInvitationRequest {
    pub email: String,
    pub role_slug: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptOrgInvitationRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct SwitchActiveOrganizationRequest {
    pub organization_id: Option<OrganizationId>,
}

#[derive(Debug, Deserialize)]
pub struct AddUserEmailRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyUserEmailRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub slug: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub slug: String,
    pub name: String,
    pub scope: String,
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

pub async fn client_list_organizations(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<Vec<UserOrganizationMembershipView>>, AppError> {
    let orgs = organizations::list_user_organizations(
        &state.db,
        user.tenant.environment_id,
        user.app_user_id,
    )
    .await?;
    Ok(Json(orgs))
}

pub async fn client_create_organization(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<OrganizationView>), AppError> {
    let org = organizations::create_organization(
        &state.db,
        user.tenant.environment_id,
        Some(user.app_user_id),
        "user",
        Some(user.user_id.into_inner()),
        CreateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn client_get_organization(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<OrganizationView>, AppError> {
    organizations::require_org_member(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let org =
        organizations::get_organization(&state.db, user.tenant.environment_id, org_id).await?;
    Ok(Json(org))
}

pub async fn client_update_organization(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
    Json(body): Json<UpdateOrgRequest>,
) -> Result<Json<OrganizationView>, AppError> {
    organizations::require_org_admin_or_owner(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let org = organizations::update_organization(
        &state.db,
        user.tenant.environment_id,
        org_id,
        "user",
        Some(user.user_id.into_inner()),
        UpdateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            status: body.status,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok(Json(org))
}

pub async fn client_delete_organization(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
) -> Result<StatusCode, AppError> {
    let (_, roles) = organizations::require_org_member(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    if !roles.iter().any(|r| r == "org:owner") {
        return Err(AppError::Forbidden(
            "Only organization owners may delete an organization".to_string(),
        ));
    }
    organizations::delete_organization(
        &state.db,
        user.tenant.environment_id,
        org_id,
        "user",
        Some(user.user_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_list_org_members(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<Vec<OrganizationMemberView>>, AppError> {
    organizations::require_org_member(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let members =
        organizations::list_organization_members(&state.db, user.tenant.environment_id, org_id)
            .await?;
    Ok(Json(members))
}

pub async fn client_update_org_member_role(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path((org_id, member_id)): Path<(OrganizationId, Uuid)>,
    Json(body): Json<UpdateOrgMemberRoleRequest>,
) -> Result<Json<OrganizationMemberView>, AppError> {
    organizations::require_org_admin_or_owner(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let updated = organizations::update_organization_member_role(
        &state.db,
        user.tenant.environment_id,
        org_id,
        member_id,
        &body.role_slug,
        "user",
        Some(user.user_id.into_inner()),
    )
    .await?;
    Ok(Json(updated))
}

pub async fn client_remove_org_member(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path((org_id, member_id)): Path<(OrganizationId, Uuid)>,
) -> Result<StatusCode, AppError> {
    let (self_member_id, roles) = organizations::require_org_member(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let is_admin = roles.iter().any(|r| r == "org:owner" || r == "org:admin");
    if self_member_id != member_id && !is_admin {
        return Err(AppError::Forbidden(
            "Organization admin or owner role required to remove another member".to_string(),
        ));
    }
    organizations::remove_organization_member(
        &state.db,
        user.tenant.environment_id,
        org_id,
        member_id,
        "user",
        Some(user.user_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_list_org_invitations(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<Vec<OrganizationInvitationView>>, AppError> {
    organizations::require_org_admin_or_owner(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let invitations =
        organizations::list_invitations(&state.db, user.tenant.environment_id, org_id).await?;
    Ok(Json(invitations))
}

pub async fn client_create_org_invitation(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(org_id): Path<OrganizationId>,
    Json(body): Json<CreateOrgInvitationRequest>,
) -> Result<(StatusCode, Json<CreatedInvitationResponse>), AppError> {
    organizations::require_org_admin_or_owner(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    let created = organizations::create_invitation(
        &state.db,
        user.tenant.environment_id,
        org_id,
        CreateInvitationInput {
            email: &body.email,
            role_slug: body.role_slug.as_deref(),
            expires_in_days: body.expires_in_days,
            invited_by_app_user_id: Some(user.app_user_id),
            actor_type: "user",
            actor_id: Some(user.user_id.into_inner()),
        },
    )
    .await?;
    if let Ok(org) =
        organizations::get_organization(&state.db, user.tenant.environment_id, org_id).await
    {
        let brand = EmailBrand::from_environment_branding(&serde_json::json!({}), "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    if let Ok(org) =
        organizations::get_organization(&state.db, user.tenant.environment_id, org_id).await
    {
        let brand = EmailBrand::from_environment_branding(&serde_json::json!({}), "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn client_revoke_org_invitation(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path((org_id, invitation_id)): Path<(OrganizationId, Uuid)>,
) -> Result<StatusCode, AppError> {
    organizations::require_org_admin_or_owner(
        &state.db,
        user.tenant.environment_id,
        org_id,
        user.app_user_id,
    )
    .await?;
    organizations::revoke_invitation(
        &state.db,
        user.tenant.environment_id,
        org_id,
        invitation_id,
        "user",
        Some(user.user_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_accept_org_invitation(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<AcceptOrgInvitationRequest>,
) -> Result<Json<OrganizationMemberView>, AppError> {
    let member = organizations::accept_invitation(
        &state.db,
        user.tenant.environment_id,
        user.user_id,
        &body.token,
    )
    .await?;
    Ok(Json(member))
}

pub async fn client_switch_active_organization(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<SwitchActiveOrganizationRequest>,
) -> Result<Json<ActiveOrganizationSwitchResponse>, AppError> {
    let switched = organizations::switch_active_organization(
        &state.db,
        &state.config,
        SwitchActiveOrganizationInput {
            environment_id: user.tenant.environment_id,
            authorized_party: &user.authorized_party,
            user_id: user.user_id,
            app_user_id: user.app_user_id,
            session_id: user.session_id,
            target_org_id: body.organization_id,
        },
    )
    .await?;
    Ok(Json(switched))
}

pub async fn client_list_emails(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<Vec<UserEmailView>>, AppError> {
    user.require_permission("profile.read")?;
    let emails = identity_advanced::list_user_emails(&state.db, user.user_id).await?;
    Ok(Json(emails))
}

pub async fn client_add_email(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<AddUserEmailRequest>,
) -> Result<(StatusCode, Json<UserEmailView>), AppError> {
    user.require_permission("profile.write")?;
    let (view, otp) =
        identity_advanced::add_user_email(&state.db, &state.redis, user.user_id, &body.email)
            .await?;
    let brand = EmailBrand::from_environment_branding(&serde_json::json!({}), "Signet");
    email::send_branded_otp_email(
        &state.config,
        &brand,
        &view.email,
        &view.email,
        &otp,
        identity_advanced::SECONDARY_EMAIL_OTP_TTL_SECS,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn client_verify_email_address(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Json(body): Json<VerifyUserEmailRequest>,
) -> Result<Json<UserEmailView>, AppError> {
    user.require_permission("profile.write")?;
    let verified = identity_advanced::verify_user_email(
        &state.db,
        &state.redis,
        user.tenant.environment_id,
        user.user_id,
        &body.email,
        &body.code,
    )
    .await?;
    Ok(Json(verified))
}

pub async fn client_set_primary_email(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(email_id): Path<Uuid>,
) -> Result<Json<UserEmailView>, AppError> {
    user.require_permission("profile.write")?;
    let primary = identity_advanced::set_primary_email(&state.db, user.user_id, email_id).await?;
    Ok(Json(primary))
}

pub async fn client_remove_email(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(email_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    user.require_permission("profile.write")?;
    identity_advanced::remove_user_email(&state.db, user.user_id, email_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_list_oauth_accounts(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<Vec<LinkedOAuthAccountView>>, AppError> {
    user.require_permission("profile.read")?;
    let accounts = identity_advanced::list_oauth_accounts(&state.db, user.user_id).await?;
    Ok(Json(accounts))
}

pub async fn client_unlink_oauth_account(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
    Path(provider): Path<String>,
) -> Result<StatusCode, AppError> {
    user.require_permission("profile.write")?;
    identity_advanced::unlink_oauth_account(&state.db, user.user_id, &provider).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn client_export_user_data(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<IdentityDataExportResponse>, AppError> {
    user.require_permission("profile.read")?;
    let export = identity_advanced::export_user_data(&state.db, user.user_id).await?;
    Ok(Json(export))
}

pub async fn client_delete_account(
    State(state): State<AppState>,
    user: AuthenticatedClientUser,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_permission("profile.write")?;
    let notified_environments =
        identity_advanced::delete_user_account(&state.db, user.user_id).await?;
    Ok(Json(json!({
        "deleted": true,
        "notified_environments": notified_environments
    })))
}

pub async fn backend_list_organizations(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Query(query): Query<ListOrganizationsQuery>,
) -> Result<Json<PaginatedOrganizationsResponse>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let page = organizations::list_organizations_paginated(
        &state.db,
        auth.tenant.environment_id,
        query.updated_since,
        query.cursor.as_deref(),
        query.q.as_deref(),
        query.limit,
    )
    .await?;
    Ok(Json(page))
}

pub async fn backend_create_organization(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Json(body): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<OrganizationView>), AppError> {
    auth.require_scope("organizations:write")?;
    let org = organizations::create_organization(
        &state.db,
        auth.tenant.environment_id,
        body.creator_app_user_id,
        "secret_key",
        None,
        CreateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn backend_get_organization(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<OrganizationView>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let org =
        organizations::get_organization(&state.db, auth.tenant.environment_id, org_id).await?;
    Ok(Json(org))
}

pub async fn backend_update_organization(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
    Json(body): Json<UpdateOrgRequest>,
) -> Result<Json<OrganizationView>, AppError> {
    auth.require_scope("organizations:write")?;
    let org = organizations::update_organization(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        "secret_key",
        None,
        UpdateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            status: body.status,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok(Json(org))
}

pub async fn backend_delete_organization(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
) -> Result<StatusCode, AppError> {
    auth.require_scope("organizations:write")?;
    organizations::delete_organization(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        "secret_key",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn backend_list_org_members(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<Vec<OrganizationMemberView>>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let members =
        organizations::list_organization_members(&state.db, auth.tenant.environment_id, org_id)
            .await?;
    Ok(Json(members))
}

pub async fn backend_add_org_member(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
    Json(body): Json<AddOrgMemberRequest>,
) -> Result<(StatusCode, Json<OrganizationMemberView>), AppError> {
    auth.require_scope("organizations:write")?;
    let role = body.role_slug.as_deref().unwrap_or("org:member");
    let member = organizations::add_organization_member(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        body.application_user_id,
        role,
        "secret_key",
        None,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(member)))
}

pub async fn backend_update_org_member_role(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path((org_id, member_id)): Path<(OrganizationId, Uuid)>,
    Json(body): Json<UpdateOrgMemberRoleRequest>,
) -> Result<Json<OrganizationMemberView>, AppError> {
    auth.require_scope("organizations:write")?;
    let member = organizations::update_organization_member_role(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        member_id,
        &body.role_slug,
        "secret_key",
        None,
    )
    .await?;
    Ok(Json(member))
}

pub async fn backend_remove_org_member(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path((org_id, member_id)): Path<(OrganizationId, Uuid)>,
) -> Result<StatusCode, AppError> {
    auth.require_scope("organizations:write")?;
    organizations::remove_organization_member(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        member_id,
        "secret_key",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn backend_list_org_invitations(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
) -> Result<Json<Vec<OrganizationInvitationView>>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let list =
        organizations::list_invitations(&state.db, auth.tenant.environment_id, org_id).await?;
    Ok(Json(list))
}

pub async fn backend_create_org_invitation(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path(org_id): Path<OrganizationId>,
    Json(body): Json<CreateOrgInvitationRequest>,
) -> Result<(StatusCode, Json<CreatedInvitationResponse>), AppError> {
    auth.require_scope("organizations:write")?;
    let created = organizations::create_invitation(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        CreateInvitationInput {
            email: &body.email,
            role_slug: body.role_slug.as_deref(),
            expires_in_days: body.expires_in_days,
            invited_by_app_user_id: None,
            actor_type: "secret_key",
            actor_id: None,
        },
    )
    .await?;
    if let Ok(org) =
        organizations::get_organization(&state.db, auth.tenant.environment_id, org_id).await
    {
        let brand = EmailBrand::from_environment_branding(&serde_json::json!({}), "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    if let Ok(org) =
        organizations::get_organization(&state.db, auth.tenant.environment_id, org_id).await
    {
        let brand = EmailBrand::from_environment_branding(&serde_json::json!({}), "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn backend_revoke_org_invitation(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
    Path((org_id, invitation_id)): Path<(OrganizationId, Uuid)>,
) -> Result<StatusCode, AppError> {
    auth.require_scope("organizations:write")?;
    organizations::revoke_invitation(
        &state.db,
        auth.tenant.environment_id,
        org_id,
        invitation_id,
        "secret_key",
        None,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn backend_list_permissions(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
) -> Result<Json<Vec<EnvironmentPermissionView>>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let list =
        organizations::list_environment_permissions(&state.db, auth.tenant.environment_id).await?;
    Ok(Json(list))
}

pub async fn backend_list_roles(
    State(state): State<AppState>,
    auth: SecretKeyAuth,
) -> Result<Json<Vec<EnvironmentRoleView>>, AppError> {
    if auth.require_scope("sync:read").is_err() {
        auth.require_scope("organizations:read")?;
    }
    let list = organizations::list_environment_roles(&state.db, auth.tenant.environment_id).await?;
    Ok(Json(list))
}

pub async fn control_list_organizations(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Query(query): Query<ListOrganizationsQuery>,
) -> Result<Json<PaginatedOrganizationsResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let page = organizations::list_organizations_paginated(
        &state.db,
        env.id,
        query.updated_since,
        query.cursor.as_deref(),
        query.q.as_deref(),
        query.limit,
    )
    .await?;
    Ok(Json(page))
}

pub async fn control_create_organization(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreateOrgRequest>,
) -> Result<(StatusCode, Json<OrganizationView>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let org = organizations::create_organization(
        &state.db,
        env.id,
        body.creator_app_user_id,
        "developer",
        Some(dev.developer_id.into_inner()),
        CreateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn control_get_organization(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
) -> Result<Json<OrganizationView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let org = organizations::get_organization(&state.db, env.id, org_id).await?;
    Ok(Json(org))
}

pub async fn control_update_organization(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
    Json(body): Json<UpdateOrgRequest>,
) -> Result<Json<OrganizationView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let org = organizations::update_organization(
        &state.db,
        env.id,
        org_id,
        "developer",
        Some(dev.developer_id.into_inner()),
        UpdateOrganizationInput {
            name: body.name,
            slug: body.slug,
            logo_url: body.logo_url,
            status: body.status,
            metadata: body.metadata,
        },
    )
    .await?;
    Ok(Json(org))
}

pub async fn control_delete_organization(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    organizations::delete_organization(
        &state.db,
        env.id,
        org_id,
        "developer",
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn control_list_org_members(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
) -> Result<Json<Vec<OrganizationMemberView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let members = organizations::list_organization_members(&state.db, env.id, org_id).await?;
    Ok(Json(members))
}

pub async fn control_add_org_member(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
    Json(body): Json<AddOrgMemberRequest>,
) -> Result<(StatusCode, Json<OrganizationMemberView>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let role = body.role_slug.as_deref().unwrap_or("org:member");
    let member = organizations::add_organization_member(
        &state.db,
        env.id,
        org_id,
        body.application_user_id,
        role,
        "developer",
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(member)))
}

pub async fn control_update_org_member_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id, member_id)): Path<(
        ApplicationId,
        String,
        OrganizationId,
        Uuid,
    )>,
    Json(body): Json<UpdateOrgMemberRoleRequest>,
) -> Result<Json<OrganizationMemberView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let member = organizations::update_organization_member_role(
        &state.db,
        env.id,
        org_id,
        member_id,
        &body.role_slug,
        "developer",
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(Json(member))
}

pub async fn control_remove_org_member(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id, member_id)): Path<(
        ApplicationId,
        String,
        OrganizationId,
        Uuid,
    )>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    organizations::remove_organization_member(
        &state.db,
        env.id,
        org_id,
        member_id,
        "developer",
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn control_list_org_invitations(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
) -> Result<Json<Vec<OrganizationInvitationView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let invitations = organizations::list_invitations(&state.db, env.id, org_id).await?;
    Ok(Json(invitations))
}

pub async fn control_create_org_invitation(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id)): Path<(ApplicationId, String, OrganizationId)>,
    Json(body): Json<CreateOrgInvitationRequest>,
) -> Result<(StatusCode, Json<CreatedInvitationResponse>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let created = organizations::create_invitation(
        &state.db,
        env.id,
        org_id,
        CreateInvitationInput {
            email: &body.email,
            role_slug: body.role_slug.as_deref(),
            expires_in_days: body.expires_in_days,
            invited_by_app_user_id: None,
            actor_type: "developer",
            actor_id: Some(dev.developer_id.into_inner()),
        },
    )
    .await?;
    if let Ok(org) = organizations::get_organization(&state.db, env.id, org_id).await {
        let brand = EmailBrand::from_environment_branding(&env.branding, "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    if let Ok(org) = organizations::get_organization(&state.db, env.id, org_id).await {
        let brand = EmailBrand::from_environment_branding(&env.branding, "Signet");
        let _ = email::send_organization_invitation_email(
            &state.config,
            &brand,
            &created.invitation.email,
            &org.name,
            created
                .invitation
                .role_slug
                .as_deref()
                .unwrap_or("org:member"),
            &created.invitation_token,
        )
        .await;
    }
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn control_revoke_org_invitation(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, org_id, invitation_id)): Path<(
        ApplicationId,
        String,
        OrganizationId,
        Uuid,
    )>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    organizations::revoke_invitation(
        &state.db,
        env.id,
        org_id,
        invitation_id,
        "developer",
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn control_list_permissions(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<EnvironmentPermissionView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let list = organizations::list_environment_permissions(&state.db, env.id).await?;
    Ok(Json(list))
}

pub async fn control_create_permission(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreatePermissionRequest>,
) -> Result<(StatusCode, Json<EnvironmentPermissionView>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let created = organizations::create_environment_permission(
        &state.db,
        env.id,
        &body.slug,
        body.description.as_deref(),
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn control_delete_permission(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, permission_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    organizations::delete_environment_permission(
        &state.db,
        env.id,
        permission_id,
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn control_list_roles(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<EnvironmentRoleView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Viewer,
    )
    .await?;
    let list = organizations::list_environment_roles(&state.db, env.id).await?;
    Ok(Json(list))
}

pub async fn control_create_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<EnvironmentRoleView>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let created = organizations::create_environment_role(
        &state.db,
        env.id,
        CreateEnvironmentRoleInput {
            slug: &body.slug,
            name: &body.name,
            scope: &body.scope,
            description: body.description.as_deref(),
            permission_slugs: &body.permissions,
            actor_id: Some(dev.developer_id.into_inner()),
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn control_update_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, role_id)): Path<(ApplicationId, String, Uuid)>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<Json<EnvironmentRoleView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    let updated = organizations::update_environment_role(
        &state.db,
        env.id,
        role_id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.permissions.as_deref(),
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(Json(updated))
}

pub async fn control_delete_role(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, role_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let env = control_plane::get_environment_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        WorkspaceRole::Developer,
    )
    .await?;
    organizations::delete_environment_role(
        &state.db,
        env.id,
        role_id,
        Some(dev.developer_id.into_inner()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
