# Observability Architecture

This directory defines the configuration for the complete telemetry and observability stack of `secure-auth-platform`:

```
┌──────────────────────────────────────────────────────────┐
│                   Apps (auth-api & web)                  │
└────────────┬─────────────────────────────┬───────────────┘
             │ OTLP traces & metrics       │ Logs (JSON stdout)
             ▼                             ▼
┌──────────────────────────┐          ┌──────────────┐
│  OpenTelemetry Collector │          │     Loki     │
└──────┬────────────┬──────┘          └──────┬───────┘
       │            │                        │
       ▼            ▼                        ▼
┌─────────────┐┌──────────────┐       ┌──────────────┐
│ Prometheus  ││    Tempo     │       │   Grafana    │
│  (Metrics)  ││   (Traces)   │◄──────┤ (Dashboards) │
└─────────────┘└──────────────┘       └──────────────┘
```

## Components

1. **Prometheus (`prometheus/`)**:
   - `prometheus-local.yaml`: Scrapes `auth-api:8080/metrics` and `otel-collector:8889` every 15s.
2. **Grafana (`grafana/`)**:
   - `grafana-datasources.yaml`: Auto-provisions Prometheus, Loki, and Tempo data sources with trace-to-log correlation.
   - `grafana-dashboards.yaml`: Auto-loads pre-configured dashboards.
   - `dashboards/auth-dashboard.json`: Real-time HTTP RPS, latency percentiles (p50, p95, p99), auth status codes, token verification rates, and active sessions.
3. **Loki (`loki/`)**:
   - `loki-local.yaml`: Multi-tenant log aggregation system with structured index and schema.
4. **Tempo (`tempo/`)**:
   - `tempo-local.yaml`: Distributed tracing backend supporting OTLP gRPC/HTTP ingest.
5. **OpenTelemetry Collector (`otel-collector/`)**:
   - `otel-collector-local.yaml`: Ingests OTLP from `auth-api`, processes batching, exports to Prometheus and Tempo.

## Local Access

When running via `docker compose up`:

- **Grafana**: [http://localhost:3001](http://localhost:3001) (Credentials: `admin` / `admin`)
- **Prometheus**: [http://localhost:9090](http://localhost:9090)
- **Tempo**: [http://localhost:3200](http://localhost:3200)
- **Loki**: [http://localhost:3100](http://localhost:3100)
