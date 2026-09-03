use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, display_name: String, password_hash: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            display_name,
            password_hash,
            avatar_url: None,
            email_verified: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserProfile {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
            email_verified: u.email_verified,
            created_at: u.created_at,
        }
    }
}
