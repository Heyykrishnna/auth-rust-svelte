use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::{from_fn, FromFnLayer, Next};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use ipnet::IpNet;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::OnceLock;

use crate::services::tokens::hash_token;
use crate::AppState;

type LimitFuture = Pin<Box<dyn Future<Output = Response> + Send>>;
type LimitHandler = fn(Extension<AppState>, Request<Body>, Next) -> LimitFuture;
type RateLimitLayer = FromFnLayer<LimitHandler, (), (Extension<AppState>, Request<Body>)>;

const LIMIT_SCRIPT: &str = r#"
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000000 + tonumber(time[2])
local interval = tonumber(ARGV[1])
local capacity = tonumber(ARGV[2])
local previous = tonumber(redis.call('GET', KEYS[1]) or now)
local next = math.max(previous, now) + interval
local wait = next - capacity * interval - now
if wait > 0 then return math.ceil(wait / 1000000) end
redis.call('SET', KEYS[1], next, 'PX', math.ceil(capacity * interval / 1000))
return 0
"#;

pub fn trusted_proxy_ranges() -> Result<Vec<IpNet>, String> {
    let raw = std::env::var("TRUSTED_PROXY_CIDRS").unwrap_or_default();
    parse_trusted_proxy_ranges(&raw)
}

pub fn parse_trusted_proxy_ranges(raw: &str) -> Result<Vec<IpNet>, String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<IpNet>()
                .map_err(|_| "TRUSTED_PROXY_CIDRS contains an invalid CIDR".to_string())
        })
        .collect()
}

#[derive(Clone, Copy)]
pub struct ResolvedClientIp(pub IpAddr);

pub fn resolve_client_ip(
    headers: &HeaderMap,
    peer: IpAddr,
    trusted: &[IpNet],
) -> Result<IpAddr, StatusCode> {
    let mut current = peer.to_canonical();
    if !trusted.iter().any(|range| range.contains(&current)) {
        return Ok(current);
    }
    let Some(forwarded) = headers.get("x-forwarded-for") else {
        return Ok(current);
    };
    let raw = forwarded.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    let hops: Vec<_> = raw.split(',').collect();
    if hops.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }
    for hop in hops.iter().rev() {
        if !trusted.iter().any(|range| range.contains(&current)) {
            break;
        }
        current = hop
            .trim()
            .parse::<IpAddr>()
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .to_canonical();
    }
    Ok(current)
}

pub fn auth_rate_limit_layer() -> RateLimitLayer {
    from_fn(auth_limit as LimitHandler)
}
pub fn sensitive_rate_limit_layer() -> RateLimitLayer {
    from_fn(sensitive_limit as LimitHandler)
}
pub fn api_rate_limit_layer() -> RateLimitLayer {
    from_fn(api_limit as LimitHandler)
}

fn auth_limit(
    Extension(state): Extension<AppState>,
    request: Request<Body>,
    next: Next,
) -> LimitFuture {
    Box::pin(limit(state, request, next, "auth", 2, 5))
}
fn sensitive_limit(
    Extension(state): Extension<AppState>,
    request: Request<Body>,
    next: Next,
) -> LimitFuture {
    Box::pin(limit(state, request, next, "sensitive", 1, 3))
}
fn api_limit(
    Extension(state): Extension<AppState>,
    request: Request<Body>,
    next: Next,
) -> LimitFuture {
    Box::pin(limit(state, request, next, "api", 20, 60))
}

async fn limit(
    state: AppState,
    mut request: Request<Body>,
    next: Next,
    scope: &str,
    rate: u32,
    capacity: u32,
) -> Response {
    static TRUSTED: OnceLock<Result<Vec<IpNet>, String>> = OnceLock::new();
    let Ok(trusted) = TRUSTED.get_or_init(trusted_proxy_ranges) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let ip = match resolve_client_ip(request.headers(), peer.ip(), trusted) {
        Ok(ip) => ip,
        Err(status) => return status.into_response(),
    };
    let plane = request
        .uri()
        .path()
        .split('/')
        .filter(|part| !part.is_empty())
        .take(2)
        .collect::<Vec<_>>()
        .join("/");
    let key = format!(
        "auth:limit:v1:{scope}:{}",
        hash_token(&format!("{plane}:{ip}"))
    );
    let Ok(mut connection) = state.redis.get().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let outcome: redis::RedisResult<i64> = redis::Script::new(LIMIT_SCRIPT)
        .key(key)
        .arg(1_000_000 / rate)
        .arg(capacity)
        .invoke_async(&mut connection)
        .await;
    match outcome {
        Ok(0) => {
            request.extensions_mut().insert(ResolvedClientIp(ip));
            next.run(request).await
        }
        Ok(wait) => (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", wait.to_string())],
            "Too many requests",
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
