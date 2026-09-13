use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::errors::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{Permission, PermissionCheck, Role, RoleCheck};
use crate::AppState;

pub async fn extract_or_authenticate_user(
    req: &mut axum::extract::Request,
) -> Result<AuthenticatedUser, AppError> {
    if let Some(user) = req.extensions().get::<AuthenticatedUser>().cloned() {
        return Ok(user);
    }

    let state = req.extensions().get::<AppState>().cloned().ok_or_else(|| {
        AppError::Internal("AppState not found in request extensions".to_string())
    })?;

    let (mut parts, body) = std::mem::take(req).into_parts();
    let user_res = AuthenticatedUser::from_request_parts(&mut parts, &state).await;
    *req = axum::extract::Request::from_parts(parts, body);

    let user = user_res?;
    req.extensions_mut().insert(user.clone());
    Ok(user)
}

#[derive(Debug, Clone)]
pub struct RequirePermissionLayer {
    permission: Permission,
}

pub fn require_permission(permission: Permission) -> RequirePermissionLayer {
    RequirePermissionLayer { permission }
}

impl<S> Layer<S> for RequirePermissionLayer {
    type Service = RequirePermissionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequirePermissionService {
            inner,
            permission: self.permission.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequirePermissionService<S> {
    inner: S,
    permission: Permission,
}

impl<S> Service<axum::extract::Request> for RequirePermissionService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::extract::Request) -> Self::Future {
        let permission = self.permission.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let user = match extract_or_authenticate_user(&mut req).await {
                Ok(u) => u,
                Err(err) => return Ok(err.into_response()),
            };

            if !user.has_permission(&permission) {
                return Ok(AppError::Forbidden(format!(
                    "Missing required permission: {}",
                    permission
                ))
                .into_response());
            }

            inner.call(req).await
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequireRoleLayer {
    role: Role,
}

pub fn require_role(role: Role) -> RequireRoleLayer {
    RequireRoleLayer { role }
}

impl<S> Layer<S> for RequireRoleLayer {
    type Service = RequireRoleService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireRoleService {
            inner,
            role: self.role.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequireRoleService<S> {
    inner: S,
    role: Role,
}

impl<S> Service<axum::extract::Request> for RequireRoleService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::extract::Request) -> Self::Future {
        let role = self.role.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let user = match extract_or_authenticate_user(&mut req).await {
                Ok(u) => u,
                Err(err) => return Ok(err.into_response()),
            };

            if !user.has_role(&role) {
                return Ok(
                    AppError::Forbidden(format!("Missing required role: {}", role)).into_response(),
                );
            }

            inner.call(req).await
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequireAnyPermissionLayer {
    permissions: Vec<Permission>,
}

pub fn require_any_permission(permissions: Vec<Permission>) -> RequireAnyPermissionLayer {
    RequireAnyPermissionLayer { permissions }
}

impl<S> Layer<S> for RequireAnyPermissionLayer {
    type Service = RequireAnyPermissionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireAnyPermissionService {
            inner,
            permissions: self.permissions.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequireAnyPermissionService<S> {
    inner: S,
    permissions: Vec<Permission>,
}

impl<S> Service<axum::extract::Request> for RequireAnyPermissionService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::extract::Request) -> Self::Future {
        let permissions = self.permissions.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let user = match extract_or_authenticate_user(&mut req).await {
                Ok(u) => u,
                Err(err) => return Ok(err.into_response()),
            };

            if !user.has_any_permission(&permissions) {
                return Ok(AppError::Forbidden(
                    "User lacks any of the required permissions".to_string(),
                )
                .into_response());
            }

            inner.call(req).await
        })
    }
}

#[derive(Debug, Clone)]
pub struct RequireAllPermissionsLayer {
    permissions: Vec<Permission>,
}

pub fn require_all_permissions(permissions: Vec<Permission>) -> RequireAllPermissionsLayer {
    RequireAllPermissionsLayer { permissions }
}

impl<S> Layer<S> for RequireAllPermissionsLayer {
    type Service = RequireAllPermissionsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireAllPermissionsService {
            inner,
            permissions: self.permissions.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequireAllPermissionsService<S> {
    inner: S,
    permissions: Vec<Permission>,
}

impl<S> Service<axum::extract::Request> for RequireAllPermissionsService<S>
where
    S: Service<axum::extract::Request, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::extract::Request) -> Self::Future {
        let permissions = self.permissions.clone();
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let user = match extract_or_authenticate_user(&mut req).await {
                Ok(u) => u,
                Err(err) => return Ok(err.into_response()),
            };

            if !user.has_all_permissions(&permissions) {
                return Ok(AppError::Forbidden(
                    "User lacks one or more required permissions".to_string(),
                )
                .into_response());
            }

            inner.call(req).await
        })
    }
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
