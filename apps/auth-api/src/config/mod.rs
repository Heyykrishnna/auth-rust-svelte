use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry_secs: u64,
    pub jwt_refresh_expiry_secs: u64,
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
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let host = env::var("AUTH_API_HOST")
            .or_else(|_| env::var("HOST"))
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("AUTH_API_PORT")
            .or_else(|_| env::var("PORT"))
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be specified")?;
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            anyhow::bail!("DATABASE_URL must start with postgres:// or postgresql://");
        }

        let db_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "dev_jwt_secret_change_me_in_production_min_32_chars".to_string());

        if jwt_secret.trim().len() < 32 {
            anyhow::bail!(
                "JWT_SECRET must be at least 32 characters long for cryptographic security"
            );
        }

        let jwt_access_expiry_secs = env::var("JWT_ACCESS_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(900);

        let jwt_refresh_expiry_secs = env::var("JWT_REFRESH_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(604_800);

        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let cookie_domain = env::var("COOKIE_DOMAIN").ok();

        let google_client_id = env::var("GOOGLE_CLIENT_ID").ok();
        let google_client_secret = env::var("GOOGLE_CLIENT_SECRET").ok();
        let google_redirect_uri = env::var("GOOGLE_REDIRECT_URI").ok();

        let github_client_id = env::var("GITHUB_CLIENT_ID").ok();
        let github_client_secret = env::var("GITHUB_CLIENT_SECRET").ok();
        let github_redirect_uri = env::var("GITHUB_REDIRECT_URI").ok();

        let cors_origins = env::var("CORS_ORIGINS")
            .map(|s| {
                s.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|_| vec!["http://localhost:3000".to_string()]);

        let otel_exporter_otlp_endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:4317".to_string());

        let otel_service_name =
            env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "auth-api".to_string());

        let otel_service_version = env::var("OTEL_SERVICE_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

        Ok(Self {
            host,
            port,
            database_url,
            db_max_connections,
            redis_url,
            jwt_secret,
            jwt_access_expiry_secs,
            jwt_refresh_expiry_secs,
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
        })
    }

    pub fn access_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_access_expiry_secs)
    }

    pub fn refresh_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_refresh_expiry_secs)
    }
}
