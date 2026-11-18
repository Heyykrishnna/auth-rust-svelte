use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::{
    AppSessionId, AppUserId, EnvironmentId, OrganizationId, OrganizationMemberId, PermissionId,
    RoleId, SsoSessionId, UserEmailId, UserId,
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserEmail {
    pub id: UserEmailId,
    pub user_id: UserId,
    pub email: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserEmail {
    pub fn is_verified(&self) -> bool {
        self.verified_at.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthAccount {
    pub id: Uuid,
    pub user_id: UserId,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SsoSession {
    pub id: SsoSessionId,
    pub user_id: UserId,
    pub session_token_hash: String,
    pub authenticated_at: DateTime<Utc>,
    pub amr: Vec<String>,
    pub acr: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SsoSession {
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && Utc::now() <= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApplicationUser {
    pub id: AppUserId,
    pub environment_id: EnvironmentId,
    pub user_id: UserId,
    pub status: String,
    pub external_id: Option<String>,
    pub public_metadata: serde_json::Value,
    pub private_metadata: serde_json::Value,
    pub version: i64,
    pub joined_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ApplicationUser {
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AppSession {
    pub id: AppSessionId,
    pub environment_id: EnvironmentId,
    pub application_user_id: AppUserId,
    pub user_id: UserId,
    pub sso_session_id: Option<SsoSessionId>,
    pub session_hash: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub active_organization_id: Option<OrganizationId>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AppSession {
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && Utc::now() <= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub id: OrganizationId,
    pub environment_id: EnvironmentId,
    pub slug: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrganizationMember {
    pub id: OrganizationMemberId,
    pub environment_id: EnvironmentId,
    pub organization_id: OrganizationId,
    pub application_user_id: AppUserId,
    pub status: String,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvironmentRole {
    pub id: RoleId,
    pub environment_id: EnvironmentId,
    pub slug: String,
    pub name: String,
    pub scope: String,
    pub description: String,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnvironmentPermission {
    pub id: PermissionId,
    pub environment_id: EnvironmentId,
    pub slug: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
