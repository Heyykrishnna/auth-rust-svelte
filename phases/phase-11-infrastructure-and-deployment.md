# Phase 11: Production Infrastructure, CI/CD & GitOps Deployment

- **Status:** Planned
- **Prerequisites:** Phase 10 completed (security, load, and DR gates passed)
- **Primary Targets:** `infrastructure/terraform/`, `infrastructure/kubernetes/`, `infrastructure/helm/`, `docker/`, `.github/workflows/`

---

## Objective

Evolve the repository's existing Terraform, Docker, Kubernetes/Kustomize, Helm, ArgoCD, and GitHub Actions scaffolds into a hardened, multi-environment (`dev`, `staging`, `prod`), zero-downtime production deployment pipeline covering `auth-api`, `webhook_worker`, `web` (Hosted Portal + Developer Dashboard), PostgreSQL, Redis, and the observability stack.

---

## 1. Infrastructure as Code (`infrastructure/terraform/`)

Harden existing Terraform modules (`networking`, `database`, `redis`, `kubernetes`):

- Strict VPC network segmentation: public ingress/load balancers, private compute subnets for `auth-api`/`webhook_worker`/`web`, isolated data subnets for managed PostgreSQL (Multi-AZ + automated backups + PITR) and Redis (TLS + AUTH + replication).
- Dedicated egress control / NAT + egress NetworkPolicy for `webhook_worker` (defense-in-depth complementing application-level SSRF IP checks, blocking access to internal cluster services and cloud metadata endpoints `169.254.169.254`).
- Managed KMS key for envelope-encrypting `signing_keys`, OAuth client secrets, and webhook signing secrets; External Secrets Operator / cloud Secret Manager integration.
- TLS certificate automation (cert-manager / ACM) supporting separate origins for API (`api.<domain>`), Hosted Auth Portal (`accounts.<domain>`), and Developer Dashboard (`dashboard.<domain>`).
- Strict environment isolation (`dev`, `staging`, `prod` separate Terraform workspaces/state backends and credentials).

---

## 2. Hardened Containerization (`docker/`)

- Multi-stage builds for `auth-api`, `webhook_worker` (dedicated minimal runtime image/entrypoint), and `web`.
- Non-root UIDs, read-only root filesystems, dropped Linux capabilities (`drop: ["ALL"]`), explicit CPU/memory requests & limits, liveness/readiness/startup probes (`/health/live`, `/health/ready`).
- Immutable digest-pinned image tags, Trivy CVE scanning (fail build on Critical/High), and SPDX/CycloneDX SBOM generation attached to every release.

---

## 3. Kubernetes, Helm & ArgoCD GitOps (`infrastructure/kubernetes/`, `infrastructure/helm/`)

- Add `webhook-worker` Deployment, HPA (scale on `outbox_pending_depth` / CPU), and PodDisruptionBudget alongside `auth-api` and `web`.
- Enforce Kubernetes `NetworkPolicy` manifests (`infrastructure/kubernetes/base/network-policies.yaml`) restricting pod-to-pod traffic strictly to required paths (e.g., only `auth-api` & `webhook-worker` may connect to Postgres/Redis; `webhook-worker` blocked from internal cluster CIDRs and metadata IP).
- Automated pre-deployment SQLx migration Job (with expand-and-contract zero-downtime migration discipline so rolling deployments never break previous pods).
- Kustomize overlays for `dev`, `staging`, and `prod` synced via ArgoCD with automated health checks and rollback verification.

---

## 4. CI/CD Pipelines (`.github/workflows/`)

Harden existing `ci.yml`, `security.yml`, and `release.yml`:

- **PR Quality Gate (`ci.yml`):**
  - `cargo fmt --check`, `cargo clippy -- -D warnings`, Rust unit + Postgres/Redis integration + tenant-isolation tests
  - Migration validation (up/clean boot + expand-contract lint)
  - `apps/web` (`svelte-check`, `eslint`, `vitest`, production `vite build`)
  - All SDK packages (`packages/*` typecheck, unit/contract/Edge-runtime/SSR tests, package export check)
- **Security Gate (`security.yml`):** `cargo audit`, `cargo deny`, `npm audit`, Gitleaks secret scanning, SAST, Trivy container scanning, SBOM attestation.
- **Release & Deployment (`release.yml`):** Build & push signed immutable images -> run migration job -> deploy to `staging` -> run automated smoke & OIDC/webhook E2E test -> gated promotion to `prod` with automated rollback on readiness/SLO failure.

---

## 5. Security & Reliability Operations

- Automated encrypted Postgres daily snapshots + continuous WAL archiving (PITR) + scheduled restore verification job.
- Documented procedures for zero-downtime KMS KEK rotation, asymmetric `signing_keys` scheduled rotation, TLS certificate rotation, audit log retention/archival, and least-privilege production break-glass access.

---

## Deliverables

- Hardened Terraform modules for `dev`, `staging`, `prod`
- Hardened Dockerfiles + SBOM & Trivy scan pipeline for `auth-api`, `webhook_worker`, and `web`
- Updated Helm charts, Kustomize overlays, `NetworkPolicy`, HPA, PDB, migration Job, and ArgoCD manifests
- Complete `.github/workflows/` CI, security, staging smoke-test, and production release pipelines
- Production Deployment & Operations Runbook (`docs/deployment.md`)

---

## Acceptance Criteria

- [ ] A clean push to `main` builds, tests (including tenant isolation & SDKs), scans, migrates, deploys to `staging`, and passes automated OIDC SSO + webhook delivery smoke tests end-to-end.
- [ ] `webhook_worker` runs with Kubernetes `NetworkPolicy` blocking cluster-internal and cloud-metadata egress, verified by an in-cluster connectivity test.
- [ ] Rolling upgrades execute with zero failed requests under continuous synthetic auth traffic.
- [ ] Secrets are injected exclusively from managed secret stores (zero plaintext secrets in Git, images, or manifests).
- [ ] Backup restoration and rollback to the previous immutable image tag are tested and documented in `staging`.

---

## Out of Scope

- Public documentation portal, external developer onboarding, and billing/plan enforcement (Phase 12)

