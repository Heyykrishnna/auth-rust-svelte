# Signet

Authentication infrastructure for every application.

Signet is a multi-tenant authentication and identity platform built with Rust and SvelteKit. It gives independent applications a shared identity service while keeping their credentials, memberships, sessions, and development/production environments separate.

[![CI](https://github.com/Heyykrishnna/auth-rust-svelte/actions/workflows/ci.yml/badge.svg)](https://github.com/Heyykrishnna/auth-rust-svelte/actions/workflows/ci.yml)
[![Security](https://github.com/Heyykrishnna/auth-rust-svelte/actions/workflows/security.yml/badge.svg)](https://github.com/Heyykrishnna/auth-rust-svelte/actions/workflows/security.yml)

**Status: active development.** The identity backend, OIDC/SSO flows, and synchronization infrastructure are implemented and tested. Framework SDK work, expanded dashboard functionality, and later milestones are tracked in the [implementation roadmap](phases/README.md). Deployment templates require environment-specific configuration before use.

## Contents

- [Capabilities](#capabilities)
- [Architecture](#architecture)
- [Quick start](#quick-start)
- [Local services](#local-services)
- [Configuration](#configuration)
- [Application integration](#application-integration)
- [Development and validation](#development-and-validation)
- [Deployment and GitHub Actions](#deployment-and-github-actions)
- [Repository layout](#repository-layout)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Capabilities

| Area                   | Implemented functionality                                                                                                                       |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Application management | Developer accounts, workspaces, application registration, development/production environments, scoped credentials, and domain configuration     |
| Identity               | Email/password authentication, verification codes, password recovery, profile updates, and upstream Google/GitHub OAuth flows                   |
| Tokens and sessions    | ES256 signing keys and JWKS, access tokens, refresh rotation and reuse detection, session listing, and revocation                               |
| OIDC and SSO           | Discovery, authorization-code flow with PKCE, hosted authentication transactions, enrollment policies, and local/global logout                  |
| Synchronization        | Cursor-based user reconciliation, metadata updates, environment-scoped suspension, session management, and event catch-up                       |
| Webhooks               | Transactional outbox, environment-isolated fan-out, signed delivery, retries, dead-letter handling, secret rotation, and replay                 |
| Operations             | Health/readiness endpoints, Prometheus metrics, structured logging, OpenTelemetry tracing, containers, and Kubernetes deployment configurations |

The [phase documents](phases/README.md) distinguish completed work from planned APIs and integrations. Treat examples in future-phase specifications as design targets until their implementation is available.

## Architecture

The platform separates three API surfaces:

| Surface                 | Base path     | Purpose                                                                           | Authentication                                          |
| ----------------------- | ------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Developer Control Plane | `/v1/control` | Manage workspaces, applications, environments, credentials, domains, and webhooks | Developer session or developer bearer token             |
| Client API              | `/v1/client`  | Authenticate users and manage their current profile and sessions                  | Environment publishable key; user tokens where required |
| Backend API             | `/v1/backend` | Reconcile users, retrieve events, and perform privileged management operations    | Environment secret key with the required scopes         |

A **User** is a global identity. An **Application User** links that identity to one application environment. An **SSO Session** represents the central identity-provider session; an **App Session** belongs to a specific environment. These boundaries are defined in the [domain glossary](GLOSSARY.md).

```text
Browser / consuming application
    |
    +-- SvelteKit web application
    |     +-- Developer Control Plane
    |     +-- Hosted Auth Portal
    |
    +-- Rust identity API
          +-- /v1/control, /v1/client, /v1/backend
          +-- /oauth2 and /.well-known
          +-- PostgreSQL: identities, memberships, sessions, keys, outbox
          +-- Redis: ephemeral authentication state and caches
          +-- OpenTelemetry and Prometheus instrumentation

Webhook worker
    +-- Reads PostgreSQL outbox/delivery records
    +-- Sends signed events to consuming backends
```

The API applies database migrations at startup. The webhook worker is a separate executable and should start after the schema has been initialized.

### Technology stack

| Component      | Technology                                                     |
| -------------- | -------------------------------------------------------------- |
| Backend        | Rust 1.96+, Axum 0.7, Tokio, SQLx 0.9                          |
| Frontend       | Svelte 5, SvelteKit 3, TypeScript 6, Vite 8                    |
| Data services  | PostgreSQL 16 and Redis 7 in Compose and CI                    |
| Testing        | Rust tests, Vitest 5, and Node security regressions            |
| Packaging      | npm workspaces, Cargo, Docker/BuildKit, Alpine runtimes        |
| Infrastructure | Kubernetes/Kustomize, Helm, Argo CD, and AWS Terraform modules |
| Observability  | OpenTelemetry Collector, Prometheus, Grafana, Tempo, and Loki  |

Use the manifests and committed lockfiles as the dependency source of truth.

## Quick start

### Prerequisites

For container-based development, install Docker with Docker Compose v2. GNU Make is optional; equivalent Compose commands are shown below.

For native development or local checks, also install Node.js 24 LTS with npm, Rust 1.96 or newer, and access to PostgreSQL and Redis. CI uses Node.js 24 and Rust 1.96.0.

### 1. Clone and configure

```bash
git clone https://github.com/Heyykrishnna/auth-rust-svelte.git
cd auth-rust-svelte
npm run setup:env
```

Review `.env` before starting. Its sample database, Redis, and Grafana credentials are for local development. Generate separate values for `JWT_SECRET` and `SIGNING_KEY_KEK`, each at least 32 characters long:

```bash
openssl rand -base64 48
```

Run the command separately for each secret and place the results in `.env`. Configure SMTP to receive verification and password-reset emails; configure OAuth credentials only for providers you intend to use.

### 2. Start the development stack

```bash
make dev
```

Equivalent command:

```bash
docker compose -f docker-compose.yml -f docker-compose.override.yml up --build
```

The development override enables API and frontend hot reload. The first run compiles the Rust services and installs frontend dependencies, so it takes longer than subsequent starts.

### 3. Check service health

```bash
curl --fail http://localhost:8080/health
curl --fail http://localhost:8080/ready
```

Open the [Developer Control Plane](http://localhost:3000/control/login) to create a developer account and configure an application. Hosted portal sign-in starts from an OIDC authorization transaction; opening the portal directly does not create one.

### 4. Run the webhook worker when needed

The default Compose stack does not start a worker service. With the API and database running, start the packaged worker separately:

```bash
docker compose -f docker-compose.yml run --rm --build --no-deps \
  --entrypoint /app/webhook-worker auth-api
```

For native development:

```bash
cd apps/auth-api
cargo run --locked --bin webhook-worker
```

Both executables read the application configuration. The container command uses Compose's internal database connection; the native command uses your host-accessible `.env` settings.

### 5. Stop services

```bash
make stop
```

Stopping containers preserves named database volumes. `make clean` removes volumes and build artifacts; use it only when you intend to discard local state.

## Local services

| Service                 | Address                                        | Purpose                                                |
| ----------------------- | ---------------------------------------------- | ------------------------------------------------------ |
| Web application         | [localhost:3000](http://localhost:3000)        | Developer dashboard, hosted portal, and application UI |
| Auth API                | [localhost:8080](http://localhost:8080/health) | Authentication, identity, OIDC, and management APIs    |
| Grafana                 | [localhost:3001](http://localhost:3001)        | Dashboards and configured telemetry data sources       |
| Prometheus              | [localhost:9090](http://localhost:9090)        | API metrics                                            |
| Tempo                   | `localhost:3200`                               | Trace backend                                          |
| Loki                    | `localhost:3100`                               | Log backend                                            |
| OpenTelemetry Collector | `localhost:4317` / `localhost:4318`            | OTLP gRPC / HTTP ingestion                             |
| PostgreSQL              | `localhost:5432`                               | Application database                                   |
| Redis                   | `localhost:6379`                               | Password-protected ephemeral storage                   |

Database, Redis, and Grafana credentials come from `.env` and Compose defaults. See [observability configuration](observability/README.md) for the telemetry stack.

## Configuration

The complete example is in [`.env.example`](.env.example); configuration parsing lives in [`apps/auth-api/src/config/mod.rs`](apps/auth-api/src/config/mod.rs).

| Variables                                                       | Purpose                                                                                          |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `DATABASE_URL`, `DATABASE_MAX_CONNECTIONS`                      | PostgreSQL connection and pool size                                                              |
| `REDIS_URL`, `REDIS_PASSWORD`                                   | API Redis connection and Compose Redis authentication                                            |
| `JWT_SECRET`                                                    | Secret used by the authentication configuration; requires at least 32 characters                 |
| `ISSUER_URL`                                                    | Public identity-provider issuer URL                                                              |
| `SIGNING_KEY_KEK`, `SIGNING_KEY_KEK_VERSION`                    | Encryption material and version for stored signing keys; the key requires at least 32 characters |
| `AUTH_API_HOST`, `AUTH_API_PORT`                                | API bind address and port                                                                        |
| `CORS_ORIGINS`, `COOKIE_SECURE`, `COOKIE_DOMAIN`                | Browser origins and cookie policy                                                                |
| `PUBLIC_API_BASE_URL`, `PORTAL_ORIGIN`, `ORIGIN`                | Browser API URL, hosted portal origin, and SvelteKit server origin                               |
| `GOOGLE_*`, `GITHUB_*`                                          | Optional upstream OAuth provider configuration                                                   |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` | Email delivery configuration                                                                     |
| `WEBHOOK_WORKER_POLL_MS`, `WEBHOOK_WORKER_BATCH_SIZE`           | Webhook worker polling and batch limits                                                          |
| `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME`, `RUST_LOG`  | Tracing and logging                                                                              |

Compose supplies internal API database/Redis addresses separately from the host connection values. Preserve that distinction when running services partly on the host and partly in containers.

If `SMTP_HOST` is empty, email dispatch is skipped. Flows that require an emailed code or reset link need a working SMTP configuration to complete. Example OAuth credentials also need replacement before provider sign-in can work.

Keep `.env`, secret keys, signing-key encryption material, and production secret values outside Git. Changing the signing-key encryption material without a migration strategy can make existing encrypted keys unreadable.

## Application integration

A typical integration follows this sequence:

1. Create a developer account and workspace in the Control Plane.
2. Register an application and choose its development or production environment.
3. Configure allowed origins and redirect URIs, then obtain that environment's credentials.
4. Use hosted OIDC/SSO redirects or the direct Client API for user authentication.
5. Use the Backend API and signed webhooks to keep the consuming service synchronized.

Publishable keys use `pk_test_` / `pk_live_` prefixes and identify the environment through the `x-publishable-key` header. Secret keys use `sk_test_` / `sk_live_` prefixes and authenticate server-side requests through `Authorization: Bearer …`. Secret keys must remain on the consuming backend.

The OIDC provider exposes:

- `/.well-known/openid-configuration`
- `/.well-known/jwks.json`
- `/oauth2/authorize`, `/oauth2/token`, `/oauth2/userinfo`, and `/oauth2/logout`

For request bodies, required scopes, enrollment policies, and synchronization semantics, use the [API contract](docs/api-specification.md), [OIDC provider guide](docs/oidc-provider.md), and [service synchronization guide](docs/service-synchronization.md). Some specifications include planned routes; the [registered routes](apps/auth-api/src/routes/mod.rs) and phase status identify what is currently available.

## Development and validation

### Run services natively

Start PostgreSQL and Redis, configure host-accessible connection URLs in `.env`, then run the frontend and API in separate terminals:

```bash
npm ci
npm run dev:web
```

```bash
cd apps/auth-api
cargo run --locked --bin auth-api
```

For API hot reload, install `cargo-watch` and use `npm run dev:api` or `make dev-api`. Native development also needs an OTLP collector if you want exported traces.

### Frontend and shared contracts

```bash
npm ci
npm run check
npm run check:vendor
npm run lint
npm test
npm run build
```

`npm test` runs the private dependency security regressions, configured workspace tests, and the web Vitest suite. Shared TypeScript contracts are checked by `npm run check` and `npm run build`; the root package scripts define the complete workspace validation sequence.

### Rust checks

```bash
cargo fmt --manifest-path apps/auth-api/Cargo.toml --check
cargo clippy --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features
cargo build --manifest-path apps/auth-api/Cargo.toml --locked --release --bins
```

Database integration tests are explicitly ignored by the default Rust test command. Run them against a dedicated, disposable database, with Redis available without a password at `127.0.0.1:6399`:

```bash
TEST_DATABASE_URL=postgres://authuser:authpassword@127.0.0.1:5432/authdb_test \
  cargo test --manifest-path apps/auth-api/Cargo.toml --locked --all-targets --all-features \
  -- --ignored --test-threads=1
```

These tests apply migrations and mutate database state, including signing-key cleanup. Run one integration suite at a time against a given database. The test Redis endpoint differs from the password-protected Compose development instance.

### Security and infrastructure checks

```bash
npm audit --workspaces --audit-level=high
cargo audit --file apps/auth-api/Cargo.lock --deny unsound
helm lint infrastructure/helm
kustomize build infrastructure/kubernetes/dev
kustomize build infrastructure/kubernetes/prod
```

`cargo-audit`, Helm, and Kustomize are separate tools. The audit command above uses the API lockfile explicitly, so it can run from the repository root.

The repository carries a [private Picomatch security patch](vendor/picomatch/README.md) because the relevant upstream advisory has no patched published release. Its source archive, integrity check, patch, license, and regression tests are retained in `vendor/picomatch`. CI verifies reproducibility and requires review when the upstream release changes.

## Deployment and GitHub Actions

### Container images

Build the production images from the repository root:

```bash
docker build -f docker/Dockerfile.api -t secure-auth-api:local .
docker build -f docker/Dockerfile.web -t secure-auth-web:local .
```

The API image contains both `auth-api` and `webhook-worker`. Production images run as non-root users. The web runtime contains production dependencies and omits npm/Yarn after installation.

### Workflows

| Workflow                                                | Responsibilities                                                                                                                                                              |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [CI](.github/workflows/ci.yml)                          | Workflow and infrastructure validation, Rust checks and database tests, frontend checks and build, container smoke tests, and GHCR publication on successful pushes to `main` |
| [Security & Compliance](.github/workflows/security.yml) | Rust/npm audits, private patch regressions, full-history secret scanning, container vulnerability scans, SARIF reports, and SBOM artifacts                                    |
| [GitOps release](.github/workflows/release.yml)         | Verify successful CI/security runs for the same current-main commit, check published images, and update production image tags                                                 |

Published images use `ghcr.io/heyykrishnna/auth-api:sha-<full commit SHA>` and `ghcr.io/heyykrishnna/web:sha-<full commit SHA>`. Pull requests build and validate without publishing images.

Automatic promotion requires successful CI and security push runs for the same commit. Manual release dispatch supports an existing image tag as an explicit operator promotion or rollback. A successful GitOps workflow updates manifests; cluster rollout and application health must still be verified in Argo CD.

### Infrastructure options

- [Kustomize](infrastructure/kubernetes): development and production overlays, including the webhook worker.
- [Helm](infrastructure/helm/README.md): chart for the API, frontend, ingress, and autoscaling.
- [Terraform](infrastructure/terraform/README.md): AWS networking, EKS, PostgreSQL, and Redis modules.
- [Argo CD](infrastructure/kubernetes/argocd): project and application-of-applications configuration.

Choose the deployment model appropriate to your environment. Configure real domains, TLS, secrets, registry access, repository permissions, and infrastructure variables before applying templates. See the [deployment guide](docs/deployment.md) for operational details.

## Repository layout

```text
apps/auth-api/          Rust API, webhook worker, migrations, and tests
apps/web/               SvelteKit frontend and Vitest tests
packages/shared-types/  Shared TypeScript interfaces and API contracts
vendor/picomatch/       Reproducible private dependency security patch
docker/                 Production and development container definitions
infrastructure/         Terraform, Kubernetes, Helm, and Argo CD configuration
observability/          Collector configuration, metrics, and dashboards
docs/                   Architecture, API contracts, operations, and decisions
phases/                 Implementation milestones and acceptance criteria
.github/workflows/      CI, security, and GitOps automation
```

## Documentation

| Guide                                                      | Contents                                                        |
| ---------------------------------------------------------- | --------------------------------------------------------------- |
| [Domain glossary](GLOSSARY.md)                             | Identity, environment, session, and synchronization terminology |
| [Architecture](docs/architecture.md)                       | Service boundaries and platform design                          |
| [Architecture decisions](docs/adr)                         | Recorded domain and infrastructure decisions                    |
| [API specification](docs/api-specification.md)             | HTTP contracts, scopes, and event envelopes                     |
| [Authentication](docs/authentication.md)                   | Authentication and token lifecycle                              |
| [OIDC provider](docs/oidc-provider.md)                     | Hosted authentication and SSO integration                       |
| [Service synchronization](docs/service-synchronization.md) | Reconciliation, webhooks, and consumer guidance                 |
| [Migrations](docs/migrations.md)                           | Database migration workflow                                     |
| [Security](docs/security.md)                               | Security controls, scanning, and dependency maintenance         |
| [Threat model](docs/threat-model.md)                       | Trust boundaries and attack scenarios                           |
| [Deployment](docs/deployment.md)                           | Containers, Kubernetes, Helm, and GitOps                        |
| [Validation record](docs/ci-validation.md)                 | CI repair findings and verification evidence                    |
| [Roadmap](phases/README.md)                                | Milestones, implementation status, and planned work             |

## Contributing

Review the relevant phase, domain terminology, and architecture decisions before changing behavior. Keep changes focused, add regression coverage where appropriate, and run the checks relevant to the affected service. Commit dependency lockfiles with manifest changes and keep the private dependency patch reproducible.

For a pull request, describe the problem, resulting behavior, checks executed, and any remaining limitations. Use the [issue tracker](https://github.com/Heyykrishnna/auth-rust-svelte/issues) for reproducible bugs and feature proposals. Keep credentials and sensitive diagnostic data out of public reports.

## License

A project-level `LICENSE` file has not yet been added. Third-party code retains its own license notices, including the [vendored Picomatch MIT license](vendor/picomatch/LICENSE).
