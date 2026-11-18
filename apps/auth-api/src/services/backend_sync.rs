use std::collections::HashMap;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::{
    AggregateType, AppSessionId, AppUserId, DomainEventEnvelope, DomainEventType, EnvironmentId,
    OrganizationId, UserId,
};
use crate::repositories::db::{begin_tx, commit_tx};
use crate::repositories::outbox;
use crate::repositories::tenant_memberships;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendUserSyncItem {
    pub sub: UserId,
    pub app_user_id: AppUserId,
    pub email: String,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub status: String,
    pub roles: Vec<String>,
    pub public_metadata: serde_json::Value,
    pub private_metadata: serde_json::Value,
    pub external_id: Option<String>,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedBackendUsersResponse {
    pub data: Vec<BackendUserSyncItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEventItem {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub aggregate_version: i64,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedBackendEventsResponse {
    pub data: Vec<BackendEventItem>,
    pub next_event_id: Option<Uuid>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSessionStatusView {
    pub session_id: AppSessionId,
    pub environment_id: EnvironmentId,
    pub app_user_id: AppUserId,
    pub sub: UserId,
    pub status: String,
    pub active_organization_id: Option<OrganizationId>,
    pub last_used_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateBackendUserMetadataInput {
    pub external_id: Option<Option<String>>,
    pub public_metadata: Option<serde_json::Value>,
    pub private_metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncCursorPayload {
    updated_at: DateTime<Utc>,
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct SyncUserRow {
    sub: Uuid,
    app_user_id: Uuid,
    email: String,
    email_verified: bool,
    display_name: String,
    avatar_url: Option<String>,
    status: String,
    public_metadata: serde_json::Value,
    private_metadata: serde_json::Value,
    external_id: Option<String>,
    version: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct UserRoleRow {
    application_user_id: Uuid,
    slug: String,
}

#[derive(Debug, FromRow)]
struct EventCatchupRow {
    id: Uuid,
    event_type: String,
    aggregate_type: String,
    aggregate_id: Uuid,
    aggregate_version: i64,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

pub fn encode_sync_cursor(updated_at: DateTime<Utc>, id: Uuid) -> Result<String, AppError> {
    let payload = SyncCursorPayload { updated_at, id };
    let json = serde_json::to_vec(&payload)
        .map_err(|e| AppError::Internal(format!("Cursor serialization failed: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(json))
}

pub fn decode_sync_cursor(cursor: &str) -> Result<(DateTime<Utc>, Uuid), AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.trim())
        .map_err(|_| AppError::Validation("Invalid sync cursor encoding".to_string()))?;
    let payload: SyncCursorPayload = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Validation("Invalid sync cursor payload".to_string()))?;
    Ok((payload.updated_at, payload.id))
}

async fn load_roles_map(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<String>>, AppError> {
    if app_user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, UserRoleRow>(
        r#"
        SELECT aur.application_user_id, er.slug
        FROM application_user_roles aur
        JOIN environment_roles er
          ON er.environment_id = aur.environment_id AND er.id = aur.role_id
        WHERE aur.environment_id = $1 AND aur.application_user_id = ANY($2)
        ORDER BY er.slug ASC
        "#,
    )
    .bind(environment_id)
    .bind(app_user_ids)
    .fetch_all(db)
    .await?;

    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for row in rows {
        map.entry(row.application_user_id)
            .or_default()
            .push(row.slug);
    }
    Ok(map)
}

fn row_to_sync_item(row: SyncUserRow, roles: Vec<String>) -> BackendUserSyncItem {
    BackendUserSyncItem {
        sub: UserId::from_uuid(row.sub),
        app_user_id: AppUserId::from_uuid(row.app_user_id),
        email: row.email,
        email_verified: row.email_verified,
        display_name: row.display_name,
        avatar_url: row.avatar_url,
        status: row.status,
        roles,
        public_metadata: row.public_metadata,
        private_metadata: row.private_metadata,
        external_id: row.external_id,
        version: row.version,
        updated_at: row.updated_at,
    }
}

pub async fn list_users_for_sync(
    db: &PgPool,
    environment_id: EnvironmentId,
    updated_since: Option<DateTime<Utc>>,
    cursor: Option<&str>,
    limit: Option<i64>,
) -> Result<PaginatedBackendUsersResponse, AppError> {
    let page_size = limit.unwrap_or(50).clamp(1, 100);
    let fetch_limit = page_size + 1;

    let decoded_cursor = match cursor.map(str::trim).filter(|c| !c.is_empty()) {
        Some(raw) => Some(decode_sync_cursor(raw)?),
        None => None,
    };

    let rows = match (updated_since, decoded_cursor) {
        (Some(since), Some((c_ts, c_id))) => {
            sqlx::query_as::<_, SyncUserRow>(
                r#"
                SELECT
                    au.user_id AS sub,
                    au.id AS app_user_id,
                    u.email,
                    u.email_verified,
                    u.display_name,
                    u.avatar_url,
                    au.status,
                    au.public_metadata,
                    au.private_metadata,
                    au.external_id,
                    au.version,
                    au.updated_at
                FROM application_users au
                JOIN users u ON u.id = au.user_id
                WHERE au.environment_id = $1
                  AND au.updated_at >= $2
                  AND (au.updated_at > $3 OR (au.updated_at = $3 AND au.id > $4))
                ORDER BY au.updated_at ASC, au.id ASC
                LIMIT $5
                "#,
            )
            .bind(environment_id)
            .bind(since)
            .bind(c_ts)
            .bind(c_id)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (None, Some((c_ts, c_id))) => {
            sqlx::query_as::<_, SyncUserRow>(
                r#"
                SELECT
                    au.user_id AS sub,
                    au.id AS app_user_id,
                    u.email,
                    u.email_verified,
                    u.display_name,
                    u.avatar_url,
                    au.status,
                    au.public_metadata,
                    au.private_metadata,
                    au.external_id,
                    au.version,
                    au.updated_at
                FROM application_users au
                JOIN users u ON u.id = au.user_id
                WHERE au.environment_id = $1
                  AND (au.updated_at > $2 OR (au.updated_at = $2 AND au.id > $3))
                ORDER BY au.updated_at ASC, au.id ASC
                LIMIT $4
                "#,
            )
            .bind(environment_id)
            .bind(c_ts)
            .bind(c_id)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (Some(since), None) => {
            sqlx::query_as::<_, SyncUserRow>(
                r#"
                SELECT
                    au.user_id AS sub,
                    au.id AS app_user_id,
                    u.email,
                    u.email_verified,
                    u.display_name,
                    u.avatar_url,
                    au.status,
                    au.public_metadata,
                    au.private_metadata,
                    au.external_id,
                    au.version,
                    au.updated_at
                FROM application_users au
                JOIN users u ON u.id = au.user_id
                WHERE au.environment_id = $1
                  AND au.updated_at >= $2
                ORDER BY au.updated_at ASC, au.id ASC
                LIMIT $3
                "#,
            )
            .bind(environment_id)
            .bind(since)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, SyncUserRow>(
                r#"
                SELECT
                    au.user_id AS sub,
                    au.id AS app_user_id,
                    u.email,
                    u.email_verified,
                    u.display_name,
                    u.avatar_url,
                    au.status,
                    au.public_metadata,
                    au.private_metadata,
                    au.external_id,
                    au.version,
                    au.updated_at
                FROM application_users au
                JOIN users u ON u.id = au.user_id
                WHERE au.environment_id = $1
                ORDER BY au.updated_at ASC, au.id ASC
                LIMIT $2
                "#,
            )
            .bind(environment_id)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
    };

    let has_more = (rows.len() as i64) > page_size;
    let page_rows: Vec<SyncUserRow> = rows.into_iter().take(page_size as usize).collect();

    let next_cursor = if has_more {
        page_rows
            .last()
            .map(|r| encode_sync_cursor(r.updated_at, r.app_user_id))
            .transpose()?
    } else {
        None
    };

    let ids: Vec<Uuid> = page_rows.iter().map(|r| r.app_user_id).collect();
    let mut roles_map = load_roles_map(db, environment_id, &ids).await?;

    let data = page_rows
        .into_iter()
        .map(|row| {
            let roles = roles_map.remove(&row.app_user_id).unwrap_or_default();
            row_to_sync_item(row, roles)
        })
        .collect();

    Ok(PaginatedBackendUsersResponse {
        data,
        next_cursor,
        has_more,
    })
}

pub async fn get_user_by_app_user_id(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<BackendUserSyncItem, AppError> {
    let row = sqlx::query_as::<_, SyncUserRow>(
        r#"
        SELECT
            au.user_id AS sub,
            au.id AS app_user_id,
            u.email,
            u.email_verified,
            u.display_name,
            u.avatar_url,
            au.status,
            au.public_metadata,
            au.private_metadata,
            au.external_id,
            au.version,
            au.updated_at
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1 AND au.id = $2
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let mut roles_map = load_roles_map(db, environment_id, &[row.app_user_id]).await?;
    let roles = roles_map.remove(&row.app_user_id).unwrap_or_default();
    Ok(row_to_sync_item(row, roles))
}

pub async fn get_user_by_sub(
    db: &PgPool,
    environment_id: EnvironmentId,
    sub: UserId,
) -> Result<BackendUserSyncItem, AppError> {
    let row = sqlx::query_as::<_, SyncUserRow>(
        r#"
        SELECT
            au.user_id AS sub,
            au.id AS app_user_id,
            u.email,
            u.email_verified,
            u.display_name,
            u.avatar_url,
            au.status,
            au.public_metadata,
            au.private_metadata,
            au.external_id,
            au.version,
            au.updated_at
        FROM application_users au
        JOIN users u ON u.id = au.user_id
        WHERE au.environment_id = $1 AND au.user_id = $2
        "#,
    )
    .bind(environment_id)
    .bind(sub.into_inner())
    .fetch_optional(db)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let mut roles_map = load_roles_map(db, environment_id, &[row.app_user_id]).await?;
    let roles = roles_map.remove(&row.app_user_id).unwrap_or_default();
    Ok(row_to_sync_item(row, roles))
}

pub async fn update_user_metadata(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    input: UpdateBackendUserMetadataInput,
) -> Result<BackendUserSyncItem, AppError> {
    let mut tx = begin_tx(db).await?;
    let existing =
        tenant_memberships::find_application_user_by_id(&mut *tx, environment_id, app_user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

    let next_external_id = match input.external_id {
        Some(opt) => opt.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        None => existing.external_id,
    };
    let next_public = input.public_metadata.unwrap_or(existing.public_metadata);
    let next_private = input.private_metadata.unwrap_or(existing.private_metadata);

    let updated = sqlx::query_as::<_, crate::models::ApplicationUser>(
        r#"
        UPDATE application_users
        SET
            external_id = $3,
            public_metadata = $4,
            private_metadata = $5,
            updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .bind(&next_external_id)
    .bind(&next_public)
    .bind(&next_private)
    .fetch_one(&mut *tx)
    .await?;

    let evt = DomainEventEnvelope::new(
        DomainEventType::MembershipUpdated,
        Some(environment_id.into_inner()),
        AggregateType::ApplicationUser,
        updated.id.into_inner(),
        updated.version,
        serde_json::json!({
            "sub": updated.user_id,
            "app_user_id": updated.id,
            "external_id": updated.external_id,
            "public_metadata": updated.public_metadata,
            "status": updated.status,
            "version": updated.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &evt).await?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            environment_id, actor_type, event, resource_type, resource_id, metadata
        )
        VALUES ($1, 'secret_key', 'USER_METADATA_UPDATED', 'application_user', $2, $3)
        "#,
    )
    .bind(environment_id)
    .bind(updated.id.into_inner())
    .bind(serde_json::json!({
        "app_user_id": updated.id,
        "external_id": updated.external_id,
        "version": updated.version
    }))
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    get_user_by_app_user_id(db, environment_id, app_user_id).await
}

pub async fn suspend_application_user(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
    reason: Option<&str>,
) -> Result<BackendUserSyncItem, AppError> {
    let mut tx = begin_tx(db).await?;
    let updated = sqlx::query_as::<_, crate::models::ApplicationUser>(
        r#"
        UPDATE application_users
        SET status = 'suspended', updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let revoked_sessions: Vec<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1 AND application_user_id = $2 AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(updated.user_id.into_inner())
    .execute(&mut *tx)
    .await?;

    let suspend_evt = DomainEventEnvelope::new(
        DomainEventType::UserSuspended,
        Some(environment_id.into_inner()),
        AggregateType::ApplicationUser,
        updated.id.into_inner(),
        updated.version,
        serde_json::json!({
            "sub": updated.user_id,
            "app_user_id": updated.id,
            "status": updated.status,
            "reason": reason,
            "version": updated.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &suspend_evt).await?;

    for sess_id in &revoked_sessions {
        let sess_evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(environment_id.into_inner()),
            AggregateType::AppSession,
            *sess_id,
            2,
            serde_json::json!({
                "session_id": sess_id,
                "sub": updated.user_id,
                "app_user_id": updated.id,
                "reason": "user_suspended"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &sess_evt).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            environment_id, user_id, actor_type, event, resource_type, resource_id, metadata
        )
        VALUES ($1, $2, 'secret_key', 'APPLICATION_USER_SUSPENDED', 'application_user', $3, $4)
        "#,
    )
    .bind(environment_id)
    .bind(updated.user_id.into_inner())
    .bind(updated.id.into_inner())
    .bind(serde_json::json!({
        "reason": reason,
        "revoked_sessions": revoked_sessions.len()
    }))
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    get_user_by_app_user_id(db, environment_id, app_user_id).await
}

pub async fn restore_application_user(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<BackendUserSyncItem, AppError> {
    let mut tx = begin_tx(db).await?;
    let updated = sqlx::query_as::<_, crate::models::ApplicationUser>(
        r#"
        UPDATE application_users
        SET status = 'active', updated_at = NOW()
        WHERE environment_id = $1 AND id = $2
        RETURNING id, environment_id, user_id, status, external_id, public_metadata, private_metadata, version, joined_at, last_active_at, created_at, updated_at
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::UserNotFound)?;

    let restore_evt = DomainEventEnvelope::new(
        DomainEventType::MembershipUpdated,
        Some(environment_id.into_inner()),
        AggregateType::ApplicationUser,
        updated.id.into_inner(),
        updated.version,
        serde_json::json!({
            "sub": updated.user_id,
            "app_user_id": updated.id,
            "status": updated.status,
            "version": updated.version
        }),
    );
    outbox::insert_outbox_event(&mut *tx, environment_id, &restore_evt).await?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            environment_id, user_id, actor_type, event, resource_type, resource_id, metadata
        )
        VALUES ($1, $2, 'secret_key', 'APPLICATION_USER_RESTORED', 'application_user', $3, $4)
        "#,
    )
    .bind(environment_id)
    .bind(updated.user_id.into_inner())
    .bind(updated.id.into_inner())
    .bind(serde_json::json!({ "version": updated.version }))
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    get_user_by_app_user_id(db, environment_id, app_user_id).await
}

pub async fn list_events_catchup(
    db: &PgPool,
    environment_id: EnvironmentId,
    after_event_id: Option<Uuid>,
    types: Option<&[String]>,
    limit: Option<i64>,
) -> Result<PaginatedBackendEventsResponse, AppError> {
    let page_size = limit.unwrap_or(50).clamp(1, 100);
    let fetch_limit = page_size + 1;

    let anchor: Option<(DateTime<Utc>, Uuid)> = if let Some(after_id) = after_event_id {
        let row: Option<(DateTime<Utc>, Uuid)> = sqlx::query_as(
            r#"
            SELECT created_at, id
            FROM outbox_events
            WHERE environment_id = $1 AND id = $2
            "#,
        )
        .bind(environment_id)
        .bind(after_id)
        .fetch_optional(db)
        .await?;

        let found = row.ok_or_else(|| {
            AppError::Validation("after_event_id not found in this environment".to_string())
        })?;
        Some(found)
    } else {
        None
    };

    let filtered_types = types.filter(|t| !t.is_empty());

    let rows = match (anchor, filtered_types) {
        (Some((a_ts, a_id)), Some(event_types)) => {
            sqlx::query_as::<_, EventCatchupRow>(
                r#"
                SELECT id, event_type, aggregate_type, aggregate_id, aggregate_version, payload, created_at
                FROM outbox_events
                WHERE environment_id = $1
                  AND (created_at > $2 OR (created_at = $2 AND id > $3))
                  AND event_type = ANY($4)
                ORDER BY created_at ASC, id ASC
                LIMIT $5
                "#,
            )
            .bind(environment_id)
            .bind(a_ts)
            .bind(a_id)
            .bind(event_types)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (Some((a_ts, a_id)), None) => {
            sqlx::query_as::<_, EventCatchupRow>(
                r#"
                SELECT id, event_type, aggregate_type, aggregate_id, aggregate_version, payload, created_at
                FROM outbox_events
                WHERE environment_id = $1
                  AND (created_at > $2 OR (created_at = $2 AND id > $3))
                ORDER BY created_at ASC, id ASC
                LIMIT $4
                "#,
            )
            .bind(environment_id)
            .bind(a_ts)
            .bind(a_id)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (None, Some(event_types)) => {
            sqlx::query_as::<_, EventCatchupRow>(
                r#"
                SELECT id, event_type, aggregate_type, aggregate_id, aggregate_version, payload, created_at
                FROM outbox_events
                WHERE environment_id = $1
                  AND event_type = ANY($2)
                ORDER BY created_at ASC, id ASC
                LIMIT $3
                "#,
            )
            .bind(environment_id)
            .bind(event_types)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, EventCatchupRow>(
                r#"
                SELECT id, event_type, aggregate_type, aggregate_id, aggregate_version, payload, created_at
                FROM outbox_events
                WHERE environment_id = $1
                ORDER BY created_at ASC, id ASC
                LIMIT $2
                "#,
            )
            .bind(environment_id)
            .bind(fetch_limit)
            .fetch_all(db)
            .await?
        }
    };

    let has_more = (rows.len() as i64) > page_size;
    let page_rows: Vec<EventCatchupRow> = rows.into_iter().take(page_size as usize).collect();
    let next_event_id = if has_more {
        page_rows.last().map(|r| r.id)
    } else {
        None
    };

    let data = page_rows
        .into_iter()
        .map(|r| BackendEventItem {
            id: r.id,
            event_type: r.event_type,
            aggregate_type: r.aggregate_type,
            aggregate_id: r.aggregate_id,
            aggregate_version: r.aggregate_version,
            payload: r.payload,
            created_at: r.created_at,
        })
        .collect();

    Ok(PaginatedBackendEventsResponse {
        data,
        next_event_id,
        has_more,
    })
}

pub async fn get_session_status(
    db: &PgPool,
    environment_id: EnvironmentId,
    session_id: AppSessionId,
) -> Result<BackendSessionStatusView, AppError> {
    let session = tenant_memberships::find_app_session_by_id(db, environment_id, session_id)
        .await?
        .ok_or(AppError::SessionNotFound)?;

    let status = if session.revoked_at.is_some() {
        "revoked"
    } else if Utc::now() > session.expires_at {
        "expired"
    } else {
        "active"
    };

    Ok(BackendSessionStatusView {
        session_id: session.id,
        environment_id: session.environment_id,
        app_user_id: session.application_user_id,
        sub: session.user_id,
        status: status.to_string(),
        active_organization_id: session.active_organization_id,
        last_used_at: session.last_used_at,
        expires_at: session.expires_at,
        revoked_at: session.revoked_at,
        created_at: session.created_at,
    })
}

pub async fn revoke_session(
    db: &PgPool,
    environment_id: EnvironmentId,
    session_id: AppSessionId,
) -> Result<BackendSessionStatusView, AppError> {
    let mut tx = begin_tx(db).await?;
    let session = tenant_memberships::find_app_session_by_id(&mut *tx, environment_id, session_id)
        .await?
        .ok_or(AppError::SessionNotFound)?;

    if session.revoked_at.is_none() {
        tenant_memberships::revoke_app_session(&mut *tx, environment_id, session_id).await?;
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = NOW()
            WHERE environment_id = $1 AND app_session_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(environment_id)
        .bind(session_id.into_inner())
        .execute(&mut *tx)
        .await?;

        let evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(environment_id.into_inner()),
            AggregateType::AppSession,
            session_id.into_inner(),
            2,
            serde_json::json!({
                "session_id": session_id,
                "sub": session.user_id,
                "app_user_id": session.application_user_id,
                "reason": "backend_revoked"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &evt).await?;

        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                environment_id, user_id, actor_type, event, resource_type, resource_id, metadata
            )
            VALUES ($1, $2, 'secret_key', 'SESSION_REVOKED', 'app_session', $3, $4)
            "#,
        )
        .bind(environment_id)
        .bind(session.user_id.into_inner())
        .bind(session_id.into_inner())
        .bind(serde_json::json!({ "reason": "backend_revoked" }))
        .execute(&mut *tx)
        .await?;
    }

    commit_tx(tx).await?;
    get_session_status(db, environment_id, session_id).await
}

pub async fn revoke_all_user_sessions(
    db: &PgPool,
    environment_id: EnvironmentId,
    app_user_id: AppUserId,
) -> Result<u64, AppError> {
    let mut tx = begin_tx(db).await?;
    let membership =
        tenant_memberships::find_application_user_by_id(&mut *tx, environment_id, app_user_id)
            .await?
            .ok_or(AppError::UserNotFound)?;

    let revoked_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE app_sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE environment_id = $1 AND application_user_id = $2 AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(environment_id)
    .bind(app_user_id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked_at = NOW()
        WHERE environment_id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#,
    )
    .bind(environment_id)
    .bind(membership.user_id.into_inner())
    .execute(&mut *tx)
    .await?;

    for sess_id in &revoked_ids {
        let evt = DomainEventEnvelope::new(
            DomainEventType::SessionRevoked,
            Some(environment_id.into_inner()),
            AggregateType::AppSession,
            *sess_id,
            2,
            serde_json::json!({
                "session_id": sess_id,
                "sub": membership.user_id,
                "app_user_id": app_user_id,
                "reason": "backend_revoke_all"
            }),
        );
        outbox::insert_outbox_event(&mut *tx, environment_id, &evt).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            environment_id, user_id, actor_type, event, resource_type, resource_id, metadata
        )
        VALUES ($1, $2, 'secret_key', 'SESSION_REVOKED', 'application_user', $3, $4)
        "#,
    )
    .bind(environment_id)
    .bind(membership.user_id.into_inner())
    .bind(app_user_id.into_inner())
    .bind(serde_json::json!({
        "reason": "backend_revoke_all",
        "revoked_count": revoked_ids.len()
    }))
    .execute(&mut *tx)
    .await?;

    commit_tx(tx).await?;
    Ok(revoked_ids.len() as u64)
}
