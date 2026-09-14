# HashiCorp Vault — Bootstrap Guide

This guide covers running Vault inside your cluster for the learning project.
For production, use Vault Cloud (HCP) or a dedicated Vault deployment.

## Install Vault via Helm

```bash
helm repo add hashicorp https://helm.releases.hashicorp.com
helm repo update

helm install vault hashicorp/vault \
  --namespace vault \
  --create-namespace \
  --set server.dev.enabled=true   # Dev mode only — DO NOT use in production
```

> [!WARNING]
> `server.dev.enabled=true` runs Vault in-memory with no persistence.
> All secrets are lost on pod restart. Use it only for local learning.

---

## Initialize and Unseal (production mode)

```bash
kubectl exec -n vault vault-0 -- vault operator init \
  -key-shares=5 \
  -key-threshold=3 \
  -format=json > vault-init.json

kubectl exec -n vault vault-0 -- vault operator unseal <unseal-key-1>
kubectl exec -n vault vault-0 -- vault operator unseal <unseal-key-2>
kubectl exec -n vault vault-0 -- vault operator unseal <unseal-key-3>
```

---

## Enable KV v2 and write secrets

```bash
kubectl port-forward -n vault svc/vault 8200:8200 &
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=<root-token>

vault secrets enable -path=secret kv-v2

vault kv put secret/auth-api \
  DATABASE_URL="postgres://authuser:<password>@postgres.data.svc.cluster.local:5432/authdb" \
  REDIS_URL="redis://redis.data.svc.cluster.local:6379" \
  JWT_SECRET="$(openssl rand -hex 32)" \
  GOOGLE_CLIENT_ID="<your-client-id>" \
  GOOGLE_CLIENT_SECRET="<your-client-secret>" \
  GITHUB_CLIENT_ID="<your-client-id>" \
  GITHUB_CLIENT_SECRET="<your-client-secret>"

vault kv put secret/postgres \
  POSTGRES_USER="authuser" \
  POSTGRES_PASSWORD="$(openssl rand -hex 16)" \
  POSTGRES_DB="authdb"
```

---

## Create the ESO Vault token Secret

```bash
kubectl create secret generic vault-token \
  --namespace external-secrets \
  --from-literal=token="${VAULT_TOKEN}"
```

---

## Apply ESO manifests

```bash
kubectl apply -k infra/k8s/secrets/external-secrets/
```

---

## Verify

```bash
kubectl get externalsecret -n auth
kubectl get externalsecret -n data

kubectl describe externalsecret auth-api-secrets -n auth
```
