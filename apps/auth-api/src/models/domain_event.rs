use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const DOMAIN_EVENT_SPEC_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainEventType {
    #[serde(rename = "user.created")]
    UserCreated,
    #[serde(rename = "user.updated")]
    UserUpdated,
    #[serde(rename = "user.deleted")]
    UserDeleted,
    #[serde(rename = "user.suspended")]
    UserSuspended,
    #[serde(rename = "user.email_verified")]
    UserEmailVerified,
    #[serde(rename = "membership.created")]
    MembershipCreated,
    #[serde(rename = "membership.updated")]
    MembershipUpdated,
    #[serde(rename = "membership.revoked")]
    MembershipRevoked,
    #[serde(rename = "session.created")]
    SessionCreated,
    #[serde(rename = "session.revoked")]
    SessionRevoked,
    #[serde(rename = "role.assigned")]
    RoleAssigned,
    #[serde(rename = "role.revoked")]
    RoleRevoked,
    #[serde(rename = "organization.created")]
    OrganizationCreated,
    #[serde(rename = "organization.updated")]
    OrganizationUpdated,
    #[serde(rename = "organization.deleted")]
    OrganizationDeleted,
    #[serde(rename = "organization.member_added")]
    OrganizationMemberAdded,
    #[serde(rename = "organization.member_updated")]
    OrganizationMemberUpdated,
    #[serde(rename = "organization.member_removed")]
    OrganizationMemberRemoved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggregateType {
    User,
    ApplicationUser,
    AppSession,
    Organization,
    OrganizationMember,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainEventEnvelope {
    pub spec_version: String,
    pub event_id: Uuid,
    pub event_type: DomainEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<Uuid>,
    pub occurred_at: DateTime<Utc>,
    pub aggregate_type: AggregateType,
    pub aggregate_id: Uuid,
    pub aggregate_version: i64,
    pub data: serde_json::Value,
}

impl DomainEventEnvelope {
    pub fn new(
        event_type: DomainEventType,
        environment_id: Option<Uuid>,
        aggregate_type: AggregateType,
        aggregate_id: Uuid,
        aggregate_version: i64,
        data: serde_json::Value,
    ) -> Self {
        Self {
            spec_version: DOMAIN_EVENT_SPEC_VERSION.to_string(),
            event_id: Uuid::new_v4(),
            event_type,
            environment_id,
            occurred_at: Utc::now(),
            aggregate_type,
            aggregate_id,
            aggregate_version,
            data,
        }
    }
}
