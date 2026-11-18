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
