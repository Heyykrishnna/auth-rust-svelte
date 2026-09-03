use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::UserProfile;
use crate::repositories::users as user_repo;
use crate::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(
        min = 2,
        max = 100,
        message = "Display name must be between 2 and 100 characters"
    ))]
    pub display_name: String,
    pub avatar_url: Option<String>,
}

pub async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Json<UserProfile>, AppError> {
    let user_entity = user_repo::find_user_by_id(&state.db, user.user_id)
        .await?
        .ok_or(AppError::UserNotFound)?;

    Ok(Json(UserProfile::from(user_entity)))
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfile>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let updated = user_repo::update_profile(
        &state.db,
        user.user_id,
        &payload.display_name,
        payload.avatar_url.as_deref(),
    )
    .await?;

    Ok(Json(UserProfile::from(updated)))
}
