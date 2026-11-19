use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Utc;
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm as JwtAlgorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{Developer, DeveloperId, WorkspaceRole};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::signing_keys::SigningKeyRecord;
use crate::repositories::{
    redis_ephemeral, redis_keys, signing_keys as key_repo, tenants as tenant_repo,
};
use crate::services::developer_security::{
    self, devices, factors, risk, verification, SignInClient,
};
use crate::services::signing_keys::{self, EcPublicJwk};
use crate::AppState;

pub const DEV_SESSION_COOKIE: &str = "dev_session_token";
pub const CONTROL_PLANE_AUDIENCE: &str = "control-plane";
pub const DEVELOPER_TOKEN_TYP: &str = "dev+jwt";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperTokenClaims {
    pub iss: String,
    pub sub: String,
    pub email: String,
    pub display_name: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperProfileResponse {
    pub id: DeveloperId,
    pub email: String,
    pub display_name: String,
    pub email_verified: bool,
    pub status: String,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<Developer> for DeveloperProfileResponse {
    fn from(d: Developer) -> Self {
        Self {
            id: d.id,
            email: d.email,
            display_name: d.display_name,
            email_verified: d.email_verified,
            status: d.status,
            created_at: d.created_at,
        }
    }
}

use crate::services::password_hasher::{
    hash_password_async, verify_dummy_password_async, verify_password_async,
};

fn issue_developer_token(
    active_key: SigningKeyRecord,
    developer: &Developer,
    config: &AppConfig,
    session_id: Uuid,
    expires_at: chrono::DateTime<Utc>,
) -> Result<String, AppError> {
    let pkcs8_der = signing_keys::decrypt_signing_key_pkcs8(&active_key, config)?;
    let now = Utc::now().timestamp();
    let claims = DeveloperTokenClaims {
        iss: config.issuer_url.clone(),
        sub: developer.id.to_string(),
        email: developer.email.clone(),
        display_name: developer.display_name.clone(),
        aud: CONTROL_PLANE_AUDIENCE.to_string(),
        iat: now,
        exp: expires_at.timestamp(),
        jti: session_id.to_string(),
    };

    let mut header = Header::new(JwtAlgorithm::ES256);
    header.kid = Some(active_key.kid);
    header.typ = Some(DEVELOPER_TOKEN_TYP.to_string());

    let key = EncodingKey::from_ec_der(&pkcs8_der);
    encode(&header, &claims, &key).map_err(|e| AppError::TokenCreation(e.to_string()))
}

pub async fn validate_developer_token(
    db: &PgPool,
    token: &str,
    config: &AppConfig,
) -> Result<DeveloperTokenClaims, AppError> {
    let header = decode_header(token)
        .map_err(|e| AppError::Unauthorized(format!("Invalid developer token header: {e}")))?;
    if header.alg != JwtAlgorithm::ES256 {
        return Err(AppError::Unauthorized(
            "Unsupported developer token algorithm".to_string(),
        ));
    }
    if header.typ.as_deref() != Some(DEVELOPER_TOKEN_TYP) {
        return Err(AppError::Unauthorized(
            "Invalid developer token type".to_string(),
        ));
    }
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("Missing kid in developer token".to_string()))?;

    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Unknown or revoked signing key".to_string()))?;
    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;

    let key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::Unauthorized(format!("Invalid JWK components: {e}")))?;
    let mut validation = Validation::new(JwtAlgorithm::ES256);
    validation.validate_exp = true;
    validation.set_issuer(&[config.issuer_url.as_str()]);
    validation.set_audience(&[CONTROL_PLANE_AUDIENCE]);

    decode::<DeveloperTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::Unauthorized(format!("Invalid developer session: {e}")))
}

pub fn build_dev_session_cookie<'a>(config: &AppConfig, token: String) -> Cookie<'a> {
    let mut cookie = Cookie::build((DEV_SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            config.jwt_refresh_expiry_secs as i64,
        ));
    if config.cookie_secure {
        cookie = cookie.secure(true);
    }
    if let Some(ref domain) = config.cookie_domain {
        cookie = cookie.domain(domain.clone());
    }
    cookie.build()
}

pub fn attach_dev_auth_cookies(jar: CookieJar, config: &AppConfig, token: String) -> CookieJar {
    let csrf_token = crate::middleware::generate_csrf_token();
    let csrf_cookie = crate::services::cookies::build_csrf_cookie(config, csrf_token);
    let dev_cookie = build_dev_session_cookie(config, token);
    jar.add(dev_cookie).add(csrf_cookie)
}

pub fn clear_dev_auth_cookies(jar: CookieJar, config: &AppConfig) -> CookieJar {
    let mut clear_dev = Cookie::build((DEV_SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);
    if config.cookie_secure {
        clear_dev = clear_dev.secure(true);
    }
    if let Some(ref domain) = config.cookie_domain {
        clear_dev = clear_dev.domain(domain.clone());
    }
    jar.add(clear_dev.build())
}

pub async fn register_developer(
    db: &PgPool,
    config: &AppConfig,
    email: &str,
    password: &str,
    display_name: &str,
    workspace_name: Option<&str>,
    client: SignInClient<'_>,
) -> Result<(DeveloperProfileResponse, String), AppError> {
    let normalized_email = redis_ephemeral::normalize_email(email);
    if !normalized_email.contains('@') || normalized_email.len() < 5 {
        return Err(AppError::Validation(
            "Valid email address is required".to_string(),
        ));
    }
    if password.trim().len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    if tenant_repo::find_developer_by_email(db, &normalized_email)
        .await?
        .is_some()
    {
        return Err(AppError::EmailAlreadyExists);
    }

    let password_hash = hash_password_async(password.to_string()).await?;

    let clean_name = display_name.trim();
    let ws_name = workspace_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if clean_name.is_empty() {
                "Personal Workspace".to_string()
            } else {
                format!("{clean_name}'s Workspace")
            }
        });
    let ws_slug = format!("ws-{}", &Uuid::new_v4().simple().to_string()[..12]);

    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let mut tx = begin_tx(db).await?;
    let developer = tenant_repo::create_developer(
        &mut *tx,
        &normalized_email,
        clean_name,
        Some(&password_hash),
    )
    .await?;
    let workspace = tenant_repo::create_workspace(&mut *tx, &ws_slug, &ws_name).await?;
    tenant_repo::add_workspace_member(&mut *tx, workspace.id, developer.id, WorkspaceRole::Owner)
        .await?;
    tenant_repo::record_control_audit_log(
        &mut *tx,
        None,
        developer.id,
        "DEVELOPER_REGISTERED",
        "developer",
        Some(developer.id.into_inner()),
        serde_json::json!({ "workspace_id": workspace.id }),
    )
    .await?;
    let session_id = Uuid::new_v4();
    let expires_at = session_expiry(config);
    let token = issue_developer_token(active_key, &developer, config, session_id, expires_at)?;
    developer_security::create_session(&mut tx, session_id, developer.id, expires_at, client, None)
        .await?;
    commit_tx(tx).await?;
    Ok((DeveloperProfileResponse::from(developer), token))
}

fn session_expiry(config: &AppConfig) -> chrono::DateTime<Utc> {
    Utc::now() + chrono::Duration::seconds(config.jwt_refresh_expiry_secs as i64)
}

pub async fn login_developer(
    state: &AppState,
    email: &str,
    password: &str,
    second_factor_code: Option<&str>,
    client: SignInClient<'_>,
) -> Result<(DeveloperProfileResponse, String), AppError> {
    let normalized_email = redis_ephemeral::normalize_email(email);
    let login_key = redis_keys::global_key("dev_login_attempt", &normalized_email);
    let db = &state.db;
    let redis = &state.redis;
    let config = &state.config;
    redis_ephemeral::check_attempts(redis, &login_key, config.login_max_attempts).await?;
    if risk::network_limit(state, client.ip, &normalized_email, false).await? {
        return Err(network_locked());
    }

    let account = tenant_repo::find_developer_by_email(db, &normalized_email).await?;
    let valid = if let Some(hash) = account
        .as_ref()
        .filter(|dev| dev.status == "active")
        .and_then(|dev| dev.password_hash.as_ref())
    {
        verify_password_async(password.to_string(), hash.clone()).await?
    } else {
        verify_dummy_password_async(password.to_string()).await;
        false
    };
    if !valid {
        let blocked = risk::network_limit(state, client.ip, &normalized_email, true).await?;
        if let Some(dev) = account {
            let event = if blocked {
                "DEVELOPER_CREDENTIAL_STUFFING"
            } else {
                "DEVELOPER_SIGN_IN_FAILED"
            };
            record_failure(state, dev.id, &login_key, client, event).await?;
        } else {
            redis_ephemeral::record_failed_attempt(
                redis,
                &login_key,
                config.login_lockout_duration_secs,
            )
            .await?;
        }
        return Err(if blocked {
            network_locked()
        } else {
            AppError::InvalidCredentials
        });
    };
    let developer = account.ok_or(AppError::InvalidCredentials)?;

    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let session_id = Uuid::new_v4();
    let expires_at = session_expiry(config);
    let token = issue_developer_token(active_key, &developer, config, session_id, expires_at)?;
    let mut tx = db.begin().await?;
    if verification::lock_account(&mut tx, developer.id).await? != developer.password_hash {
        return Err(AppError::InvalidCredentials);
    }
    let factor = match factors::verify_second_factor(
        &mut tx,
        config,
        developer.id,
        second_factor_code,
    )
    .await
    {
        Ok(factor) => factor,
        Err(error) => {
            tx.rollback().await?;
            if matches!(error, AppError::InvalidSecondFactor) {
                record_failure(
                    state,
                    developer.id,
                    &login_key,
                    client,
                    "DEVELOPER_SIGN_IN_FAILED",
                )
                .await?;
            }
            return Err(error);
        }
    };
    let device_id = devices::observe_sign_in(&mut tx, developer.id, client.trusted_device).await?;
    let flags = risk::inspect_sign_in(
        &mut tx,
        developer.id,
        client,
        device_id.is_some(),
        matches!(factor, factors::VerifiedFactor::BackupCode),
    )
    .await?;
    let preferences = risk::settings(&mut *tx, developer.id).await?;
    developer_security::create_session(
        &mut tx,
        session_id,
        developer.id,
        expires_at,
        client,
        device_id,
    )
    .await?;
    developer_security::record_activity(
        &mut *tx,
        developer.id,
        "DEVELOPER_SIGNED_IN",
        Some(client),
    )
    .await?;
    if !flags.is_empty() {
        developer_security::record_event(
            &mut *tx,
            developer.id,
            "DEVELOPER_SUSPICIOUS_SIGN_IN",
            Some(client),
            &flags,
        )
        .await?;
    }
    tx.commit().await?;
    if let Err(error) = redis_ephemeral::clear_attempts(redis, &login_key).await {
        tracing::warn!(error = %error, "Could not clear developer sign-in attempts");
    }
    risk::notify(
        state,
        developer.id,
        developer.email.clone(),
        preferences,
        !flags.is_empty(),
    );
    Ok((DeveloperProfileResponse::from(developer), token))
}

async fn record_failure(
    state: &AppState,
    id: DeveloperId,
    login_key: &str,
    client: SignInClient<'_>,
    event: &str,
) -> Result<(), AppError> {
    redis_ephemeral::record_failed_attempt(
        &state.redis,
        login_key,
        state.config.login_lockout_duration_secs,
    )
    .await?;
    developer_security::record_activity(&state.db, id, event, Some(client)).await
}

fn network_locked() -> AppError {
    metrics::counter!("developer_sign_in_blocked_total", "reason" => "credential_stuffing")
        .increment(1);
    AppError::TooManyRequests(
        "Too many sign-in attempts from this network. Try again later.".to_string(),
    )
}
