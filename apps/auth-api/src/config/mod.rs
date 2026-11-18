use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::time::Duration;

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub redis_url: String,
    pub jwt_secret: String,
    pub issuer_url: String,
    pub signing_key_kek: String,
    pub signing_key_kek_version: String,
    pub jwt_access_expiry_secs: u64,
    pub jwt_refresh_expiry_secs: u64,
    pub login_max_attempts: u32,
    pub login_lockout_duration_secs: u64,
    pub verification_code_expiry_secs: u64,
    pub password_reset_expiry_secs: u64,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_redirect_uri: Option<String>,
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_redirect_uri: Option<String>,
    pub cors_origins: Vec<String>,
    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub otel_exporter_otlp_endpoint: String,
    pub otel_service_name: String,
    pub otel_service_version: String,
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_user: Option<String>,
    pub smtp_pass: Option<String>,
    pub smtp_from: String,
    pub smtp_from_name: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        Self::from_reader(|name| env::var(name))
    }

    pub fn from_map(values: &std::collections::HashMap<String, String>) -> Result<Self> {
        Self::from_reader(|name| values.get(name).cloned().ok_or(env::VarError::NotPresent))
    }

    fn from_reader(read: impl Fn(&str) -> Result<String, env::VarError>) -> Result<Self> {
        let host = read("AUTH_API_HOST")
            .or_else(|_| read("HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = read("AUTH_API_PORT")
            .or_else(|_| read("PORT"))
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        let database_url = read("DATABASE_URL").context("DATABASE_URL must be specified")?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            anyhow::bail!("DATABASE_URL must start with postgres:// or postgresql://");
        }

        let db_max_connections = read("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let redis_url = read("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let jwt_secret = required_secret("JWT_SECRET", read("JWT_SECRET"))?;

        if jwt_secret.trim().len() < 32 {
            anyhow::bail!(
                "JWT_SECRET must be at least 32 characters long for cryptographic security"
            );
        }

        let issuer_url = read("ISSUER_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string())
            .trim_end_matches('/')
            .to_string();

        let signing_key_kek = required_secret("SIGNING_KEY_KEK", read("SIGNING_KEY_KEK"))?;
        if signing_key_kek.trim().len() < 32 {
            anyhow::bail!(
                "SIGNING_KEY_KEK must be at least 32 characters long for AES-256-GCM envelope encryption"
            );
        }

        let signing_key_kek_version =
            read("SIGNING_KEY_KEK_VERSION").unwrap_or_else(|_| "v1".to_string());

        let jwt_access_expiry_secs = read("JWT_ACCESS_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900);

        let jwt_refresh_expiry_secs = read("JWT_REFRESH_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(604_800);

        let login_max_attempts = read("LOGIN_MAX_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(5);

        let login_lockout_duration_secs = read("LOGIN_LOCKOUT_DURATION_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900);

        let verification_code_expiry_secs = read("VERIFICATION_CODE_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900);

        let password_reset_expiry_secs = read("PASSWORD_RESET_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900);

        let cookie_secure = read("COOKIE_SECURE")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let cookie_domain = read("COOKIE_DOMAIN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        crate::middleware::rate_limit::parse_trusted_proxy_ranges(
            &read("TRUSTED_PROXY_CIDRS").unwrap_or_default(),
        )
        .map_err(anyhow::Error::msg)?;
        let production = read("APP_ENV")
            .map(|value| value.eq_ignore_ascii_case("production"))
            .unwrap_or(false);
        let issuer = url::Url::parse(&issuer_url).context("ISSUER_URL must be an absolute URL")?;
        if issuer.host_str().is_none()
            || !issuer.username().is_empty()
            || issuer.password().is_some()
        {
            anyhow::bail!("ISSUER_URL must have a host and no credentials");
        }
        if production && (!cookie_secure || issuer.scheme() != "https") {
            anyhow::bail!("Production requires COOKIE_SECURE=true and an HTTPS ISSUER_URL");
        }

        let google_client_id = read("GOOGLE_CLIENT_ID").ok();
        let google_client_secret = read("GOOGLE_CLIENT_SECRET").ok();
        let google_redirect_uri = read("GOOGLE_REDIRECT_URI").ok();

        let github_client_id = read("GITHUB_CLIENT_ID").ok();
        let github_client_secret = read("GITHUB_CLIENT_SECRET").ok();
        let github_redirect_uri = read("GITHUB_REDIRECT_URI").ok();

        let cors_origins = read("CORS_ORIGINS")
            .map(|s| {
                s.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|_| vec!["http://localhost:3000".to_string()]);

        let otel_exporter_otlp_endpoint = read("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4317".to_string());

        let otel_service_name =
            read("OTEL_SERVICE_NAME").unwrap_or_else(|_| "auth-api".to_string());

        let otel_service_version =
            read("OTEL_SERVICE_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

        let smtp_host = read("SMTP_HOST").ok().filter(|s| !s.trim().is_empty());
        let smtp_port = read("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(587);
        let smtp_user = read("SMTP_USER")
            .or_else(|_| read("SMTP_USERNAME"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let smtp_pass = read("SMTP_PASS")
            .or_else(|_| read("SMTP_PASSWORD"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let smtp_from = read("SMTP_FROM")
            .or_else(|_| read("SMTP_FROM_EMAIL"))
            .or_else(|_| read("SENDER_EMAIL"))
            .unwrap_or_else(|_| "Signet <support@signet.dev>".to_string());
        let smtp_from_name = read("SMTP_FROM_NAME").unwrap_or_else(|_| "Signet".to_string());

        Ok(Self {
            host,
            port,
            database_url,
            db_max_connections,
            redis_url,
            jwt_secret,
            issuer_url,
            signing_key_kek,
            signing_key_kek_version,
            jwt_access_expiry_secs,
            jwt_refresh_expiry_secs,
            login_max_attempts,
            login_lockout_duration_secs,
            verification_code_expiry_secs,
            password_reset_expiry_secs,
            cookie_secure,
            cookie_domain,
            google_client_id,
            google_client_secret,
            google_redirect_uri,
            github_client_id,
            github_client_secret,
            github_redirect_uri,
            cors_origins,
            otel_exporter_otlp_endpoint,
            otel_service_name,
            otel_service_version,
            smtp_host,
            smtp_port,
            smtp_user,
            smtp_pass,
            smtp_from,
            smtp_from_name,
        })
    }

    pub fn access_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_access_expiry_secs)
    }

    pub fn refresh_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_refresh_expiry_secs)
    }

    pub fn portal_origin(&self) -> String {
        env::var("PORTAL_ORIGIN")
            .ok()
            .map(|v| v.trim().trim_end_matches('/').to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                self.cors_origins
                    .first()
                    .map(|v| v.trim_end_matches('/').to_string())
                    .unwrap_or_else(|| "http://localhost:3000".to_string())
            })
    }
}

fn required_secret(name: &str, value: Result<String, env::VarError>) -> Result<String> {
    let value = value.with_context(|| format!("{name} must be explicitly specified"))?;
    let normalized = value.to_ascii_lowercase();
    if value.trim().len() < 32
        || [
            "change_me",
            "change-me",
            "replace_me",
            "replace-me",
            "your_secret",
            "dev_jwt",
            "dev_signing",
        ]
        .iter()
        .any(|placeholder| normalized.contains(placeholder))
    {
        anyhow::bail!(
            "{name} must contain at least 32 characters of non-placeholder secret material"
        );
    }
    Ok(value)
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("db_max_connections", &self.db_max_connections)
            .field("cookie_secure", &self.cookie_secure)
            .finish_non_exhaustive()
    }
}

impl Drop for AppConfig {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.database_url.zeroize();
        self.redis_url.zeroize();
        self.jwt_secret.zeroize();
        self.signing_key_kek.zeroize();
        self.google_client_secret.zeroize();
        self.github_client_secret.zeroize();
        self.smtp_pass.zeroize();
    }
}
