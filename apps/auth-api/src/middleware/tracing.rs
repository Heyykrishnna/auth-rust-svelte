use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use opentelemetry::propagation::{Extractor, Injector, TextMapPropagator};
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{
    trace::{Config, Sampler},
    Resource,
};
use tracing::{info, info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::config::AppConfig;
use crate::services::metrics as platform_metrics;

const REDACTED: &str = "[REDACTED]";

const SENSITIVE_JSON_KEYS: &[&str] = &[
    "password",
    "new_password",
    "current_password",
    "password_hash",
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "invitation_token",
    "reset_token",
    "otp",
    "code",
    "secret",
    "secret_key",
    "client_secret",
    "signing_secret",
    "authorization",
    "cookie",
    "set-cookie",
    "private_key",
    "x-webhook-signature",
];

struct MapCarrier(HashMap<String, String>);

impl Injector for MapCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_lowercase(), value);
    }
}

impl Extractor for MapCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(&key.to_lowercase()).map(String::as_str)
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

pub fn classify_plane(path: &str) -> &'static str {
    if path.starts_with("/oauth2") || path.starts_with("/.well-known") {
        "oidc"
    } else if path.starts_with("/v1/client") {
        "client"
    } else if path.starts_with("/v1/backend") {
        "backend"
    } else if path.starts_with("/v1/control") {
        "control"
    } else if path.starts_with("/health") {
        "health"
    } else if path.starts_with("/metrics") {
        "metrics"
    } else {
        "legacy"
    }
}

pub fn redact_sensitive_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut idx = 0usize;
    let bytes = raw.as_bytes();
    while idx < bytes.len() {
        let rest = &raw[idx..];
        let matched_prefix = [
            "sk_live_", "sk_test_", "whsec_", "orginv_", "Bearer ", "eyJ",
        ]
        .iter()
        .find(|prefix| rest.starts_with(*prefix));

        if let Some(prefix) = matched_prefix {
            if *prefix == "Bearer " {
                out.push_str("Bearer ");
                idx += prefix.len();
            }
            let token_rest = &raw[idx..];
            let token_len = token_rest
                .bytes()
                .take_while(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(*b, b'_' | b'-' | b'.' | b'=' | b'+' | b'/')
                })
                .count();
            if *prefix == "eyJ" && token_rest[..token_len].matches('.').count() < 2 {
                out.push_str(&raw[idx..idx + 3]);
                idx += 3;
                continue;
            }
            out.push_str(REDACTED);
            idx += token_len.max(1);
            continue;
        }

        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            idx += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}

pub fn redact_sensitive_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let lower = k.to_lowercase();
                if SENSITIVE_JSON_KEYS.iter().any(|sensitive| {
                    lower == *sensitive || lower.ends_with(&format!("_{sensitive}"))
                }) {
                    *v = serde_json::Value::String(REDACTED.to_string());
                } else {
                    redact_sensitive_json(v);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                redact_sensitive_json(item);
            }
        }
        serde_json::Value::String(s) => {
            *s = redact_sensitive_text(s);
        }
        _ => {}
    }
}

pub fn current_traceparent() -> Option<String> {
    let cx = tracing::Span::current().context();
    let span_ref = cx.span();
    let span_ctx = span_ref.span_context();
    if !span_ctx.is_valid() {
        return None;
    }
    let propagator = TraceContextPropagator::new();
    let mut carrier = MapCarrier(HashMap::new());
    propagator.inject_context(&cx, &mut carrier);
    carrier.0.remove("traceparent")
}

pub fn extract_context_from_traceparent(traceparent: &str) -> Context {
    let mut map = HashMap::new();
    map.insert("traceparent".to_string(), traceparent.trim().to_string());
    let carrier = MapCarrier(map);
    TraceContextPropagator::new().extract(&carrier)
}

pub fn current_trace_and_span_ids() -> (String, String) {
    let cx = tracing::Span::current().context();
    let span_ref = cx.span();
    let span_ctx = span_ref.span_context();
    if span_ctx.is_valid() {
        (
            span_ctx.trace_id().to_string(),
            span_ctx.span_id().to_string(),
        )
    } else {
        (String::new(), String::new())
    }
}

pub async fn telemetry_correlation_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().as_str().to_string();
    let path = req.uri().path().to_string();
    let plane = classify_plane(&path);
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let span = info_span!(
        "http_request",
        request_id = %request_id,
        plane = plane,
        method = %method,
        path = %path,
        trace_id = tracing::field::Empty,
        span_id = tracing::field::Empty,
        status = tracing::field::Empty
    );

    async move {
        let (trace_id, span_id) = current_trace_and_span_ids();
        if !trace_id.is_empty() {
            tracing::Span::current().record("trace_id", trace_id.as_str());
            tracing::Span::current().record("span_id", span_id.as_str());
        }

        let response = next.run(req).await;
        let status = response.status().as_u16();
        let elapsed = start.elapsed().as_secs_f64();

        tracing::Span::current().record("status", status);
        platform_metrics::record_http_plane_request(plane, status, elapsed);

        if plane != "health" && plane != "metrics" {
            info!(
                request_id = %request_id,
                trace_id = %trace_id,
                span_id = %span_id,
                plane = plane,
                method = %method,
                path = %path,
                status = status,
                latency_ms = (elapsed * 1000.0).round() as u64,
                "Handled HTTP request"
            );
        }

        response
    }
    .instrument(span)
    .await
}

pub fn init_telemetry(config: &AppConfig) -> Result<()> {
    init_service_telemetry(config, &config.otel_service_name)
}

pub fn init_worker_telemetry(config: &AppConfig) -> Result<()> {
    init_service_telemetry(config, "webhook-worker")
}

fn init_service_telemetry(config: &AppConfig, service_name: &str) -> Result<()> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otel_exporter_otlp_endpoint),
        )
        .with_trace_config(
            Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", config.otel_service_version.clone()),
                ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let tracer = provider.tracer(service_name.to_string());
    opentelemetry::global::set_tracer_provider(provider);
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_current_span(true)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    Ok(())
}
