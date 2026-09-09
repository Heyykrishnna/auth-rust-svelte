use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::User;

pub async fn create_user(pool: &PgPool, user: User) -> Result<User, AppError> {
    let created = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at
        "#
    )
    .bind(user.id)
    .bind(user.email)
    .bind(user.display_name)
    .bind(user.password_hash)
    .bind(user.avatar_url)
    .bind(user.email_verified)
    .bind(user.status)
    .bind(user.created_at)
    .bind(user.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(created)
}

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at
        FROM users
        WHERE email = $1
        LIMIT 1
        "#
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at
        FROM users
        WHERE id = $1
        LIMIT 1
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn set_email_verified(pool: &PgPool, id: Uuid, verified: bool) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE users
        SET email_verified = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(verified)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_profile(
    pool: &PgPool,
    id: Uuid,
    display_name: &str,
    avatar_url: Option<&str>,
) -> Result<User, AppError> {
    let updated = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET display_name = $1, avatar_url = $2, updated_at = NOW()
        WHERE id = $3
        RETURNING id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at
        "#
    )
    .bind(display_name)
    .bind(avatar_url)
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(updated)
}

pub async fn update_password_hash(
    pool: &PgPool,
    id: Uuid,
    password_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $1, updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(password_hash)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}
