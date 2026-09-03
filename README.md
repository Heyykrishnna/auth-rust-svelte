# 🔐 auth-rust-svelte

> Production-grade authentication platform — Rust (Axum) + SvelteKit + PostgreSQL + Redis + Kubernetes + Argo CD + OpenTelemetry

[![CI](https://github.com/your-org/auth-rust-svelte/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/auth-rust-svelte/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-violet.svg)](LICENSE)

---

## Architecture

```
Internet → Cloudflare (WAF + DDoS + TLS)
         → Kubernetes
           → NGINX Ingress
             → SvelteKit Frontend    (apps/frontend)
             → Rust Auth API (Axum)  (apps/auth-api)
               ├── PostgreSQL        (sessions, users, OIDC accounts)
               ├── Redis             (session cache, JWT blacklist)
               └── OIDC             (Google, GitHub)
         → Observability
           → OpenTelemetry Collector
             ├── Tempo   (traces)
             ├── Loki    (logs)
             └── Prometheus → Grafana (metrics + dashboards)
```

### DevOps Flow

```
Developer → GitHub → CI (Rust/Svelte tests, Clippy, security scan)
                  → Container Registry (ghcr.io)
                  → Argo CD → Kubernetes
```

---

## Quick Start (Local Dev)

### Prerequisites
- Docker + Docker Compose
- `make`

### Start everything

```bash
cp .env.example .env      # fill in your secrets
make dev                  # starts all 9 services
```

| Service      | URL                        |
|-------------|----------------------------|
| Frontend    | http://localhost:3000       |
| Auth API    | http://localhost:8080       |
| Grafana     | http://localhost:3001       |
| Prometheus  | http://localhost:9090       |
| Tempo       | http://localhost:3200       |

### Common commands

```bash
make test          # run all tests (Rust + Svelte)
make lint          # clippy + fmt check + eslint
make migrate       # run SQLx migrations
make db-shell      # open psql shell
make redis-shell   # open redis-cli
make build         # build Docker images
make audit         # cargo-audit + npm audit
```

---

## Project Structure

```
auth-rust-svelte/
├── apps/
│   ├── frontend/              # SvelteKit 5 (Svelte Runes)
│   │   └── src/
│   │       ├── lib/
│   │       │   ├── api/       # Type-safe API client
│   │       │   ├── stores/    # auth.svelte.ts (Runes state)
│   │       │   └── components/
│   │       └── routes/        # login, register, dashboard, callback
│   │
│   └── auth-api/              # Rust Axum API (DDD)
│       └── src/
│           ├── domain/        # User, Session, Token entities
│           ├── application/   # Use cases (register, login, logout, refresh, oidc)
│           ├── infrastructure/# PostgreSQL, Redis, OTel adapters
│           └── api/           # Axum routes, handlers, middleware
│
├── infra/
│   ├── k8s/
│   │   ├── base/              # Kustomize base (frontend, auth-api, postgres, redis, ingress)
│   │   ├── overlays/          # dev + prod patches
│   │   └── observability/     # OTel, Prometheus, Grafana, ServiceMonitors
│   ├── argocd/                # Argo CD app-of-apps + child apps
│   └── otel/                  # Local observability config files
│
├── .github/workflows/
│   ├── ci.yml                 # Tests, Clippy, fmt, Trivy
│   ├── docker-build.yml       # Build + push + cosign sign
│   └── deploy.yml             # Update Kustomize tags → trigger Argo CD
│
├── docker-compose.yml         # Full local stack
├── docker-compose.override.yml# Hot-reload overrides
└── Makefile                   # All convenience targets
```

---

## Auth API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/auth/register` | Register with email + password |
| `POST` | `/auth/login`    | Login, get JWT pair |
| `POST` | `/auth/logout`   | Revoke access token (blacklist JTI) |
| `POST` | `/auth/refresh`  | Rotate refresh token, get new pair |
| `GET`  | `/auth/me`       | Get current user profile |
| `GET`  | `/auth/oidc/:provider` | Get OIDC authorization URL |
| `POST` | `/auth/oidc/:provider/callback` | Handle OIDC callback |
| `GET`  | `/health`  | Liveness probe |
| `GET`  | `/ready`   | Readiness probe (checks DB + Redis) |
| `GET`  | `/metrics` | Prometheus metrics |

---

## Kubernetes Deployment

### Prerequisites
- Kubernetes cluster
- `kubectl` + `kustomize`
- NGINX Ingress Controller
- `cert-manager` (Let's Encrypt)
- Argo CD

### Bootstrap Argo CD (once)

```bash
kubectl apply -f infra/argocd/app-of-apps.yaml
```

### Manual apply

```bash
make k8s-apply-dev    # dev overlay
make k8s-apply-prod   # prod overlay
make k8s-diff         # diff before applying
```

### Required Secrets

Create these secrets in the `auth-system` namespace before deploying:

```bash
# Auth API secrets
kubectl create secret generic auth-api-secrets \
  --from-literal=DATABASE_URL=postgres://... \
  --from-literal=REDIS_URL=redis://... \
  --from-literal=JWT_SECRET=... \
  --from-literal=GOOGLE_CLIENT_ID=... \
  --from-literal=GOOGLE_CLIENT_SECRET=... \
  --from-literal=GITHUB_CLIENT_ID=... \
  --from-literal=GITHUB_CLIENT_SECRET=... \
  -n auth-system

# PostgreSQL credentials
kubectl create secret generic postgres-secret \
  --from-literal=POSTGRES_USER=authuser \
  --from-literal=POSTGRES_PASSWORD=... \
  --from-literal=POSTGRES_DB=authdb \
  -n auth-system
```

---

## Security Features

- **Argon2id** password hashing (memory-hard, tunable)
- **JWT** access tokens (15 min) + **refresh tokens** (7 days) with rotation
- **Token blacklisting** on logout (Redis TTL-matched)
- **Refresh token reuse detection** (detects token theft)
- **Rate limiting** per IP (tower-governor in Rust, NGINX annotations in K8s)
- **Non-root containers** with `readOnlyRootFilesystem`
- **Pod Anti-Affinity** for HA across nodes
- **PodDisruptionBudget** ensures 2+ auth-api replicas during drains
- **SBOM generation** + **cosign image signing** in CI
- **Trivy** vulnerability scanning on every PR

---

## TODO — Before Production

Replace all `# TODO` comments:

- [ ] `infra/argocd/*.yaml` — set your GitHub repo URL
- [ ] `infra/k8s/base/ingress.yaml` — set your domain (`auth.example.com`)
- [ ] `infra/k8s/overlays/prod/kustomization.yaml` — set your registry (`ghcr.io/your-org`)
- [ ] `.github/workflows/docker-build.yml` — set `ORG` variable
- [ ] `.github/workflows/deploy.yml` — set `ARGOCD_SERVER` + `ARGOCD_TOKEN` secrets
- [ ] `.env` — fill in all secrets (never commit!)
- [ ] OIDC — provide Google/GitHub OAuth2 app credentials

---

## License

MIT
