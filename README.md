# 🔐 Secure Auth Platform

> Production-grade authentication and identity platform built with Rust (Axum), SvelteKit 5, PostgreSQL 16, Redis 7, Terraform, Kubernetes (Kustomize & Helm), Argo CD GitOps, and OpenTelemetry.

[![CI](https://github.com/your-org/auth-rust-svelte/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/auth-rust-svelte/actions/workflows/ci.yml)
[![Security & Compliance](https://github.com/your-org/auth-rust-svelte/actions/workflows/security.yml/badge.svg)](https://github.com/your-org/auth-rust-svelte/actions/workflows/security.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-violet.svg)](LICENSE)

---

## Architecture Overview

```
Internet → Cloudflare / Edge WAF (TLS Termination + Rate Limiting)
         → Kubernetes Ingress (NGINX)
           ├── /api/* → Rust Auth API (Axum)       [apps/auth-api]
           │            ├── PostgreSQL 16 (Users, Sessions, Audit Logs, OIDC)
           │            ├── Redis 7 (Token Families, Session Cache, Rate Limiting)
           │            └── External IdPs (Google, GitHub OIDC)
           └── /*     → SvelteKit Web Application  [apps/web]
         → Observability Stack                     [observability/]
           └── OpenTelemetry Collector
               ├── Tempo (Distributed Traces)
               ├── Loki (Structured Logs)
               └── Prometheus → Grafana (Metrics & Dashboards)
```

For in-depth architecture diagrams, sequence charts, and design details, see [docs/architecture.md](docs/architecture.md).

---

## Repository Structure

```text
secure-auth-platform/
│
├── apps/
│   ├── auth-api/          # Rust 1.80 + Axum REST & Auth Service
│   └── web/               # SvelteKit 5 + TypeScript Web Frontend
│
├── packages/
│   └── shared-types/      # Canonical TypeScript DTOs & API Contracts
│
├── infrastructure/
│   ├── terraform/         # Cloud IaC (VPC, EKS, RDS PostgreSQL, Redis)
│   ├── kubernetes/        # GitOps K8s manifests (base, dev, prod overlays)
│   │   ├── base/
│   │   ├── dev/
│   │   └── prod/
│   └── helm/              # Unified Helm Chart (values-dev, values-prod)
│
├── observability/
│   ├── prometheus/        # Scrape configs and metric definitions
│   ├── grafana/           # Datasources and pre-configured auth dashboards
│   ├── loki/              # High-performance log aggregation
│   └── tempo/             # Distributed tracing backend
│
├── .github/
│   └── workflows/
│       ├── ci.yml         # Fast feedback: tests, clippy, svelte-check, vitest
│       ├── security.yml   # Cargo audit, npm audit, Gitleaks, Trivy CVE scan
│       └── release.yml    # GitOps production manifest tag bump
│
├── docker/
│   ├── Dockerfile.api     # Multi-stage distroless Rust build
│   ├── Dockerfile.web     # Multi-stage Alpine Node.js build
│   └── README.md
│
├── docs/
│   ├── architecture.md    # System topology, layers, and service boundaries
│   ├── authentication.md  # Argon2id, JWT lifecycle, refresh rotation, OIDC
│   ├── security.md        # RBAC matrix, token-bucket limits, audit trails
│   └── deployment.md      # Docker Compose, K8s overlays, Helm & Argo CD
│
├── docker-compose.yml     # Local orchestration for all 9 services
└── README.md              # Monorepo developer guide
```

---

## Quick Start (Local Development)

### Prerequisites
- [Docker Engine & Docker Compose](https://docs.docker.com/get-docker/)
- [Node.js 22+](https://nodejs.org/) & `npm`
- [Rust 1.80+](https://rustup.rs/) (optional for local host compilation)
- `make`

### 1. Launch Services
```bash
# 1. Copy sample environment
cp .env.example .env

# 2. Start full stack (Hot-Reloading enabled for both API and Web)
make dev
```

### 2. Service Endpoints

| Service | Local URL | Description | Default Credentials |
|---------|-----------|-------------|---------------------|
| **Web Frontend** | [http://localhost:3000](http://localhost:3000) | SvelteKit UI | — |
| **Auth API** | [http://localhost:8080](http://localhost:8080) | Axum REST Service | — |
| **Grafana** | [http://localhost:3001](http://localhost:3001) | Metrics & Tracing Dashboards | `admin` / `admin` |
| **Prometheus** | [http://localhost:9090](http://localhost:9090) | Metric Scraper | — |
| **Tempo** | [http://localhost:3200](http://localhost:3200) | Tracing Backend | — |
| **Loki** | [http://localhost:3100](http://localhost:3100) | Log Collector | — |
| **PostgreSQL** | `localhost:5432` | Relational Database | `authuser` / `authpassword` |
| **Redis** | `localhost:6379` | In-Memory Cache | — |

---

## Developer Commands

```bash
# Testing
make test             # Run Rust tests + Vitest frontend suite
make test-api         # Run cargo test --all-features
make test-web         # Run npm -w @secure-auth/web run test

# Linting & Type Checking
make lint             # Check formatting, run clippy and eslint
npm run check         # Type-check shared-types and web via TypeScript

# Database
make migrate          # Run pending SQLx database migrations
make db-reset         # Reinitialize and wipe development database
make db-shell         # Open interactive psql shell

# Security
make audit            # Run cargo-audit + npm audit --workspaces
make scan             # Scan container images with Trivy

# Kubernetes & GitOps
make k8s-apply-dev    # Deploy development Kustomize overlay
make k8s-apply-prod   # Deploy production Kustomize overlay
make argocd-bootstrap # Bootstrap Argo CD App-of-Apps
```

---

## Core Documentation

- 📐 **[System Architecture](docs/architecture.md)** — Architectural design, data flow diagrams, and tech stack specification.
- 🔑 **[Authentication Specs](docs/authentication.md)** — Argon2id password hashing, JWT pairs, refresh token rotation with reuse detection, and OIDC flows.
- 🛡️ **[Security Architecture](docs/security.md)** — Role-Based Access Control (RBAC), rate-limiting matrices, security headers, and compliance audits.
- 🚀 **[Deployment Guide](docs/deployment.md)** — Comprehensive operations guide for Docker Compose, Kubernetes, Helm, and Argo CD GitOps.

---

## License

This project is licensed under the [MIT License](LICENSE).
