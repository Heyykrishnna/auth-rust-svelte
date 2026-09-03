use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::OsRng;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{TokenPair, User, UserProfile};
use crate::repositories::sessions as session_repo;
use crate::repositories::users as user_repo;
use crate::services::sessions as session_service;
use crate::services::tokens::{
    generate_token_pair, hash_token, validate_access_token, validate_email_verification_token,
    validate_refresh_token,
};
use deadpool_redis::Pool as RedisPool;

pub struct AuthContext<'a> {
    pub config: &'a Arc<AppConfig>,
    pub db: &'a PgPool,
    pub redis: &'a RedisPool,
}

pub async fn register(
    ctx: AuthContext<'_>,
    email: String,
    password: String,
    display_name: String,
) -> Result<(UserProfile, TokenPair), AppError> {
    let existing = user_repo::find_user_by_email(ctx.db, &email).await?;
    if existing.is_some() {
        return Err(AppError::EmailAlreadyExists);
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHash(e.to_string()))?
        .to_string();

    let user = User::new(email, display_name, Some(password_hash));
    let user = user_repo::create_user(ctx.db, user).await?;

    let session_id = Uuid::new_v4();
    let tokens = generate_token_pair(
        user.id,
        &user.email,
        &user.display_name,
        session_id,
        &ctx.config.jwt_secret,
        ctx.config.jwt_access_expiry_secs,
        ctx.config.jwt_refresh_expiry_secs,
    )?;

    session_service::create_session(
        ctx.db,
        ctx.redis,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        None,
        None,
    )
    .await?;

    Ok((UserProfile::from(user), tokens))
}

pub async fn login(
    ctx: AuthContext<'_>,
    email: String,
    password: String,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<(UserProfile, TokenPair), AppError> {
    let user = user_repo::find_user_by_email(ctx.db, &email)
        .await?
        .ok_or(AppError::InvalidCredentials)?;

    let password_hash = user
        .password_hash
        .as_deref()
        .ok_or(AppError::InvalidCredentials)?;

    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| AppError::InvalidCredentials)?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    let session_id = Uuid::new_v4();
    let tokens = generate_token_pair(
        user.id,
        &user.email,
        &user.display_name,
        session_id,
        &ctx.config.jwt_secret,
        ctx.config.jwt_access_expiry_secs,
        ctx.config.jwt_refresh_expiry_secs,
    )?;

    session_service::create_session(
        ctx.db,
        ctx.redis,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        user_agent,
        ip_address,
    )
    .await?;

    Ok((UserProfile::from(user), tokens))
}

pub async fn refresh(ctx: AuthContext<'_>, refresh_token: &str) -> Result<TokenPair, AppError> {
    let claims = validate_refresh_token(refresh_token, &ctx.config.jwt_secret)?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::InvalidToken("Invalid user ID in refresh token".to_string()))?;

    let session_id = Uuid::parse_str(&claims.session_id)
        .map_err(|_| AppError::InvalidToken("Invalid session ID in refresh token".to_string()))?;

    let session = session_service::get_session(ctx.db, ctx.redis, session_id)
        .await?
        .ok_or(AppError::SessionNotFound)?;

    if session.is_expired() {
        let _ = session_service::invalidate_session(ctx.db, ctx.redis, session_id).await;
        return Err(AppError::SessionNotFound);
    }

    let token_hash = hash_token(refresh_token);
    if session.refresh_token_hash != token_hash {
        let _ = session_service::invalidate_session(ctx.db, ctx.redis, session_id).await;
        return Err(AppError::InvalidToken(
            "Refresh token reuse detected".to_string(),
        ));
    }

    let user = user_repo::find_user_by_id(ctx.db, user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let new_session_id = Uuid::new_v4();
    let tokens = generate_token_pair(
        user.id,
        &user.email,
        &user.display_name,
        new_session_id,
        &ctx.config.jwt_secret,
        ctx.config.jwt_access_expiry_secs,
        ctx.config.jwt_refresh_expiry_secs,
    )?;

    let _ = session_service::invalidate_session(ctx.db, ctx.redis, session_id).await;

    session_service::create_session(
        ctx.db,
        ctx.redis,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        session.user_agent,
        session.ip_address,
    )
    .await?;

    Ok(tokens)
}

pub async fn logout(ctx: AuthContext<'_>, access_token: &str) -> Result<(), AppError> {
    let claims = validate_access_token(access_token, &ctx.config.jwt_secret)?;

    let ttl_secs = (claims.exp - chrono::Utc::now().timestamp()).max(0) as u64;
    session_repo::blacklist_token(ctx.redis, &claims.jti, ttl_secs).await?;

    Ok(())
}

pub async fn verify_email(ctx: AuthContext<'_>, token: &str) -> Result<(), AppError> {
    let claims = validate_email_verification_token(token, &ctx.config.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::InvalidToken("Invalid user ID in verification token".to_string()))?;

    user_repo::set_email_verified(ctx.db, user_id, true).await
}

pub async fn get_oidc_authorization_url(
    ctx: AuthContext<'_>,
    provider: &str,
) -> Result<String, AppError> {
    match provider.to_lowercase().as_str() {
        "google" => {
            let client_id = ctx.config.google_client_id.as_deref().ok_or_else(|| {
                AppError::OidcError("Google client ID not configured".to_string())
            })?;
            let redirect_uri = ctx.config.google_redirect_uri.as_deref().ok_or_else(|| {
                AppError::OidcError("Google redirect URI not configured".to_string())
            })?;

            Ok(format!(
                "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&state={}",
                client_id,
                urlencoding::encode(redirect_uri),
                Uuid::new_v4()
            ))
        }
        "github" => {
            let client_id = ctx.config.github_client_id.as_deref().ok_or_else(|| {
                AppError::OidcError("GitHub client ID not configured".to_string())
            })?;
            let redirect_uri = ctx.config.github_redirect_uri.as_deref().ok_or_else(|| {
                AppError::OidcError("GitHub redirect URI not configured".to_string())
            })?;

            Ok(format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=user:email&state={}",
                client_id,
                urlencoding::encode(redirect_uri),
                Uuid::new_v4()
            ))
        }
        _ => Err(AppError::OidcError(format!(
            "Unsupported provider: {provider}"
        ))),
    }
}

pub async fn handle_oidc_callback(
    _ctx: AuthContext<'_>,
    _provider: &str,
    _code: String,
    _state_param: String,
) -> Result<(), AppError> {
    Err(AppError::OidcError(
        "OIDC code exchange requires provider credentials".to_string(),
    ))
}
