# Deployment & Operations Guide

This guide covers deploying `secure-auth-platform` across local development, Kubernetes via Kustomize, unified Helm packaging, and automated GitOps with Argo CD.

---

## 1. Local Development (Docker Compose)

### 1.1 Prerequisites
- Docker Engine 24+ & Docker Compose v2+
- Node.js 22+ & npm 10+
- Rust 1.80+ (for local host compilation)
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
kubectl apply -f infrastructure/kubernetes/argocd/app-of-apps.yaml
```

Once applied, Argo CD continuously tracks the `main` branch. When CI triggers `.github/workflows/release.yml`, new container image digests are committed to `infrastructure/kubernetes/prod/kustomization.yaml`, and Argo CD reconciles the cluster automatically without manual intervention.
