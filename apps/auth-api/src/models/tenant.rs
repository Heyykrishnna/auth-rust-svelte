use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use uuid::Uuid;

use crate::models::{ApplicationId, DeveloperId, EnvironmentId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentKind {
    Development,
    Production,
}

impl EnvironmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Production => "production",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "development" => Some(Self::Development),
            "production" => Some(Self::Production),
            _ => None,
        }
    }
}

impl fmt::Display for EnvironmentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentPolicy {
    ConsentRequired,
    AutoJoin,
    InviteOnly,
}

impl EnrollmentPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConsentRequired => "consent_required",
            Self::AutoJoin => "auto_join",
            Self::InviteOnly => "invite_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Developer,
    Viewer,
}

impl WorkspaceRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Developer => "developer",
            Self::Viewer => "viewer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    Publishable,
    Secret,
}

impl CredentialType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Publishable => "publishable",
            Self::Secret => "secret",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Developer {
    pub id: DeveloperId,
    pub email: String,
    pub password_hash: Option<String>,
    pub display_name: String,
    pub email_verified: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeveloperWorkspace {
    pub id: WorkspaceId,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeveloperWorkspaceMember {
    pub workspace_id: WorkspaceId,
    pub developer_id: DeveloperId,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Application {
    pub id: ApplicationId,
    pub workspace_id: WorkspaceId,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationEnvironment {
    pub id: EnvironmentId,
    pub application_id: ApplicationId,
    pub kind: String,
    pub slug: String,
    pub enrollment_policy: String,
    pub session_ttl_secs: i32,
    pub access_token_ttl_secs: i32,
    pub require_email_verification: bool,
    pub require_mfa: bool,
    pub branding: serde_json::Value,
    pub settings: serde_json::Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationDomain {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub domain: String,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub allowed_origins: Vec<String>,
    pub redirect_uris: Vec<String>,
    pub post_logout_redirect_uris: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationCredential {
    pub id: Uuid,
    pub environment_id: EnvironmentId,
    pub credential_type: String,
    pub key_prefix: String,
    pub public_key: Option<String>,
    pub secret_hash: Option<String>,
    pub name: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_by: Option<DeveloperId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
