# Secrets Management

This directory contains the production-grade secrets management setup using the
[External Secrets Operator (ESO)](https://external-secrets.io/) and HashiCorp Vault.

## Two Strategies

### Strategy A — Kubernetes Secrets (learning / dev cluster)

Create secrets directly with `kubectl`. Quick and sufficient for learning:

```bash
kubectl create secret generic postgres-secret \
  --namespace data \
  --from-literal=POSTGRES_USER="authuser" \
  --from-literal=POSTGRES_PASSWORD="$(openssl rand -hex 16)" \
  --from-literal=POSTGRES_DB="authdb"

kubectl create secret generic auth-api-secrets \
  --namespace auth \
  --from-literal=DATABASE_URL="postgres://authuser:<password>@postgres.data.svc.cluster.local:5432/authdb" \
  --from-literal=REDIS_URL="redis://redis.data.svc.cluster.local:6379" \
  --from-literal=JWT_SECRET="$(openssl rand -hex 32)" \
  --from-literal=GOOGLE_CLIENT_ID="<your-client-id>" \
  --from-literal=GOOGLE_CLIENT_SECRET="<your-client-secret>" \
  --from-literal=GITHUB_CLIENT_ID="<your-client-id>" \
  --from-literal=GITHUB_CLIENT_SECRET="<your-client-secret>"
```

**Limitations**: Secrets are not automatically rotated, not audited, and must be
re-created manually on each new cluster.

---

### Strategy B — External Secrets Operator + Vault (production)

ESO acts as a bridge between your cluster and a secret store (Vault, AWS Secrets
Manager, GCP Secret Manager, etc.). It creates and **continuously syncs**
Kubernetes Secrets from the external store.

```
Vault / AWS SM / GCP SM
        │
        │  ESO polls every refreshInterval
        ▼
ExternalSecret (CRD)
        │
        │  ESO creates/updates
        ▼
Kubernetes Secret
        │
        ▼
  Rust API Pod
```

#### Installation

```bash
helm repo add external-secrets https://charts.external-secrets.io
helm repo update
helm install external-secrets external-secrets/external-secrets \
  --namespace external-secrets \
  --create-namespace \
  --set installCRDs=true
```

#### Apply the manifests in this directory

```bash
kubectl apply -k infra/k8s/secrets/external-secrets/
```

---

## Files

```
secrets/
├── README.md                              ← you are here
├── kustomization.yaml                     ← top-level opt-in kustomization
├── external-secrets/
│   ├── kustomization.yaml
│   ├── secretstore.yaml                   ← ClusterSecretStore → Vault
│   ├── externalsecret-auth.yaml           ← ExternalSecret for auth-api-secrets
│   └── externalsecret-data.yaml           ← ExternalSecret for postgres-secret
└── vault/
    └── README.md                          ← How to bootstrap Vault in-cluster
```
