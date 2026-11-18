# Deployment & Operations Guide

This guide covers deploying `secure-auth-platform` across local development, Kubernetes via Kustomize, unified Helm packaging, and automated GitOps with Argo CD.

---

## 1. Local Development (Docker Compose)

### 1.1 Prerequisites
- Docker Engine 24+ & Docker Compose v2+
- Node.js 24 LTS & npm 11+
- Rust 1.96+ (for local host compilation)
- GNU Make

### 1.2 Running the Full Stack
1. Prepare your environment file:
   ```bash
   cp .env.example .env
   ```
2. Launch all services with hot-reloading:
   ```bash
   make dev
   # Or explicitly:
   docker compose -f docker-compose.yml -f docker-compose.override.yml up --build
   ```

### 1.3 Service URLs & Ports

| Service | Port | Endpoint | Notes |
|---------|------|----------|-------|
| **Web Frontend** | 3000 | `http://localhost:3000` | SvelteKit dev server with HMR |
| **Auth API** | 8080 | `http://localhost:8080` | Rust Axum with `cargo watch` |
| **PostgreSQL** | 5432 | `localhost:5432` | `authdb` (user: `authuser`) |
| **Redis** | 6379 | `localhost:6379` | Token families & rate limits |
| **Grafana** | 3001 | `http://localhost:3001` | Pre-configured dashboard (admin/admin) |
| **Prometheus** | 9090 | `http://localhost:9090` | Scrapes API metrics |
| **Tempo** | 3200 | `http://localhost:3200` | Tracing endpoint |
| **Loki** | 3100 | `http://localhost:3100` | Log aggregation endpoint |
| **OTel Collector** | 4317 / 4318 | `localhost:4317` | OTLP gRPC / HTTP receiver |

---

## 2. Kubernetes Deployment (Kustomize)

The platform organizes Kubernetes resources hierarchically:
- `infrastructure/kubernetes/base/`: Canonical definitions of namespaces, network policies, deployments, services, and HPAs.
- `infrastructure/kubernetes/dev/`: Dev overlay (1 replica, debug log levels, dev image tags).
- `infrastructure/kubernetes/prod/`: Production overlay (HA 3+ replicas, production resource limits, production container registries).

### Deploying Overlays

```bash
# Verify dev manifests
kustomize build infrastructure/kubernetes/dev | kubectl apply --dry-run=client -f -

# Deploy dev
kubectl apply -k infrastructure/kubernetes/dev

# Verify prod manifests
kustomize build infrastructure/kubernetes/prod | kubectl apply --dry-run=client -f -

# Deploy prod
kubectl apply -k infrastructure/kubernetes/prod
```

---

## 3. Helm Chart Deployment

For environments managed via Helm releases:

```bash
# Validate chart syntax
helm lint infrastructure/helm

# Deploy Development Release
helm upgrade --install secure-auth-dev ./infrastructure/helm \
  -f ./infrastructure/helm/values-dev.yaml \
  -n auth --create-namespace

# Deploy Production Release
helm upgrade --install secure-auth-prod ./infrastructure/helm \
  -f ./infrastructure/helm/values-prod.yaml \
  -n auth --create-namespace
```

---

## 4. GitOps with Argo CD

The platform uses an **App-of-Apps** pattern located in `infrastructure/kubernetes/argocd/`:

```text
infrastructure/kubernetes/argocd/
├── app-of-apps.yaml    # Root application provisioning child apps
├── project.yaml        # Argo CD AppProject definition
└── apps/
    ├── auth.yaml       # Syncs infrastructure/kubernetes/prod auth workloads
    ├── data.yaml       # Syncs PostgreSQL & Redis
    └── observability.yaml # Syncs Prometheus, Grafana, Loki, Tempo
```

### Bootstrapping Argo CD

```bash
# Apply root App-of-Apps to your Argo CD cluster
kubectl apply -f infrastructure/kubernetes/argocd/project.yaml
kubectl apply -f infrastructure/kubernetes/argocd/app-of-apps.yaml
```

Argo CD tracks `main` after bootstrap and repository access are configured. Image differences are reconciled; the worker runs `/app/webhook-worker` from the API image.

## 5. GitHub Actions

- `ci.yml` validates workflow syntax, Kustomize overlays, and Helm values; runs Rust formatting, Clippy, unit tests, and the explicitly ignored database integration suite; checks, lints, tests, and builds the web application; then builds both container images independently.
- Successful pushes to `main` publish `ghcr.io/heyykrishnna/auth-api:sha-<full commit SHA>` and `ghcr.io/heyykrishnna/web:sha-<full commit SHA>`. Pull requests and manual CI runs build without publishing.
- `security.yml` audits the complete dependency lockfiles, scans Git history, and builds and scans both images for HIGH/CRITICAL vulnerabilities. Scan reports and SPDX SBOMs are retained as artifacts. SARIF uploads are skipped for fork pull requests, whose tokens cannot write security events; vulnerability findings still fail the scan.
- `release.yml` reacts to completed CI and security runs. Automatic deployment requires successful push runs of both workflows for the exact same commit, and that commit must still be the head of `main`. It checks both registry images before updating production image tags. Scheduled security scans and fork pull requests cannot trigger production updates.
- Manual release dispatch on `main` accepts an existing image tag for rollback or promotion. It validates the tag and verifies both images exist. This is an explicit operator override of the automatic commit gate.
- The GitOps commit uses `[skip ci]` to avoid another image build. A concurrent change to `main` causes the push to fail safely; it never force-pushes.

Repository settings must permit the Actions token to publish GHCR packages and update the production manifest on `main`. Existing GHCR packages must grant this repository Actions access. If branch protection prohibits bot pushes, route the manifest change through the required review process. Configure registry pull credentials for private packages and replace example domains and production secrets before bootstrapping a cluster. A successful manifest update does not verify a cluster rollout; check Argo CD health and sync status separately.

### Local checks

```bash
npm ci
npm run check
npm run lint
npm test
npm run build
cargo fmt --manifest-path apps/auth-api/Cargo.toml --check
cargo clippy --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features
TEST_DATABASE_URL=postgres://authuser:authpassword@127.0.0.1:5432/authdb_test \
  cargo test --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features -- --ignored --test-threads=1
```

The database suite requires an isolated PostgreSQL database and Redis at `127.0.0.1:6399`, matching CI. Tests apply migrations themselves. Run one integration suite at a time against a given database; concurrent runs can interfere with shared signing-key state.
