use chrono::{DateTime, Duration, Utc};
use deadpool_redis::Pool as RedisPool;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::middleware::client_auth::blacklist_env_token;
use crate::models::{
    AggregateType, AppSession, AppSessionId, AppUserId, DomainEventEnvelope, DomainEventType,
    EnvironmentId, User, UserId,
};
use crate::repositories::auth_transactions as auth_tx_repo;
use crate::repositories::db::{begin_tx, commit_tx, PgTx};
use crate::repositories::{
    global_identity, outbox, redis_ephemeral, redis_keys, tenant_memberships, tenant_rbac,
    tenants as tenant_repo, users as user_repo,
};
use crate::services::tokens::{
    generate_opaque_refresh_token, hash_token, issue_tenant_access_token, IssueAccessTokenInput,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUserView {
    pub sub: UserId,
    pub app_user_id: AppUserId,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub status: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientAuthResponse {
    pub user: ClientUserView,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub session_id: AppSessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTokenRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub session_id: AppSessionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSignUpInitiatedResponse {
    pub status: String,
    pub email: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSessionView {
    pub id: AppSessionId,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub active_organization_id: Option<crate::models::OrganizationId>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingClientSignUp {
    email: String,
    display_name: String,
    password_hash: String,
    code: String,
    attempts: u32,
}

#[derive(Debug, FromRow)]
struct LockedRefreshTokenRow {
    id: Uuid,
    app_session_id: Option<Uuid>,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

use crate::services::password_hasher::{
    hash_password_async, verify_dummy_password_async, verify_password_async,
};

fn generate_otp_code() -> String {
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let num = u32::from_be_bytes(bytes);
    format!("{:06}", (num % 900_000) + 100_000)
}

fn signup_state_hash(environment_id: EnvironmentId, normalized_email: &str) -> String {
    hash_token(&format!("signup:{environment_id}:{normalized_email}"))
}

fn constant_time_str_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    if ab.len() != bb.len() {
        return false;
    }
    ab.ct_eq(bb).into()
}

#[allow(clippy::too_many_arguments)]
pub async fn record_client_audit_log_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: Option<EnvironmentId>,
    user_id: Option<UserId>,
    event: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    metadata: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            user_id, environment_id, actor_type, actor_id, event, resource_type, resource_id, ip_address, user_agent, metadata
        )
        VALUES ($1, $2, 'user', $1, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(user_id.map(|u| u.into_inner()))
    .bind(environment_id)
    .bind(event)
    .bind(resource_type)
    .bind(resource_id)
    .bind(ip_address)
    .bind(user_agent)
    .bind(crate::services::audit::sanitize_audit_metadata(metadata))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_app_session_and_refresh_in_tx(
    tx: &mut PgTx<'_>,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    user_id: UserId,
    session_ttl_secs: i64,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(AppSession, String), AppError> {
    let refresh_token = generate_opaque_refresh_token();
    let refresh_hash = hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::seconds(session_ttl_secs);

    let session = tenant_memberships::create_app_session(
        &mut **tx,
        environment_id,
        app_user_id,
        user_id,
        None,
        &refresh_hash,
        user_agent,
        ip_address,
        expires_at,
    )
    .await?;

    let family_id = session.id.into_inner();
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (
            id, user_id, environment_id, app_session_id, token_hash, family_id, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id.into_inner())
    .bind(environment_id)
    .bind(session.id.into_inner())
    .bind(&refresh_hash)
    .bind(family_id)
    .bind(expires_at)
    .execute(&mut **tx)
    .await?;

    let session_evt = DomainEventEnvelope::new(
        DomainEventType::SessionCreated,
        Some(environment_id.into_inner()),
        AggregateType::AppSession,
        session.id.into_inner(),
        1,
        serde_json::json!({
            "session_id": session.id,
            "sub": user_id,
            "app_user_id": app_user_id
        }),
    );
    outbox::insert_outbox_event(&mut **tx, environment_id, &session_evt).await?;

    Ok((session, refresh_token))
}

#[allow(clippy::too_many_arguments)]
pub async fn initiate_sign_up(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    email: &str,
    password: &str,
    display_name: &str,
) -> Result<(ClientSignUpInitiatedResponse, String), AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    if !normalized.contains('@') || normalized.len() < 5 {
        return Err(AppError::Validation(
            "Valid email address is required".to_string(),
        ));
    }
    if password.trim().len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    if env.enrollment_policy == "invite_only" {
        return Err(AppError::Forbidden(
            "Registration for this application is invite-only".to_string(),
        ));
    }

    let existing_verified = global_identity::find_verified_email(db, &normalized).await?;
    let existing_user = user_repo::find_user_by_email(db, &normalized).await?;
    let password_hash = hash_password_async(password.to_string()).await?;
    if existing_verified.is_some() || existing_user.is_some() {
        return Ok((
            ClientSignUpInitiatedResponse {
                status: "pending_verification".to_string(),
                email: normalized,
                message: "If this email is eligible, a verification code has been sent".to_string(),
            },
            String::new(),
        ));
    }

    let code = generate_otp_code();
    let clean_name = display_name.trim().to_string();
    let pending = PendingClientSignUp {
        email: normalized.clone(),
        display_name: clean_name.clone(),
        password_hash,
        code: hash_token(&code),
        attempts: 0,
    };

    let payload_json =
        serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.to_string()))?;
    let state_hash = signup_state_hash(environment_id, &normalized);
    let expires_at = Utc::now() + Duration::seconds(config.verification_code_expiry_secs as i64);

    let mut tx = begin_tx(db).await?;
    auth_tx_repo::delete_pending_by_state_in_tx(&mut tx, environment_id, "email_otp", &state_hash)
        .await?;
    auth_tx_repo::create_auth_transaction(
        &mut *tx,
        environment_id,
        "email_otp",
        Some(&state_hash),
        None,
        None,
        None,
        None,
        Some(&payload_json),
        None,
        None,
        expires_at,
    )
    .await?;
    commit_tx(tx).await?;

    let brand = crate::services::email::EmailBrand::from_environment_branding(
        &env.branding,
        &config.smtp_from_name,
    );
    let cfg = config.clone();
    let to_email = normalized.clone();
    let otp_for_mail = code.clone();
    crate::services::email::send_branded_otp_email(
        &cfg,
        &brand,
        &to_email,
        &clean_name,
        &otp_for_mail,
    )
    .await?;

    Ok((
        ClientSignUpInitiatedResponse {
            status: "pending_verification".to_string(),
            email: normalized,
            message: "If this email is eligible, a verification code has been sent".to_string(),
        },
        code,
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn verify_sign_up_otp(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    authorized_party: &str,
    email: &str,
    code: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ClientAuthResponse, AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    let state_hash = signup_state_hash(environment_id, &normalized);

    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let mut tx = begin_tx(db).await?;
    let auth_tx = auth_tx_repo::find_pending_auth_transaction_by_state_in_tx(
        &mut tx,
        environment_id,
        "email_otp",
        &state_hash,
    )
    .await?
    .ok_or_else(|| {
        AppError::InvalidToken("Verification code expired or registration not found".to_string())
    })?;

    let raw_payload = auth_tx
        .scope
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing registration payload".to_string()))?;
    let mut pending: PendingClientSignUp =
        serde_json::from_str(raw_payload).map_err(|e| AppError::Internal(e.to_string()))?;

    if !constant_time_str_eq(&pending.code, &hash_token(code.trim())) {
        pending.attempts += 1;
        if pending.attempts >= 5 {
            auth_tx_repo::revoke_auth_transaction_in_tx(&mut tx, auth_tx.id).await?;
            commit_tx(tx).await?;
            return Err(AppError::TooManyRequests(
                "Too many failed verification attempts".to_string(),
            ));
        }
        let updated_json =
            serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.to_string()))?;
        auth_tx_repo::update_auth_transaction_scope_in_tx(
            &mut tx,
            auth_tx.id,
            &updated_json,
            auth_tx.expires_at,
        )
        .await?;
        commit_tx(tx).await?;
        return Err(AppError::InvalidToken(
            "Invalid verification code".to_string(),
        ));
    }

    if user_repo::find_user_by_email(db, &pending.email)
        .await?
        .is_some()
    {
        return Err(AppError::EmailAlreadyExists);
    }

    auth_tx_repo::mark_auth_transaction_consumed_in_tx(&mut tx, auth_tx.id).await?;

    let mut new_user = User::new(
        pending.email.clone(),
        pending.display_name.clone(),
        Some(pending.password_hash),
    );
    new_user.email_verified = true;
    let created_user = user_repo::create_user(&mut *tx, new_user).await?;
    let user_id = UserId::from_uuid(created_user.id);

    let verified_email = global_identity::create_user_email(
        &mut *tx,
        user_id,
        &created_user.email,
        Some(Utc::now()),
        true,
    )
    .await?;

    sqlx::query("UPDATE users SET primary_email_id = $2 WHERE id = $1")
        .bind(created_user.id)
        .bind(verified_email.id)
        .execute(&mut *tx)
        .await?;

    let membership =
        tenant_memberships::create_application_user(&mut *tx, environment_id, user_id, None)
            .await?;
    tenant_rbac::assign_environment_role_in_tx(&mut tx, environment_id, membership.id, "user")
        .await?;
    let (roles, permissions) = tenant_rbac::resolve_environment_roles_and_permissions_in_tx(
        &mut tx,
        environment_id,
        membership.id,
    )
    .await?;

    let (session, refresh_token) = create_app_session_and_refresh_in_tx(
        &mut tx,
        environment_id,
        membership.id,
        user_id,
        env.session_ttl_secs as i64,
        user_agent,
        ip_address,
    )
    .await?;

    let user_evt = DomainEventEnvelope::new(
        DomainEventType::UserCreated,
        Some(environment_id.into_inner()),
        AggregateType::ApplicationUser,
        membership.id.into_inner(),
        membership.version,
        serde_json::json!({
            "sub": user_id,
            "app_user_id": membership.id,
            "email": created_user.email,
            "email_verified": true,
            "display_name": created_user.display_name,
            "status": membership.status,
            "version": membership.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &user_evt).await?;

    let member_evt = DomainEventEnvelope::new(
        DomainEventType::MembershipCreated,
        Some(environment_id.into_inner()),
        AggregateType::ApplicationUser,
        membership.id.into_inner(),
        membership.version,
        serde_json::json!({
            "sub": user_id,
            "app_user_id": membership.id,
            "roles": roles,
            "version": membership.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &member_evt).await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "EMAIL_VERIFIED",
        "application_user",
        Some(membership.id.into_inner()),
        ip_address,
        user_agent,
        serde_json::json!({ "method": "registration_otp" }),
    )
    .await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "LOGIN_SUCCESS",
        "app_session",
        Some(session.id.into_inner()),
        ip_address,
        user_agent,
        serde_json::json!({ "amr": ["pwd", "otp"] }),
    )
    .await?;

    commit_tx(tx).await?;

    let (access_token, _) = issue_tenant_access_token(
        db,
        config,
        IssueAccessTokenInput {
            issuer_url: &config.issuer_url,
            user_id,
            environment_id,
            authorized_party,
            app_user_id: membership.id,
            session_id: session.id,
            active_organization_id: None,
            roles: roles.clone(),
            permissions: permissions.clone(),
            amr: vec!["pwd".to_string(), "otp".to_string()],
            acr: "aal1".to_string(),
            version: membership.version,
            ttl_secs: env.access_token_ttl_secs as i64,
        },
    )
    .await?;

    crate::services::metrics::record_session_created();
    crate::services::metrics::record_sign_up("verified");

    Ok(ClientAuthResponse {
        user: ClientUserView {
            sub: user_id,
            app_user_id: membership.id,
            email: created_user.email,
            display_name: created_user.display_name,
            avatar_url: created_user.avatar_url,
            email_verified: true,
            status: membership.status,
            roles,
            permissions,
            version: membership.version,
        },
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: env.access_token_ttl_secs as i64,
        session_id: session.id,
    })
}

pub async fn resend_sign_up_otp(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    email: &str,
) -> Result<String, AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    let state_hash = signup_state_hash(environment_id, &normalized);

    let mut tx = begin_tx(db).await?;
    let auth_tx = auth_tx_repo::find_pending_auth_transaction_by_state_in_tx(
        &mut tx,
        environment_id,
        "email_otp",
        &state_hash,
    )
    .await?
    .ok_or_else(|| AppError::InvalidToken("No pending registration found".to_string()))?;

    let raw_payload = auth_tx
        .scope
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing registration payload".to_string()))?;
    let mut pending: PendingClientSignUp =
        serde_json::from_str(raw_payload).map_err(|e| AppError::Internal(e.to_string()))?;

    let new_code = generate_otp_code();
    pending.code = hash_token(&new_code);
    pending.attempts = 0;

    let updated_json =
        serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.to_string()))?;
    let expires_at = Utc::now() + Duration::seconds(config.verification_code_expiry_secs as i64);
    auth_tx_repo::update_auth_transaction_scope_in_tx(
        &mut tx,
        auth_tx.id,
        &updated_json,
        expires_at,
    )
    .await?;
    commit_tx(tx).await?;

    let brand = match tenant_repo::find_environment_by_id(db, environment_id).await? {
        Some(env) => crate::services::email::EmailBrand::from_environment_branding(
            &env.branding,
            &config.smtp_from_name,
        ),
        None => crate::services::email::EmailBrand::from_environment_branding(
            &serde_json::json!({}),
            &config.smtp_from_name,
        ),
    };
    let cfg = config.clone();
    let mail_code = new_code.clone();
    crate::services::email::send_branded_otp_email(
        &cfg,
        &brand,
        &pending.email,
        &pending.display_name,
        &mail_code,
    )
    .await?;

    Ok(new_code)
}

#[allow(clippy::too_many_arguments)]
pub async fn sign_in(
    db: &PgPool,
    redis: &RedisPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    authorized_party: &str,
    email: &str,
    password: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ClientAuthResponse, AppError> {
    let sign_in_started = std::time::Instant::now();
    let normalized = redis_ephemeral::normalize_email(email);
    let lockout_key = redis_keys::env_login_attempt_key(environment_id, &normalized);

    if let Ok(mut conn) = redis.get().await {
        let redis_started = std::time::Instant::now();
        let attempts_res: Result<Option<String>, _> = redis::cmd("GET")
            .arg(&lockout_key)
            .query_async(&mut conn)
            .await;
        crate::services::metrics::record_redis_op(
            "get",
            redis_started.elapsed().as_secs_f64(),
            attempts_res.is_err(),
        );
        let attempts = attempts_res.unwrap_or(None);
        if let Some(count) = attempts.and_then(|s| s.parse::<u32>().ok()) {
            if count >= config.login_max_attempts {
                crate::services::metrics::record_lockout();
                crate::services::metrics::record_sign_in(
                    "password",
                    "locked_out",
                    sign_in_started.elapsed().as_secs_f64(),
                );
                return Err(AppError::TooManyRequests(
                    "Too many failed login attempts".to_string(),
                ));
            }
        }
    }

    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let user = match find_global_user_by_email(db, &normalized).await? {
        Some(u) => u,
        None => {
            verify_dummy_password_async(password.to_string()).await;
            increment_env_failed_attempt(redis, &lockout_key, config.login_lockout_duration_secs)
                .await;
            crate::services::metrics::record_sign_in(
                "password",
                "failure",
                sign_in_started.elapsed().as_secs_f64(),
            );
            return Err(AppError::InvalidCredentials);
        }
    };

    let hash_str = match user.password_hash.as_deref() {
        Some(h) => h.to_string(),
        None => {
            verify_dummy_password_async(password.to_string()).await;
            return Err(AppError::InvalidCredentials);
        }
    };

    let verified_ok = verify_password_async(password.to_string(), hash_str).await?;

    if !verified_ok {
        increment_env_failed_attempt(redis, &lockout_key, config.login_lockout_duration_secs).await;
        crate::services::metrics::record_sign_in(
            "password",
            "failure",
            sign_in_started.elapsed().as_secs_f64(),
        );
        return Err(AppError::InvalidCredentials);
    }

    if !user.is_active() {
        return Err(AppError::Forbidden(
            "Global user account is suspended or inactive".to_string(),
        ));
    }

    if env.require_email_verification && !user.email_verified {
        return Err(AppError::Forbidden(
            "Email address must be verified before signing in".to_string(),
        ));
    }

    if let Ok(mut conn) = redis.get().await {
        let _: Result<(), _> = redis::cmd("DEL")
            .arg(&lockout_key)
            .query_async(&mut conn)
            .await;
    }

    let user_id = UserId::from_uuid(user.id);
    let mut tx = begin_tx(db).await?;

    let membership = match tenant_memberships::find_application_user_by_user_id(
        &mut *tx,
        environment_id,
        user_id,
    )
    .await?
    {
        Some(existing) => {
            if !existing.is_active() {
                return Err(AppError::Forbidden(
                    "Your membership in this application is suspended or revoked".to_string(),
                ));
            }
            existing
        }
        None => {
            if env.enrollment_policy == "invite_only" {
                return Err(AppError::Forbidden(
                    "An invitation is required to join this application".to_string(),
                ));
            }
            let created = tenant_memberships::create_application_user(
                &mut *tx,
                environment_id,
                user_id,
                None,
            )
            .await?;
            tenant_rbac::assign_environment_role_in_tx(&mut tx, environment_id, created.id, "user")
                .await?;
            let member_evt = DomainEventEnvelope::new(
                DomainEventType::MembershipCreated,
                Some(environment_id.into_inner()),
                AggregateType::ApplicationUser,
                created.id.into_inner(),
                created.version,
                serde_json::json!({
                    "sub": user_id,
                    "app_user_id": created.id,
                    "email": user.email,
                    "version": created.version
                }),
            );
            outbox::insert_outbox_event(&mut *tx, environment_id, &member_evt).await?;
            created
        }
    };

    let (roles, permissions) = tenant_rbac::resolve_environment_roles_and_permissions_in_tx(
        &mut tx,
        environment_id,
        membership.id,
    )
    .await?;

    let (session, refresh_token) = create_app_session_and_refresh_in_tx(
        &mut tx,
        environment_id,
        membership.id,
        user_id,
        env.session_ttl_secs as i64,
        user_agent,
        ip_address,
    )
    .await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "LOGIN_SUCCESS",
        "app_session",
        Some(session.id.into_inner()),
        ip_address,
        user_agent,
        serde_json::json!({ "amr": ["pwd"] }),
    )
    .await?;

    commit_tx(tx).await?;

    let (access_token, _) = issue_tenant_access_token(
        db,
        config,
        IssueAccessTokenInput {
            issuer_url: &config.issuer_url,
            user_id,
            environment_id,
            authorized_party,
            app_user_id: membership.id,
            session_id: session.id,
            active_organization_id: session.active_organization_id,
            roles: roles.clone(),
            permissions: permissions.clone(),
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: membership.version,
            ttl_secs: env.access_token_ttl_secs as i64,
        },
    )
    .await?;

    crate::services::metrics::record_session_created();
    crate::services::metrics::record_sign_in(
        "password",
        "success",
        sign_in_started.elapsed().as_secs_f64(),
    );

    Ok(ClientAuthResponse {
        user: ClientUserView {
            sub: user_id,
            app_user_id: membership.id,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            email_verified: user.email_verified,
            status: membership.status,
            roles,
            permissions,
            version: membership.version,
        },
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: env.access_token_ttl_secs as i64,
        session_id: session.id,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn refresh_tokens(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    authorized_party: &str,
    raw_refresh_token: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ClientTokenRefreshResponse, AppError> {
    let token_hash = hash_token(raw_refresh_token.trim());
    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let mut tx = begin_tx(db).await?;
    let locked = sqlx::query_as::<_, LockedRefreshTokenRow>(
        r#"
        SELECT id, app_session_id, user_id, family_id, expires_at, revoked_at
        FROM refresh_tokens
        WHERE token_hash = $1 AND environment_id = $2
        FOR UPDATE
        "#,
    )
    .bind(&token_hash)
    .bind(environment_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(token_row) = locked else {
        return Err(AppError::InvalidToken(
            "Invalid refresh token for this environment".to_string(),
        ));
    };

    let app_session_id = AppSessionId::from_uuid(
        token_row
            .app_session_id
            .ok_or_else(|| AppError::InvalidToken("Refresh token has no session".to_string()))?,
    );
    let user_id = UserId::from_uuid(token_row.user_id);

    let session =
        tenant_memberships::find_app_session_by_id(&mut *tx, environment_id, app_session_id)
            .await?
            .ok_or(AppError::SessionNotFound)?;

    if token_row.revoked_at.is_some() || session.revoked_at.is_some() {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = NOW()
            WHERE environment_id = $1 AND family_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(environment_id)
        .bind(token_row.family_id)
        .execute(&mut *tx)
        .await?;

        tenant_memberships::revoke_app_session(&mut *tx, environment_id, app_session_id).await?;

        record_client_audit_log_in_tx(
            &mut tx,
            Some(environment_id),
            Some(user_id),
            "REFRESH_TOKEN_REUSE_DETECTED",
            "app_session",
            Some(app_session_id.into_inner()),
            ip_address,
            user_agent,
            serde_json::json!({ "family_id": token_row.family_id }),
        )
        .await?;

        let revoke_evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(environment_id.into_inner()),
            AggregateType::AppSession,
            app_session_id.into_inner(),
            2,
            serde_json::json!({
                "session_id": app_session_id,
                "sub": user_id,
                "reason": "refresh_token_reuse_detected"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &revoke_evt).await?;
        commit_tx(tx).await?;

        crate::services::metrics::record_refresh_token_reuse();
        crate::services::metrics::record_session_revoked("refresh_token_reuse");

        return Err(AppError::InvalidToken(
            "Refresh token reuse detected".to_string(),
        ));
    }

    if Utc::now() > token_row.expires_at || Utc::now() > session.expires_at {
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = $1")
            .bind(token_row.id)
            .execute(&mut *tx)
            .await?;
        commit_tx(tx).await?;
        return Err(AppError::TokenExpired);
    }

    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;
    if !user.is_active() {
        return Err(AppError::Forbidden("Account is suspended".to_string()));
    }

    let membership =
        tenant_memberships::find_application_user_by_user_id(&mut *tx, environment_id, user_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Application membership not found".to_string()))?;
    if !membership.is_active() {
        return Err(AppError::Forbidden(
            "Application membership is suspended or revoked".to_string(),
        ));
    }

    sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE id = $1")
        .bind(token_row.id)
        .execute(&mut *tx)
        .await?;

    let new_refresh = generate_opaque_refresh_token();
    let new_refresh_hash = hash_token(&new_refresh);
    let new_expires_at = Utc::now() + Duration::seconds(env.session_ttl_secs as i64);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (
            id, user_id, environment_id, app_session_id, token_hash, family_id, expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id.into_inner())
    .bind(environment_id)
    .bind(app_session_id.into_inner())
    .bind(&new_refresh_hash)
    .bind(token_row.family_id)
    .bind(new_expires_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE app_sessions
        SET session_hash = $3, last_used_at = NOW(), expires_at = $4, updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        "#,
    )
    .bind(environment_id)
    .bind(app_session_id)
    .bind(&new_refresh_hash)
    .bind(new_expires_at)
    .execute(&mut *tx)
    .await?;

    let (roles, permissions) = tenant_rbac::resolve_environment_roles_and_permissions_in_tx(
        &mut tx,
        environment_id,
        membership.id,
    )
    .await?;

    let org_ctx = match session.active_organization_id {
        Some(active_org_id) => {
            tenant_rbac::resolve_organization_context_in_tx(
                &mut tx,
                environment_id,
                membership.id,
                active_org_id,
            )
            .await?
        }
        None => None,
    };

    commit_tx(tx).await?;

    let (access_token, _) = crate::services::tokens::issue_tenant_access_token_with_org(
        db,
        config,
        IssueAccessTokenInput {
            issuer_url: &config.issuer_url,
            user_id,
            environment_id,
            authorized_party,
            app_user_id: membership.id,
            session_id: app_session_id,
            active_organization_id: org_ctx.as_ref().and_then(|c| c.org_id),
            roles,
            permissions,
            amr: vec!["pwd".to_string()],
            acr: "aal1".to_string(),
            version: membership.version,
            ttl_secs: env.access_token_ttl_secs as i64,
        },
        org_ctx,
    )
    .await?;

    Ok(ClientTokenRefreshResponse {
        access_token,
        refresh_token: new_refresh,
        token_type: "Bearer".to_string(),
        expires_in: env.access_token_ttl_secs as i64,
        session_id: app_session_id,
    })
}

pub async fn sign_out(
    db: &PgPool,
    redis: &RedisPool,
    environment_id: EnvironmentId,
    user_id: UserId,
    session_id: AppSessionId,
    jti: &str,
    access_ttl_secs: u64,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    tenant_memberships::revoke_app_session(&mut *tx, environment_id, session_id).await?;
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND app_session_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(session_id.into_inner())
    .execute(&mut *tx)
    .await?;

    let revoke_evt = DomainEventEnvelope::new(
        DomainEventType::SessionRevoked,
        Some(environment_id.into_inner()),
        AggregateType::AppSession,
        session_id.into_inner(),
        2,
        serde_json::json!({
            "session_id": session_id,
            "sub": user_id,
            "reason": "sign_out"
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &revoke_evt).await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "LOGOUT",
        "app_session",
        Some(session_id.into_inner()),
        None,
        None,
        serde_json::json!({ "jti": jti }),
    )
    .await?;
    commit_tx(tx).await?;

    blacklist_env_token(redis, environment_id, jti, access_ttl_secs).await;
    Ok(())
}

pub async fn list_user_environment_sessions(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    current_session_id: AppSessionId,
) -> Result<Vec<ClientSessionView>, AppError> {
    let sessions = sqlx::query_as::<_, AppSession>(
        r#"
        SELECT id, environment_id, application_user_id, user_id, sso_session_id, session_hash, user_agent, host(ip_address) AS ip_address, active_organization_id, expires_at, last_used_at, revoked_at, created_at, updated_at
        FROM app_sessions
        WHERE environment_id = $1 AND application_user_id = $2 AND revoked_at IS NULL AND expires_at > NOW()
        ORDER BY last_used_at DESC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(db)
    .await?;

    Ok(sessions
        .into_iter()
        .map(|s| ClientSessionView {
            is_current: s.id == current_session_id,
            id: s.id,
            user_agent: s.user_agent,
            ip_address: s.ip_address,
            active_organization_id: s.active_organization_id,
            created_at: s.created_at,
            last_used_at: s.last_used_at,
            expires_at: s.expires_at,
        })
        .collect())
}

pub async fn revoke_single_session(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    user_id: UserId,
    target_session_id: AppSessionId,
) -> Result<(), AppError> {
    let mut tx = begin_tx(db).await?;
    let session =
        tenant_memberships::find_app_session_by_id(&mut *tx, environment_id, target_session_id)
            .await?
            .ok_or(AppError::SessionNotFound)?;

    if session.application_user_id != app_user_id {
        return Err(AppError::Forbidden(
            "Cannot revoke a session belonging to another user".to_string(),
        ));
    }

    tenant_memberships::revoke_app_session(&mut *tx, environment_id, target_session_id).await?;
    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND app_session_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(target_session_id.into_inner())
    .execute(&mut *tx)
    .await?;

    let evt = DomainEventEnvelope::new(
        DomainEventType::SessionRevoked,
        Some(environment_id.into_inner()),
        AggregateType::AppSession,
        target_session_id.into_inner(),
        2,
        serde_json::json!({
            "session_id": target_session_id,
            "sub": user_id,
            "app_user_id": app_user_id,
            "reason": "user_revoked"
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &evt).await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "SESSION_REVOKED",
        "app_session",
        Some(target_session_id.into_inner()),
        None,
        None,
        serde_json::json!({}),
    )
    .await?;
    commit_tx(tx).await?;
    Ok(())
}

pub async fn revoke_all_environment_sessions(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    user_id: UserId,
) -> Result<u64, AppError> {
    let mut tx = begin_tx(db).await?;
    let revoked_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1 AND application_user_id = $2 AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(user_id.into_inner())
    .execute(&mut *tx)
    .await?;

    for sess_id in &revoked_ids {
        let evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(environment_id.into_inner()),
            AggregateType::AppSession,
            *sess_id,
            2,
            serde_json::json!({
                "session_id": sess_id,
                "sub": user_id,
                "app_user_id": app_user_id,
                "reason": "revoke_all"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &evt).await?;
    }

    commit_tx(tx).await?;
    Ok(revoked_ids.len() as u64)
}

pub async fn request_password_reset(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    email: &str,
) -> Result<Option<String>, AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    let Some(user) = find_global_user_by_email(db, &normalized).await? else {
        return Ok(None);
    };

    let mut token_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut token_bytes);
    let raw_token = hex::encode(token_bytes);
    let token_hash = hash_token(&raw_token);
    let expires_at = Utc::now() + Duration::seconds(config.password_reset_expiry_secs as i64);

    auth_tx_repo::create_auth_transaction(
        db,
        environment_id,
        "password_reset",
        Some(&token_hash),
        None,
        None,
        None,
        None,
        None,
        None,
        Some(UserId::from_uuid(user.id)),
        expires_at,
    )
    .await?;

    let brand = match tenant_repo::find_environment_by_id(db, environment_id).await? {
        Some(env) => crate::services::email::EmailBrand::from_environment_branding(
            &env.branding,
            &config.smtp_from_name,
        ),
        None => crate::services::email::EmailBrand::from_environment_branding(
            &serde_json::json!({}),
            &config.smtp_from_name,
        ),
    };
    let cfg = config.clone();
    let token_copy = raw_token.clone();
    crate::services::email::send_branded_password_reset_email(
        &cfg,
        &brand,
        &user.email,
        &user.display_name,
        &token_copy,
    )
    .await?;

    Ok(Some(raw_token))
}

pub async fn complete_password_reset(
    db: &PgPool,
    environment_id: EnvironmentId,
    raw_token: &str,
    new_password: &str,
) -> Result<(), AppError> {
    if new_password.trim().len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let token_hash = hash_token(raw_token.trim());
    let password_hash = hash_password_async(new_password.to_string()).await?;

    let mut tx = begin_tx(db).await?;
    let auth_tx = auth_tx_repo::consume_auth_transaction_by_state_in_tx(
        &mut tx,
        environment_id,
        "password_reset",
        &token_hash,
    )
    .await?
    .ok_or_else(|| AppError::InvalidToken("Invalid or expired password reset token".to_string()))?;

    let user_id = auth_tx
        .user_id
        .ok_or_else(|| AppError::InvalidToken("Reset token has no user".to_string()))?;

    sqlx::query("UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id.into_inner())
        .bind(&password_hash)
        .execute(&mut *tx)
        .await?;

    revoke_all_global_sessions_in_tx(&mut tx, user_id, "password_reset").await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "PASSWORD_CHANGED",
        "user",
        Some(user_id.into_inner()),
        None,
        None,
        serde_json::json!({ "method": "password_reset" }),
    )
    .await?;

    commit_tx(tx).await?;
    Ok(())
}

pub async fn change_password(
    db: &PgPool,
    environment_id: EnvironmentId,
    user_id: UserId,
    current_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    if new_password.trim().len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;

    let existing_hash = user
        .password_hash
        .as_deref()
        .ok_or(AppError::InvalidCredentials)?
        .to_string();
    if !verify_password_async(current_password.to_string(), existing_hash).await? {
        return Err(AppError::InvalidCredentials);
    }

    let new_hash = hash_password_async(new_password.to_string()).await?;

    let mut tx = begin_tx(db).await?;
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id.into_inner())
        .bind(&new_hash)
        .execute(&mut *tx)
        .await?;
    revoke_all_global_sessions_in_tx(&mut tx, user_id, "password_changed").await?;
    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "PASSWORD_CHANGED",
        "user",
        Some(user_id.into_inner()),
        None,
        None,
        serde_json::json!({ "method": "password_change" }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(())
}

async fn revoke_all_global_sessions_in_tx(
    tx: &mut PgTx<'_>,
    user_id: UserId,
    reason: &str,
) -> Result<(), AppError> {
    let active_sessions: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE user_id = $1 AND revoked_at IS NULL
        RETURNING id, environment_id
        "#,
    )
    .bind(user_id.into_inner())
    .fetch_all(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user_id.into_inner())
    .execute(&mut **tx)
    .await?;

    global_identity::revoke_all_user_sso_sessions(&mut **tx, user_id).await?;

    for (sess_id, env_id) in active_sessions {
        let evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(env_id),
            AggregateType::AppSession,
            sess_id,
            2,
            serde_json::json!({
                "session_id": sess_id,
                "sub": user_id,
                "reason": reason
            }),
        );
        outbox::insert_outbox_event(&mut **tx, EnvironmentId::from_uuid(env_id), &evt).await?;
    }

    Ok(())
}

pub async fn get_client_user_profile(
    db: &PgPool,
    environment_id: EnvironmentId,
    user_id: UserId,
) -> Result<ClientUserView, AppError> {
    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;
    let membership =
        tenant_memberships::find_application_user_by_user_id(db, environment_id, user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;
    let (roles, permissions) =
        tenant_rbac::resolve_environment_roles_and_permissions(db, environment_id, membership.id)
            .await?;

    Ok(ClientUserView {
        sub: user_id,
        app_user_id: membership.id,
        email: user.email,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        email_verified: user.email_verified,
        status: membership.status,
        roles,
        permissions,
        version: membership.version,
    })
}

pub async fn update_client_user_profile(
    db: &PgPool,
    environment_id: EnvironmentId,
    user_id: UserId,
    display_name: &str,
    avatar_url: Option<&str>,
) -> Result<ClientUserView, AppError> {
    let clean_name = display_name.trim();
    if clean_name.is_empty() || clean_name.len() > 100 {
        return Err(AppError::Validation(
            "Display name must be between 1 and 100 characters".to_string(),
        ));
    }

    let mut tx = begin_tx(db).await?;
    let updated_user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET display_name = $2, avatar_url = $3, updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, display_name, password_hash, avatar_url, email_verified, status, created_at, updated_at
        "#,
    )
    .bind(user_id.into_inner())
    .bind(clean_name)
    .bind(avatar_url)
    .fetch_one(&mut *tx)
    .await?;

    let joined_memberships: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        r#"
        UPDATE application_users
        SET updated_at = NOW()
        WHERE user_id = $1
        RETURNING id, environment_id, version
        "#,
    )
    .bind(user_id.into_inner())
    .fetch_all(&mut *tx)
    .await?;

    for (app_user_uuid, env_uuid, version) in joined_memberships {
        let evt = DomainEventEnvelope::new(
            DomainEventType::UserUpdated,
            Some(env_uuid),
            AggregateType::ApplicationUser,
            app_user_uuid,
            version,
            serde_json::json!({
                "sub": user_id,
                "app_user_id": app_user_uuid,
                "email": updated_user.email,
                "display_name": updated_user.display_name,
                "avatar_url": updated_user.avatar_url,
                "version": version
            }),
        );
        outbox::insert_outbox_event(&mut *tx, EnvironmentId::from_uuid(env_uuid), &evt).await?;
    }

    commit_tx(tx).await?;
    get_client_user_profile(db, environment_id, user_id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryEmailVerificationResponse {
    pub status: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingSecondaryEmailVerification {
    user_id: UserId,
    email: String,
    code: String,
    attempts: u32,
}

fn secondary_email_state_hash(
    environment_id: EnvironmentId,
    user_id: UserId,
    email: &str,
) -> String {
    hash_token(&format!("verify_email:{environment_id}:{user_id}:{email}"))
}

pub async fn request_secondary_email_verification(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    user_id: UserId,
    email: &str,
) -> Result<(SecondaryEmailVerificationResponse, String), AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    if !normalized.contains('@') || normalized.len() < 5 {
        return Err(AppError::Validation(
            "Valid email address is required".to_string(),
        ));
    }

    if let Some(existing_verified) = global_identity::find_verified_email(db, &normalized).await? {
        let _ = existing_verified;
        return Ok((
            SecondaryEmailVerificationResponse {
                status: "verification_sent".to_string(),
                email: normalized,
            },
            String::new(),
        ));
    }

    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;
    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let code = generate_otp_code();
    let state_hash = secondary_email_state_hash(environment_id, user_id, &normalized);
    let payload = PendingSecondaryEmailVerification {
        user_id,
        email: normalized.clone(),
        code: hash_token(&code),
        attempts: 0,
    };
    let payload_json =
        serde_json::to_string(&payload).map_err(|e| AppError::Internal(e.to_string()))?;
    let expires_at = Utc::now() + Duration::seconds(config.verification_code_expiry_secs as i64);

    let mut tx = begin_tx(db).await?;
    sqlx::query(
        r#"
        INSERT INTO user_emails (user_id, email, is_primary, verified_at)
        VALUES ($1, $2, FALSE, NULL)
        ON CONFLICT (user_id, LOWER(email)) DO NOTHING
        "#,
    )
    .bind(user_id.into_inner())
    .bind(&normalized)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET status = 'revoked'
        WHERE environment_id = $1 AND flow_type = 'email_otp' AND state_hash = $2 AND status = 'pending'
        "#,
    )
    .bind(environment_id)
    .bind(&state_hash)
    .execute(&mut *tx)
    .await?;

    auth_tx_repo::create_auth_transaction(
        &mut *tx,
        environment_id,
        "email_otp",
        Some(&state_hash),
        None,
        None,
        None,
        None,
        Some(&payload_json),
        None,
        Some(user_id),
        expires_at,
    )
    .await?;
    commit_tx(tx).await?;

    let brand = crate::services::email::EmailBrand::from_environment_branding(
        &env.branding,
        &config.smtp_from_name,
    );
    let cfg = config.clone();
    let to_email = normalized.clone();
    let mail_code = code.clone();
    crate::services::email::send_branded_otp_email(
        &cfg,
        &brand,
        &to_email,
        &user.display_name,
        &mail_code,
    )
    .await?;

    Ok((
        SecondaryEmailVerificationResponse {
            status: "verification_sent".to_string(),
            email: normalized,
        },
        code,
    ))
}

pub async fn confirm_secondary_email_verification(
    db: &PgPool,
    environment_id: EnvironmentId,
    user_id: UserId,
    email: &str,
    code: &str,
) -> Result<SecondaryEmailVerificationResponse, AppError> {
    let normalized = redis_ephemeral::normalize_email(email);
    let state_hash = secondary_email_state_hash(environment_id, user_id, &normalized);

    let mut tx = begin_tx(db).await?;
    let auth_tx = auth_tx_repo::find_pending_auth_transaction_by_state_in_tx(
        &mut tx,
        environment_id,
        "email_otp",
        &state_hash,
    )
    .await?
    .ok_or_else(|| AppError::InvalidToken("Verification code expired or not found".to_string()))?;

    let raw_payload = auth_tx
        .scope
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing email verification payload".to_string()))?;
    let mut pending: PendingSecondaryEmailVerification =
        serde_json::from_str(raw_payload).map_err(|e| AppError::Internal(e.to_string()))?;

    if pending.user_id != user_id {
        return Err(AppError::Forbidden(
            "Verification code does not belong to current user".to_string(),
        ));
    }

    if !constant_time_str_eq(&pending.code, &hash_token(code.trim())) {
        pending.attempts += 1;
        if pending.attempts >= 5 {
            auth_tx_repo::revoke_auth_transaction_in_tx(&mut tx, auth_tx.id).await?;
            commit_tx(tx).await?;
            return Err(AppError::TooManyRequests(
                "Too many failed email verification attempts".to_string(),
            ));
        }
        let updated_json =
            serde_json::to_string(&pending).map_err(|e| AppError::Internal(e.to_string()))?;
        auth_tx_repo::update_auth_transaction_scope_in_tx(
            &mut tx,
            auth_tx.id,
            &updated_json,
            auth_tx.expires_at,
        )
        .await?;
        commit_tx(tx).await?;
        return Err(AppError::InvalidToken(
            "Invalid verification code".to_string(),
        ));
    }

    if let Some(existing_verified) =
        global_identity::find_verified_email(&mut *tx, &normalized).await?
    {
        if existing_verified.user_id != user_id {
            return Err(AppError::EmailAlreadyExists);
        }
    }

    auth_tx_repo::mark_auth_transaction_consumed_in_tx(&mut tx, auth_tx.id).await?;

    sqlx::query(
        r#"
        INSERT INTO user_emails (user_id, email, is_primary, verified_at)
        VALUES ($1, $2, FALSE, NOW())
        ON CONFLICT (user_id, LOWER(email)) DO UPDATE SET verified_at = NOW()
        "#,
    )
    .bind(user_id.into_inner())
    .bind(&normalized)
    .execute(&mut *tx)
    .await?;

    let memberships: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        r#"
        UPDATE application_users
        SET updated_at = NOW()
        WHERE user_id = $1
        RETURNING id, environment_id, version
        "#,
    )
    .bind(user_id.into_inner())
    .fetch_all(&mut *tx)
    .await?;

    for (app_user_uuid, env_uuid, version) in memberships {
        let evt = DomainEventEnvelope::new(
            DomainEventType::UserEmailVerified,
            Some(env_uuid),
            AggregateType::ApplicationUser,
            app_user_uuid,
            version,
            serde_json::json!({
                "sub": user_id,
                "app_user_id": app_user_uuid,
                "email": normalized,
                "email_verified": true,
                "version": version
            }),
        );
        outbox::insert_outbox_event(&mut *tx, EnvironmentId::from_uuid(env_uuid), &evt).await?;
    }

    record_client_audit_log_in_tx(
        &mut tx,
        Some(environment_id),
        Some(user_id),
        "EMAIL_VERIFIED",
        "user_email",
        Some(user_id.into_inner()),
        None,
        None,
        serde_json::json!({ "email": normalized }),
    )
    .await?;

    commit_tx(tx).await?;

    Ok(SecondaryEmailVerificationResponse {
        status: "verified".to_string(),
        email: normalized,
    })
}

async fn find_global_user_by_email(
    db: &PgPool,
    normalized: &str,
) -> Result<Option<User>, AppError> {
    if let Some(email_row) = global_identity::find_verified_email(db, normalized).await? {
        return user_repo::find_user_by_id(db, email_row.user_id.into_inner()).await;
    }
    user_repo::find_user_by_email(db, normalized).await
}

async fn increment_env_failed_attempt(redis: &RedisPool, key: &str, lockout_secs: u64) {
    if let Ok(mut conn) = redis.get().await {
        let count: Result<i64, _> = redis::cmd("INCR").arg(key).query_async(&mut conn).await;
        if let Ok(1) = count {
            let _: Result<(), _> = redis::cmd("EXPIRE")
                .arg(key)
                .arg(lockout_secs)
                .query_async(&mut conn)
                .await;
        }
    }
}
