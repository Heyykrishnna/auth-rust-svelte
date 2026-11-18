use deadpool_redis::Pool as RedisPool;
use rand_core::{OsRng, RngCore};
use sqlx::PgPool;

use crate::errors::AppError;
use crate::models::{EnvironmentId, EnvironmentKind};
use crate::repositories::{redis_keys, tenants as tenant_repo};

pub fn generate_dns_verification_token() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    format!("auth-platform-verify={}", hex::encode(bytes))
}

pub fn validate_slug(slug: &str) -> Result<String, AppError> {
    let normalized = slug.trim().to_lowercase();
    if normalized.len() < 2 || normalized.len() > 64 {
        return Err(AppError::Validation(
            "Slug must be between 2 and 64 characters".to_string(),
        ));
    }
    if normalized.starts_with('-') || normalized.ends_with('-') {
        return Err(AppError::Validation(
            "Slug cannot start or end with a hyphen".to_string(),
        ));
    }
    if !normalized
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AppError::Validation(
            "Slug may only contain lowercase letters, digits, and hyphens".to_string(),
        ));
    }
    Ok(normalized)
}

pub fn validate_domain_name(kind: EnvironmentKind, raw_domain: &str) -> Result<String, AppError> {
    let domain = raw_domain.trim().to_lowercase();
    if domain.is_empty() || domain.len() > 253 {
        return Err(AppError::Validation(
            "Domain must be between 1 and 253 characters".to_string(),
        ));
    }
    if domain.contains("://")
        || domain.contains('/')
        || domain.contains('*')
        || domain.contains('@')
        || domain.contains(' ')
    {
        return Err(AppError::Validation(
            "Domain must be a bare hostname without scheme, path, wildcard, or credentials"
                .to_string(),
        ));
    }

    let host_only = domain.split(':').next().unwrap_or("");
    let is_local = host_only == "localhost" || host_only == "127.0.0.1";

    match kind {
        EnvironmentKind::Development => {
            if is_local {
                if let Some((_, port_str)) = domain.split_once(':') {
                    validate_port(port_str)?;
                }
                return Ok(domain);
            }
            if domain.contains(':') {
                return Err(AppError::Validation(
                    "Custom ports are only allowed for localhost or 127.0.0.1".to_string(),
                ));
            }
            validate_hostname_labels(host_only)?;
            Ok(domain)
        }
        EnvironmentKind::Production => {
            if is_local || host_only == "0.0.0.0" || domain.contains(':') {
                return Err(AppError::Validation(
                    "Production domains must be valid public hostnames without localhost or ports"
                        .to_string(),
                ));
            }
            validate_hostname_labels(host_only)?;
            Ok(domain)
        }
    }
}

pub fn validate_origin(kind: EnvironmentKind, raw_origin: &str) -> Result<String, AppError> {
    let origin = raw_origin.trim().trim_end_matches('/');
    if origin.contains('*') || origin.contains('@') || origin.contains('#') || origin.contains('?')
    {
        return Err(AppError::Validation(
            "Origin must not contain wildcards, credentials, query strings, or fragments"
                .to_string(),
        ));
    }

    let (scheme, rest) = origin.split_once("://").ok_or_else(|| {
        AppError::Validation("Origin must include http:// or https://".to_string())
    })?;

    if rest.contains('/') {
        return Err(AppError::Validation(
            "Origin must not contain a path".to_string(),
        ));
    }

    let (host, port) = split_host_port(rest)?;
    let lower_host = host.to_lowercase();
    let is_local = lower_host == "localhost" || lower_host == "127.0.0.1";

    match (kind, scheme) {
        (EnvironmentKind::Production, "https") => {
            if is_local || lower_host == "0.0.0.0" {
                return Err(AppError::Validation(
                    "Production origins cannot target localhost or loopback addresses".to_string(),
                ));
            }
            validate_hostname_labels(&lower_host)?;
        }
        (EnvironmentKind::Production, _) => {
            return Err(AppError::Validation(
                "Production origins must use https://".to_string(),
            ));
        }
        (EnvironmentKind::Development, "http") => {
            if !is_local {
                return Err(AppError::Validation(
                    "Development http:// origins are restricted to localhost or 127.0.0.1"
                        .to_string(),
                ));
            }
        }
        (EnvironmentKind::Development, "https") => {
            if !is_local {
                validate_hostname_labels(&lower_host)?;
            }
        }
        _ => {
            return Err(AppError::Validation(
                "Origin scheme must be http or https".to_string(),
            ));
        }
    }

    if let Some(p) = port {
        Ok(format!("{scheme}://{lower_host}:{p}"))
    } else {
        Ok(format!("{scheme}://{lower_host}"))
    }
}

pub fn validate_redirect_uri(kind: EnvironmentKind, raw_uri: &str) -> Result<String, AppError> {
    let uri = raw_uri.trim();
    if uri.is_empty() || uri.len() > 2048 {
        return Err(AppError::Validation(
            "Redirect URI must be between 1 and 2048 characters".to_string(),
        ));
    }
    if uri.contains('*') || uri.contains('#') || uri.contains('\\') || uri.contains(' ') {
        return Err(AppError::Validation(
            "Redirect URI must be an exact URI without wildcards, fragments, or spaces".to_string(),
        ));
    }

    let (scheme, rest) = uri
        .split_once("://")
        .ok_or_else(|| AppError::Validation("Redirect URI must include a scheme".to_string()))?;

    let (authority, path_and_query) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    };

    if authority.contains('@') {
        return Err(AppError::Validation(
            "Redirect URI must not contain userinfo credentials".to_string(),
        ));
    }

    if path_and_query.contains("/../")
        || path_and_query.ends_with("/..")
        || path_and_query.contains("/./")
    {
        return Err(AppError::Validation(
            "Redirect URI must not contain relative path traversal segments".to_string(),
        ));
    }

    let origin_candidate = format!("{scheme}://{authority}");
    let normalized_origin = validate_origin(kind, &origin_candidate)?;

    Ok(format!("{normalized_origin}{path_and_query}"))
}

fn split_host_port(authority: &str) -> Result<(&str, Option<u16>), AppError> {
    if authority.is_empty() {
        return Err(AppError::Validation("Missing hostname".to_string()));
    }
    match authority.split_once(':') {
        Some((host, port_str)) => {
            let port = validate_port(port_str)?;
            Ok((host, Some(port)))
        }
        None => Ok((authority, None)),
    }
}

fn validate_port(port_str: &str) -> Result<u16, AppError> {
    let port = port_str
        .parse::<u16>()
        .map_err(|_| AppError::Validation(format!("Invalid port: {port_str}")))?;
    if port == 0 {
        return Err(AppError::Validation(
            "Port must be greater than 0".to_string(),
        ));
    }
    Ok(port)
}

fn validate_hostname_labels(host: &str) -> Result<(), AppError> {
    if !host.contains('.') {
        return Err(AppError::Validation(format!(
            "Hostname must be a fully qualified domain: {host}"
        )));
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(AppError::Validation(format!(
                "Invalid hostname label: {label}"
            )));
        }
    }
    Ok(())
}

pub async fn get_cached_allowed_origins(
    db: &PgPool,
    redis: &RedisPool,
    environment_id: EnvironmentId,
) -> Result<Vec<String>, AppError> {
    let cache_key = redis_keys::env_cors_origins_key(environment_id);

    if let Ok(mut conn) = redis.get().await {
        let cached: Result<Option<String>, _> = redis::cmd("GET")
            .arg(&cache_key)
            .query_async(&mut conn)
            .await;
        if let Ok(Some(json_str)) = cached {
            if let Ok(origins) = serde_json::from_str::<Vec<String>>(&json_str) {
                return Ok(origins);
            }
        }
    }

    let origins = tenant_repo::list_environment_allowed_origins(db, environment_id).await?;

    if let Ok(mut conn) = redis.get().await {
        if let Ok(serialized) = serde_json::to_string(&origins) {
            let _: Result<(), _> = redis::cmd("SETEX")
                .arg(&cache_key)
                .arg(300)
                .arg(&serialized)
                .query_async(&mut conn)
                .await;
        }
    }

    Ok(origins)
}

pub async fn invalidate_allowed_origins_cache(redis: &RedisPool, environment_id: EnvironmentId) {
    if let Ok(mut conn) = redis.get().await {
        let key = redis_keys::env_cors_origins_key(environment_id);
        let _: Result<(), _> = redis::cmd("DEL").arg(&key).query_async(&mut conn).await;
    }
}
