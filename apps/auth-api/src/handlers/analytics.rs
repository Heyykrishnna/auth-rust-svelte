use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::errors::AppError;
use crate::middleware::AuthenticatedDeveloper;
use crate::models::{ApplicationId, EnvironmentKind};
use crate::services::analytics::{self, AnalyticsWindow, EnvironmentAnalyticsResponse};
use crate::AppState;

fn parse_env_kind(raw: &str) -> Result<EnvironmentKind, AppError> {
    EnvironmentKind::from_db_str(raw.trim().to_lowercase().as_str()).ok_or_else(|| {
        AppError::Validation(format!(
            "Invalid environment kind '{raw}'; expected 'development' or 'production'"
        ))
    })
}

#[derive(Debug, Deserialize, Default)]
pub struct AnalyticsQuery {
    pub window: Option<String>,
}

pub async fn get_analytics(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<EnvironmentAnalyticsResponse>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let window = AnalyticsWindow::parse(query.window.as_deref())?;
    let report = analytics::get_environment_analytics_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        window,
    )
    .await?;
    Ok(Json(report))
}

pub async fn refresh_analytics(
    State(state): State<AppState>,
    dev: AuthenticatedDeveloper,
    Path((app_id, env_kind)): Path<(ApplicationId, String)>,
    Query(query): Query<AnalyticsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kind = parse_env_kind(&env_kind)?;
    let window = AnalyticsWindow::parse(query.window.as_deref())?;
    let buckets_updated = analytics::trigger_environment_rollup_refresh_for_developer(
        &state.db,
        dev.developer_id,
        app_id,
        kind,
        window,
    )
    .await?;
    Ok(Json(json!({
        "window": window.as_str(),
        "granularity": window.granularity(),
        "buckets_updated": buckets_updated
    })))
}
