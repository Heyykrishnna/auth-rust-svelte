use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "auth_audit_event", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEvent {
    LoginSuccess,
    LoginFailed,
    Logout,
    PasswordChanged,
    EmailVerified,
    SessionRevoked,
    MfaEnabled,
    MfaFailed,
    AccountLocked,
}

impl std::fmt::Display for AuditEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoginSuccess => write!(f, "LOGIN_SUCCESS"),
            Self::LoginFailed => write!(f, "LOGIN_FAILED"),
            Self::Logout => write!(f, "LOGOUT"),
            Self::PasswordChanged => write!(f, "PASSWORD_CHANGED"),
            Self::EmailVerified => write!(f, "EMAIL_VERIFIED"),
            Self::SessionRevoked => write!(f, "SESSION_REVOKED"),
            Self::MfaEnabled => write!(f, "MFA_ENABLED"),
            Self::MfaFailed => write!(f, "MFA_FAILED"),
            Self::AccountLocked => write!(f, "ACCOUNT_LOCKED"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event: AuditEvent,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditLog {
    pub fn new(
        user_id: Option<Uuid>,
        event: AuditEvent,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            event,
            ip_address,
            user_agent,
            metadata,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuditLog {
    pub user_id: Option<Uuid>,
    pub event: AuditEvent,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
}
