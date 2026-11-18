use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{TokenPair, User, UserProfile};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::redis_ephemeral;
use crate::repositories::refresh_tokens as refresh_repo;
use crate::repositories::sessions as session_repo;
use crate::repositories::users as user_repo;
use crate::services::sessions as session_service;
use crate::services::tokens::{
    hash_token, issue_legacy_es256_token_pair, validate_email_verification_token,
    verify_legacy_es256_access_token, verify_legacy_es256_refresh_token,
};
use deadpool_redis::Pool as RedisPool;

pub struct AuthContext<'a> {
    pub config: &'a Arc<AppConfig>,
    pub db: &'a PgPool,
    pub redis: &'a RedisPool,
}

use crate::services::password_hasher::{hash_password_async, verify_password_async};

fn generate_otp_code() -> String {
    use rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let num = u32::from_be_bytes(bytes);
    format!("{:06}", (num % 900_000) + 100_000)
}

pub async fn initiate_registration(
    ctx: AuthContext<'_>,
    email: String,
    password: String,
    display_name: String,
) -> Result<String, AppError> {
    let normalized = redis_ephemeral::normalize_email(&email);
    let existing = user_repo::find_user_by_email(ctx.db, &normalized).await?;
    if existing.is_some() {
        return Err(AppError::EmailAlreadyExists);
    }

    let password_hash = hash_password_async(password).await?;

    let code_str = generate_otp_code();

    let pending = redis_ephemeral::PendingRegistration {
        email: normalized.clone(),
        display_name: display_name.clone(),
        password_hash,
        code: code_str.clone(),
        attempts: 0,
    };

    redis_ephemeral::store_pending_registration(
        ctx.redis,
        &pending,
        ctx.config.verification_code_expiry_secs,
    )
    .await?;

    let config = ctx.config.clone();
    let to_email = normalized.clone();
    let to_name = display_name;
    let otp_code = code_str;

    tokio::spawn(async move {
        if let Err(err) =
            crate::services::email::send_registration_otp(&config, &to_email, &to_name, &otp_code)
                .await
        {
            tracing::error!(to = %to_email, error = %err, "Failed background registration OTP delivery");
        }
    });

    Ok(normalized)
}

pub async fn verify_registration_otp(
    ctx: AuthContext<'_>,
    email: String,
    code: String,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<(UserProfile, TokenPair), AppError> {
    let normalized = redis_ephemeral::normalize_email(&email);
    let pending = redis_ephemeral::get_pending_registration(ctx.redis, &normalized)
        .await?
        .ok_or_else(|| {
            AppError::InvalidToken(
                "Verification code has expired or registration session not found".to_string(),
            )
        })?;

    if pending.attempts >= 5 {
        let _ = redis_ephemeral::remove_pending_registration(ctx.redis, &normalized).await;
        return Err(AppError::TooManyRequests(
            "Too many failed verification attempts. Please restart registration.".to_string(),
        ));
    }

    use subtle::ConstantTimeEq;
    let expected_bytes = pending.code.as_bytes();
    let actual_bytes = code.trim().as_bytes();
    let matches: bool = if expected_bytes.len() == actual_bytes.len() {
        expected_bytes.ct_eq(actual_bytes).into()
    } else {
        false
    };

    if !matches {
        let attempts = redis_ephemeral::update_pending_registration_attempts(
            ctx.redis,
            &normalized,
            pending,
            ctx.config.verification_code_expiry_secs,
        )
        .await?;

        if attempts >= 5 {
            return Err(AppError::TooManyRequests(
                "Too many failed verification attempts. Registration session invalidated."
                    .to_string(),
            ));
        }

        return Err(AppError::InvalidToken(
            "Invalid verification code. Please check and try again.".to_string(),
        ));
    }

    let _ = redis_ephemeral::remove_pending_registration(ctx.redis, &normalized).await;

    if user_repo::find_user_by_email(ctx.db, &pending.email)
        .await?
        .is_some()
    {
        return Err(AppError::EmailAlreadyExists);
    }

    let mut new_user = User::new(
        pending.email,
        pending.display_name,
        Some(pending.password_hash),
    );
    new_user.email_verified = true;

    let mut tx = begin_tx(ctx.db).await?;
    let user = user_repo::create_user(&mut *tx, new_user).await?;
    crate::repositories::authorization::assign_role_to_user(&mut *tx, user.id, "user").await?;
    commit_tx(tx).await?;

    let (roles, permissions) =
        crate::services::authorization::resolve_user_roles_and_permissions(ctx.db, user.id).await?;

    let session_id = Uuid::new_v4();
    let tokens = issue_legacy_es256_token_pair(
        ctx.db,
        ctx.config,
        user.id,
        &user.email,
        &user.display_name,
        roles.into_iter().map(|r| r.to_string()).collect(),
        permissions.into_iter().map(|p| p.to_string()).collect(),
        session_id,
    )
    .await?;

    session_service::create_session_with_id(
        ctx.db,
        ctx.redis,
        session_id,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        user_agent.clone(),
        ip_address.clone(),
        None,
    )
    .await?;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user.id),
        crate::models::AuditEvent::LoginSuccess,
        ip_address.clone(),
        user_agent.clone(),
        serde_json::json!({ "email": user.email, "action": "register_otp_verified" }),
    )
    .await;

    let _ = crate::services::audit::log_event(
        ctx.db,
        Some(user.id),
        crate::models::AuditEvent::EmailVerified,
        ip_address,
        user_agent,
        serde_json::json!({ "email": user.email, "method": "registration_otp" }),
    )
    .await;

    Ok((UserProfile::from(user), tokens))
}

pub async fn resend_registration_otp(ctx: AuthContext<'_>, email: String) -> Result<(), AppError> {
    let normalized = redis_ephemeral::normalize_email(&email);
    let mut pending = redis_ephemeral::get_pending_registration(ctx.redis, &normalized)
        .await?
        .ok_or_else(|| {
            AppError::InvalidToken("No pending registration found for this email".to_string())
        })?;

    let code_str = generate_otp_code();

    pending.code = code_str.clone();
    pending.attempts = 0;

    redis_ephemeral::store_pending_registration(
        ctx.redis,
        &pending,
        ctx.config.verification_code_expiry_secs,
    )
    .await?;

    let config = ctx.config.clone();
    let to_email = normalized;
    let to_name = pending.display_name;
    let otp_code = code_str;

    tokio::spawn(async move {
        if let Err(err) =
            crate::services::email::send_registration_otp(&config, &to_email, &to_name, &otp_code)
                .await
        {
            tracing::error!(to = %to_email, error = %err, "Failed background registration OTP resend");
        }
    });

    Ok(())
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

    let password_hash = hash_password_async(password).await?;

    let user = User::new(email, display_name, Some(password_hash));
    let mut tx = begin_tx(ctx.db).await?;
    let user = user_repo::create_user(&mut *tx, user).await?;
    crate::repositories::authorization::assign_role_to_user(&mut *tx, user.id, "user").await?;
    commit_tx(tx).await?;

    let (roles, permissions) =
        crate::services::authorization::resolve_user_roles_and_permissions(ctx.db, user.id).await?;

    let session_id = Uuid::new_v4();
    let tokens = issue_legacy_es256_token_pair(
        ctx.db,
        ctx.config,
        user.id,
        &user.email,
        &user.display_name,
        roles.into_iter().map(|r| r.to_string()).collect(),
        permissions.into_iter().map(|p| p.to_string()).collect(),
        session_id,
    )
    .await?;

    session_service::create_session_with_id(
        ctx.db,
        ctx.redis,
        session_id,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        None,
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

    let verified = verify_password_async(password.to_string(), password_hash.to_string()).await?;
    if !verified {
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

    let _ = redis_ephemeral::clear_login_attempts(ctx.redis, &email).await;

    let (roles, permissions) =
        crate::services::authorization::resolve_user_roles_and_permissions(ctx.db, user.id).await?;

    let session_id = Uuid::new_v4();
    let tokens = issue_legacy_es256_token_pair(
        ctx.db,
        ctx.config,
        user.id,
        &user.email,
        &user.display_name,
        roles.into_iter().map(|r| r.to_string()).collect(),
        permissions.into_iter().map(|p| p.to_string()).collect(),
        session_id,
    )
    .await?;

    session_service::create_session_with_id(
        ctx.db,
        ctx.redis,
        session_id,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        user_agent.clone(),
        ip_address.clone(),
        None,
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
    let claims = verify_legacy_es256_refresh_token(ctx.db, refresh_token).await?;

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
    let existing_record = refresh_repo::find_by_token_hash(ctx.db, &token_hash).await?;
    let family_id = existing_record
        .as_ref()
        .map(|r| r.family_id)
        .unwrap_or(session_id);

    let token_already_revoked = existing_record
        .as_ref()
        .map(|r| r.is_revoked())
        .unwrap_or(false);

    if session.refresh_token_hash != token_hash || token_already_revoked {
        let mut tx = begin_tx(ctx.db).await?;
        session_repo::revoke_pg_session(&mut *tx, session_id).await?;
        refresh_repo::revoke_family(&mut *tx, family_id).await?;
        commit_tx(tx).await?;
        let _ = session_repo::delete_redis_session(ctx.redis, session_id).await;
        return Err(AppError::InvalidToken(
            "Refresh token reuse detected".to_string(),
        ));
    }

    let user = user_repo::find_user_by_id(ctx.db, user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let (roles, permissions) =
        crate::services::authorization::resolve_user_roles_and_permissions(ctx.db, user.id).await?;

    let new_session_id = Uuid::new_v4();
    let tokens = issue_legacy_es256_token_pair(
        ctx.db,
        ctx.config,
        user.id,
        &user.email,
        &user.display_name,
        roles.into_iter().map(|r| r.to_string()).collect(),
        permissions.into_iter().map(|p| p.to_string()).collect(),
        new_session_id,
    )
    .await?;

    let mut tx = begin_tx(ctx.db).await?;
    session_repo::revoke_pg_session(&mut *tx, session_id).await?;
    if let Some(ref record) = existing_record {
        refresh_repo::revoke_token(&mut *tx, record.id).await?;
    }
    let new_session = session_service::create_session_in_tx(
        &mut tx,
        new_session_id,
        user.id,
        &tokens.refresh_token,
        ctx.config.refresh_token_duration(),
        session.user_agent,
        session.ip_address,
        Some(family_id),
    )
    .await?;
    commit_tx(tx).await?;

    let _ = session_repo::delete_redis_session(ctx.redis, session_id).await;
    session_repo::store_redis_session(ctx.redis, &new_session).await?;

    Ok(tokens)
}

pub async fn logout(
    ctx: AuthContext<'_>,
    access_token: Option<&str>,
    refresh_token: Option<&str>,
) -> Result<(), AppError> {
    if let Some(token) = access_token {
        if let Ok(claims) = verify_legacy_es256_access_token(ctx.db, token).await {
            let ttl_secs = (claims.exp - chrono::Utc::now().timestamp()).max(0) as u64;
            let _ = session_repo::blacklist_token(ctx.redis, &claims.jti, ttl_secs).await;

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
        }
    }

    if let Some(token) = refresh_token {
        if let Ok(claims) = verify_legacy_es256_refresh_token(ctx.db, token).await {
            if let Ok(session_id) = Uuid::parse_str(&claims.session_id) {
                let _ = session_service::invalidate_session(ctx.db, ctx.redis, session_id).await;
            }
        }
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

pub async fn request_password_reset(ctx: AuthContext<'_>, email: String) -> Result<(), AppError> {
    let normalized = redis_ephemeral::normalize_email(&email);
    let user_opt = user_repo::find_user_by_email(ctx.db, &normalized).await?;

    if let Some(user) = user_opt {
        use rand_core::{OsRng, RngCore};
        let mut token_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut token_bytes);
        let token = hex::encode(token_bytes);

        redis_ephemeral::store_password_reset_token(
            ctx.redis,
            &token,
            user.id,
            ctx.config.password_reset_expiry_secs,
        )
        .await?;

        let config = ctx.config.clone();
        let to_email = normalized;
        let to_name = user.display_name;
        let reset_token = token;

        tokio::spawn(async move {
            if let Err(err) = crate::services::email::send_password_reset_email(
                &config,
                &to_email,
                &to_name,
                &reset_token,
            )
            .await
            {
                tracing::error!(to = %to_email, error = %err, "Failed background password reset email dispatch");
            }
        });
    }

    Ok(())
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
        .ok_or_else(|| {
            AppError::InvalidToken("Invalid or expired password reset token".to_string())
        })?;

    let user = user_repo::find_user_by_id(ctx.db, user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let password_hash = hash_password_async(new_password.to_string()).await?;

    user_repo::update_password_hash(ctx.db, user.id, &password_hash).await?;

    let _ = session_service::revoke_all_user_sessions(ctx.db, user.id).await;
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
    let code_str = generate_otp_code();

    let data = redis_ephemeral::VerificationCodeData { user_id, email };

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
        .ok_or_else(|| {
            AppError::InvalidToken("Invalid or expired verification code".to_string())
        })?;

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
