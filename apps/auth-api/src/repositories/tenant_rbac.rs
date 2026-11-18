use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{AppUserId, EnvironmentId};
use crate::repositories::db::PgTx;

pub async fn assign_environment_role_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    role_slug: &str,
) -> Result<(), AppError> {
    let role_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM environment_roles
        WHERE environment_id = $1 AND slug = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(role_slug)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(role_id) = role_id else {
        return Err(AppError::Internal(format!(
            "Environment role '{role_slug}' not seeded for environment {environment_id}"
        )));
    };

    sqlx::query(
        r#"
        INSERT INTO application_user_roles (environment_id, application_user_id, role_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (environment_id, application_user_id, role_id) DO NOTHING
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .bind(role_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn resolve_environment_roles_and_permissions_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT er.slug
        FROM application_user_roles aur
        JOIN environment_roles er
          ON er.environment_id = aur.environment_id AND er.id = aur.role_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY er.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(&mut **tx)
    .await?;

    let permissions = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT ep.slug
        FROM application_user_roles aur
        JOIN environment_role_permissions erp
          ON erp.environment_id = aur.environment_id AND erp.role_id = aur.role_id
        JOIN environment_permissions ep
          ON ep.environment_id = erp.environment_id AND ep.id = erp.permission_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY ep.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok((roles, permissions))
}

pub async fn resolve_environment_roles_and_permissions(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    let roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT er.slug
        FROM application_user_roles aur
        JOIN environment_roles er
          ON er.environment_id = aur.environment_id AND er.id = aur.role_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY er.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    let permissions = sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT ep.slug
        FROM application_user_roles aur
        JOIN environment_role_permissions erp
          ON erp.environment_id = aur.environment_id AND erp.role_id = aur.role_id
        JOIN environment_permissions ep
          ON ep.environment_id = erp.environment_id AND ep.id = erp.permission_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = $2
        ORDER BY ep.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    Ok((roles, permissions))
}

pub async fn ensure_default_org_roles_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
) -> Result<(), AppError> {
    for (slug, name, desc) in [
        (
            "org:owner",
            "Organization Owner",
            "Full organization control including ownership transfer",
        ),
        (
            "org:admin",
            "Organization Admin",
            "Manage organization members, invitations, and settings",
        ),
        (
            "org:member",
            "Organization Member",
            "Standard organization membership",
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO environment_roles (environment_id, slug, name, scope, description, is_system)
            VALUES ($1, $2, $3, 'organization', $4, TRUE)
            ON CONFLICT (environment_id, slug) DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(slug)
        .bind(name)
        .bind(desc)
        .execute(&mut **tx)
        .await?;
    }

    for (perm_slug, perm_desc) in [
        ("org:read", "Read organization profile and members"),
        (
            "org:members.manage",
            "Invite, update, and remove organization members",
        ),
        ("org:settings.manage", "Modify organization settings"),
        ("org:delete", "Delete the organization"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO environment_permissions (environment_id, slug, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (environment_id, slug) DO NOTHING
            "#,
        )
        .bind(environment_id)
        .bind(perm_slug)
        .bind(perm_desc)
        .execute(&mut **tx)
        .await?;
    }

    for (role_slug, perm_slugs) in [
        (
            "org:owner",
            vec![
                "org:read",
                "org:members.manage",
                "org:settings.manage",
                "org:delete",
            ],
        ),
        (
            "org:admin",
            vec!["org:read", "org:members.manage", "org:settings.manage"],
        ),
        ("org:member", vec!["org:read"]),
    ] {
        for perm_slug in perm_slugs {
            sqlx::query(
                r#"
                INSERT INTO environment_role_permissions (environment_id, role_id, permission_id)
                SELECT $1, er.id, ep.id
                FROM environment_roles er
                JOIN environment_permissions ep
                  ON ep.environment_id = er.environment_id AND ep.slug = $3
                WHERE er.environment_id = $1 AND er.slug = $2
                ON CONFLICT (environment_id, role_id, permission_id) DO NOTHING
                "#,
            )
            .bind(environment_id)
            .bind(role_slug)
            .bind(perm_slug)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

pub async fn resolve_organization_context_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    organization_id: crate::models::OrganizationId,
) -> Result<Option<crate::services::tokens::OrganizationTokenContext>, AppError> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        r#"
        SELECT om.id, o.slug
        FROM organization_members om
        JOIN organizations o
          ON o.environment_id = om.environment_id AND o.id = om.organization_id
        WHERE om.environment_id = $1
          AND om.application_user_id = $2
          AND om.organization_id = $3
          AND om.status = 'active'
          AND o.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .bind(organization_id)
    .fetch_optional(&mut **tx)
    .await?;

    let Some((member_id, org_slug)) = row else {
        return Ok(None);
    };

    let org_roles: Vec<String> = sqlx::query_scalar(
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
    .fetch_all(&mut **tx)
    .await?;

    let org_permissions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT DISTINCT ep.slug
        FROM organization_member_roles omr
        JOIN environment_role_permissions erp
          ON erp.environment_id = omr.environment_id AND erp.role_id = omr.role_id
        JOIN environment_permissions ep
          ON ep.environment_id = erp.environment_id AND ep.id = erp.permission_id
        WHERE omr.environment_id = $1 AND omr.organization_member_id = $2
        ORDER BY ep.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(member_id)
    .fetch_all(&mut **tx)
    .await?;

    let primary_role = if org_roles.iter().any(|r| r == "org:owner") {
        Some("org:owner".to_string())
    } else if org_roles.iter().any(|r| r == "org:admin") {
        Some("org:admin".to_string())
    } else {
        org_roles.into_iter().next()
    };

    Ok(Some(crate::services::tokens::OrganizationTokenContext {
        org_id: Some(organization_id),
        org_slug: Some(org_slug),
        org_role: primary_role,
        org_permissions,
    }))
}
