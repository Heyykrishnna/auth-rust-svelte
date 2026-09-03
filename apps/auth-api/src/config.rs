use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // ─── Server ──────────────────────────────────────────────────────────────
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    // ─── Database ────────────────────────────────────────────────────────────
    pub database_url: String,

    #[serde(default = "default_db_max_connections")]
    pub db_max_connections: u32,

    // ─── Redis ───────────────────────────────────────────────────────────────
    pub redis_url: String,

    // ─── JWT ─────────────────────────────────────────────────────────────────
    pub jwt_secret: String,

    #[serde(default = "default_access_expiry")]
    pub jwt_access_expiry_secs: u64,

    #[serde(default = "default_refresh_expiry")]
    pub jwt_refresh_expiry_secs: u64,

    // ─── OIDC — Google ───────────────────────────────────────────────────────
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_redirect_uri: Option<String>,

    // ─── OIDC — GitHub ───────────────────────────────────────────────────────
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub github_redirect_uri: Option<String>,

    // ─── CORS ────────────────────────────────────────────────────────────────
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,

    // ─── Observability ───────────────────────────────────────────────────────
    #[serde(default = "default_otel_endpoint")]
    pub otel_exporter_otlp_endpoint: String,

    #[serde(default = "default_service_name")]
    pub otel_service_name: String,

    #[serde(default = "default_service_version")]
    pub otel_service_version: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        envy::from_env::<AppConfig>()
            .context("Failed to load configuration from environment variables")
    }

    pub fn access_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_access_expiry_secs)
    }

    pub fn refresh_token_duration(&self) -> Duration {
        Duration::from_secs(self.jwt_refresh_expiry_secs)
    }
}

// ─── Defaults ─────────────────────────────────────────────────────────────────

fn default_host()            -> String { "0.0.0.0".to_string() }
fn default_port()            -> u16    { 8080 }
fn default_db_max_connections() -> u32 { 10 }
fn default_access_expiry()   -> u64    { 900 }        // 15 min
fn default_refresh_expiry()  -> u64    { 604_800 }    // 7 days
fn default_otel_endpoint()   -> String { "http://localhost:4317".to_string() }
fn default_service_name()    -> String { "auth-api".to_string() }
fn default_service_version() -> String { env!("CARGO_PKG_VERSION").to_string() }
fn default_cors_origins()    -> Vec<String> {
    vec!["http://localhost:3000".to_string()]
}
