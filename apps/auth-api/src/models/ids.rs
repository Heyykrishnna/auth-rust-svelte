use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_uuid_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
            sqlx::Type,
        )]
        #[sqlx(transparent)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn into_inner(self) -> Uuid {
                self.0
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

define_uuid_id!(DeveloperId);
define_uuid_id!(WorkspaceId);
define_uuid_id!(ApplicationId);
define_uuid_id!(EnvironmentId);
define_uuid_id!(UserId);
define_uuid_id!(UserEmailId);
define_uuid_id!(AppUserId);
define_uuid_id!(SsoSessionId);
define_uuid_id!(AppSessionId);
define_uuid_id!(OrganizationId);
define_uuid_id!(OrganizationMemberId);
define_uuid_id!(RoleId);
define_uuid_id!(PermissionId);
define_uuid_id!(WebhookEndpointId);
define_uuid_id!(OutboxEventId);
