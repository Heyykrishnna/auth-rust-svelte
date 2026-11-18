# Docker Architecture & Packaging

This directory contains containerization definitions, guidelines, and production Dockerfiles for the `secure-auth-platform`.

## Container Images

### 1. `auth-api` (`docker/Dockerfile.api` & `apps/auth-api/Dockerfile`)

- **Base image**: `rust:1.96.0-alpine` (builder) → `alpine:3.24` (runtime)
- **Features**:
  - Static musl binaries in a minimal Alpine runtime with CA certificates and wget for the Compose health check
  - Multi-stage build with BuildKit Cargo cache mounts
  - Runs as non-root user (`65534:65534`)
  - Both `auth-api` and `webhook-worker` binaries plus database migrations are included
- **Port**: `8080`

### 2. `web` (`docker/Dockerfile.web` & `apps/web/Dockerfile`)

- **Base image**: `node:24-alpine`
- **Features**:
  - Multi-stage build caching `npm ci`
  - Workspace integration with `@secure-auth/shared-types`
  - SvelteKit Node adapter output (`build/`)
  - Dedicated non-root user (`appuser:1001`)
- **Port**: `3000`

## Build Commands

From the repository root:

```bash
# Build auth-api
docker build -f docker/Dockerfile.api -t secure-auth-api:latest .

# Build web frontend
docker build -f docker/Dockerfile.web -t secure-auth-web:latest .
```

Alternatively using the root `docker-compose.yml`:

```bash
docker compose build
```
