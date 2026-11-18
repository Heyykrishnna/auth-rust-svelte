use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version};
use axum_extra::extract::cookie::{Cookie, SameSite};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use deadpool_redis::Pool as RedisPool;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    AggregateType, AppSessionId, ApplicationEnvironment, DomainEventEnvelope, DomainEventType,
    EnrollmentPolicy, EnvironmentId, SsoSession, SsoSessionId, User, UserId,
};
use crate::repositories::db::{begin_tx, commit_tx, PgTx};
use crate::repositories::{
    global_identity, outbox, redis_ephemeral, tenant_memberships, tenant_rbac,
    tenants as tenant_repo, users as user_repo,
};
use crate::services::client_auth::{
    self, create_app_session_and_refresh_in_tx, record_client_audit_log_in_tx,
};
use crate::services::tokens::{
    hash_token, issue_tenant_access_token_with_scope, issue_tenant_id_token,
    verify_any_tenant_access_token, verify_tenant_id_token_hint, IdTokenClaims,
    IssueAccessTokenInput,
};

pub const PROD_SSO_COOKIE_NAME: &str = "__Host-idp_sso_session";
pub const DEV_SSO_COOKIE_NAME: &str = "idp_sso_session";
pub const AUTH_CODE_TTL_SECS: i64 = 60;
pub const OIDC_TX_TTL_SECS: i64 = 600;
pub const SSO_SESSION_TTL_SECS: i64 = 60 * 60 * 24 * 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscoveryDocument {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
    pub end_session_endpoint: String,
    pub revocation_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub claims_supported: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsoPolicyDecision {
    IssueCode,
    AutoJoinAndIssueCode,
    RequireLogin,
    RequireConsent,
    ErrorRedirect {
        error: &'static str,
        description: &'static str,
    },
}

#[derive(Debug, Clone)]
pub struct SsoPolicyInput {
    pub has_active_sso_session: bool,
    pub sso_authenticated_at: Option<DateTime<Utc>>,
    pub sso_amr: Vec<String>,
    pub user_active: bool,
    pub user_email_verified: bool,
    pub existing_membership_status: Option<String>,
    pub enrollment_policy: EnrollmentPolicy,
    pub require_email_verification: bool,
    pub require_mfa: bool,
    pub prompt: Option<String>,
    pub max_age: Option<i64>,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorizeQueryInput {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: Option<String>,
    pub nonce: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub prompt: Option<String>,
    pub max_age: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OidcTxMetadata {
    client_id: String,
    state: Option<String>,
    prompt: Option<String>,
    max_age: Option<i64>,
    portal_csrf_token: String,
    code_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct OidcTxRow {
    id: Uuid,
    environment_id: EnvironmentId,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    redirect_uri: Option<String>,
    scope: Option<String>,
    user_id: Option<UserId>,
    status: String,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
    issued_app_session_id: Option<Uuid>,
    sso_session_id: Option<Uuid>,
    request_metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum AuthorizeOutcome {
    Redirect(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalTransactionContext {
    pub tx_id: Uuid,
    pub application_name: String,
    pub application_slug: String,
    pub environment_kind: String,
    pub enrollment_policy: String,
    pub branding: serde_json::Value,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    pub csrf_token: String,
    pub requires_consent: bool,
    pub active_user: Option<PortalActiveUser>,
    pub social_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalActiveUser {
    pub sub: UserId,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalInteractionOutcome {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalConnectedAppView {
    pub environment_id: EnvironmentId,
    pub application_name: String,
    pub environment_kind: String,
    pub active_sessions: i64,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalSsoSessionView {
    pub id: SsoSessionId,
    pub is_current: bool,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub authenticated_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalAccountView {
    pub user: PortalActiveUser,
    pub sso_sessions: Vec<PortalSsoSessionView>,
    pub connected_apps: Vec<PortalConnectedAppView>,
}

#[derive(Debug, Clone)]
pub struct LogoutOutcome {
    pub redirect_url: Option<String>,
    pub clear_sso_cookie: bool,
    pub revoked_app_sessions: u64,
    pub global_logout: bool,
}

pub fn sso_cookie_name(config: &AppConfig) -> &'static str {
    if config.cookie_secure {
        PROD_SSO_COOKIE_NAME
    } else {
        DEV_SSO_COOKIE_NAME
    }
}

pub fn build_sso_session_cookie<'a>(config: &AppConfig, raw_token: String) -> Cookie<'a> {
    let mut builder = Cookie::build((sso_cookie_name(config), raw_token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(SSO_SESSION_TTL_SECS));
    if config.cookie_secure {
        builder = builder.secure(true);
    }
    builder.build()
}

pub fn build_clear_sso_cookie<'a>(config: &AppConfig) -> Cookie<'a> {
    let mut builder = Cookie::build((sso_cookie_name(config), ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::ZERO);
    if config.cookie_secure {
        builder = builder.secure(true);
    }
    builder.build()
}

pub fn build_discovery_document(config: &AppConfig) -> OidcDiscoveryDocument {
    let iss = config.issuer_url.trim_end_matches('/').to_string();
    OidcDiscoveryDocument {
        issuer: iss.clone(),
        authorization_endpoint: format!("{iss}/oauth2/authorize"),
        token_endpoint: format!("{iss}/oauth2/token"),
        userinfo_endpoint: format!("{iss}/oauth2/userinfo"),
        jwks_uri: format!("{iss}/.well-known/jwks.json"),
        end_session_endpoint: format!("{iss}/oauth2/logout"),
        revocation_endpoint: format!("{iss}/oauth2/revoke"),
        response_types_supported: vec!["code".to_string()],
        subject_types_supported: vec!["public".to_string()],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string()],
        id_token_signing_alg_values_supported: vec!["ES256".to_string()],
        token_endpoint_auth_methods_supported: vec![
            "none".to_string(),
            "client_secret_post".to_string(),
            "client_secret_basic".to_string(),
        ],
        scopes_supported: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "offline_access".to_string(),
        ],
        claims_supported: vec![
            "sub".to_string(),
            "iss".to_string(),
            "aud".to_string(),
            "exp".to_string(),
            "iat".to_string(),
            "nonce".to_string(),
            "auth_time".to_string(),
            "amr".to_string(),
            "acr".to_string(),
            "email".to_string(),
            "email_verified".to_string(),
            "name".to_string(),
            "picture".to_string(),
            "app_user_id".to_string(),
            "ver".to_string(),
        ],
    }
}

pub fn verify_pkce_s256(code_verifier: &str, expected_challenge: &str) -> bool {
    let v = code_verifier.trim();
    if v.len() < 43 || v.len() > 128 {
        return false;
    }
    if !v
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
    {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(v.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(hasher.finalize());
    let a = computed.as_bytes();
    let b = expected_challenge.as_bytes();
    a.len() == b.len() && bool::from(a.ct_eq(b))
}

pub fn evaluate_sso_policy(input: &SsoPolicyInput) -> SsoPolicyDecision {
    let prompt_tokens: Vec<&str> = input
        .prompt
        .as_deref()
        .map(|p| p.split_whitespace().collect())
        .unwrap_or_default();
    let prompt_none = prompt_tokens.contains(&"none");
    let prompt_login = prompt_tokens.contains(&"login");
    let prompt_consent = prompt_tokens.contains(&"consent");

    if !input.has_active_sso_session {
        if prompt_none {
            return SsoPolicyDecision::ErrorRedirect {
                error: "login_required",
                description: "No active IdP SSO session",
            };
        }
        return SsoPolicyDecision::RequireLogin;
    }

    if !input.user_active {
        return SsoPolicyDecision::ErrorRedirect {
            error: "access_denied",
            description: "Global user account is suspended or inactive",
        };
    }

    if prompt_login {
        if prompt_none {
            return SsoPolicyDecision::ErrorRedirect {
                error: "login_required",
                description: "Interactive login requested with prompt=none",
            };
        }
        return SsoPolicyDecision::RequireLogin;
    }

    if let Some(max_age) = input.max_age {
        let too_old = match input.sso_authenticated_at {
            Some(auth_at) => max_age <= 0 || (input.now - auth_at).num_seconds() > max_age,
            None => true,
        };
        if too_old {
            if prompt_none {
                return SsoPolicyDecision::ErrorRedirect {
                    error: "login_required",
                    description: "SSO session exceeded max_age",
                };
            }
            return SsoPolicyDecision::RequireLogin;
        }
    }

    if let Some(ref status) = input.existing_membership_status {
        if status != "active" {
            return SsoPolicyDecision::ErrorRedirect {
                error: "access_denied",
                description: "Application membership is suspended or revoked",
            };
        }
    }

    if input.require_email_verification && !input.user_email_verified {
        if prompt_none {
            return SsoPolicyDecision::ErrorRedirect {
                error: "interaction_required",
                description: "Verified email address required",
            };
        }
        return SsoPolicyDecision::RequireLogin;
    }

    if input.require_mfa {
        let has_mfa = input
            .sso_amr
            .iter()
            .any(|m| matches!(m.as_str(), "mfa" | "totp" | "webauthn" | "otp"));
        if !has_mfa {
            if prompt_none {
                return SsoPolicyDecision::ErrorRedirect {
                    error: "interaction_required",
                    description: "Multi-factor authentication required",
                };
            }
            return SsoPolicyDecision::RequireLogin;
        }
    }

    if input.existing_membership_status.is_some() {
        if prompt_consent {
            if prompt_none {
                return SsoPolicyDecision::ErrorRedirect {
                    error: "consent_required",
                    description: "Explicit consent requested with prompt=none",
                };
            }
            return SsoPolicyDecision::RequireConsent;
        }
        return SsoPolicyDecision::IssueCode;
    }

    match input.enrollment_policy {
        EnrollmentPolicy::AutoJoin => {
            if prompt_consent {
                if prompt_none {
                    SsoPolicyDecision::ErrorRedirect {
                        error: "consent_required",
                        description: "Consent required",
                    }
                } else {
                    SsoPolicyDecision::RequireConsent
                }
            } else {
                SsoPolicyDecision::AutoJoinAndIssueCode
            }
        }
        EnrollmentPolicy::ConsentRequired => {
            if prompt_none {
                SsoPolicyDecision::ErrorRedirect {
                    error: "consent_required",
                    description: "User consent is required to join this application",
                }
            } else {
                SsoPolicyDecision::RequireConsent
            }
        }
        EnrollmentPolicy::InviteOnly => SsoPolicyDecision::ErrorRedirect {
            error: "access_denied",
            description: "An invitation is required to join this application",
        },
    }
}

fn generate_random_urlsafe(num_bytes: usize) -> String {
    let mut bytes = vec![0u8; num_bytes];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn append_query_params(base_uri: &str, params: &[(&str, &str)]) -> String {
    let mut sep = if base_uri.contains('?') { '&' } else { '?' };
    let mut out = base_uri.to_string();
    for (k, v) in params {
        out.push(sep);
        out.push_str(&urlencoding::encode(k));
        out.push('=');
        out.push_str(&urlencoding::encode(v));
        sep = '&';
    }
    out
}

fn build_error_redirect(
    redirect_uri: &str,
    error: &str,
    description: &str,
    state: Option<&str>,
) -> String {
    let mut params = vec![("error", error), ("error_description", description)];
    if let Some(st) = state {
        params.push(("state", st));
    }
    append_query_params(redirect_uri, &params)
}

fn build_code_redirect(redirect_uri: &str, code: &str, state: Option<&str>) -> String {
    let mut params = vec![("code", code)];
    if let Some(st) = state {
        params.push(("state", st));
    }
    append_query_params(redirect_uri, &params)
}

fn constant_time_str_eq(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    ab.len() == bb.len() && bool::from(ab.ct_eq(bb))
}

fn create_argon2id() -> Argon2<'static> {
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default())
}

async fn resolve_oidc_client_environment(
    db: &PgPool,
    client_id: &str,
) -> Result<ApplicationEnvironment, AppError> {
    let trimmed = client_id.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("Missing client_id".to_string()));
    }

    if let Ok(env_id) = trimmed.parse::<EnvironmentId>() {
        if let Some(env) = tenant_repo::find_environment_by_id(db, env_id).await? {
            return Ok(env);
        }
    }

    if let Some(env) = tenant_repo::find_active_environment_by_publishable_key(db, trimmed).await? {
        return Ok(env);
    }

    Err(AppError::Validation("Unknown OIDC client_id".to_string()))
}

fn parse_enrollment_policy(raw: &str) -> EnrollmentPolicy {
    match raw {
        "auto_join" => EnrollmentPolicy::AutoJoin,
        "invite_only" => EnrollmentPolicy::InviteOnly,
        _ => EnrollmentPolicy::ConsentRequired,
    }
}

async fn is_exact_registered_redirect_uri(
    db: &PgPool,
    environment_id: EnvironmentId,
    candidate_uri: &str,
) -> Result<bool, AppError> {
    let domains = tenant_repo::list_environment_domains(db, environment_id).await?;
    Ok(domains
        .iter()
        .any(|d| d.redirect_uris.iter().any(|u| u == candidate_uri)))
}

async fn is_exact_registered_post_logout_uri(
    db: &PgPool,
    environment_id: EnvironmentId,
    candidate_uri: &str,
) -> Result<bool, AppError> {
    let domains = tenant_repo::list_environment_domains(db, environment_id).await?;
    let trimmed = candidate_uri.trim();
    Ok(domains.iter().any(|d| {
        d.redirect_uris.iter().any(|u| u == trimmed)
            || d.allowed_origins
                .iter()
                .any(|o| o == trimmed || o.trim_end_matches('/') == trimmed.trim_end_matches('/'))
    }))
}

async fn resolve_active_sso_session(
    db: &PgPool,
    raw_cookie: Option<&str>,
) -> Result<Option<(SsoSession, User)>, AppError> {
    let Some(token) = raw_cookie.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let token_hash = hash_token(token);
    let Some(sso) = global_identity::find_active_sso_session_by_hash(db, &token_hash).await? else {
        return Ok(None);
    };
    let Some(user) = user_repo::find_user_by_id(db, sso.user_id.into_inner()).await? else {
        return Ok(None);
    };
    Ok(Some((sso, user)))
}

#[allow(clippy::too_many_arguments)]
async fn ensure_membership_and_issue_code_in_tx(
    tx: &mut PgTx<'_>,
    tx_id: Uuid,
    env: &ApplicationEnvironment,
    user: &User,
    sso_session_id: SsoSessionId,
    auto_join: bool,
) -> Result<String, AppError> {
    let user_id = UserId::from_uuid(user.id);
    let existing =
        tenant_memberships::find_application_user_by_user_id(&mut **tx, env.id, user_id).await?;

    if existing.is_none() {
        if !auto_join {
            return Err(AppError::Forbidden(
                "Membership required before issuing authorization code".to_string(),
            ));
        }
        let created =
            tenant_memberships::create_application_user(&mut **tx, env.id, user_id, None).await?;
        tenant_rbac::assign_environment_role_in_tx(tx, env.id, created.id, "user").await?;
        let member_evt = DomainEventEnvelope::new(
            DomainEventType::MembershipCreated,
            Some(env.id.into_inner()),
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
        outbox::insert_outbox_event(&mut **tx, env.id, &member_evt).await?;
    }

    let raw_code = generate_random_urlsafe(32);
    let code_hash = hash_token(&raw_code);
    let code_expires_at = Utc::now() + Duration::seconds(AUTH_CODE_TTL_SECS);

    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET
            user_id = $2,
            sso_session_id = $3,
            authorization_code_hash = $4,
            status = 'code_issued',
            expires_at = $5,
            request_metadata = jsonb_set(request_metadata, '{code_expires_at}', to_jsonb($5::timestamptz), true),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(tx_id)
    .bind(user_id.into_inner())
    .bind(sso_session_id.into_inner())
    .bind(&code_hash)
    .bind(code_expires_at)
    .execute(&mut **tx)
    .await?;

    sqlx::query("UPDATE sso_sessions SET last_seen_at = NOW() WHERE id = $1")
        .bind(sso_session_id.into_inner())
        .execute(&mut **tx)
        .await?;

    Ok(raw_code)
}

pub async fn handle_authorize(
    db: &PgPool,
    config: &AppConfig,
    query: AuthorizeQueryInput,
    raw_sso_cookie: Option<&str>,
) -> Result<AuthorizeOutcome, AppError> {
    let env = resolve_oidc_client_environment(db, &query.client_id).await?;

    if !is_exact_registered_redirect_uri(db, env.id, &query.redirect_uri).await? {
        return Err(AppError::Validation(
            "Invalid or unregistered redirect_uri for this client".to_string(),
        ));
    }

    if query.response_type.trim() != "code" {
        return Ok(AuthorizeOutcome::Redirect(build_error_redirect(
            &query.redirect_uri,
            "unsupported_response_type",
            "Only response_type=code is supported",
            query.state.as_deref(),
        )));
    }

    let scopes: Vec<&str> = query.scope.split_whitespace().collect();
    if !scopes.contains(&"openid") {
        return Ok(AuthorizeOutcome::Redirect(build_error_redirect(
            &query.redirect_uri,
            "invalid_scope",
            "The openid scope is required",
            query.state.as_deref(),
        )));
    }

    let challenge = match query
        .code_challenge
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(c) => c,
        None => {
            return Ok(AuthorizeOutcome::Redirect(build_error_redirect(
                &query.redirect_uri,
                "invalid_request",
                "PKCE code_challenge is required",
                query.state.as_deref(),
            )));
        }
    };

    if query.code_challenge_method.as_deref() != Some("S256") {
        return Ok(AuthorizeOutcome::Redirect(build_error_redirect(
            &query.redirect_uri,
            "invalid_request",
            "PKCE code_challenge_method must be S256",
            query.state.as_deref(),
        )));
    }

    let portal_csrf_token = generate_random_urlsafe(32);
    let tx_metadata = OidcTxMetadata {
        client_id: query.client_id.trim().to_string(),
        state: query.state.clone(),
        prompt: query.prompt.clone(),
        max_age: query.max_age,
        portal_csrf_token,
        code_expires_at: None,
    };
    let metadata_json =
        serde_json::to_value(&tx_metadata).map_err(|e| AppError::Internal(e.to_string()))?;
    let tx_expires_at = Utc::now() + Duration::seconds(OIDC_TX_TTL_SECS);
    let state_hash = query
        .state
        .as_deref()
        .map(|s| hash_token(&format!("{}:{}", Uuid::new_v4(), s)));

    let tx_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO authentication_transactions (
            environment_id, flow_type, state_hash, nonce, code_challenge, code_challenge_method,
            redirect_uri, scope, status, expires_at, request_metadata
        )
        VALUES ($1, 'oidc_authorize', $2, $3, $4, 'S256', $5, $6, 'pending', $7, $8)
        RETURNING id
        "#,
    )
    .bind(env.id)
    .bind(state_hash)
    .bind(query.nonce.as_deref())
    .bind(challenge)
    .bind(&query.redirect_uri)
    .bind(scopes.join(" "))
    .bind(tx_expires_at)
    .bind(metadata_json)
    .fetch_one(db)
    .await?;

    let active_sso = resolve_active_sso_session(db, raw_sso_cookie).await?;
    let existing_membership = match &active_sso {
        Some((_, user)) => {
            tenant_memberships::find_application_user_by_user_id(
                db,
                env.id,
                UserId::from_uuid(user.id),
            )
            .await?
        }
        None => None,
    };

    let enrollment_policy = parse_enrollment_policy(&env.enrollment_policy);

    let policy_input = SsoPolicyInput {
        has_active_sso_session: active_sso.is_some(),
        sso_authenticated_at: active_sso.as_ref().map(|(s, _)| s.authenticated_at),
        sso_amr: active_sso
            .as_ref()
            .map(|(s, _)| s.amr.clone())
            .unwrap_or_default(),
        user_active: active_sso
            .as_ref()
            .map(|(_, u)| u.is_active())
            .unwrap_or(false),
        user_email_verified: active_sso
            .as_ref()
            .map(|(_, u)| u.email_verified)
            .unwrap_or(false),
        existing_membership_status: existing_membership.map(|m| m.status),
        enrollment_policy,
        require_email_verification: env.require_email_verification,
        require_mfa: env.require_mfa,
        prompt: query.prompt.clone(),
        max_age: query.max_age,
        now: Utc::now(),
    };

    let portal_origin = config.portal_origin();
    match evaluate_sso_policy(&policy_input) {
        SsoPolicyDecision::IssueCode => {
            let (sso, user) = active_sso.expect("sso present for IssueCode");
            let mut tx = begin_tx(db).await?;
            let code =
                ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, false)
                    .await?;
            commit_tx(tx).await?;
            Ok(AuthorizeOutcome::Redirect(build_code_redirect(
                &query.redirect_uri,
                &code,
                query.state.as_deref(),
            )))
        }
        SsoPolicyDecision::AutoJoinAndIssueCode => {
            let (sso, user) = active_sso.expect("sso present for AutoJoinAndIssueCode");
            let mut tx = begin_tx(db).await?;
            let code =
                ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, true)
                    .await?;
            commit_tx(tx).await?;
            Ok(AuthorizeOutcome::Redirect(build_code_redirect(
                &query.redirect_uri,
                &code,
                query.state.as_deref(),
            )))
        }
        SsoPolicyDecision::RequireLogin => Ok(AuthorizeOutcome::Redirect(format!(
            "{portal_origin}/portal/login?tx={tx_id}"
        ))),
        SsoPolicyDecision::RequireConsent => {
            if let Some((sso, user)) = active_sso {
                sqlx::query(
                    "UPDATE authentication_transactions SET user_id = $2, sso_session_id = $3, status = 'authenticated', updated_at = NOW() WHERE id = $1",
                )
                .bind(tx_id)
                .bind(user.id)
                .bind(sso.id.into_inner())
                .execute(db)
                .await?;
            }
            Ok(AuthorizeOutcome::Redirect(format!(
                "{portal_origin}/portal/consent?tx={tx_id}"
            )))
        }
        SsoPolicyDecision::ErrorRedirect { error, description } => {
            sqlx::query(
                "UPDATE authentication_transactions SET status = 'revoked', updated_at = NOW() WHERE id = $1",
            )
            .bind(tx_id)
            .execute(db)
            .await?;
            Ok(AuthorizeOutcome::Redirect(build_error_redirect(
                &query.redirect_uri,
                error,
                description,
                query.state.as_deref(),
            )))
        }
    }
}

pub async fn get_portal_transaction_context(
    db: &PgPool,
    config: &AppConfig,
    tx_id: Uuid,
    raw_sso_cookie: Option<&str>,
) -> Result<PortalTransactionContext, AppError> {
    let row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE id = $1 AND flow_type = 'oidc_authorize'
        "#,
    )
    .bind(tx_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Authorization transaction not found".to_string()))?;

    if matches!(row.status.as_str(), "consumed" | "revoked" | "expired")
        || Utc::now() > row.expires_at
    {
        return Err(AppError::InvalidToken(
            "Authorization transaction expired or already completed".to_string(),
        ));
    }

    let env = tenant_repo::find_environment_by_id(db, row.environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;
    let app = tenant_repo::find_application_by_id(db, env.application_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Application not found".to_string()))?;

    let meta: OidcTxMetadata = serde_json::from_value(row.request_metadata)
        .map_err(|e| AppError::Internal(format!("Corrupted tx metadata: {e}")))?;
    let active_sso = resolve_active_sso_session(db, raw_sso_cookie).await?;

    let mut social_providers = Vec::new();
    if config.google_client_id.is_some() {
        social_providers.push("google".to_string());
    }
    if config.github_client_id.is_some() {
        social_providers.push("github".to_string());
    }

    let scopes = row
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(ToString::to_string)
        .collect();

    Ok(PortalTransactionContext {
        tx_id: row.id,
        application_name: app.name,
        application_slug: app.slug,
        environment_kind: env.kind,
        enrollment_policy: env.enrollment_policy,
        branding: env.branding,
        scopes,
        redirect_uri: row.redirect_uri.unwrap_or_default(),
        csrf_token: meta.portal_csrf_token,
        requires_consent: row.status == "authenticated",
        active_user: active_sso.map(|(_, u)| PortalActiveUser {
            sub: UserId::from_uuid(u.id),
            email: u.email,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
        }),
        social_providers,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn portal_login_for_transaction(
    db: &PgPool,
    redis: &RedisPool,
    _config: &AppConfig,
    tx_id: Uuid,
    csrf_token: &str,
    email: &str,
    password: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(PortalInteractionOutcome, String), AppError> {
    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE id = $1 AND flow_type = 'oidc_authorize'
        FOR UPDATE
        "#,
    )
    .bind(tx_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Authorization transaction not found".to_string()))?;

    if !matches!(row.status.as_str(), "pending" | "authenticated") || Utc::now() > row.expires_at {
        return Err(AppError::InvalidToken(
            "Authorization transaction expired or already consumed".to_string(),
        ));
    }

    let meta: OidcTxMetadata = serde_json::from_value(row.request_metadata.clone())
        .map_err(|e| AppError::Internal(format!("Corrupted tx metadata: {e}")))?;

    if !constant_time_str_eq(&meta.portal_csrf_token, csrf_token.trim()) {
        return Err(AppError::Forbidden("Invalid portal CSRF token".to_string()));
    }

    let env = tenant_repo::find_environment_by_id(&mut *tx, row.environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let normalized = redis_ephemeral::normalize_email(email);
    let user = if let Some(verified_email) =
        global_identity::find_verified_email(&mut *tx, &normalized).await?
    {
        user_repo::find_user_by_id(db, verified_email.user_id.into_inner()).await?
    } else {
        user_repo::find_user_by_email(db, &normalized).await?
    }
    .ok_or(AppError::InvalidCredentials)?;

    let hash_str = user
        .password_hash
        .as_deref()
        .ok_or(AppError::InvalidCredentials)?;
    let parsed_hash = PasswordHash::new(hash_str).map_err(|_| AppError::InvalidCredentials)?;
    if create_argon2id()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_err()
    {
        let _ = redis;
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

    let user_id = UserId::from_uuid(user.id);
    let raw_sso_token = generate_random_urlsafe(32);
    let sso_hash = hash_token(&raw_sso_token);
    let sso_expires = Utc::now() + Duration::seconds(SSO_SESSION_TTL_SECS);
    let sso = global_identity::create_sso_session(
        &mut *tx,
        user_id,
        &sso_hash,
        &["pwd".to_string()],
        "aal1",
        user_agent,
        ip_address,
        sso_expires,
    )
    .await?;

    let existing_membership =
        tenant_memberships::find_application_user_by_user_id(&mut *tx, env.id, user_id).await?;
    let redirect_uri = row
        .redirect_uri
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing redirect_uri".to_string()))?;

    if let Some(ref member) = existing_membership {
        if !member.is_active() {
            sqlx::query(
                "UPDATE authentication_transactions SET status = 'revoked', updated_at = NOW() WHERE id = $1",
            )
            .bind(tx_id)
            .execute(&mut *tx)
            .await?;
            commit_tx(tx).await?;
            return Ok((
                PortalInteractionOutcome {
                    action: "redirect".to_string(),
                    redirect_url: Some(build_error_redirect(
                        redirect_uri,
                        "access_denied",
                        "Application membership is suspended or revoked",
                        meta.state.as_deref(),
                    )),
                    tx_id: None,
                },
                raw_sso_token,
            ));
        }

        let code =
            ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, false)
                .await?;
        record_client_audit_log_in_tx(
            &mut tx,
            Some(env.id),
            Some(user_id),
            "SSO_LOGIN_SUCCESS",
            "sso_session",
            Some(sso.id.into_inner()),
            ip_address,
            user_agent,
            serde_json::json!({ "amr": ["pwd"] }),
        )
        .await?;
        commit_tx(tx).await?;
        return Ok((
            PortalInteractionOutcome {
                action: "redirect".to_string(),
                redirect_url: Some(build_code_redirect(
                    redirect_uri,
                    &code,
                    meta.state.as_deref(),
                )),
                tx_id: None,
            },
            raw_sso_token,
        ));
    }

    let policy = parse_enrollment_policy(&env.enrollment_policy);

    match policy {
        EnrollmentPolicy::AutoJoin => {
            let code =
                ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, true)
                    .await?;
            commit_tx(tx).await?;
            Ok((
                PortalInteractionOutcome {
                    action: "redirect".to_string(),
                    redirect_url: Some(build_code_redirect(
                        redirect_uri,
                        &code,
                        meta.state.as_deref(),
                    )),
                    tx_id: None,
                },
                raw_sso_token,
            ))
        }
        EnrollmentPolicy::ConsentRequired => {
            sqlx::query(
                "UPDATE authentication_transactions SET user_id = $2, sso_session_id = $3, status = 'authenticated', updated_at = NOW() WHERE id = $1",
            )
            .bind(tx_id)
            .bind(user.id)
            .bind(sso.id.into_inner())
            .execute(&mut *tx)
            .await?;
            commit_tx(tx).await?;
            Ok((
                PortalInteractionOutcome {
                    action: "consent".to_string(),
                    redirect_url: None,
                    tx_id: Some(tx_id),
                },
                raw_sso_token,
            ))
        }
        EnrollmentPolicy::InviteOnly => {
            sqlx::query(
                "UPDATE authentication_transactions SET status = 'revoked', updated_at = NOW() WHERE id = $1",
            )
            .bind(tx_id)
            .execute(&mut *tx)
            .await?;
            commit_tx(tx).await?;
            Ok((
                PortalInteractionOutcome {
                    action: "redirect".to_string(),
                    redirect_url: Some(build_error_redirect(
                        redirect_uri,
                        "access_denied",
                        "An invitation is required to join this application",
                        meta.state.as_deref(),
                    )),
                    tx_id: None,
                },
                raw_sso_token,
            ))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn portal_consent_for_transaction(
    db: &PgPool,
    tx_id: Uuid,
    csrf_token: &str,
    approved: bool,
    raw_sso_cookie: Option<&str>,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<PortalInteractionOutcome, AppError> {
    let active_sso = resolve_active_sso_session(db, raw_sso_cookie)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Active SSO session required".to_string()))?;
    let (sso, user) = active_sso;

    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE id = $1 AND flow_type = 'oidc_authorize'
        FOR UPDATE
        "#,
    )
    .bind(tx_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Authorization transaction not found".to_string()))?;

    if !matches!(row.status.as_str(), "pending" | "authenticated") || Utc::now() > row.expires_at {
        return Err(AppError::InvalidToken(
            "Authorization transaction expired or already consumed".to_string(),
        ));
    }

    let meta: OidcTxMetadata = serde_json::from_value(row.request_metadata.clone())
        .map_err(|e| AppError::Internal(format!("Corrupted tx metadata: {e}")))?;

    if !constant_time_str_eq(&meta.portal_csrf_token, csrf_token.trim()) {
        return Err(AppError::Forbidden("Invalid portal CSRF token".to_string()));
    }

    let redirect_uri = row
        .redirect_uri
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing redirect_uri".to_string()))?;

    if !approved {
        sqlx::query(
            "UPDATE authentication_transactions SET status = 'revoked', updated_at = NOW() WHERE id = $1",
        )
        .bind(tx_id)
        .execute(&mut *tx)
        .await?;
        commit_tx(tx).await?;
        return Ok(PortalInteractionOutcome {
            action: "redirect".to_string(),
            redirect_url: Some(build_error_redirect(
                redirect_uri,
                "access_denied",
                "User denied application consent",
                meta.state.as_deref(),
            )),
            tx_id: None,
        });
    }

    let env = tenant_repo::find_environment_by_id(&mut *tx, row.environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    if !user.is_active() {
        return Err(AppError::Forbidden(
            "Global user account is suspended or inactive".to_string(),
        ));
    }

    let code =
        ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, true).await?;
    record_client_audit_log_in_tx(
        &mut tx,
        Some(env.id),
        Some(UserId::from_uuid(user.id)),
        "OIDC_CONSENT_GRANTED",
        "application_environment",
        Some(env.id.into_inner()),
        ip_address,
        user_agent,
        serde_json::json!({ "scope": row.scope }),
    )
    .await?;
    commit_tx(tx).await?;

    Ok(PortalInteractionOutcome {
        action: "redirect".to_string(),
        redirect_url: Some(build_code_redirect(
            redirect_uri,
            &code,
            meta.state.as_deref(),
        )),
        tx_id: None,
    })
}

pub async fn portal_initiate_sign_up_for_transaction(
    db: &PgPool,
    config: &AppConfig,
    tx_id: Uuid,
    csrf_token: &str,
    email: &str,
    password: &str,
    display_name: &str,
) -> Result<client_auth::ClientSignUpInitiatedResponse, AppError> {
    let row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE id = $1 AND flow_type = 'oidc_authorize'
        "#,
    )
    .bind(tx_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Authorization transaction not found".to_string()))?;

    if !matches!(row.status.as_str(), "pending" | "authenticated") || Utc::now() > row.expires_at {
        return Err(AppError::InvalidToken(
            "Authorization transaction expired or already consumed".to_string(),
        ));
    }

    let meta: OidcTxMetadata = serde_json::from_value(row.request_metadata)
        .map_err(|e| AppError::Internal(format!("Corrupted tx metadata: {e}")))?;

    if !constant_time_str_eq(&meta.portal_csrf_token, csrf_token.trim()) {
        return Err(AppError::Forbidden("Invalid portal CSRF token".to_string()));
    }

    let (res, _) = client_auth::initiate_sign_up(
        db,
        config,
        row.environment_id,
        email,
        password,
        display_name,
    )
    .await?;
    Ok(res)
}

#[allow(clippy::too_many_arguments)]
pub async fn portal_verify_sign_up_for_transaction(
    db: &PgPool,
    config: &AppConfig,
    tx_id: Uuid,
    csrf_token: &str,
    email: &str,
    code: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(PortalInteractionOutcome, String), AppError> {
    let tx_row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE id = $1 AND flow_type = 'oidc_authorize'
        "#,
    )
    .bind(tx_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Authorization transaction not found".to_string()))?;

    if !matches!(tx_row.status.as_str(), "pending" | "authenticated")
        || Utc::now() > tx_row.expires_at
    {
        return Err(AppError::InvalidToken(
            "Authorization transaction expired or already consumed".to_string(),
        ));
    }

    let meta: OidcTxMetadata = serde_json::from_value(tx_row.request_metadata.clone())
        .map_err(|e| AppError::Internal(format!("Corrupted tx metadata: {e}")))?;

    if !constant_time_str_eq(&meta.portal_csrf_token, csrf_token.trim()) {
        return Err(AppError::Forbidden("Invalid portal CSRF token".to_string()));
    }

    let auth_res = client_auth::verify_sign_up_otp(
        db,
        config,
        tx_row.environment_id,
        &meta.client_id,
        email,
        code,
        user_agent,
        ip_address,
    )
    .await?;

    let env = tenant_repo::find_environment_by_id(db, tx_row.environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;
    let user = user_repo::find_user_by_id(db, auth_res.user.sub.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;

    let mut tx = begin_tx(db).await?;
    let raw_sso_token = generate_random_urlsafe(32);
    let sso_hash = hash_token(&raw_sso_token);
    let sso_expires = Utc::now() + Duration::seconds(SSO_SESSION_TTL_SECS);
    let sso = global_identity::create_sso_session(
        &mut *tx,
        auth_res.user.sub,
        &sso_hash,
        &["pwd".to_string(), "otp".to_string()],
        "aal1",
        user_agent,
        ip_address,
        sso_expires,
    )
    .await?;

    let auth_code =
        ensure_membership_and_issue_code_in_tx(&mut tx, tx_id, &env, &user, sso.id, false).await?;
    commit_tx(tx).await?;

    let redirect_uri = tx_row
        .redirect_uri
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing redirect_uri".to_string()))?;

    Ok((
        PortalInteractionOutcome {
            action: "redirect".to_string(),
            redirect_url: Some(build_code_redirect(
                redirect_uri,
                &auth_code,
                meta.state.as_deref(),
            )),
            tx_id: None,
        },
        raw_sso_token,
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn exchange_authorization_code(
    db: &PgPool,
    config: &AppConfig,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<OidcTokenResponse, AppError> {
    let env = resolve_oidc_client_environment(db, client_id).await?;
    let code_hash = hash_token(code.trim());

    let mut tx = begin_tx(db).await?;
    let row = sqlx::query_as::<_, OidcTxRow>(
        r#"
        SELECT id, environment_id, nonce, code_challenge, code_challenge_method, redirect_uri,
               scope, user_id, status, expires_at, consumed_at,
               issued_app_session_id, sso_session_id, request_metadata
        FROM authentication_transactions
        WHERE authorization_code_hash = $1 AND flow_type = 'oidc_authorize'
        FOR UPDATE
        "#,
    )
    .bind(&code_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::InvalidToken("Invalid authorization code".to_string()))?;

    if row.status == "consumed" || row.consumed_at.is_some() {
        if let Some(issued_session_uuid) = row.issued_app_session_id {
            let sess_id = AppSessionId::from_uuid(issued_session_uuid);
            tenant_memberships::revoke_app_session(&mut *tx, row.environment_id, sess_id).await?;
            sqlx::query(
                r#"
                UPDATE refresh_tokens
                SET revoked_at = NOW()
                WHERE environment_id = $1 AND app_session_id = $2 AND revoked_at IS NULL
                "#,
            )
            .bind(row.environment_id)
            .bind(issued_session_uuid)
            .execute(&mut *tx)
            .await?;

            let evt = DomainEventEnvelope::new(
                DomainEventType::SessionRevoked,
                Some(row.environment_id.into_inner()),
                AggregateType::AppSession,
                issued_session_uuid,
                2,
                serde_json::json!({
                    "session_id": issued_session_uuid,
                    "sub": row.user_id,
                    "reason": "authorization_code_replay"
                }),
            );
            outbox::insert_outbox_event(&mut *tx, row.environment_id, &evt).await?;
        }

        record_client_audit_log_in_tx(
            &mut tx,
            Some(row.environment_id),
            row.user_id,
            "OIDC_CODE_REPLAY_DETECTED",
            "authentication_transaction",
            Some(row.id),
            ip_address,
            user_agent,
            serde_json::json!({ "issued_app_session_id": row.issued_app_session_id }),
        )
        .await?;
        commit_tx(tx).await?;
        return Err(AppError::InvalidToken(
            "Authorization code replay detected; issued session has been revoked".to_string(),
        ));
    }

    if row.status != "code_issued" || Utc::now() > row.expires_at {
        return Err(AppError::InvalidToken(
            "Authorization code expired or invalid".to_string(),
        ));
    }

    if row.environment_id != env.id {
        return Err(AppError::InvalidToken(
            "Authorization code does not belong to this client_id".to_string(),
        ));
    }

    if row.redirect_uri.as_deref() != Some(redirect_uri.trim()) {
        return Err(AppError::InvalidToken(
            "redirect_uri mismatch for authorization code".to_string(),
        ));
    }

    if row.code_challenge_method.as_deref() != Some("S256") {
        return Err(AppError::InvalidToken(
            "Unsupported PKCE code_challenge_method".to_string(),
        ));
    }

    let expected_challenge = row
        .code_challenge
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing PKCE challenge".to_string()))?;
    if !verify_pkce_s256(code_verifier, expected_challenge) {
        return Err(AppError::InvalidToken(
            "PKCE code_verifier verification failed".to_string(),
        ));
    }

    let user_id = row.user_id.ok_or_else(|| {
        AppError::Internal("Authorization transaction missing user_id".to_string())
    })?;
    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;
    if !user.is_active() {
        return Err(AppError::Forbidden(
            "Global user account is suspended or inactive".to_string(),
        ));
    }

    let membership =
        tenant_memberships::find_application_user_by_user_id(&mut *tx, env.id, user_id)
            .await?
            .ok_or_else(|| AppError::Forbidden("Application membership not found".to_string()))?;
    if !membership.is_active() {
        return Err(AppError::Forbidden(
            "Application membership is suspended or revoked".to_string(),
        ));
    }

    let (roles, permissions) = tenant_rbac::resolve_environment_roles_and_permissions_in_tx(
        &mut tx,
        env.id,
        membership.id,
    )
    .await?;

    let scope_str = row.scope.clone().unwrap_or_else(|| "openid".to_string());
    let scopes: Vec<&str> = scope_str.split_whitespace().collect();
    let wants_refresh = scopes.contains(&"offline_access");

    let (session, raw_refresh) = create_app_session_and_refresh_in_tx(
        &mut tx,
        env.id,
        membership.id,
        user_id,
        env.session_ttl_secs as i64,
        user_agent,
        ip_address,
    )
    .await?;

    if let Some(sso_uuid) = row.sso_session_id {
        sqlx::query("UPDATE app_sessions SET sso_session_id = $2 WHERE id = $1")
            .bind(session.id.into_inner())
            .bind(sso_uuid)
            .execute(&mut *tx)
            .await?;
    }

    let (auth_time, amr, acr) = if let Some(sso_uuid) = row.sso_session_id {
        let sso_meta: Option<(DateTime<Utc>, Vec<String>, String)> =
            sqlx::query_as("SELECT authenticated_at, amr, acr FROM sso_sessions WHERE id = $1")
                .bind(sso_uuid)
                .fetch_optional(&mut *tx)
                .await?;
        match sso_meta {
            Some((at, amr, acr)) => (at.timestamp(), amr, acr),
            None => (
                Utc::now().timestamp(),
                vec!["pwd".to_string()],
                "aal1".to_string(),
            ),
        }
    } else {
        (
            Utc::now().timestamp(),
            vec!["pwd".to_string()],
            "aal1".to_string(),
        )
    };

    sqlx::query(
        r#"
        UPDATE authentication_transactions
        SET
            status = 'consumed',
            consumed_at = NOW(),
            issued_app_session_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(row.id)
    .bind(session.id.into_inner())
    .execute(&mut *tx)
    .await?;

    record_client_audit_log_in_tx(
        &mut tx,
        Some(env.id),
        Some(user_id),
        "OIDC_TOKEN_ISSUED",
        "app_session",
        Some(session.id.into_inner()),
        ip_address,
        user_agent,
        serde_json::json!({ "scope": scope_str, "grant_type": "authorization_code" }),
    )
    .await?;

    commit_tx(tx).await?;

    let (access_token, _) = issue_tenant_access_token_with_scope(
        db,
        config,
        IssueAccessTokenInput {
            issuer_url: &config.issuer_url,
            user_id,
            environment_id: env.id,
            authorized_party: client_id.trim(),
            app_user_id: membership.id,
            session_id: session.id,
            active_organization_id: session.active_organization_id,
            roles,
            permissions,
            amr: amr.clone(),
            acr: acr.clone(),
            version: membership.version,
            ttl_secs: env.access_token_ttl_secs as i64,
        },
        &scope_str,
    )
    .await?;

    let now = Utc::now().timestamp();
    let id_claims = IdTokenClaims {
        iss: config.issuer_url.clone(),
        sub: user_id.to_string(),
        aud: env.id.to_string(),
        nonce: row.nonce,
        auth_time,
        amr,
        acr,
        email: user.email,
        email_verified: user.email_verified,
        name: user.display_name,
        picture: user.avatar_url,
        app_user_id: membership.id.to_string(),
        ver: membership.version,
        iat: now,
        exp: now + env.access_token_ttl_secs as i64,
    };
    let id_token = issue_tenant_id_token(db, config, &id_claims).await?;

    Ok(OidcTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: env.access_token_ttl_secs as i64,
        scope: scope_str,
        id_token: Some(id_token),
        refresh_token: if wants_refresh {
            Some(raw_refresh)
        } else {
            None
        },
    })
}

pub async fn exchange_refresh_token(
    db: &PgPool,
    config: &AppConfig,
    client_id: &str,
    refresh_token: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<OidcTokenResponse, AppError> {
    let env = resolve_oidc_client_environment(db, client_id).await?;
    let rotated = client_auth::refresh_tokens(
        db,
        config,
        env.id,
        client_id.trim(),
        refresh_token,
        user_agent,
        ip_address,
    )
    .await?;

    Ok(OidcTokenResponse {
        access_token: rotated.access_token,
        token_type: rotated.token_type,
        expires_in: rotated.expires_in,
        scope: "openid profile email offline_access".to_string(),
        id_token: None,
        refresh_token: Some(rotated.refresh_token),
    })
}

pub async fn get_userinfo(
    db: &PgPool,
    config: &AppConfig,
    bearer_token: &str,
) -> Result<serde_json::Value, AppError> {
    let claims = verify_any_tenant_access_token(db, config, bearer_token).await?;
    let scope_str = claims.scope.as_deref().unwrap_or("");
    let scopes: Vec<&str> = scope_str.split_whitespace().collect();
    if !scopes.contains(&"openid") {
        return Err(AppError::Forbidden(
            "Access token lacks required 'openid' scope for userinfo".to_string(),
        ));
    }

    let env_id: EnvironmentId = claims
        .aud
        .parse()
        .map_err(|_| AppError::InvalidToken("Invalid aud claim".to_string()))?;
    let user_id: UserId = claims
        .sub
        .parse()
        .map_err(|_| AppError::InvalidToken("Invalid sub claim".to_string()))?;
    let session_id: AppSessionId = claims
        .sid
        .parse()
        .map_err(|_| AppError::InvalidToken("Invalid sid claim".to_string()))?;

    let session = tenant_memberships::find_app_session_by_id(db, env_id, session_id)
        .await?
        .ok_or(AppError::SessionNotFound)?;
    if session.revoked_at.is_some() || Utc::now() > session.expires_at {
        return Err(AppError::Unauthorized(
            "Session has been revoked".to_string(),
        ));
    }

    let user = user_repo::find_user_by_id(db, user_id.into_inner())
        .await?
        .ok_or(AppError::UserNotFound)?;
    if !user.is_active() {
        return Err(AppError::Forbidden(
            "Global user account is inactive".to_string(),
        ));
    }

    let membership = tenant_memberships::find_application_user_by_user_id(db, env_id, user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;
    if !membership.is_active() {
        return Err(AppError::Forbidden(
            "Application membership is inactive".to_string(),
        ));
    }

    let mut out = serde_json::Map::new();
    out.insert("sub".to_string(), serde_json::json!(user_id));
    out.insert("app_user_id".to_string(), serde_json::json!(membership.id));
    out.insert("ver".to_string(), serde_json::json!(membership.version));
    out.insert(
        "public_metadata".to_string(),
        membership.public_metadata.clone(),
    );

    if scopes.contains(&"email") {
        out.insert("email".to_string(), serde_json::json!(user.email));
        out.insert(
            "email_verified".to_string(),
            serde_json::json!(user.email_verified),
        );
    }
    if scopes.contains(&"profile") {
        out.insert("name".to_string(), serde_json::json!(user.display_name));
        out.insert("picture".to_string(), serde_json::json!(user.avatar_url));
    }

    Ok(serde_json::Value::Object(out))
}

pub async fn revoke_oidc_token(
    db: &PgPool,
    raw_token: &str,
    client_id: Option<&str>,
) -> Result<(), AppError> {
    let token_hash = hash_token(raw_token.trim());
    let expected_env = match client_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(cid) => Some(resolve_oidc_client_environment(db, cid).await?.id),
        None => None,
    };

    let mut tx = begin_tx(db).await?;
    let found: Option<(Uuid, Option<Uuid>, Option<Uuid>, Uuid)> = sqlx::query_as(
        r#"
        SELECT family_id, environment_id, app_session_id, user_id
        FROM refresh_tokens
        WHERE token_hash = $1
        FOR UPDATE
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((family_id, Some(env_uuid), Some(sess_uuid), user_uuid)) = found else {
        commit_tx(tx).await?;
        return Ok(());
    };

    let env_id = EnvironmentId::from_uuid(env_uuid);
    if let Some(exp) = expected_env {
        if exp != env_id {
            commit_tx(tx).await?;
            return Ok(());
        }
    }

    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = NOW() WHERE family_id = $1 AND revoked_at IS NULL",
    )
    .bind(family_id)
    .execute(&mut *tx)
    .await?;

    tenant_memberships::revoke_app_session(&mut *tx, env_id, AppSessionId::from_uuid(sess_uuid))
        .await?;

    let evt = DomainEventEnvelope::new(
        DomainEventType::SessionRevoked,
        Some(env_uuid),
        AggregateType::AppSession,
        sess_uuid,
        2,
        serde_json::json!({
            "session_id": sess_uuid,
            "sub": user_uuid,
            "reason": "oidc_local_revoke"
        }),
    );
    outbox::insert_outbox_event(&mut *tx, env_id, &evt).await?;
    commit_tx(tx).await?;
    Ok(())
}

pub async fn handle_oidc_logout(
    db: &PgPool,
    config: &AppConfig,
    id_token_hint: Option<&str>,
    post_logout_redirect_uri: Option<&str>,
    state: Option<&str>,
    global: bool,
    raw_sso_cookie: Option<&str>,
) -> Result<LogoutOutcome, AppError> {
    let hint_claims = match id_token_hint.map(str::trim).filter(|s| !s.is_empty()) {
        Some(tok) => Some(verify_tenant_id_token_hint(db, config, tok).await?),
        None => None,
    };

    let hint_env_id: Option<EnvironmentId> = hint_claims
        .as_ref()
        .and_then(|c| c.aud.parse::<EnvironmentId>().ok());

    let validated_redirect = match post_logout_redirect_uri
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(uri) => {
            let env_id = hint_env_id.ok_or_else(|| {
                AppError::Validation(
                    "id_token_hint is required when post_logout_redirect_uri is provided"
                        .to_string(),
                )
            })?;
            if !is_exact_registered_post_logout_uri(db, env_id, uri).await? {
                return Err(AppError::Validation(
                    "Invalid or unregistered post_logout_redirect_uri".to_string(),
                ));
            }
            let target = match state.map(str::trim).filter(|s| !s.is_empty()) {
                Some(st) => append_query_params(uri, &[("state", st)]),
                None => uri.to_string(),
            };
            Some(target)
        }
        None => None,
    };

    let active_sso = resolve_active_sso_session(db, raw_sso_cookie).await?;
    let mut tx = begin_tx(db).await?;
    let mut revoked_count = 0u64;

    if global {
        if let Some((sso, user)) = active_sso {
            global_identity::revoke_sso_session(&mut *tx, sso.id).await?;
            let revoked_sessions: Vec<(Uuid, Uuid)> = sqlx::query_as(
                r#"
                UPDATE app_sessions
                SET revoked_at = NOW(), updated_at = NOW()
                WHERE (sso_session_id = $1 OR user_id = $2) AND revoked_at IS NULL
                RETURNING id, environment_id
                "#,
            )
            .bind(sso.id.into_inner())
            .bind(user.id)
            .fetch_all(&mut *tx)
            .await?;

            sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(user.id)
            .execute(&mut *tx)
            .await?;

            revoked_count = revoked_sessions.len() as u64;
            for (sess_id, env_uuid) in revoked_sessions {
                let evt = DomainEventEnvelope::new(
                    DomainEventType::SessionRevoked,
                    Some(env_uuid),
                    AggregateType::AppSession,
                    sess_id,
                    2,
                    serde_json::json!({
                        "session_id": sess_id,
                        "sub": user.id,
                        "reason": "oidc_global_logout"
                    }),
                );
                outbox::insert_outbox_event(&mut *tx, EnvironmentId::from_uuid(env_uuid), &evt)
                    .await?;
            }
        } else if let Some(ref claims) = hint_claims {
            if let Ok(uid) = claims.sub.parse::<UserId>() {
                global_identity::revoke_all_user_sso_sessions(&mut *tx, uid).await?;
                let revoked_sessions: Vec<(Uuid, Uuid)> = sqlx::query_as(
                    r#"
                    UPDATE app_sessions
                    SET revoked_at = NOW(), updated_at = NOW()
                    WHERE user_id = $1 AND revoked_at IS NULL
                    RETURNING id, environment_id
                    "#,
                )
                .bind(uid.into_inner())
                .fetch_all(&mut *tx)
                .await?;
                sqlx::query(
                    "UPDATE refresh_tokens SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
                )
                .bind(uid.into_inner())
                .execute(&mut *tx)
                .await?;
                revoked_count = revoked_sessions.len() as u64;
                for (sess_id, env_uuid) in revoked_sessions {
                    let evt = DomainEventEnvelope::new(
                        DomainEventType::SessionRevoked,
                        Some(env_uuid),
                        AggregateType::AppSession,
                        sess_id,
                        2,
                        serde_json::json!({
                            "session_id": sess_id,
                            "sub": uid,
                            "reason": "oidc_global_logout"
                        }),
                    );
                    outbox::insert_outbox_event(&mut *tx, EnvironmentId::from_uuid(env_uuid), &evt)
                        .await?;
                }
            }
        }
    } else if let (Some(env_id), Some(claims)) = (hint_env_id, hint_claims.as_ref()) {
        if let Ok(uid) = claims.sub.parse::<UserId>() {
            let revoked_sessions: Vec<Uuid> = sqlx::query_scalar(
                r#"
                UPDATE app_sessions
                SET revoked_at = NOW(), updated_at = NOW()
                WHERE environment_id = $1 AND user_id = $2 AND revoked_at IS NULL
                RETURNING id
                "#,
            )
            .bind(env_id)
            .bind(uid.into_inner())
            .fetch_all(&mut *tx)
            .await?;

            sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = NOW() WHERE environment_id = $1 AND user_id = $2 AND revoked_at IS NULL",
            )
            .bind(env_id)
            .bind(uid.into_inner())
            .execute(&mut *tx)
            .await?;

            revoked_count = revoked_sessions.len() as u64;
            for sess_id in revoked_sessions {
                let evt = DomainEventEnvelope::new(
                    DomainEventType::SessionRevoked,
                    Some(env_id.into_inner()),
                    AggregateType::AppSession,
                    sess_id,
                    2,
                    serde_json::json!({
                        "session_id": sess_id,
                        "sub": uid,
                        "reason": "oidc_local_logout"
                    }),
                );
                outbox::insert_outbox_event(&mut *tx, env_id, &evt).await?;
            }
        }
    }

    commit_tx(tx).await?;

    Ok(LogoutOutcome {
        redirect_url: validated_redirect,
        clear_sso_cookie: global,
        revoked_app_sessions: revoked_count,
        global_logout: global,
    })
}

pub async fn get_portal_account_view(
    db: &PgPool,
    raw_sso_cookie: Option<&str>,
) -> Result<PortalAccountView, AppError> {
    let (current_sso, user) = resolve_active_sso_session(db, raw_sso_cookie)
        .await?
        .ok_or_else(|| AppError::Unauthorized("No active IdP SSO session".to_string()))?;

    let sso_rows = sqlx::query_as::<_, SsoSession>(
        r#"
        SELECT id, user_id, session_token_hash, authenticated_at, amr, acr, user_agent,
               host(ip_address) AS ip_address, expires_at, last_seen_at, revoked_at, created_at, updated_at
        FROM sso_sessions
        WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
        ORDER BY last_seen_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(db)
    .await?;

    #[derive(FromRow)]
    struct ConnectedRow {
        environment_id: EnvironmentId,
        application_name: String,
        environment_kind: String,
        active_sessions: i64,
        joined_at: DateTime<Utc>,
    }

    let connected_rows = sqlx::query_as::<_, ConnectedRow>(
        r#"
        SELECT
            au.environment_id,
            a.name AS application_name,
            ae.kind AS environment_kind,
            COUNT(s.id) FILTER (WHERE s.revoked_at IS NULL AND s.expires_at > NOW()) AS active_sessions,
            au.joined_at
        FROM application_users au
        JOIN application_environments ae ON ae.id = au.environment_id
        JOIN applications a ON a.id = ae.application_id
        LEFT JOIN app_sessions s ON s.environment_id = au.environment_id AND s.application_user_id = au.id
        WHERE au.user_id = $1 AND au.status = 'active'
        GROUP BY au.environment_id, a.name, ae.kind, au.joined_at
        ORDER BY au.joined_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(db)
    .await?;

    Ok(PortalAccountView {
        user: PortalActiveUser {
            sub: UserId::from_uuid(user.id),
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
        },
        sso_sessions: sso_rows
            .into_iter()
            .map(|s| PortalSsoSessionView {
                is_current: s.id == current_sso.id,
                id: s.id,
                user_agent: s.user_agent,
                ip_address: s.ip_address,
                authenticated_at: s.authenticated_at,
                last_seen_at: s.last_seen_at,
                expires_at: s.expires_at,
            })
            .collect(),
        connected_apps: connected_rows
            .into_iter()
            .map(|r| PortalConnectedAppView {
                environment_id: r.environment_id,
                application_name: r.application_name,
                environment_kind: r.environment_kind,
                active_sessions: r.active_sessions,
                joined_at: r.joined_at,
            })
            .collect(),
    })
}

pub async fn revoke_portal_sso_session(
    db: &PgPool,
    raw_sso_cookie: Option<&str>,
    target_sso_id: SsoSessionId,
) -> Result<(), AppError> {
    let (_, user) = resolve_active_sso_session(db, raw_sso_cookie)
        .await?
        .ok_or_else(|| AppError::Unauthorized("No active IdP SSO session".to_string()))?;

    let mut tx = begin_tx(db).await?;
    let owner: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM sso_sessions WHERE id = $1")
        .bind(target_sso_id.into_inner())
        .fetch_optional(&mut *tx)
        .await?;

    if owner != Some(user.id) {
        return Err(AppError::Forbidden(
            "Cannot revoke an SSO session belonging to another user".to_string(),
        ));
    }

    global_identity::revoke_sso_session(&mut *tx, target_sso_id).await?;
    let revoked_sessions: Vec<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE sso_session_id = $1 AND revoked_at IS NULL
        RETURNING id, environment_id
        "#,
    )
    .bind(target_sso_id.into_inner())
    .fetch_all(&mut *tx)
    .await?;

    for (sess_id, env_uuid) in revoked_sessions {
        let evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(env_uuid),
            AggregateType::AppSession,
            sess_id,
            2,
            serde_json::json!({
                "session_id": sess_id,
                "sub": user.id,
                "reason": "portal_sso_session_revoked"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, EnvironmentId::from_uuid(env_uuid), &evt).await?;
    }

    commit_tx(tx).await?;
    Ok(())
}
