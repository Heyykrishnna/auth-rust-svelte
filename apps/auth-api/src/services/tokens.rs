use chrono::Utc;
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::models::{
    AccessTokenClaims, AppSessionId, AppUserId, EmailVerificationClaims, EnvironmentId,
    OrganizationId, RefreshTokenClaims, TokenPair, UserId,
};
use crate::repositories::signing_keys as key_repo;
use crate::services::signing_keys::{self, EcPublicJwk};

pub const ACCESS_TOKEN_TYP: &str = "at+jwt";
pub const ID_TOKEN_TYP: &str = "JWT";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TenantAccessTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub azp: String,
    pub app_user_id: String,
    pub sid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub amr: Vec<String>,
    pub acr: String,
    pub ver: i64,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    pub auth_time: i64,
    pub amr: Vec<String>,
    pub acr: String,
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture: Option<String>,
    pub app_user_id: String,
    pub ver: i64,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct IssueAccessTokenInput<'a> {
    pub issuer_url: &'a str,
    pub user_id: UserId,
    pub environment_id: EnvironmentId,
    pub authorized_party: &'a str,
    pub app_user_id: AppUserId,
    pub session_id: AppSessionId,
    pub active_organization_id: Option<OrganizationId>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub amr: Vec<String>,
    pub acr: String,
    pub version: i64,
    pub ttl_secs: i64,
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_opaque_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("rt_{}", hex::encode(bytes))
}

pub fn issue_es256_access_token_with_key(
    input: IssueAccessTokenInput<'_>,
    kid: &str,
    pkcs8_der: &[u8],
) -> Result<(String, TenantAccessTokenClaims), AppError> {
    let now = Utc::now().timestamp();
    let claims = TenantAccessTokenClaims {
        iss: input.issuer_url.to_string(),
        sub: input.user_id.to_string(),
        aud: input.environment_id.to_string(),
        azp: input.authorized_party.to_string(),
        app_user_id: input.app_user_id.to_string(),
        sid: input.session_id.to_string(),
        org_id: input.active_organization_id.map(|id| id.to_string()),
        scope: None,
        roles: input.roles,
        permissions: input.permissions,
        amr: input.amr,
        acr: input.acr,
        ver: input.version,
        iat: now,
        exp: now + input.ttl_secs,
        jti: Uuid::new_v4().to_string(),
    };

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(kid.to_string());
    header.typ = Some(ACCESS_TOKEN_TYP.to_string());

    let encoding_key = EncodingKey::from_ec_der(pkcs8_der);
    let token = encode(&header, &claims, &encoding_key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    Ok((token, claims))
}

pub fn issue_es256_id_token_with_key(
    claims: &IdTokenClaims,
    kid: &str,
    pkcs8_der: &[u8],
) -> Result<String, AppError> {
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(kid.to_string());
    header.typ = Some(ID_TOKEN_TYP.to_string());

    let encoding_key = EncodingKey::from_ec_der(pkcs8_der);
    encode(&header, claims, &encoding_key).map_err(|e| AppError::TokenCreation(e.to_string()))
}

pub async fn issue_tenant_access_token(
    db: &PgPool,
    config: &AppConfig,
    input: IssueAccessTokenInput<'_>,
) -> Result<(String, TenantAccessTokenClaims), AppError> {
    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let pkcs8_der = signing_keys::decrypt_signing_key_pkcs8(&active_key, config)?;
    issue_es256_access_token_with_key(input, &active_key.kid, &pkcs8_der)
}

pub async fn issue_tenant_access_token_with_scope(
    db: &PgPool,
    config: &AppConfig,
    input: IssueAccessTokenInput<'_>,
    scope: &str,
) -> Result<(String, TenantAccessTokenClaims), AppError> {
    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let pkcs8_der = signing_keys::decrypt_signing_key_pkcs8(&active_key, config)?;
    let now = Utc::now().timestamp();
    let claims = TenantAccessTokenClaims {
        iss: input.issuer_url.to_string(),
        sub: input.user_id.to_string(),
        aud: input.environment_id.to_string(),
        azp: input.authorized_party.to_string(),
        app_user_id: input.app_user_id.to_string(),
        sid: input.session_id.to_string(),
        org_id: input.active_organization_id.map(|id| id.to_string()),
        scope: Some(scope.to_string()),
        roles: input.roles,
        permissions: input.permissions,
        amr: input.amr,
        acr: input.acr,
        ver: input.version,
        iat: now,
        exp: now + input.ttl_secs,
        jti: Uuid::new_v4().to_string(),
    };

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(active_key.kid);
    header.typ = Some(ACCESS_TOKEN_TYP.to_string());

    let encoding_key = EncodingKey::from_ec_der(&pkcs8_der);
    let token = encode(&header, &claims, &encoding_key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    Ok((token, claims))
}

pub async fn issue_tenant_id_token(
    db: &PgPool,
    config: &AppConfig,
    claims: &IdTokenClaims,
) -> Result<String, AppError> {
    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let pkcs8_der = signing_keys::decrypt_signing_key_pkcs8(&active_key, config)?;
    issue_es256_id_token_with_key(claims, &active_key.kid, &pkcs8_der)
}

pub async fn verify_any_tenant_access_token(
    db: &PgPool,
    config: &AppConfig,
    token: &str,
) -> Result<TenantAccessTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;

    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported JWT signing algorithm".to_string(),
        ));
    }
    if header.typ.as_deref() != Some(ACCESS_TOKEN_TYP) {
        return Err(AppError::InvalidToken("Invalid token type".to_string()));
    }
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing kid header".to_string()))?;

    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Unknown or revoked signing key".to_string()))?;

    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;
    validation.validate_aud = false;
    validation.set_issuer(&[config.issuer_url.as_str()]);

    decode::<TenantAccessTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub async fn verify_tenant_id_token_hint(
    db: &PgPool,
    config: &AppConfig,
    token: &str,
) -> Result<IdTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;

    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported ID token signing algorithm".to_string(),
        ));
    }
    if header.typ.as_deref() != Some(ID_TOKEN_TYP) {
        return Err(AppError::InvalidToken("Invalid ID token type".to_string()));
    }
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing kid header".to_string()))?;

    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Unknown or revoked signing key".to_string()))?;

    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.set_issuer(&[config.issuer_url.as_str()]);

    decode::<IdTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn verify_es256_access_token_with_jwk(
    token: &str,
    jwk: &EcPublicJwk,
    expected_issuer: &str,
    expected_audience: &str,
) -> Result<TenantAccessTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;

    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported JWT signing algorithm".to_string(),
        ));
    }
    if header.typ.as_deref() != Some(ACCESS_TOKEN_TYP) {
        return Err(AppError::InvalidToken("Invalid token type".to_string()));
    }

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;
    validation.set_issuer(&[expected_issuer]);
    validation.set_audience(&[expected_audience]);

    decode::<TenantAccessTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn verify_es256_id_token_with_jwk(
    token: &str,
    jwk: &EcPublicJwk,
    expected_issuer: &str,
    expected_audience: &str,
) -> Result<IdTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;

    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported ID token signing algorithm".to_string(),
        ));
    }
    if header.typ.as_deref() != Some(ID_TOKEN_TYP) {
        return Err(AppError::InvalidToken("Invalid ID token type".to_string()));
    }

    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;
    validation.set_issuer(&[expected_issuer]);
    validation.set_audience(&[expected_audience]);

    decode::<IdTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub async fn verify_tenant_access_token(
    db: &PgPool,
    config: &AppConfig,
    token: &str,
    expected_environment_id: EnvironmentId,
) -> Result<TenantAccessTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;

    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported JWT signing algorithm".to_string(),
        ));
    }

    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing kid header".to_string()))?;

    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Unknown or revoked signing key".to_string()))?;

    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;

    verify_es256_access_token_with_jwk(
        token,
        &jwk,
        &config.issuer_url,
        &expected_environment_id.to_string(),
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn issue_legacy_es256_token_pair(
    db: &PgPool,
    config: &AppConfig,
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    session_id: Uuid,
) -> Result<TokenPair, AppError> {
    let active_key = signing_keys::ensure_active_signing_key(db, config).await?;
    let pkcs8_der = signing_keys::decrypt_signing_key_pkcs8(&active_key, config)?;
    let now = Utc::now().timestamp();

    let access_claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        roles,
        permissions,
        iat: now,
        exp: now + config.jwt_access_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let refresh_claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        session_id: session_id.to_string(),
        iat: now,
        exp: now + config.jwt_refresh_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let mut access_header = Header::new(Algorithm::ES256);
    access_header.kid = Some(active_key.kid.clone());
    access_header.typ = Some(ACCESS_TOKEN_TYP.to_string());

    let mut refresh_header = Header::new(Algorithm::ES256);
    refresh_header.kid = Some(active_key.kid);
    refresh_header.typ = Some("rt+jwt".to_string());

    let encoding_key = EncodingKey::from_ec_der(&pkcs8_der);
    let access_token = encode(&access_header, &access_claims, &encoding_key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;
    let refresh_token = encode(&refresh_header, &refresh_claims, &encoding_key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: config.jwt_access_expiry_secs,
    })
}

pub async fn verify_legacy_es256_access_token(
    db: &PgPool,
    token: &str,
) -> Result<AccessTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;
    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported JWT signing algorithm".to_string(),
        ));
    }
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing kid header".to_string()))?;
    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Unknown or revoked signing key".to_string()))?;
    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;
    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;

    decode::<AccessTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub async fn verify_legacy_es256_refresh_token(
    db: &PgPool,
    token: &str,
) -> Result<RefreshTokenClaims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::InvalidToken(format!("Malformed JWT: {e}")))?;
    if header.alg != Algorithm::ES256 {
        return Err(AppError::InvalidToken(
            "Unsupported JWT signing algorithm".to_string(),
        ));
    }
    let kid = header
        .kid
        .as_deref()
        .ok_or_else(|| AppError::InvalidToken("Missing kid header".to_string()))?;
    let key_record = key_repo::find_verifiable_signing_key_by_kid(db, kid)
        .await?
        .ok_or_else(|| AppError::InvalidToken("Unknown or revoked signing key".to_string()))?;
    let jwk: EcPublicJwk = serde_json::from_value(key_record.public_jwk)
        .map_err(|e| AppError::Internal(format!("Corrupted public JWK: {e}")))?;
    let decoding_key = DecodingKey::from_ec_components(&jwk.x, &jwk.y)
        .map_err(|e| AppError::InvalidToken(format!("Invalid JWK components: {e}")))?;
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;

    decode::<RefreshTokenClaims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn generate_token_pair(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    session_id: Uuid,
    secret: &str,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
) -> Result<TokenPair, AppError> {
    let default_roles = vec!["user".to_string()];
    let default_permissions = vec![
        "profile.read".to_string(),
        "profile.write".to_string(),
        "sessions.read".to_string(),
        "sessions.delete".to_string(),
    ];
    generate_token_pair_with_roles_and_permissions(
        user_id,
        email,
        display_name,
        default_roles,
        default_permissions,
        session_id,
        secret,
        access_expiry_secs,
        refresh_expiry_secs,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generate_token_pair_with_roles_and_permissions(
    user_id: Uuid,
    email: &str,
    display_name: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    session_id: Uuid,
    secret: &str,
    access_expiry_secs: u64,
    refresh_expiry_secs: u64,
) -> Result<TokenPair, AppError> {
    let now = Utc::now().timestamp();

    let access_claims = AccessTokenClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        display_name: display_name.to_string(),
        roles,
        permissions,
        iat: now,
        exp: now + access_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let refresh_claims = RefreshTokenClaims {
        sub: user_id.to_string(),
        session_id: session_id.to_string(),
        iat: now,
        exp: now + refresh_expiry_secs as i64,
        jti: Uuid::new_v4().to_string(),
    };

    let key = EncodingKey::from_secret(secret.as_bytes());

    let access_token = encode(&Header::new(Algorithm::HS256), &access_claims, &key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    let refresh_token = encode(&Header::new(Algorithm::HS256), &refresh_claims, &key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: access_expiry_secs,
    })
}

pub fn validate_access_token(token: &str, secret: &str) -> Result<AccessTokenClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<AccessTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn validate_refresh_token(token: &str, secret: &str) -> Result<RefreshTokenClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<RefreshTokenClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}

pub fn generate_email_verification_token(
    user_id: Uuid,
    email: &str,
    secret: &str,
    expiry_secs: u64,
) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = EmailVerificationClaims {
        sub: user_id.to_string(),
        email: email.to_string(),
        iat: now,
        exp: now + expiry_secs as i64,
    };

    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::new(Algorithm::HS256), &claims, &key)
        .map_err(|e| AppError::TokenCreation(e.to_string()))
}

pub fn validate_email_verification_token(
    token: &str,
    secret: &str,
) -> Result<EmailVerificationClaims, AppError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<EmailVerificationClaims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| AppError::InvalidToken(e.to_string()))
}
