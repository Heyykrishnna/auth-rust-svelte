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
│   │   ├── base/              # Multi-namespace base (ingress, auth, data, network-policies)
│   │   │   ├── namespaces.yaml# Declares ingress, auth, data, observability, argocd
│   │   │   ├── network-policies.yaml # Zero-trust network policies
│   │   │   ├── auth/          # auth-api, frontend, worker, ingress
│   │   │   └── data/          # postgresql statefulset, redis deployment
│   │   ├── overlays/          # dev + prod patches
│   │   └── observability/     # Full LGTM+OTel stack (Prometheus, Grafana, Loki, Tempo, OTel Collector)
│   ├── argocd/                # Argo CD app-of-apps + child apps (auth, data, observability)
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

### Cluster Architecture

```
cluster
│
├── 🌐 ingress            (Ingress controller, TLS termination, Edge routing)
│
├── 🔐 auth               (Workloads: auth-api, frontend, worker)
│
├── 💾 data               (Stateful tier: PostgreSQL, Redis)
│
├── 📊 observability      (LGTM + OTel: Prometheus, Grafana, Loki, Tempo, OTel Collector)
│
└── 🐙 argocd             (GitOps engine & child applications)
```

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

Create secrets in their respective namespaces:

```bash
# 1. PostgreSQL credentials (in `data` namespace)
kubectl create secret generic postgres-secret \
  --from-literal=POSTGRES_USER=authuser \
  --from-literal=POSTGRES_PASSWORD=your-secure-db-password \
  --from-literal=POSTGRES_DB=authdb \
  -n data

# 2. Auth API & Worker secrets (in `auth` namespace)
kubectl create secret generic auth-api-secrets \
  --from-literal=DATABASE_URL=postgres://authuser:your-secure-db-password@postgres.data.svc.cluster.local:5432/authdb \
  --from-literal=REDIS_URL=redis://redis.data.svc.cluster.local:6379 \
  --from-literal=JWT_SECRET=your-32-byte-secret-key \
  --from-literal=GOOGLE_CLIENT_ID=your-google-id \
  --from-literal=GOOGLE_CLIENT_SECRET=your-google-secret \
  --from-literal=GITHUB_CLIENT_ID=your-github-id \
  --from-literal=GITHUB_CLIENT_SECRET=your-github-secret \
  -n auth
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
