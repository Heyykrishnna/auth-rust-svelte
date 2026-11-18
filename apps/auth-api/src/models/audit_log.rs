use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, FromRow, Postgres, Type};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl AuditEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginSuccess => "LOGIN_SUCCESS",
            Self::LoginFailed => "LOGIN_FAILED",
            Self::Logout => "LOGOUT",
            Self::PasswordChanged => "PASSWORD_CHANGED",
            Self::EmailVerified => "EMAIL_VERIFIED",
            Self::SessionRevoked => "SESSION_REVOKED",
            Self::MfaEnabled => "MFA_ENABLED",
            Self::MfaFailed => "MFA_FAILED",
            Self::AccountLocked => "ACCOUNT_LOCKED",
        }
    }
}

impl FromStr for AuditEvent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LOGIN_SUCCESS" => Ok(Self::LoginSuccess),
            "LOGIN_FAILED" => Ok(Self::LoginFailed),
            "LOGOUT" => Ok(Self::Logout),
            "PASSWORD_CHANGED" => Ok(Self::PasswordChanged),
            "EMAIL_VERIFIED" => Ok(Self::EmailVerified),
            "SESSION_REVOKED" => Ok(Self::SessionRevoked),
            "MFA_ENABLED" => Ok(Self::MfaEnabled),
            "MFA_FAILED" => Ok(Self::MfaFailed),
            "ACCOUNT_LOCKED" => Ok(Self::AccountLocked),
            other => Err(format!("Unknown audit event: {other}")),
        }
    }
}

impl Type<Postgres> for AuditEvent {
    fn type_info() -> PgTypeInfo {
        <String as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as Type<Postgres>>::compatible(ty)
    }
}

impl<'q> Encode<'q, Postgres> for AuditEvent {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&str as Encode<Postgres>>::encode_by_ref(&self.as_str(), buf)
    }
}

impl<'r> Decode<'r, Postgres> for AuditEvent {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let raw = <String as Decode<Postgres>>::decode(value)?;
        AuditEvent::from_str(&raw).map_err(|err| err.into())
    }
}

impl std::fmt::Display for AuditEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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
