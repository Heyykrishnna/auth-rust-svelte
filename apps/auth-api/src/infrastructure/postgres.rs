use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::domain::user::User;

/// Type alias for the PostgreSQL connection pool.
pub type PgPool = sqlx::PgPool;

/// Create and verify a PostgreSQL connection pool.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}

// ─── User Queries ────────────────────────────────────────────────────────────

/// Create a new user in the database.
pub async fn create_user(pool: &PgPool, user: User) -> Result<User, sqlx::Error> {
    let row = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, email, display_name, password_hash, avatar_url, email_verified, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
        user.id,
        user.email,
        user.display_name,
        user.password_hash,
        user.avatar_url,
        user.email_verified,
        user.created_at,
        user.updated_at,
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Find a user by email address.
pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1 LIMIT 1",
        email
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Find a user by ID.
pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let row = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1 LIMIT 1",
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
