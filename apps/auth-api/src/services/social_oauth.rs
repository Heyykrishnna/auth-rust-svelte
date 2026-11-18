use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    AggregateType, DomainEventEnvelope, DomainEventType, EnvironmentId, EnvironmentKind, User,
    UserId,
};
use crate::repositories::auth_transactions as auth_tx_repo;
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::{
    global_identity, outbox, redis_ephemeral, tenant_memberships, tenant_rbac,
    tenants as tenant_repo, users as user_repo,
};
use crate::services::client_auth::{
    create_app_session_and_refresh_in_tx, record_client_audit_log_in_tx, ClientAuthResponse,
    ClientUserView,
};
use crate::services::domains as domain_service;
use crate::services::signing_keys::{decrypt_envelope, encrypt_envelope, OAUTH_SECRET_AAD_PURPOSE};
use crate::services::tokens::{hash_token, issue_tenant_access_token, IssueAccessTokenInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialOAuthStartResponse {
    pub authorization_url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedSocialIdentity {
    pub provider: String,
    pub provider_user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSocialOAuthTx {
    provider: String,
    code_verifier: String,
    nonce: String,
    redirect_uri: String,
}

#[derive(Debug, FromRow)]
struct EnvOAuthProviderRow {
    enabled: bool,
    client_id: Option<String>,
    client_secret_encrypted: Option<Vec<u8>>,
    kek_version: Option<String>,
    use_platform_credentials: bool,
}

pub struct ResolvedOAuthProviderCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub default_redirect_uri: Option<String>,
}

pub fn generate_pkce_s256_pair() -> (String, String) {
    let mut verifier_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut verifier_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

fn generate_random_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub async fn resolve_provider_credentials(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    provider: &str,
) -> Result<ResolvedOAuthProviderCredentials, AppError> {
    let normalized = provider.trim().to_lowercase();

    let override_row = sqlx::query_as::<_, EnvOAuthProviderRow>(
        r#"
        SELECT enabled, client_id, client_secret_encrypted, kek_version, use_platform_credentials
        FROM application_oauth_providers
        WHERE environment_id = $1 AND provider = $2
        LIMIT 1
        "#,
    )
    .bind(environment_id)
    .bind(&normalized)
    .fetch_optional(db)
    .await?;

    if let Some(row) = override_row {
        if !row.enabled {
            return Err(AppError::Forbidden(format!(
                "OAuth provider '{normalized}' is disabled for this environment"
            )));
        }
        if !row.use_platform_credentials {
            let client_id = row.client_id.ok_or_else(|| {
                AppError::OidcError(format!("Missing client_id for provider '{normalized}'"))
            })?;
            let encrypted = row.client_secret_encrypted.ok_or_else(|| {
                AppError::OidcError(format!("Missing client_secret for provider '{normalized}'"))
            })?;
            let kek_version = row
                .kek_version
                .unwrap_or_else(|| config.signing_key_kek_version.clone());
            let decrypted = decrypt_envelope(
                &encrypted,
                &config.signing_key_kek,
                &kek_version,
                OAUTH_SECRET_AAD_PURPOSE,
            )?;
            let client_secret = String::from_utf8(decrypted.to_vec())
                .map_err(|_| AppError::Internal("Corrupted OAuth client secret".to_string()))?;
            return Ok(ResolvedOAuthProviderCredentials {
                client_id,
                client_secret,
                default_redirect_uri: None,
            });
        }
    }

    match normalized.as_str() {
        "google" => Ok(ResolvedOAuthProviderCredentials {
            client_id: config.google_client_id.clone().ok_or_else(|| {
                AppError::OidcError("Google client ID not configured".to_string())
            })?,
            client_secret: config.google_client_secret.clone().ok_or_else(|| {
                AppError::OidcError("Google client secret not configured".to_string())
            })?,
            default_redirect_uri: config.google_redirect_uri.clone(),
        }),
        "github" => Ok(ResolvedOAuthProviderCredentials {
            client_id: config.github_client_id.clone().ok_or_else(|| {
                AppError::OidcError("GitHub client ID not configured".to_string())
            })?,
            client_secret: config.github_client_secret.clone().ok_or_else(|| {
                AppError::OidcError("GitHub client secret not configured".to_string())
            })?,
            default_redirect_uri: config.github_redirect_uri.clone(),
        }),
        _ => Err(AppError::Validation(format!(
            "Unsupported social OAuth provider: {normalized}"
        ))),
    }
}

pub async fn start_social_oauth(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    provider: &str,
    redirect_uri_override: Option<&str>,
) -> Result<SocialOAuthStartResponse, AppError> {
    let normalized_provider = provider.trim().to_lowercase();
    if normalized_provider != "google" && normalized_provider != "github" {
        return Err(AppError::Validation(format!(
            "Unsupported social OAuth provider: {normalized_provider}"
        )));
    }

    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;
    let env_kind = EnvironmentKind::from_db_str(&env.kind)
        .ok_or_else(|| AppError::Internal("Invalid environment kind".to_string()))?;

    let creds =
        resolve_provider_credentials(db, config, environment_id, &normalized_provider).await?;
    let raw_redirect = redirect_uri_override
        .map(str::to_string)
        .or(creds.default_redirect_uri)
        .ok_or_else(|| AppError::Validation("redirect_uri is required".to_string()))?;
    let redirect_uri = domain_service::validate_redirect_uri(env_kind, &raw_redirect)?;

    let state = generate_random_token();
    let state_hash = hash_token(&state);
    let nonce = generate_random_token();
    let (code_verifier, code_challenge) = generate_pkce_s256_pair();

    let stored = StoredSocialOAuthTx {
        provider: normalized_provider.clone(),
        code_verifier,
        nonce: nonce.clone(),
        redirect_uri: redirect_uri.clone(),
    };
    let stored_json = serde_json::to_vec(&stored).map_err(|e| AppError::Internal(e.to_string()))?;
    let encrypted_tx_payload = encrypt_envelope(
        &stored_json,
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        OAUTH_SECRET_AAD_PURPOSE,
    )?;
    let encoded_scope = URL_SAFE_NO_PAD.encode(encrypted_tx_payload);

    let expires_at = Utc::now() + Duration::minutes(10);
    auth_tx_repo::create_auth_transaction(
        db,
        environment_id,
        "social_oauth",
        Some(&state_hash),
        Some(&nonce),
        Some(&code_challenge),
        Some("S256"),
        Some(&redirect_uri),
        Some(&encoded_scope),
        None,
        None,
        expires_at,
    )
    .await?;

    let authorization_url = match normalized_provider.as_str() {
        "google" => format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid%20email%20profile&state={}&nonce={}&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&creds.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&state),
            urlencoding::encode(&nonce),
            urlencoding::encode(&code_challenge)
        ),
        "github" => format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=read%3Auser%20user%3Aemail&state={}&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&creds.client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge)
        ),
        _ => unreachable!(),
    };

    Ok(SocialOAuthStartResponse {
        authorization_url,
        state,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_social_oauth_callback(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    authorized_party: &str,
    provider: &str,
    code: &str,
    state: &str,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ClientAuthResponse, AppError> {
    if code.trim().is_empty() || state.trim().is_empty() {
        return Err(AppError::Validation(
            "Both code and state parameters are required".to_string(),
        ));
    }

    let normalized_provider = provider.trim().to_lowercase();
    let state_hash = hash_token(state.trim());

    let mut tx = begin_tx(db).await?;
    let auth_tx = auth_tx_repo::consume_auth_transaction_by_state_in_tx(
        &mut tx,
        environment_id,
        "social_oauth",
        &state_hash,
    )
    .await?
    .ok_or_else(|| {
        AppError::InvalidToken("Invalid, expired, or replayed OAuth state".to_string())
    })?;
    commit_tx(tx).await?;

    let encoded_scope = auth_tx
        .scope
        .as_deref()
        .ok_or_else(|| AppError::Internal("Missing OAuth transaction payload".to_string()))?;
    let encrypted_bytes = URL_SAFE_NO_PAD
        .decode(encoded_scope)
        .map_err(|_| AppError::Internal("Corrupted OAuth transaction encoding".to_string()))?;
    let decrypted_bytes = decrypt_envelope(
        &encrypted_bytes,
        &config.signing_key_kek,
        &config.signing_key_kek_version,
        OAUTH_SECRET_AAD_PURPOSE,
    )?;
    let stored: StoredSocialOAuthTx = serde_json::from_slice(&decrypted_bytes)
        .map_err(|_| AppError::Internal("Corrupted OAuth transaction payload".to_string()))?;

    if stored.provider != normalized_provider {
        return Err(AppError::InvalidToken(
            "OAuth state does not match callback provider".to_string(),
        ));
    }

    let creds =
        resolve_provider_credentials(db, config, environment_id, &normalized_provider).await?;

    let identity = match normalized_provider.as_str() {
        "google" => {
            exchange_google_code(
                &creds,
                code.trim(),
                &stored.redirect_uri,
                &stored.code_verifier,
                &stored.nonce,
            )
            .await?
        }
        "github" => {
            exchange_github_code(
                &creds,
                code.trim(),
                &stored.redirect_uri,
                &stored.code_verifier,
            )
            .await?
        }
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported provider: {normalized_provider}"
            )));
        }
    };

    complete_social_login_with_identity(
        db,
        config,
        environment_id,
        authorized_party,
        identity,
        user_agent,
        ip_address,
    )
    .await
}

pub fn validate_google_id_token_nonce(
    id_token: &str,
    expected_nonce: &str,
) -> Result<(), AppError> {
    let payload_segment = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::OidcError("Malformed Google ID token".to_string()))?;
    let decoded = URL_SAFE_NO_PAD.decode(payload_segment).map_err(|_| {
        AppError::OidcError("Invalid base64url payload in Google ID token".to_string())
    })?;
    let claims: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|_| AppError::OidcError("Invalid JSON payload in Google ID token".to_string()))?;
    let actual_nonce = claims
        .get("nonce")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::OidcError("Missing nonce claim in Google ID token".to_string()))?;
    if actual_nonce != expected_nonce {
        return Err(AppError::InvalidToken(
            "Google ID token nonce mismatch".to_string(),
        ));
    }
    Ok(())
}

async fn exchange_google_code(
    creds: &ResolvedOAuthProviderCredentials,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
    expected_nonce: &str,
) -> Result<VerifiedSocialIdentity, AppError> {
    let client = shared_oauth_client()?;

    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| AppError::OidcError(format!("Google token exchange failed: {e}")))?;

    if !token_res.status().is_success() {
        return Err(AppError::OidcError(
            "Google rejected authorization code exchange".to_string(),
        ));
    }

    let token_json: serde_json::Value = token_res
        .json()
        .await
        .map_err(|e| AppError::OidcError(e.to_string()))?;
    let access_token = token_json["access_token"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing Google access_token".to_string()))?;
    let id_token = token_json["id_token"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing Google id_token".to_string()))?;

    validate_google_id_token_nonce(id_token, expected_nonce)?;

    let userinfo_res = client
        .get("https://openidconnect.googleapis.com/v1/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::OidcError(format!("Google userinfo request failed: {e}")))?;

    if !userinfo_res.status().is_success() {
        return Err(AppError::OidcError(
            "Failed to fetch Google userinfo".to_string(),
        ));
    }

    let profile: serde_json::Value = userinfo_res
        .json()
        .await
        .map_err(|e| AppError::OidcError(e.to_string()))?;

    let sub = profile["sub"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing sub in Google profile".to_string()))?
        .to_string();
    let email = profile["email"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing email in Google profile".to_string()))?
        .to_string();
    let email_verified = profile["email_verified"].as_bool().unwrap_or(false);
    let display_name = profile["name"].as_str().unwrap_or(&email).to_string();
    let avatar_url = profile["picture"].as_str().map(str::to_string);

    Ok(VerifiedSocialIdentity {
        provider: "google".to_string(),
        provider_user_id: sub,
        email,
        email_verified,
        display_name,
        avatar_url,
    })
}

async fn exchange_github_code(
    creds: &ResolvedOAuthProviderCredentials,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<VerifiedSocialIdentity, AppError> {
    let client = shared_oauth_client()?;

    let token_res = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| AppError::OidcError(format!("GitHub token exchange failed: {e}")))?;

    if !token_res.status().is_success() {
        return Err(AppError::OidcError(
            "GitHub rejected authorization code exchange".to_string(),
        ));
    }

    let token_json: serde_json::Value = token_res
        .json()
        .await
        .map_err(|e| AppError::OidcError(e.to_string()))?;
    let access_token = token_json["access_token"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing GitHub access_token".to_string()))?;

    let user_res = client
        .get("https://api.github.com/user")
        .header("User-Agent", "secure-auth-platform")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::OidcError(format!("GitHub user fetch failed: {e}")))?;

    let user_json: serde_json::Value = user_res
        .json()
        .await
        .map_err(|e| AppError::OidcError(e.to_string()))?;

    let provider_user_id = user_json["id"]
        .as_i64()
        .map(|n| n.to_string())
        .ok_or_else(|| AppError::OidcError("Missing GitHub user id".to_string()))?;
    let display_name = user_json["name"]
        .as_str()
        .or_else(|| user_json["login"].as_str())
        .unwrap_or("GitHub User")
        .to_string();
    let avatar_url = user_json["avatar_url"].as_str().map(str::to_string);

    let emails_res = client
        .get("https://api.github.com/user/emails")
        .header("User-Agent", "secure-auth-platform")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::OidcError(format!("GitHub emails fetch failed: {e}")))?;

    let emails_json: Vec<serde_json::Value> = emails_res
        .json()
        .await
        .map_err(|e| AppError::OidcError(e.to_string()))?;

    let selected = emails_json
        .iter()
        .find(|e| e["primary"].as_bool() == Some(true) && e["verified"].as_bool() == Some(true))
        .or_else(|| {
            emails_json
                .iter()
                .find(|e| e["verified"].as_bool() == Some(true))
        })
        .or_else(|| emails_json.first())
        .ok_or_else(|| AppError::OidcError("No email returned by GitHub".to_string()))?;

    let email = selected["email"]
        .as_str()
        .ok_or_else(|| AppError::OidcError("Missing email string from GitHub".to_string()))?
        .to_string();
    let email_verified = selected["verified"].as_bool().unwrap_or(false);

    Ok(VerifiedSocialIdentity {
        provider: "github".to_string(),
        provider_user_id,
        email,
        email_verified,
        display_name,
        avatar_url,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn complete_social_login_with_identity(
    db: &PgPool,
    config: &AppConfig,
    environment_id: EnvironmentId,
    authorized_party: &str,
    identity: VerifiedSocialIdentity,
    user_agent: Option<&str>,
    ip_address: Option<&str>,
) -> Result<ClientAuthResponse, AppError> {
    let env = tenant_repo::find_environment_by_id(db, environment_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Environment not found".to_string()))?;

    let normalized_email = redis_ephemeral::normalize_email(&identity.email);
    let mut tx = begin_tx(db).await?;

    let user = if let Some(linked) = global_identity::find_oauth_account(
        &mut *tx,
        &identity.provider,
        &identity.provider_user_id,
    )
    .await?
    {
        user_repo::find_user_by_id(db, linked.user_id.into_inner())
            .await?
            .ok_or(AppError::UserNotFound)?
    } else if let Some(existing_email) =
        global_identity::find_verified_email(&mut *tx, &normalized_email).await?
    {
        if !identity.email_verified {
            return Err(AppError::Forbidden(
                "Refusing to auto-link social account with an unverified provider email"
                    .to_string(),
            ));
        }
        global_identity::link_oauth_account(
            &mut *tx,
            existing_email.user_id,
            &identity.provider,
            &identity.provider_user_id,
            Some(&normalized_email),
            true,
        )
        .await?;
        user_repo::find_user_by_id(db, existing_email.user_id.into_inner())
            .await?
            .ok_or(AppError::UserNotFound)?
    } else if let Some(existing_unverified) =
        user_repo::find_user_by_email(db, &normalized_email).await?
    {
        if !existing_unverified.email_verified || !identity.email_verified {
            return Err(AppError::Forbidden(
                "Cannot auto-link social account until both local and provider emails are verified"
                    .to_string(),
            ));
        }
        global_identity::link_oauth_account(
            &mut *tx,
            UserId::from_uuid(existing_unverified.id),
            &identity.provider,
            &identity.provider_user_id,
            Some(&normalized_email),
            true,
        )
        .await?;
        existing_unverified
    } else {
        if env.enrollment_policy == "invite_only" {
            return Err(AppError::Forbidden(
                "Registration for this application is invite-only".to_string(),
            ));
        }
        let mut new_user = User::new(
            normalized_email.clone(),
            identity.display_name.clone(),
            None,
        );
        new_user.email_verified = identity.email_verified;
        new_user.avatar_url = identity.avatar_url.clone();
        let created = user_repo::create_user(&mut *tx, new_user).await?;
        let uid = UserId::from_uuid(created.id);
        let verified_at = if identity.email_verified {
            Some(Utc::now())
        } else {
            None
        };
        let email_row =
            global_identity::create_user_email(&mut *tx, uid, &normalized_email, verified_at, true)
                .await?;
        sqlx::query("UPDATE users SET primary_email_id = $2 WHERE id = $1")
            .bind(created.id)
            .bind(email_row.id)
            .execute(&mut *tx)
            .await?;
        global_identity::link_oauth_account(
            &mut *tx,
            uid,
            &identity.provider,
            &identity.provider_user_id,
            Some(&normalized_email),
            identity.email_verified,
        )
        .await?;
        created
    };

    if !user.is_active() {
        return Err(AppError::Forbidden(
            "Global account is suspended".to_string(),
        ));
    }

    let user_id = UserId::from_uuid(user.id);
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
                    "Application membership is suspended or revoked".to_string(),
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
                    "provider": identity.provider,
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
        serde_json::json!({ "amr": ["oauth"], "provider": identity.provider }),
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
            amr: vec!["oauth".to_string()],
            acr: "aal1".to_string(),
            version: membership.version,
            ttl_secs: env.access_token_ttl_secs as i64,
        },
    )
    .await?;

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

fn shared_oauth_client() -> Result<reqwest::Client, AppError> {
    static CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> =
        std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(std::time::Duration::from_secs(3))
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .cloned()
        .map_err(|error| AppError::Internal(error.clone()))
}
