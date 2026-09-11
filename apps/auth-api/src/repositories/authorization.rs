use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;

pub async fn get_user_roles(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role_id
        FROM user_roles
        WHERE user_id = $1
        ORDER BY role_id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_user_permissions(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT rp.permission_id
        FROM user_roles ur
        JOIN role_permissions rp ON ur.role_id = rp.role_id
        WHERE ur.user_id = $1
        UNION
        SELECT up.permission_id
        FROM user_permissions up
        WHERE up.user_id = $1
        ORDER BY 1 ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_role_permissions(pool: &PgPool, role_id: &str) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT permission_id
        FROM role_permissions
        WHERE role_id = $1
        ORDER BY permission_id ASC
        "#,
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn assign_role_to_user(
    pool: &PgPool,
    user_id: Uuid,
    role_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_roles (user_id, role_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id, role_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(role_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_role_from_user(
    pool: &PgPool,
    user_id: Uuid,
    role_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        DELETE FROM user_roles
        WHERE user_id = $1 AND role_id = $2
        "#,
    )
    .bind(user_id)
    .bind(role_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn grant_permission_to_user(
    pool: &PgPool,
    user_id: Uuid,
    permission_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_permissions (user_id, permission_id)
        VALUES ($1, $2)
        ON CONFLICT (user_id, permission_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(permission_id)
    .execute(pool)
    .await?;

    Ok(())
}
