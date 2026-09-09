use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use rand_core::OsRng;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{TokenPair, User, UserProfile};
use crate::repositories::redis_ephemeral;
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

fn create_argon2id() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
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
    let argon2 = create_argon2id();
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

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user.id),
        crate::models::AuditEvent::LoginSuccess,
        None,
        None,
        serde_json::json!({ "email": user.email, "action": "register" }),
    )
    .await;

    Ok((UserProfile::from(user), tokens))
}

pub async fn login(
    ctx: AuthContext<'_>,
    email: String,
    password: String,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<(UserProfile, TokenPair), AppError> {
    // 1. Check login attempts counter in Redis before expensive DB query or Argon2id verification
    if let Err(err) =
        redis_ephemeral::check_login_attempts(ctx.redis, &email, ctx.config.login_max_attempts)
            .await
    {
        let _ = crate::services::audit::log_event(
            ctx.db,
            None,
            crate::models::AuditEvent::AccountLocked,
            ip_address.clone(),
            user_agent.clone(),
            serde_json::json!({
                "email": email,
                "reason": "brute_force_lockout",
                "max_attempts": ctx.config.login_max_attempts
            }),
        )
        .await;
        return Err(err);
    }

    let user_opt = user_repo::find_user_by_email(ctx.db, &email).await?;
    let user = match user_opt {
        Some(u) => u,
        None => {
            let attempts = redis_ephemeral::record_failed_login(
                ctx.redis,
                &email,
                ctx.config.login_lockout_duration_secs,
            )
            .await
            .unwrap_or(1);

            let event = if attempts >= ctx.config.login_max_attempts {
                crate::models::AuditEvent::AccountLocked
            } else {
                crate::models::AuditEvent::LoginFailed
            };

            let _ = crate::services::audit::log_event(
                ctx.db,
                None,
                event,
                ip_address,
                user_agent,
                serde_json::json!({ "email": email, "reason": "user_not_found", "attempts": attempts }),
            )
            .await;
            return Err(AppError::InvalidCredentials);
        }
    };

    let password_hash = match user.password_hash.as_deref() {
        Some(h) => h,
        None => {
            let attempts = redis_ephemeral::record_failed_login(
                ctx.redis,
                &email,
                ctx.config.login_lockout_duration_secs,
            )
            .await
            .unwrap_or(1);

            let event = if attempts >= ctx.config.login_max_attempts {
                crate::models::AuditEvent::AccountLocked
            } else {
                crate::models::AuditEvent::LoginFailed
            };

            let _ = crate::services::audit::log_event(
                ctx.db,
                Some(user.id),
                event,
                ip_address,
                user_agent,
                serde_json::json!({ "email": email, "reason": "no_password", "attempts": attempts }),
            )
            .await;
            return Err(AppError::InvalidCredentials);
        }
    };

    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => {
            let _ = crate::services::audit::log_event(
                ctx.db,
                Some(user.id),
                crate::models::AuditEvent::LoginFailed,
                ip_address,
                user_agent,
                serde_json::json!({ "email": email, "reason": "hash_error" }),
            )
            .await;
            return Err(AppError::InvalidCredentials);
        }
    };

    let argon2 = create_argon2id();
    if argon2.verify_password(password.as_bytes(), &parsed_hash).is_err() {
        let attempts = redis_ephemeral::record_failed_login(
            ctx.redis,
            &email,
            ctx.config.login_lockout_duration_secs,
        )
        .await
        .unwrap_or(1);

        let event = if attempts >= ctx.config.login_max_attempts {
            crate::models::AuditEvent::AccountLocked
        } else {
            crate::models::AuditEvent::LoginFailed
        };

        let _ = crate::services::audit::log_event(
            ctx.db,
            Some(user.id),
            event,
            ip_address,
            user_agent,
            serde_json::json!({ "email": email, "reason": "invalid_password", "attempts": attempts }),
        )
        .await;
        return Err(AppError::InvalidCredentials);
    }

    if !user.is_active() {
        let _ = crate::services::audit::log_event(
            ctx.db,
            Some(user.id),
            crate::models::AuditEvent::AccountLocked,
            ip_address,
            user_agent,
            serde_json::json!({ "email": email, "status": user.status }),
        )
        .await;
        return Err(AppError::Forbidden(
            "Account is suspended or inactive".to_string(),
        ));
    }

    // Login succeeded: clear recorded failed login attempts in Redis
    let _ = redis_ephemeral::clear_login_attempts(ctx.redis, &email).await;

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
        user_agent.clone(),
        ip_address.clone(),
    )
    .await?;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user.id),
        crate::models::AuditEvent::LoginSuccess,
        ip_address,
        user_agent,
        serde_json::json!({ "email": user.email, "session_id": session_id }),
    )
    .await;

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

    if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
        let _ = crate::services::audit::log_event(
            ctx.db,
            Some(user_id),
            crate::models::AuditEvent::Logout,
            None,
            None,
            serde_json::json!({ "jti": claims.jti }),
        )
        .await;
    }

    Ok(())
}

pub async fn verify_email(ctx: AuthContext<'_>, token: &str) -> Result<(), AppError> {
    let claims = validate_email_verification_token(token, &ctx.config.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::InvalidToken("Invalid user ID in verification token".to_string()))?;

    user_repo::set_email_verified(ctx.db, user_id, true).await?;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user_id),
        crate::models::AuditEvent::EmailVerified,
        None,
        None,
        serde_json::json!({ "email": claims.email }),
    )
    .await;

    Ok(())
}

pub async fn request_password_reset(
    ctx: AuthContext<'_>,
    email: String,
) -> Result<Option<String>, AppError> {
    let normalized = redis_ephemeral::normalize_email(&email);
    let user_opt = user_repo::find_user_by_email(ctx.db, &normalized).await?;

    if let Some(user) = user_opt {
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());

        redis_ephemeral::store_password_reset_token(
            ctx.redis,
            &token,
            user.id,
            ctx.config.password_reset_expiry_secs,
        )
        .await?;

        return Ok(Some(token));
    }

    Ok(None)
}

pub async fn reset_password(
    ctx: AuthContext<'_>,
    token: String,
    new_password: String,
) -> Result<(), AppError> {
    if new_password.trim().len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let user_id = redis_ephemeral::consume_password_reset_token(ctx.redis, &token)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Invalid or expired password reset token".to_string()))?;

    let user = user_repo::find_user_by_id(ctx.db, user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = create_argon2id();
    let password_hash = argon2
        .hash_password(new_password.as_bytes(), &salt)
        .map_err(|e| AppError::PasswordHash(e.to_string()))?
        .to_string();

    user_repo::update_password_hash(ctx.db, user.id, &password_hash).await?;

    // Invalidate all existing sessions and refresh tokens on password change
    let _ = session_service::revoke_all_user_sessions(ctx.db, user.id).await;

    // Clear any brute-force lockout counter for this email
    let _ = redis_ephemeral::clear_login_attempts(ctx.redis, &user.email).await;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user.id),
        crate::models::AuditEvent::PasswordChanged,
        None,
        None,
        serde_json::json!({ "email": user.email }),
    )
    .await;

    Ok(())
}

pub async fn generate_verification_code(
    ctx: AuthContext<'_>,
    user_id: Uuid,
    email: String,
) -> Result<String, AppError> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let code: u32 = rng.gen_range(100_000..=999_999);
    let code_str = code.to_string();

    let data = redis_ephemeral::VerificationCodeData {
        user_id,
        email,
    };

    redis_ephemeral::store_verification_code(
        ctx.redis,
        &code_str,
        &data,
        ctx.config.verification_code_expiry_secs,
    )
    .await?;

    Ok(code_str)
}

pub async fn verify_email_code(
    ctx: AuthContext<'_>,
    code: &str,
) -> Result<redis_ephemeral::VerificationCodeData, AppError> {
    let data = redis_ephemeral::consume_verification_code(ctx.redis, code)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Invalid or expired verification code".to_string()))?;

    user_repo::set_email_verified(ctx.db, data.user_id, true).await?;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(data.user_id),
        crate::models::AuditEvent::EmailVerified,
        None,
        None,
        serde_json::json!({ "email": data.email, "method": "otp_code" }),
    )
    .await;

    Ok(data)
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
