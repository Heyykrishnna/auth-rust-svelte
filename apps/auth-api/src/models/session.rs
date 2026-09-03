use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

impl Session {
    pub fn new(
        user_id: Uuid,
        refresh_token_hash: String,
        expires_at: DateTime<Utc>,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            refresh_token_hash,
            user_agent,
            ip_address,
            created_at: now,
            expires_at,
            last_used_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub is_current: bool,
}
