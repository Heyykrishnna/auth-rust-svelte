use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use std::marker::PhantomData;

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{Permission, PermissionCheck, Role, RoleCheck};
use crate::AppState;

pub fn require_permission(
    permission: Permission,
) -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn(
        move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let perm = permission.clone();
            async move {
                let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
                    Some(u) => u,
                    None => {
                        let state = req
                            .extensions()
                            .get::<AppState>()
                            .cloned()
                            .ok_or_else(|| {
                                AppError::Internal(
                                    "AppState not found in request extensions".to_string(),
                                )
                            })?;
                        let (mut parts, body) = req.into_parts();
                        let user =
                            AuthenticatedUser::from_request_parts(&mut parts, &state).await?;
                        req = axum::extract::Request::from_parts(parts, body);
                        req.extensions_mut().insert(user.clone());
                        user
                    }
                };

                if !user.has_permission(&perm) {
                    return Err(AppError::Forbidden(format!(
                        "Missing required permission: {}",
                        perm
                    )));
                }

                Ok::<Response, AppError>(next.run(req).await)
            }
        },
    )
}

pub fn require_role(role: Role) -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn(
        move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let r = role.clone();
            async move {
                let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
                    Some(u) => u,
                    None => {
                        let state = req
                            .extensions()
                            .get::<AppState>()
                            .cloned()
                            .ok_or_else(|| {
                                AppError::Internal(
                                    "AppState not found in request extensions".to_string(),
                                )
                            })?;
                        let (mut parts, body) = req.into_parts();
                        let user =
                            AuthenticatedUser::from_request_parts(&mut parts, &state).await?;
                        req = axum::extract::Request::from_parts(parts, body);
                        req.extensions_mut().insert(user.clone());
                        user
                    }
                };

                if !user.has_role(&r) {
                    return Err(AppError::Forbidden(format!(
                        "Missing required role: {}",
                        r
                    )));
                }

                Ok::<Response, AppError>(next.run(req).await)
            }
        },
    )
}

pub fn require_any_permission(
    permissions: Vec<Permission>,
) -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn(
        move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let perms = permissions.clone();
            async move {
                let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
                    Some(u) => u,
                    None => {
                        let state = req
                            .extensions()
                            .get::<AppState>()
                            .cloned()
                            .ok_or_else(|| {
                                AppError::Internal(
                                    "AppState not found in request extensions".to_string(),
                                )
                            })?;
                        let (mut parts, body) = req.into_parts();
                        let user =
                            AuthenticatedUser::from_request_parts(&mut parts, &state).await?;
                        req = axum::extract::Request::from_parts(parts, body);
                        req.extensions_mut().insert(user.clone());
                        user
                    }
                };

                if !user.has_any_permission(&perms) {
                    return Err(AppError::Forbidden(
                        "User lacks any of the required permissions".to_string(),
                    ));
                }

                Ok::<Response, AppError>(next.run(req).await)
            }
        },
    )
}

pub fn require_all_permissions(
    permissions: Vec<Permission>,
) -> impl tower::Layer<axum::routing::Route> + Clone {
    axum::middleware::from_fn(
        move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let perms = permissions.clone();
            async move {
                let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
                    Some(u) => u,
                    None => {
                        let state = req
                            .extensions()
                            .get::<AppState>()
                            .cloned()
                            .ok_or_else(|| {
                                AppError::Internal(
                                    "AppState not found in request extensions".to_string(),
                                )
                            })?;
                        let (mut parts, body) = req.into_parts();
                        let user =
                            AuthenticatedUser::from_request_parts(&mut parts, &state).await?;
                        req = axum::extract::Request::from_parts(parts, body);
                        req.extensions_mut().insert(user.clone());
                        user
                    }
                };

                if !user.has_all_permissions(&perms) {
                    return Err(AppError::Forbidden(
                        "User lacks one or more required permissions".to_string(),
                    ));
                }

                Ok::<Response, AppError>(next.run(req).await)
            }
        },
    )
}

#[derive(Debug, Clone)]
pub struct RequirePermission<P: PermissionCheck>(pub AuthenticatedUser, pub PhantomData<P>);

#[async_trait]
impl<P: PermissionCheck> FromRequestParts<AppState> for RequirePermission<P> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;

        let required = P::permission();
        if !user.has_permission(&required) {
            return Err(AppError::Forbidden(format!(
                "Missing required permission: {}",
                required
            )));
        }

        Ok(Self(user, PhantomData))
    }
}

#[derive(Debug, Clone)]
pub struct RequireRole<R: RoleCheck>(pub AuthenticatedUser, pub PhantomData<R>);

#[async_trait]
impl<R: RoleCheck> FromRequestParts<AppState> for RequireRole<R> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;

        let required = R::role();
        if !user.has_role(&required) {
            return Err(AppError::Forbidden(format!(
                "Missing required role: {}",
                required
            )));
        }

        Ok(Self(user, PhantomData))
    }
}
