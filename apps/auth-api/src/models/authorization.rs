use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::FromRow;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    Admin,
    Custom(String),
}

impl Role {
    pub fn as_str(&self) -> &str {
        match self {
            Role::User => "user",
            Role::Admin => "admin",
            Role::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Role {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "user" => Role::User,
            "admin" => Role::Admin,
            other => Role::Custom(other.to_string()),
        })
    }
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Role::Custom(s.to_string()))
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    ProfileRead,
    ProfileWrite,
    SessionsRead,
    SessionsDelete,
    UsersRead,
    UsersDelete,
    Custom(String),
}

impl Permission {
    pub fn as_str(&self) -> &str {
        match self {
            Permission::ProfileRead => "profile.read",
            Permission::ProfileWrite => "profile.write",
            Permission::SessionsRead => "sessions.read",
            Permission::SessionsDelete => "sessions.delete",
            Permission::UsersRead => "users.read",
            Permission::UsersDelete => "users.delete",
            Permission::Custom(s) => s.as_str(),
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Permission {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "profile.read" => Permission::ProfileRead,
            "profile.write" => Permission::ProfileWrite,
            "sessions.read" => Permission::SessionsRead,
            "sessions.delete" => Permission::SessionsDelete,
            "users.read" => Permission::UsersRead,
            "users.delete" => Permission::UsersDelete,
            other => Permission::Custom(other.to_string()),
        })
    }
}

impl From<&str> for Permission {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or(Permission::Custom(s.to_string()))
    }
}

impl From<String> for Permission {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl Serialize for Permission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into())
    }
}

pub trait PermissionCheck: Send + Sync + 'static {
    fn permission() -> Permission;
}

pub trait RoleCheck: Send + Sync + 'static {
    fn role() -> Role;
}

pub struct ProfileReadPermission;
impl PermissionCheck for ProfileReadPermission {
    fn permission() -> Permission {
        Permission::ProfileRead
    }
}

pub struct ProfileWritePermission;
impl PermissionCheck for ProfileWritePermission {
    fn permission() -> Permission {
        Permission::ProfileWrite
    }
}

pub struct SessionsReadPermission;
impl PermissionCheck for SessionsReadPermission {
    fn permission() -> Permission {
        Permission::SessionsRead
    }
}

pub struct SessionsDeletePermission;
impl PermissionCheck for SessionsDeletePermission {
    fn permission() -> Permission {
        Permission::SessionsDelete
    }
}

pub struct UsersReadPermission;
impl PermissionCheck for UsersReadPermission {
    fn permission() -> Permission {
        Permission::UsersRead
    }
}

pub struct UsersDeletePermission;
impl PermissionCheck for UsersDeletePermission {
    fn permission() -> Permission {
        Permission::UsersDelete
    }
}

pub struct AdminRole;
impl RoleCheck for AdminRole {
    fn role() -> Role {
        Role::Admin
    }
}

pub struct UserRole;
impl RoleCheck for UserRole {
    fn role() -> Role {
        Role::User
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoleEntity {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PermissionEntity {
    pub id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
