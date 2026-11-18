use std::collections::HashMap;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    AggregateType, AppSessionId, AppUserId, DomainEventEnvelope, DomainEventType, EnvironmentId,
    OrganizationId, UserId,
};
use crate::repositories::db::{begin_tx, commit_tx, PgTx};
use crate::repositories::{outbox, tenant_memberships, tenant_rbac};
use crate::services::tokens::{issue_tenant_access_token_with_org, IssueAccessTokenInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationView {
    pub id: OrganizationId,
    pub environment_id: EnvironmentId,
    pub slug: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub version: i64,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOrganizationMembershipView {
    pub organization: OrganizationView,
    pub member_id: Uuid,
    pub roles: Vec<String>,
    pub status: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedOrganizationsResponse {
    pub data: Vec<OrganizationView>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMemberView {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    pub application_user_id: AppUserId,
    pub sub: UserId,
    pub email: String,
    pub display_name: String,
    pub status: String,
    pub roles: Vec<String>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInvitationView {
    pub id: Uuid,
    pub organization_id: OrganizationId,
    pub email: String,
    pub role_slug: Option<String>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedInvitationResponse {
    pub invitation: OrganizationInvitationView,
    pub invitation_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOrganizationSwitchResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub active_organization_id: Option<OrganizationId>,
    pub org_slug: Option<String>,
    pub org_role: Option<String>,
    pub org_permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentPermissionView {
    pub id: Uuid,
    pub slug: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentRoleView {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub scope: String,
    pub description: String,
    pub is_system: bool,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrganizationInput {
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateOrganizationInput {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub logo_url: Option<Option<String>>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub struct CreateInvitationInput<'a> {
    pub email: &'a str,
    pub role_slug: Option<&'a str>,
    pub expires_in_days: Option<i64>,
    pub invited_by_app_user_id: Option<AppUserId>,
    pub actor_type: &'a str,
    pub actor_id: Option<Uuid>,
}

pub struct SwitchActiveOrganizationInput<'a> {
    pub environment_id: EnvironmentId,
    pub authorized_party: &'a str,
    pub user_id: UserId,
    pub app_user_id: AppUserId,
    pub session_id: AppSessionId,
    pub target_org_id: Option<OrganizationId>,
}

pub struct CreateEnvironmentRoleInput<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub scope: &'a str,
    pub description: Option<&'a str>,
    pub permission_slugs: &'a [String],
    pub actor_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrgSyncCursor {
    updated_at: DateTime<Utc>,
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct OrgRow {
    id: Uuid,
    environment_id: Uuid,
    slug: String,
    name: String,
    logo_url: Option<String>,
    status: String,
    metadata: serde_json::Value,
    version: i64,
    member_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrgRow> for OrganizationView {
    fn from(r: OrgRow) -> Self {
        Self {
            id: OrganizationId::from(r.id),
            environment_id: EnvironmentId::from(r.environment_id),
            slug: r.slug,
            name: r.name,
            logo_url: r.logo_url,
            status: r.status,
            metadata: r.metadata,
            version: r.version,
            member_count: r.member_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct OrgMemberRow {
    id: Uuid,
    organization_id: Uuid,
    application_user_id: Uuid,
    sub: Uuid,
    email: String,
    display_name: String,
    status: String,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct MemberRolePairRow {
    organization_member_id: Uuid,
    slug: String,
}

#[derive(Debug, FromRow)]
struct InvitationRow {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    role_slug: Option<String>,
    status: String,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct PermissionRow {
    id: Uuid,
    slug: String,
    description: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RoleRow {
    id: Uuid,
    slug: String,
    name: String,
    scope: String,
    description: String,
    is_system: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct RolePermPairRow {
    role_id: Uuid,
    slug: String,
}

pub fn validate_org_slug(raw: &str) -> Result<String, AppError> {
    let clean = raw.trim().to_lowercase();
    if clean.len() < 2 || clean.len() > 80 {
        return Err(AppError::Validation(
            "Organization slug must be between 2 and 80 characters".to_string(),
        ));
    }
    let valid = clean
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !clean.starts_with('-')
        && !clean.ends_with('-');
    if !valid {
        return Err(AppError::Validation(
            "Organization slug may only contain lowercase letters, digits, and internal hyphens"
                .to_string(),
        ));
    }
    Ok(clean)
}

pub fn validate_permission_slug(raw: &str) -> Result<String, AppError> {
    let clean = raw.trim().to_lowercase();
    if clean.len() < 3 || clean.len() > 100 {
        return Err(AppError::Validation(
            "Permission slug must be between 3 and 100 characters".to_string(),
        ));
    }
    let valid = clean.chars().all(|c| {
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':' || c == '.' || c == '_' || c == '-'
    });
    if !valid || !clean.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return Err(AppError::Validation(
            "Permission slug must start with a letter and use lowercase alphanumeric, :, ., _, -"
                .to_string(),
        ));
    }
    Ok(clean)
}

pub fn generate_invitation_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("orginv_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub fn hash_invitation_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn evaluate_scoped_permission(
    app_permissions: &[String],
    org_permissions: &[String],
    active_org_id: Option<&str>,
    target_org_id: Option<&str>,
    required_permission: &str,
) -> bool {
    if app_permissions.iter().any(|p| p == required_permission) {
        return true;
    }
    match (active_org_id, target_org_id) {
        (Some(active), Some(target)) if active == target => {
            org_permissions.iter().any(|p| p == required_permission)
        }
        (Some(_), None) => org_permissions.iter().any(|p| p == required_permission),
        _ => false,
    }
}

pub fn ensure_not_last_owner(
    is_target_owner: bool,
    will_remain_owner: bool,
    total_active_owners: i64,
) -> Result<(), AppError> {
    if is_target_owner && !will_remain_owner && total_active_owners <= 1 {
        return Err(AppError::Validation(
            "Organization must retain at least one active owner".to_string(),
        ));
    }
    Ok(())
}

fn encode_org_cursor(updated_at: DateTime<Utc>, id: Uuid) -> String {
    let payload = OrgSyncCursor { updated_at, id };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap_or_default())
}

fn decode_org_cursor(cursor: &str) -> Result<OrgSyncCursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.trim())
        .map_err(|_| AppError::Validation("Invalid pagination cursor".to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Validation("Malformed pagination cursor".to_string()))
}

async fn count_active_org_owners_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
) -> Result<i64, AppError> {
    let (count,): (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT om.id)::bigint
        FROM organization_members om
        JOIN organization_member_roles omr
          ON omr.environment_id = om.environment_id AND omr.organization_member_id = om.id
        JOIN environment_roles er
          ON er.environment_id = omr.environment_id AND er.id = omr.role_id
        WHERE om.environment_id = $1
          AND om.organization_id = $2
          AND om.status = 'active'
          AND er.slug = 'org:owner'
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

async fn find_role_id_by_slug_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    slug: &str,
) -> Result<Uuid, AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id
        FROM environment_roles
        WHERE environment_id = $1 AND LOWER(slug) = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(slug.trim().to_lowercase())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|(id,)| id)
        .ok_or_else(|| AppError::Validation(format!("Role '{slug}' not found in environment")))
}

async fn record_env_audit_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    actor: (&str, Option<Uuid>),
    event: &str,
    resource: (&str, Option<Uuid>),
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    let (actor_type, actor_id) = actor;
    let (resource_type, resource_id) = resource;
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            environment_id, user_id, actor_type, actor_id,
            resource_type, resource_id, event, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(environment_id)
    .bind(if actor_type == "user" { actor_id } else { None })
    .bind(actor_type)
    .bind(actor_id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(event)
    .bind(metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn require_org_member(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    app_user_id: AppUserId,
) -> Result<(Uuid, Vec<String>), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT om.id
        FROM organization_members om
        JOIN organizations o
          ON o.environment_id = om.environment_id AND o.id = om.organization_id
        WHERE om.environment_id = $1
          AND om.organization_id = $2
          AND om.application_user_id = $3
          AND om.status = 'active'
          AND o.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(app_user_id)
    .fetch_optional(db)
    .await?;

    let (member_id,) = row.ok_or_else(|| {
        AppError::Forbidden("Organization not found or membership required".to_string())
    })?;

    let roles: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT er.slug
        FROM organization_member_roles omr
        JOIN environment_roles er
          ON er.environment_id = omr.environment_id AND er.id = omr.role_id
        WHERE omr.environment_id = $1 AND omr.organization_member_id = $2
        ORDER BY er.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_all(db)
    .await?;

    Ok((member_id, roles))
}

pub async fn require_org_admin_or_owner(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    app_user_id: AppUserId,
) -> Result<(Uuid, Vec<String>), AppError> {
    let (member_id, roles) = require_org_member(db, environment_id, org_id, app_user_id).await?;
    let allowed = roles.iter().any(|r| r == "org:owner" || r == "org:admin");
    if !allowed {
        return Err(AppError::Forbidden(
            "Organization admin or owner role required".to_string(),
        ));
    }
    Ok((member_id, roles))
}

pub async fn create_organization(
    db: &PgPool,
    environment_id: EnvironmentId,
    creator_app_user_id: Option<AppUserId>,
    actor_type: &str,
    actor_id: Option<Uuid>,
    input: CreateOrganizationInput,
) -> Result<OrganizationView, AppError> {
    let clean_name = input.name.trim();
    if clean_name.len() < 2 || clean_name.len() > 150 {
        return Err(AppError::Validation(
            "Organization name must be between 2 and 150 characters".to_string(),
        ));
    }
    let clean_slug = validate_org_slug(&input.slug)?;
    let metadata = input.metadata.unwrap_or_else(|| serde_json::json!({}));

    let mut tx = begin_tx(db).await?;
    tenant_rbac::ensure_default_org_roles_in_tx(&mut tx, environment_id).await?;

    let org_row = sqlx::query_as::<_, OrgRow>(
        r#"
        INSERT INTO organizations (environment_id, slug, name, logo_url, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, environment_id, slug, name, logo_url, status, metadata, version,
                  0::bigint AS member_count, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(&clean_slug)
    .bind(clean_name)
    .bind(
        input
            .logo_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .bind(&metadata)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Validation(format!(
                "Organization slug '{clean_slug}' already exists in this environment"
            ))
        }
        other => AppError::Database(other),
    })?;

    let org_id = OrganizationId::from(org_row.id);
    let mut member_count = 0_i64;

    let created_envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationCreated,
        Some(environment_id.into_inner()),
        AggregateType::Organization,
        org_row.id,
        org_row.version,
        serde_json::json!({
            "organization_id": org_id,
            "slug": org_row.slug,
            "name": org_row.name,
            "status": org_row.status,
            "metadata": org_row.metadata,
            "version": org_row.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &created_envelope).await?;

    if let Some(app_user_id) = creator_app_user_id {
        let membership =
            tenant_memberships::find_application_user_by_id(&mut *tx, environment_id, app_user_id)
                .await?
                .ok_or_else(|| {
                    AppError::Forbidden(
                        "Creator must be an active member of this environment".to_string(),
                    )
                })?;
        if membership.status != "active" {
            return Err(AppError::Forbidden(
                "Suspended users cannot create organizations".to_string(),
            ));
        }

        let (member_id, member_version): (Uuid, i64) = sqlx::query_as(
            r#"
            INSERT INTO organization_members (environment_id, organization_id, application_user_id, status)
            VALUES ($1, $2, $3, 'active')
            RETURNING id, version
            "#,
        )
        .bind(environment_id)
        .bind(org_id)
        .bind(app_user_id)
        .fetch_one(&mut *tx)
        .await?;

        let owner_role_id =
            find_role_id_by_slug_in_tx(&mut tx, environment_id, "org:owner").await?;
        sqlx::query(
            r#"
            INSERT INTO organization_member_roles (environment_id, organization_member_id, role_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(member_id)
        .bind(owner_role_id)
        .execute(&mut *tx)
        .await?;

        let member_envelope = DomainEventEnvelope::new(
            DomainEventType::OrganizationMemberAdded,
            Some(environment_id.into_inner()),
            AggregateType::OrganizationMember,
            member_id,
            member_version,
            serde_json::json!({
                "organization_id": org_id,
                "organization_member_id": member_id,
                "app_user_id": app_user_id,
                "sub": membership.user_id,
                "roles": ["org:owner"],
                "version": member_version
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &member_envelope).await?;
        member_count = 1;
    }

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_CREATED",
        ("organization", Some(org_row.id)),
        serde_json::json!({ "slug": clean_slug, "name": clean_name }),
    )
    .await?;

    commit_tx(tx).await?;
    let mut view = OrganizationView::from(org_row);
    view.member_count = member_count;
    Ok(view)
}

pub async fn list_user_organizations(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<Vec<UserOrganizationMembershipView>, AppError> {
    #[derive(FromRow)]
    struct Row {
        member_id: Uuid,
        member_status: String,
        joined_at: DateTime<Utc>,
        id: Uuid,
        environment_id: Uuid,
        slug: String,
        name: String,
        logo_url: Option<String>,
        status: String,
        metadata: serde_json::Value,
        version: i64,
        member_count: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            om.id AS member_id,
            om.status AS member_status,
            om.created_at AS joined_at,
            o.id,
            o.environment_id,
            o.slug,
            o.name,
            o.logo_url,
            o.status,
            o.metadata,
            o.version,
            (
                SELECT COUNT(*)::bigint
                FROM organization_members m2
                WHERE m2.environment_id = o.environment_id
                  AND m2.organization_id = o.id
                  AND m2.status = 'active'
            ) AS member_count,
            o.created_at,
            o.updated_at
        FROM organization_members om
        JOIN organizations o
          ON o.environment_id = om.environment_id AND o.id = om.organization_id
        WHERE om.environment_id = $1
          AND om.application_user_id = $2
          AND om.status = 'active'
          AND o.status = 'active'
        ORDER BY o.created_at ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    let member_ids: Vec<Uuid> = rows.iter().map(|r| r.member_id).collect();
    let role_pairs = if member_ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, MemberRolePairRow>(
            r#"
            SELECT omr.organization_member_id, er.slug
            FROM organization_member_roles omr
            JOIN environment_roles er
              ON er.environment_id = omr.environment_id AND er.id = omr.role_id
            WHERE omr.environment_id = $1
              AND omr.organization_member_id = ANY($2)
            ORDER BY er.slug ASC
            "#,
        )
        .bind(environment_id)
        .bind(&member_ids)
        .fetch_all(db)
        .await?
    };

    let mut roles_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for pair in role_pairs {
        roles_map
            .entry(pair.organization_member_id)
            .or_default()
            .push(pair.slug);
    }

    Ok(rows
        .into_iter()
        .map(|r| UserOrganizationMembershipView {
            roles: roles_map.remove(&r.member_id).unwrap_or_default(),
            member_id: r.member_id,
            status: r.member_status,
            joined_at: r.joined_at,
            organization: OrganizationView {
                id: OrganizationId::from(r.id),
                environment_id: EnvironmentId::from(r.environment_id),
                slug: r.slug,
                name: r.name,
                logo_url: r.logo_url,
                status: r.status,
                metadata: r.metadata,
                version: r.version,
                member_count: r.member_count,
                created_at: r.created_at,
                updated_at: r.updated_at,
            },
        })
        .collect())
}

pub async fn list_organizations_paginated(
    db: &PgPool,
    environment_id: EnvironmentId,
    updated_since: Option<DateTime<Utc>>,
    cursor: Option<&str>,
    query: Option<&str>,
    limit: Option<i64>,
) -> Result<PaginatedOrganizationsResponse, AppError> {
    let page_limit = limit.unwrap_or(50).clamp(1, 200);
    let parsed_cursor = match cursor.filter(|c| !c.trim().is_empty()) {
        Some(c) => Some(decode_org_cursor(c)?),
        None => None,
    };
    let q_like = query
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let rows = sqlx::query_as::<_, OrgRow>(
        r#"
        SELECT
            o.id,
            o.environment_id,
            o.slug,
            o.name,
            o.logo_url,
            o.status,
            o.metadata,
            o.version,
            (
                SELECT COUNT(*)::bigint
                FROM organization_members om
                WHERE om.environment_id = o.environment_id
                  AND om.organization_id = o.id
                  AND om.status = 'active'
            ) AS member_count,
            o.created_at,
            o.updated_at
        FROM organizations o
        WHERE o.environment_id = $1
          AND ($2::timestamptz IS NULL OR o.updated_at >= $2)
          AND (
              $3::timestamptz IS NULL
              OR (o.updated_at, o.id) > ($3, $4)
          )
          AND (
              $5::text IS NULL
              OR LOWER(o.name) LIKE $5
              OR LOWER(o.slug) LIKE $5
              OR LOWER(o.id::text) LIKE $5
          )
        ORDER BY o.updated_at ASC, o.id ASC
        LIMIT $6
        "#,
    )
    .bind(environment_id)
    .bind(updated_since)
    .bind(parsed_cursor.as_ref().map(|c| c.updated_at))
    .bind(parsed_cursor.as_ref().map(|c| c.id))
    .bind(&q_like)
    .bind(page_limit + 1)
    .fetch_all(db)
    .await?;

    let has_more = rows.len() as i64 > page_limit;
    let mut items: Vec<OrganizationView> = rows
        .into_iter()
        .take(page_limit as usize)
        .map(Into::into)
        .collect();
    let next_cursor = if has_more {
        items
            .last()
            .map(|last| encode_org_cursor(last.updated_at, last.id.into_inner()))
    } else {
        None
    };

    Ok(PaginatedOrganizationsResponse {
        data: std::mem::take(&mut items),
        next_cursor,
        has_more,
    })
}

pub async fn get_organization(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
) -> Result<OrganizationView, AppError> {
    let row = sqlx::query_as::<_, OrgRow>(
        r#"
        SELECT
            o.id,
            o.environment_id,
            o.slug,
            o.name,
            o.logo_url,
            o.status,
            o.metadata,
            o.version,
            (
                SELECT COUNT(*)::bigint
                FROM organization_members om
                WHERE om.environment_id = o.environment_id
                  AND om.organization_id = o.id
                  AND om.status = 'active'
            ) AS member_count,
            o.created_at,
            o.updated_at
        FROM organizations o
        WHERE o.environment_id = $1 AND o.id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    Ok(row.into())
}

pub async fn update_organization(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    actor_type: &str,
    actor_id: Option<Uuid>,
    input: UpdateOrganizationInput,
) -> Result<OrganizationView, AppError> {
    let existing = get_organization(db, environment_id, org_id).await?;
    let next_name = match input.name {
        Some(n) => {
            let trimmed = n.trim();
            if trimmed.len() < 2 || trimmed.len() > 150 {
                return Err(AppError::Validation(
                    "Organization name must be between 2 and 150 characters".to_string(),
                ));
            }
            trimmed.to_string()
        }
        None => existing.name,
    };
    let next_slug = match input.slug {
        Some(s) => validate_org_slug(&s)?,
        None => existing.slug,
    };
    let next_logo = match input.logo_url {
        Some(v) => v.and_then(|s| {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }),
        None => existing.logo_url,
    };
    let next_status = match input.status {
        Some(s) => match s.trim().to_lowercase().as_str() {
            "active" | "suspended" => s.trim().to_lowercase(),
            other => {
                return Err(AppError::Validation(format!(
                    "Invalid organization status '{other}'"
                )));
            }
        },
        None => existing.status,
    };
    let next_meta = input.metadata.unwrap_or(existing.metadata);

    let mut tx = begin_tx(db).await?;
    let updated = sqlx::query_as::<_, OrgRow>(
        r#"
        UPDATE organizations
        SET name = $3, slug = $4, logo_url = $5, status = $6, metadata = $7, updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, slug, name, logo_url, status, metadata, version,
                  (
                      SELECT COUNT(*)::bigint
                      FROM organization_members om
                      WHERE om.environment_id = organizations.environment_id
                        AND om.organization_id = organizations.id
                        AND om.status = 'active'
                  ) AS member_count,
                  created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(&next_name)
    .bind(&next_slug)
    .bind(&next_logo)
    .bind(&next_status)
    .bind(&next_meta)
    .fetch_one(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationUpdated,
        Some(environment_id.into_inner()),
        AggregateType::Organization,
        updated.id,
        updated.version,
        serde_json::json!({
            "organization_id": org_id,
            "slug": updated.slug,
            "name": updated.name,
            "status": updated.status,
            "metadata": updated.metadata,
            "version": updated.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_UPDATED",
        ("organization", Some(updated.id)),
        serde_json::json!({ "slug": updated.slug, "status": updated.status }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(updated.into())
}

pub async fn delete_organization(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    actor_type: &str,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let deleted: Option<(Uuid, i64, String)> = sqlx::query_as(
        r#"
        UPDATE organizations
        SET status = 'deleted', updated_at = NOW()
        WHERE environment_id = $1 AND id = $2 AND status <> 'deleted'
        RETURNING id, version, slug
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((id, version, slug)) = deleted else {
        return Err(AppError::NotFound("Organization not found".to_string()));
    };

    sqlx::query(
        r#"
        UPDATE app_sessions
        SET active_organization_id = NULL, updated_at = NOW()
        WHERE environment_id = $1 AND active_organization_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationDeleted,
        Some(environment_id.into_inner()),
        AggregateType::Organization,
        id,
        version,
        serde_json::json!({
            "organization_id": org_id,
            "slug": slug,
            "version": version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_DELETED",
        ("organization", Some(id)),
        serde_json::json!({ "slug": slug }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn list_organization_members(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
) -> Result<Vec<OrganizationMemberView>, AppError> {
    let rows = sqlx::query_as::<_, OrgMemberRow>(
        r#"
        SELECT
            om.id,
            om.organization_id,
            om.application_user_id,
            u.id AS sub,
            u.email,
            u.display_name,
            om.status,
            om.version,
            om.created_at,
            om.updated_at
        FROM organization_members om
        JOIN application_users au
          ON au.environment_id = om.environment_id AND au.id = om.application_user_id
        JOIN users u ON u.id = au.user_id
        WHERE om.environment_id = $1
          AND om.organization_id = $2
          AND om.status <> 'removed'
        ORDER BY om.created_at ASC
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .fetch_all(db)
    .await?;

    let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let role_pairs = if ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, MemberRolePairRow>(
            r#"
            SELECT omr.organization_member_id, er.slug
            FROM organization_member_roles omr
            JOIN environment_roles er
              ON er.environment_id = omr.environment_id AND er.id = omr.role_id
            WHERE omr.environment_id = $1
              AND omr.organization_member_id = ANY($2)
            ORDER BY er.slug ASC
            "#,
        )
        .bind(environment_id)
        .bind(&ids)
        .fetch_all(db)
        .await?
    };

    let mut roles_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for pair in role_pairs {
        roles_map
            .entry(pair.organization_member_id)
            .or_default()
            .push(pair.slug);
    }

    Ok(rows
        .into_iter()
        .map(|r| OrganizationMemberView {
            roles: roles_map.remove(&r.id).unwrap_or_default(),
            id: r.id,
            organization_id: OrganizationId::from(r.organization_id),
            application_user_id: AppUserId::from(r.application_user_id),
            sub: UserId::from(r.sub),
            email: r.email,
            display_name: r.display_name,
            status: r.status,
            version: r.version,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

pub async fn add_organization_member(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    app_user_id: AppUserId,
    role_slug: &str,
    actor_type: &str,
    actor_id: Option<Uuid>,
) -> Result<OrganizationMemberView, AppError> {
    let mut tx = begin_tx(db).await?;
    tenant_rbac::ensure_default_org_roles_in_tx(&mut tx, environment_id).await?;

    let membership =
        tenant_memberships::find_application_user_by_id(&mut *tx, environment_id, app_user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

    let role_id = find_role_id_by_slug_in_tx(&mut tx, environment_id, role_slug).await?;
    let (member_id, member_version): (Uuid, i64) = sqlx::query_as(
        r#"
        INSERT INTO organization_members (environment_id, organization_id, application_user_id, status)
        VALUES ($1, $2, $3, 'active')
        ON CONFLICT (environment_id, organization_id, application_user_id) DO UPDATE
        SET status = 'active', updated_at = NOW()
        RETURNING id, version
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(app_user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO organization_member_roles (environment_id, organization_member_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .bind(role_id)
    .execute(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationMemberAdded,
        Some(environment_id.into_inner()),
        AggregateType::OrganizationMember,
        member_id,
        member_version,
        serde_json::json!({
            "organization_id": org_id,
            "organization_member_id": member_id,
            "app_user_id": app_user_id,
            "sub": membership.user_id,
            "roles": [role_slug.trim().to_lowercase()],
            "version": member_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_MEMBER_ADDED",
        ("organization_member", Some(member_id)),
        serde_json::json!({ "organization_id": org_id, "role": role_slug }),
    )
    .await?;

    commit_tx(tx).await?;
    let members = list_organization_members(db, environment_id, org_id).await?;
    members
        .into_iter()
        .find(|m| m.id == member_id)
        .ok_or_else(|| AppError::Internal("Added member not found".to_string()))
}

pub async fn update_organization_member_role(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    member_id: Uuid,
    new_role_slug: &str,
    actor_type: &str,
    actor_id: Option<Uuid>,
) -> Result<OrganizationMemberView, AppError> {
    let clean_role = new_role_slug.trim().to_lowercase();
    let mut tx = begin_tx(db).await?;
    tenant_rbac::ensure_default_org_roles_in_tx(&mut tx, environment_id).await?;

    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT id, application_user_id
        FROM organization_members
        WHERE environment_id = $1 AND organization_id = $2 AND id = $3 AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(member_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((_, app_user_uuid)) = existing else {
        return Err(AppError::NotFound(
            "Organization member not found".to_string(),
        ));
    };

    let current_roles: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT er.slug
        FROM organization_member_roles omr
        JOIN environment_roles er
          ON er.environment_id = omr.environment_id AND er.id = omr.role_id
        WHERE omr.environment_id = $1 AND omr.organization_member_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_all(&mut *tx)
    .await?;

    let is_owner = current_roles.iter().any(|r| r == "org:owner");
    let will_be_owner = clean_role == "org:owner";
    if is_owner && !will_be_owner {
        let total_owners = count_active_org_owners_in_tx(&mut tx, environment_id, org_id).await?;
        ensure_not_last_owner(true, false, total_owners)?;
    }

    let next_role_id = find_role_id_by_slug_in_tx(&mut tx, environment_id, &clean_role).await?;
    sqlx::query(
        r#"
        DELETE FROM organization_member_roles
        WHERE environment_id = $1 AND organization_member_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO organization_member_roles (environment_id, organization_member_id, role_id)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .bind(next_role_id)
    .execute(&mut *tx)
    .await?;

    let (next_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE organization_members
        SET updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING version
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_one(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationMemberUpdated,
        Some(environment_id.into_inner()),
        AggregateType::OrganizationMember,
        member_id,
        next_version,
        serde_json::json!({
            "organization_id": org_id,
            "organization_member_id": member_id,
            "app_user_id": app_user_uuid,
            "previous_roles": current_roles,
            "roles": [clean_role],
            "version": next_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_MEMBER_ROLE_UPDATED",
        ("organization_member", Some(member_id)),
        serde_json::json!({ "organization_id": org_id, "new_role": clean_role }),
    )
    .await?;

    commit_tx(tx).await?;
    let members = list_organization_members(db, environment_id, org_id).await?;
    members
        .into_iter()
        .find(|m| m.id == member_id)
        .ok_or_else(|| AppError::Internal("Updated member not found".to_string()))
}

pub async fn remove_organization_member(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    member_id: Uuid,
    actor_type: &str,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT id, application_user_id
        FROM organization_members
        WHERE environment_id = $1 AND organization_id = $2 AND id = $3 AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(member_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((_, app_user_uuid)) = existing else {
        return Err(AppError::NotFound(
            "Organization member not found".to_string(),
        ));
    };

    let current_roles: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT er.slug
        FROM organization_member_roles omr
        JOIN environment_roles er
          ON er.environment_id = omr.environment_id AND er.id = omr.role_id
        WHERE omr.environment_id = $1 AND omr.organization_member_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_all(&mut *tx)
    .await?;

    if current_roles.iter().any(|r| r == "org:owner") {
        let total_owners = count_active_org_owners_in_tx(&mut tx, environment_id, org_id).await?;
        ensure_not_last_owner(true, false, total_owners)?;
    }

    let (next_version,): (i64,) = sqlx::query_as(
        r#"
        UPDATE organization_members
        SET status = 'removed', updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING version
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM organization_member_roles
        WHERE environment_id = $1 AND organization_member_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE app_sessions
        SET active_organization_id = NULL, updated_at = NOW()
        WHERE environment_id = $1
          AND application_user_id = $2
          AND active_organization_id = $3
        "#,
    )
    .bind(environment_id)
    .bind(app_user_uuid)
    .bind(org_id)
    .execute(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationMemberRemoved,
        Some(environment_id.into_inner()),
        AggregateType::OrganizationMember,
        member_id,
        next_version,
        serde_json::json!({
            "organization_id": org_id,
            "organization_member_id": member_id,
            "app_user_id": app_user_uuid,
            "version": next_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_MEMBER_REMOVED",
        ("organization_member", Some(member_id)),
        serde_json::json!({ "organization_id": org_id, "app_user_id": app_user_uuid }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn create_invitation(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    input: CreateInvitationInput<'_>,
) -> Result<CreatedInvitationResponse, AppError> {
    let CreateInvitationInput {
        email,
        role_slug,
        expires_in_days,
        invited_by_app_user_id,
        actor_type,
        actor_id,
    } = input;
    let clean_email = email.trim().to_lowercase();
    if !clean_email.contains('@') || clean_email.len() < 5 || clean_email.len() > 320 {
        return Err(AppError::Validation(
            "Valid invitation email address is required".to_string(),
        ));
    }

    let mut tx = begin_tx(db).await?;
    tenant_rbac::ensure_default_org_roles_in_tx(&mut tx, environment_id).await?;

    let target_role_slug = role_slug
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("org:member")
        .to_lowercase();
    let role_id = find_role_id_by_slug_in_tx(&mut tx, environment_id, &target_role_slug).await?;

    let raw_token = generate_invitation_token();
    let token_hash = hash_invitation_token(&raw_token);
    let days = expires_in_days.unwrap_or(7).clamp(1, 30);
    let expires_at = Utc::now() + Duration::days(days);

    let (inv_id, created_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        r#"
        INSERT INTO organization_invitations (
            environment_id, organization_id, email, role_id,
            token_hash, invited_by_app_user_id, status, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7)
        RETURNING id, created_at
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(&clean_email)
    .bind(role_id)
    .bind(&token_hash)
    .bind(invited_by_app_user_id)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_INVITATION_CREATED",
        ("organization_invitation", Some(inv_id)),
        serde_json::json!({
            "organization_id": org_id,
            "email": clean_email,
            "role": target_role_slug
        }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(CreatedInvitationResponse {
        invitation: OrganizationInvitationView {
            id: inv_id,
            organization_id: org_id,
            email: clean_email,
            role_slug: Some(target_role_slug),
            status: "pending".to_string(),
            expires_at,
            accepted_at: None,
            created_at,
        },
        invitation_token: raw_token,
    })
}

pub async fn list_invitations(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
) -> Result<Vec<OrganizationInvitationView>, AppError> {
    let rows = sqlx::query_as::<_, InvitationRow>(
        r#"
        SELECT
            i.id,
            i.organization_id,
            i.email,
            er.slug AS role_slug,
            CASE
                WHEN i.status = 'pending' AND i.expires_at <= NOW() THEN 'expired'
                ELSE i.status
            END AS status,
            i.expires_at,
            i.accepted_at,
            i.created_at
        FROM organization_invitations i
        LEFT JOIN environment_roles er
          ON er.environment_id = i.environment_id AND er.id = i.role_id
        WHERE i.environment_id = $1 AND i.organization_id = $2
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OrganizationInvitationView {
            id: r.id,
            organization_id: OrganizationId::from(r.organization_id),
            email: r.email,
            role_slug: r.role_slug,
            status: r.status,
            expires_at: r.expires_at,
            accepted_at: r.accepted_at,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn revoke_invitation(
    db: &PgPool,
    environment_id: EnvironmentId,
    org_id: OrganizationId,
    invitation_id: Uuid,
    actor_type: &str,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let res = sqlx::query(
        r#"
        UPDATE organization_invitations
        SET status = 'revoked', updated_at = NOW()
        WHERE environment_id = $1
          AND organization_id = $2
          AND id = $3
          AND status = 'pending'
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(invitation_id)
    .execute(&mut *tx)
    .await?;

    if res.rows_affected() == 0 {
        return Err(AppError::Validation(
            "Pending invitation not found".to_string(),
        ));
    }

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        (actor_type, actor_id),
        "ORGANIZATION_INVITATION_REVOKED",
        ("organization_invitation", Some(invitation_id)),
        serde_json::json!({ "organization_id": org_id }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn accept_invitation(
    db: &PgPool,
    environment_id: EnvironmentId,
    accepting_user_id: UserId,
    raw_token: &str,
) -> Result<OrganizationMemberView, AppError> {
    let token_hash = hash_invitation_token(raw_token.trim());
    let mut tx = begin_tx(db).await?;
    tenant_rbac::ensure_default_org_roles_in_tx(&mut tx, environment_id).await?;

    #[derive(FromRow)]
    struct InvRecord {
        id: Uuid,
        organization_id: Uuid,
        email: String,
        role_id: Option<Uuid>,
        status: String,
        expires_at: DateTime<Utc>,
    }

    let inv = sqlx::query_as::<_, InvRecord>(
        r#"
        SELECT id, organization_id, email, role_id, status, expires_at
        FROM organization_invitations
        WHERE environment_id = $1 AND token_hash = $2
        FOR UPDATE
        "#,
    )
    .bind(environment_id)
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::Validation("Invitation token is invalid".to_string()))?;

    if inv.status != "pending" {
        return Err(AppError::Validation(format!(
            "Invitation cannot be accepted because its status is '{}'",
            inv.status
        )));
    }
    if inv.expires_at <= Utc::now() {
        sqlx::query(
            r#"
            UPDATE organization_invitations
            SET status = 'expired', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(inv.id)
        .execute(&mut *tx)
        .await?;
        commit_tx(tx).await?;
        return Err(AppError::Validation("Invitation has expired".to_string()));
    }

    let (has_verified_match,): (bool,) = sqlx::query_as(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_emails
            WHERE user_id = $1
              AND LOWER(email) = LOWER($2)
              AND verified_at IS NOT NULL
        )
        "#,
    )
    .bind(accepting_user_id)
    .bind(&inv.email)
    .fetch_one(&mut *tx)
    .await?;

    if !has_verified_match {
        return Err(AppError::Forbidden(
            "Invitation can only be accepted by a user with a matching verified email address"
                .to_string(),
        ));
    }

    let app_user = match tenant_memberships::find_application_user_by_user_id(
        &mut *tx,
        environment_id,
        accepting_user_id,
    )
    .await?
    {
        Some(existing) => existing,
        None => {
            tenant_memberships::create_application_user(
                &mut *tx,
                environment_id,
                accepting_user_id,
                None,
            )
            .await?
        }
    };

    let org_id = OrganizationId::from(inv.organization_id);
    let (member_id, member_version): (Uuid, i64) = sqlx::query_as(
        r#"
        INSERT INTO organization_members (environment_id, organization_id, application_user_id, status)
        VALUES ($1, $2, $3, 'active')
        ON CONFLICT (environment_id, organization_id, application_user_id) DO UPDATE
        SET status = 'active', updated_at = NOW()
        RETURNING id, version
        "#,
    )
    .bind(environment_id)
    .bind(org_id)
    .bind(app_user.id)
    .fetch_one(&mut *tx)
    .await?;

    let role_id = match inv.role_id {
        Some(rid) => rid,
        None => find_role_id_by_slug_in_tx(&mut tx, environment_id, "org:member").await?,
    };
    sqlx::query(
        r#"
        INSERT INTO organization_member_roles (environment_id, organization_member_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .bind(role_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE organization_invitations
        SET status = 'accepted', accepted_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(inv.id)
    .execute(&mut *tx)
    .await?;

    let role_slug: Option<String> = sqlx::query_scalar(
        r#"
        SELECT slug FROM environment_roles WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(role_id)
    .fetch_optional(&mut *tx)
    .await?;

    let envelope = DomainEventEnvelope::new(
        DomainEventType::OrganizationMemberAdded,
        Some(environment_id.into_inner()),
        AggregateType::OrganizationMember,
        member_id,
        member_version,
        serde_json::json!({
            "organization_id": org_id,
            "organization_member_id": member_id,
            "app_user_id": app_user.id,
            "sub": accepting_user_id,
            "roles": [role_slug.clone().unwrap_or_else(|| "org:member".to_string())],
            "invitation_id": inv.id,
            "version": member_version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &envelope).await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("user", Some(accepting_user_id.into_inner())),
        "ORGANIZATION_INVITATION_ACCEPTED",
        ("organization_invitation", Some(inv.id)),
        serde_json::json!({ "organization_id": org_id, "member_id": member_id }),
    )
    .await?;

    commit_tx(tx).await?;
    let members = list_organization_members(db, environment_id, org_id).await?;
    members
        .into_iter()
        .find(|m| m.id == member_id)
        .ok_or_else(|| AppError::Internal("Accepted organization member not found".to_string()))
}

pub async fn switch_active_organization(
    db: &PgPool,
    config: &AppConfig,
    input: SwitchActiveOrganizationInput<'_>,
) -> Result<ActiveOrganizationSwitchResponse, AppError> {
    let SwitchActiveOrganizationInput {
        environment_id,
        authorized_party,
        user_id,
        app_user_id,
        session_id,
        target_org_id,
    } = input;
    let mut tx = begin_tx(db).await?;

    let membership =
        tenant_memberships::find_application_user_by_id(&mut *tx, environment_id, app_user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

    let org_ctx = match target_org_id {
        Some(org_id) => {
            let resolved = tenant_rbac::resolve_organization_context_in_tx(
                &mut tx,
                environment_id,
                app_user_id,
                org_id,
            )
            .await?;
            if resolved.is_none() {
                return Err(AppError::Forbidden(
                    "Active membership in target organization is required".to_string(),
                ));
            }
            resolved
        }
        None => None,
    };

    let updated_rows = sqlx::query(
        r#"
        UPDATE app_sessions
        SET active_organization_id = $3, last_used_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1
          AND id = $2
          AND application_user_id = $4
          AND revoked_at IS NULL
          AND expires_at > NOW()
        "#,
    )
    .bind(environment_id)
    .bind(session_id)
    .bind(target_org_id)
    .bind(app_user_id)
    .execute(&mut *tx)
    .await?;

    if updated_rows.rows_affected() == 0 {
        return Err(AppError::Unauthorized(
            "Session is expired or revoked".to_string(),
        ));
    }

    let (roles, permissions) = tenant_rbac::resolve_environment_roles_and_permissions_in_tx(
        &mut tx,
        environment_id,
        app_user_id,
    )
    .await?;

    let (ttl_secs,): (i32,) = sqlx::query_as(
        r#"
        SELECT access_token_ttl_secs
        FROM application_environments
        WHERE id = $1
        "#,
    )
    .bind(environment_id)
    .fetch_one(&mut *tx)
    .await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("user", Some(user_id.into_inner())),
        "SESSION_ACTIVE_ORG_SWITCHED",
        ("app_session", Some(session_id.into_inner())),
        serde_json::json!({ "active_organization_id": target_org_id }),
    )
    .await?;

    commit_tx(tx).await?;

    let (access_token, claims) = issue_tenant_access_token_with_org(
        db,
        config,
        IssueAccessTokenInput {
            issuer_url: &config.issuer_url,
            user_id,
            environment_id,
            authorized_party,
            app_user_id,
            session_id,
            active_organization_id: target_org_id,
            roles,
            permissions,
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: membership.version,
            ttl_secs: ttl_secs as i64,
        },
        org_ctx,
    )
    .await?;

    Ok(ActiveOrganizationSwitchResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: ttl_secs as i64,
        active_organization_id: target_org_id,
        org_slug: claims.org_slug,
        org_role: claims.org_role,
        org_permissions: claims.org_permissions,
    })
}

pub async fn list_environment_permissions(
    db: &PgPool,
    environment_id: EnvironmentId,
) -> Result<Vec<EnvironmentPermissionView>, AppError> {
    let rows = sqlx::query_as::<_, PermissionRow>(
        r#"
        SELECT id, slug, description, created_at
        FROM environment_permissions
        WHERE environment_id = $1
        ORDER BY slug ASC
        "#,
    )
    .bind(environment_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| EnvironmentPermissionView {
            id: r.id,
            slug: r.slug,
            description: r.description,
            created_at: r.created_at,
        })
        .collect())
}

pub async fn create_environment_permission(
    db: &PgPool,
    environment_id: EnvironmentId,
    slug: &str,
    description: Option<&str>,
    actor_id: Option<Uuid>,
) -> Result<EnvironmentPermissionView, AppError> {
    let clean_slug = validate_permission_slug(slug)?;
    let desc = description.unwrap_or("").trim();
    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, PermissionRow>(
        r#"
        INSERT INTO environment_permissions (environment_id, slug, description)
        VALUES ($1, $2, $3)
        RETURNING id, slug, description, created_at
        "#,
    )
    .bind(environment_id)
    .bind(&clean_slug)
    .bind(desc)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Validation(format!("Permission '{clean_slug}' already exists"))
        }
        other => AppError::Database(other),
    })?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("developer", actor_id),
        "ENVIRONMENT_PERMISSION_CREATED",
        ("environment_permission", Some(row.id)),
        serde_json::json!({ "slug": clean_slug }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(EnvironmentPermissionView {
        id: row.id,
        slug: row.slug,
        description: row.description,
        created_at: row.created_at,
    })
}

pub async fn delete_environment_permission(
    db: &PgPool,
    environment_id: EnvironmentId,
    permission_id: Uuid,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let res = sqlx::query(
        r#"
        DELETE FROM environment_permissions
        WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(permission_id)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("Permission not found".to_string()));
    }
    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("developer", actor_id),
        "ENVIRONMENT_PERMISSION_DELETED",
        ("environment_permission", Some(permission_id)),
        serde_json::json!({}),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(())
}

pub async fn list_environment_roles(
    db: &PgPool,
    environment_id: EnvironmentId,
) -> Result<Vec<EnvironmentRoleView>, AppError> {
    let rows = sqlx::query_as::<_, RoleRow>(
        r#"
        SELECT id, slug, name, scope, description, is_system, created_at, updated_at
        FROM environment_roles
        WHERE environment_id = $1
        ORDER BY scope ASC, slug ASC
        "#,
    )
    .bind(environment_id)
    .fetch_all(db)
    .await?;

    let role_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let pairs = if role_ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, RolePermPairRow>(
            r#"
            SELECT erp.role_id, ep.slug
            FROM environment_role_permissions erp
            JOIN environment_permissions ep
              ON ep.environment_id = erp.environment_id AND ep.id = erp.permission_id
            WHERE erp.environment_id = $1 AND erp.role_id = ANY($2)
            ORDER BY ep.slug ASC
            "#,
        )
        .bind(environment_id)
        .bind(&role_ids)
        .fetch_all(db)
        .await?
    };

    let mut perm_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for p in pairs {
        perm_map.entry(p.role_id).or_default().push(p.slug);
    }

    Ok(rows
        .into_iter()
        .map(|r| EnvironmentRoleView {
            permissions: perm_map.remove(&r.id).unwrap_or_default(),
            id: r.id,
            slug: r.slug,
            name: r.name,
            scope: r.scope,
            description: r.description,
            is_system: r.is_system,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect())
}

pub async fn create_environment_role(
    db: &PgPool,
    environment_id: EnvironmentId,
    input: CreateEnvironmentRoleInput<'_>,
) -> Result<EnvironmentRoleView, AppError> {
    let CreateEnvironmentRoleInput {
        slug,
        name,
        scope,
        description,
        permission_slugs,
        actor_id,
    } = input;
    let clean_slug = validate_permission_slug(slug)?;
    let clean_name = name.trim();
    if clean_name.len() < 2 || clean_name.len() > 120 {
        return Err(AppError::Validation(
            "Role name must be between 2 and 120 characters".to_string(),
        ));
    }
    let clean_scope = match scope.trim().to_lowercase().as_str() {
        "application" | "organization" => scope.trim().to_lowercase(),
        other => {
            return Err(AppError::Validation(format!(
                "Invalid role scope '{other}'; expected 'application' or 'organization'"
            )));
        }
    };

    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, RoleRow>(
        r#"
        INSERT INTO environment_roles (environment_id, slug, name, scope, description, is_system)
        VALUES ($1, $2, $3, $4, $5, FALSE)
        RETURNING id, slug, name, scope, description, is_system, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(&clean_slug)
    .bind(clean_name)
    .bind(&clean_scope)
    .bind(description.unwrap_or("").trim())
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Validation(format!("Role '{clean_slug}' already exists"))
        }
        other => AppError::Database(other),
    })?;

    for perm_slug in permission_slugs {
        let clean_perm = validate_permission_slug(perm_slug)?;
        sqlx::query(
            r#"
            INSERT INTO environment_permissions (environment_id, slug, description)
            VALUES ($1, $2, '')
            ON CONFLICT (environment_id, slug) DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(&clean_perm)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO environment_role_permissions (environment_id, role_id, permission_id)
            SELECT $1, $2, id
            FROM environment_permissions
            WHERE environment_id = $1 AND slug = $3
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(row.id)
        .bind(&clean_perm)
        .execute(&mut *tx)
        .await?;
    }

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("developer", actor_id),
        "ENVIRONMENT_ROLE_CREATED",
        ("environment_role", Some(row.id)),
        serde_json::json!({ "slug": clean_slug, "scope": clean_scope, "permissions": permission_slugs }),
    )
    .await?;

    commit_tx(tx).await?;
    let roles = list_environment_roles(db, environment_id).await?;
    roles
        .into_iter()
        .find(|r| r.id == row.id)
        .ok_or_else(|| AppError::Internal("Created role not found".to_string()))
}

pub async fn update_environment_role(
    db: &PgPool,
    environment_id: EnvironmentId,
    role_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    permission_slugs: Option<&[String]>,
    actor_id: Option<Uuid>,
) -> Result<EnvironmentRoleView, AppError> {
    let mut tx = begin_tx(db).await?;
    let existing = sqlx::query_as::<_, RoleRow>(
        r#"
        SELECT id, slug, name, scope, description, is_system, created_at, updated_at
        FROM environment_roles
        WHERE environment_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(role_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Role not found".to_string()))?;

    let next_name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&existing.name);
    let next_desc = description.map(str::trim).unwrap_or(&existing.description);

    sqlx::query(
        r#"
        UPDATE environment_roles
        SET name = $3, description = $4, updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(role_id)
    .bind(next_name)
    .bind(next_desc)
    .execute(&mut *tx)
    .await?;

    if let Some(perms) = permission_slugs {
        sqlx::query(
            r#"
            DELETE FROM environment_role_permissions
            WHERE environment_id = $1 AND role_id = $2
            "#,
        )
        .bind(environment_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;

        for perm_slug in perms {
            let clean_perm = validate_permission_slug(perm_slug)?;
            sqlx::query(
                r#"
                INSERT INTO environment_permissions (environment_id, slug, description)
                VALUES ($1, $2, '')
                ON CONFLICT (environment_id, slug) DO NOTHING
                "#,
            )
            .bind(environment_id)
            .bind(&clean_perm)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO environment_role_permissions (environment_id, role_id, permission_id)
                SELECT $1, $2, id
                FROM environment_permissions
                WHERE environment_id = $1 AND slug = $3
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(environment_id)
            .bind(role_id)
            .bind(&clean_perm)
            .execute(&mut *tx)
            .await?;
        }
    }

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("developer", actor_id),
        "ENVIRONMENT_ROLE_UPDATED",
        ("environment_role", Some(role_id)),
        serde_json::json!({ "slug": existing.slug }),
    )
    .await?;

    commit_tx(tx).await?;
    let roles = list_environment_roles(db, environment_id).await?;
    roles
        .into_iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| AppError::Internal("Updated role not found".to_string()))
}

pub async fn delete_environment_role(
    db: &PgPool,
    environment_id: EnvironmentId,
    role_id: Uuid,
    actor_id: Option<Uuid>,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let row: Option<(bool, String)> = sqlx::query_as(
        r#"
        SELECT is_system, slug
        FROM environment_roles
        WHERE environment_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(role_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((is_system, slug)) = row else {
        return Err(AppError::NotFound("Role not found".to_string()));
    };
    if is_system {
        return Err(AppError::Validation(format!(
            "System role '{slug}' cannot be deleted"
        )));
    }

    sqlx::query(
        r#"
        DELETE FROM environment_roles
        WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(role_id)
    .execute(&mut *tx)
    .await?;

    record_env_audit_in_tx(
        &mut tx,
        environment_id,
        ("developer", actor_id),
        "ENVIRONMENT_ROLE_DELETED",
        ("environment_role", Some(role_id)),
        serde_json::json!({ "slug": slug }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(())
}
