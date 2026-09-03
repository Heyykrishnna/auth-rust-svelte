use anyhow::Result;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self, Sampler},
    Resource,
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::config::AppConfig;

pub fn init_telemetry(config: &AppConfig) -> Result<()> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(&config.otel_exporter_otlp_endpoint),
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", config.otel_service_name.clone()),
                    KeyValue::new("service.version", config.otel_service_version.clone()),
                ])),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    Ok(())
}
