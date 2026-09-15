# Secure Auth Platform — Helm Chart

A production-grade Helm chart for deploying the Secure Authentication Platform (`auth-api` + `web` + ingress + autoscaling) onto Kubernetes clusters.

## Chart Components

- **Auth API Deployment & Service**: Rust Axum backend with liveness/readiness probes and autoscaling.
- **Web Frontend Deployment & Service**: SvelteKit server runtime.
- **Ingress**: Configurable host and path routing with TLS support.
- **Autoscaling (HPA)**: Horizontal Pod Autoscaler targeting CPU utilization.
- **Pod Disruption Budget (PDB)**: Ensuring zero-downtime rolling node updates.

## Values Environments

- `values.yaml`: Default configuration.
- `values-dev.yaml`: Single replica, reduced resources, debug log levels.
- `values-prod.yaml`: 3+ replicas, auto-scaling up to 20 pods, production resource limits.

## Installation

```bash
# Deploy dev environment
helm upgrade --install secure-auth ./infrastructure/helm -f ./infrastructure/helm/values-dev.yaml -n auth --create-namespace

# Deploy prod environment
helm upgrade --install secure-auth ./infrastructure/helm -f ./infrastructure/helm/values-prod.yaml -n auth --create-namespace
```
