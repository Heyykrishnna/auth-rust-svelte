use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::{ApplicationId, EnvironmentKind};
use crate::repositories::webhooks::WebhookDeliveryRecord;
use crate::services::webhooks::{
    self, CreateWebhookInput, CreatedWebhookEndpointResponse, RotatedWebhookSecretResponse,
    UpdateWebhookInput, WebhookEndpointView,
};
use crate::AppState;

fn parse_env_kind(raw: &str) -> Result<EnvironmentKind, AppError> {
    EnvironmentKind::from_db_str(raw)
        .ok_or_else(|| AppError::Validation(format!("Invalid environment kind '{raw}'")))
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookEndpointRequest {
    pub url: String,
    pub description: Option<String>,
    #[serde(default)]
    pub subscribed_events: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateWebhookEndpointRequest {
    pub url: Option<String>,
    pub description: Option<String>,
    pub subscribed_events: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RotateWebhookSecretRequest {
    pub grace_hours: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListDeliveriesQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReplayRangeRequest {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub event_types: Option<Vec<String>>,
}

pub async fn list_webhooks(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
) -> Result<Json<Vec<WebhookEndpointView>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let list = webhooks::list_webhook_endpoints(&state.db, dev.developer_id, app_id, kind).await?;
    Ok(Json(list))
}

pub async fn create_webhook(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Json(body): Json<CreateWebhookEndpointRequest>,
) -> Result<(StatusCode, Json<CreatedWebhookEndpointResponse>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let created = webhooks::create_webhook_endpoint(
        &state.db,
        &state.config,
        dev.developer_id,
        app_id,
        kind,
        CreateWebhookInput {
            url: body.url,
            description: body.description,
            subscribed_events: body.subscribed_events,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn update_webhook(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
    Json(body): Json<UpdateWebhookEndpointRequest>,
) -> Result<Json<WebhookEndpointView>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let updated = webhooks::update_webhook_endpoint(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        endpoint_id,
        UpdateWebhookInput {
            url: body.url,
            description: body.description,
            subscribed_events: body.subscribed_events,
            status: body.status,
        },
    )
    .await?;
    Ok(Json(updated))
}

pub async fn delete_webhook(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<StatusCode, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    webhooks::delete_webhook_endpoint(&state.db, dev.developer_id, app_id, kind, endpoint_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn rotate_webhook_secret(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
    body: Option<Json<RotateWebhookSecretRequest>>,
) -> Result<Json<RotatedWebhookSecretResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let grace_hours = body.and_then(|Json(b)| b.grace_hours);
    let rotated = webhooks::rotate_webhook_endpoint_secret(
        &state.db,
        &state.config,
        dev.developer_id,
        app_id,
        kind,
        endpoint_id,
        grace_hours,
    )
    .await?;
    Ok(Json(rotated))
}

pub async fn test_webhook(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
) -> Result<(StatusCode, Json<WebhookDeliveryRecord>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let delivery =
        webhooks::send_test_webhook_ping(&state.db, dev.developer_id, app_id, kind, endpoint_id)
            .await?;
    Ok((StatusCode::ACCEPTED, Json(delivery)))
}

pub async fn list_deliveries(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
    Query(query): Query<ListDeliveriesQuery>,
) -> Result<Json<Vec<WebhookDeliveryRecord>>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let deliveries = webhooks::list_webhook_deliveries(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        endpoint_id,
        query.limit,
    )
    .await?;
    Ok(Json(deliveries))
}

pub async fn replay_delivery(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id, delivery_id)): Path<(ApplicationId, String, Uuid, Uuid)>,
) -> Result<(StatusCode, Json<WebhookDeliveryRecord>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let replayed = webhooks::replay_single_delivery(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        endpoint_id,
        delivery_id,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(replayed)))
}

pub async fn replay_range(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind, endpoint_id)): Path<(ApplicationId, String, Uuid)>,
    Json(body): Json<ReplayRangeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let enqueued = webhooks::replay_endpoint_time_range(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        endpoint_id,
        body.from,
        body.to,
        body.event_types,
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(json!({ "enqueued": enqueued }))))
}
