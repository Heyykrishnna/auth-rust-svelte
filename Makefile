.PHONY: dev dev-frontend dev-api build test lint migrate db-reset k8s-apply k8s-diff argocd-bootstrap clean help

# ─── Colors ──────────────────────────────────────────────────────────────────
BOLD   := \033[1m
RESET  := \033[0m
GREEN  := \033[32m
YELLOW := \033[33m
CYAN   := \033[36m

## help: Show this help message
help:
	@echo ""
	@echo "$(BOLD)auth-rust-svelte$(RESET) — available targets:"
	@echo ""
	@grep -E '^## ' Makefile | sed 's/## /  $(CYAN)/' | sed 's/:/$(RESET)/'
	@echo ""

# ─── Local Development ───────────────────────────────────────────────────────

## dev: Start full local stack (postgres + redis + api + frontend)
dev:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build

## dev-d: Start full local stack in detached mode
dev-d:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build -d

## dev-frontend: Run SvelteKit dev server only (requires API running)
dev-frontend:
	npm -w apps/frontend run dev

## dev-api: Run Rust API with cargo-watch hot-reload (requires postgres + redis)
dev-api:
	cd apps/auth-api && cargo watch -x run

## stop: Stop all local services
stop:
	docker compose down

## clean: Stop services and remove volumes
clean:
	docker compose down -v --remove-orphans
	rm -rf apps/frontend/.svelte-kit apps/frontend/build
	cd apps/auth-api && cargo clean

# ─── Build ────────────────────────────────────────────────────────────────────

## build: Build all Docker images
build:
	docker build -t auth-api:local apps/auth-api
	docker build -t frontend:local apps/frontend

## build-api: Build Rust Docker image only
build-api:
	docker build -t auth-api:local apps/auth-api

## build-frontend: Build frontend Docker image only
build-frontend:
	docker build -t frontend:local apps/frontend

# ─── Testing ──────────────────────────────────────────────────────────────────

## test: Run all tests (Rust + Svelte)
test: test-api test-frontend

## test-api: Run Rust tests
test-api:
	cd apps/auth-api && cargo test --all-features

## test-frontend: Run Svelte/Vitest tests
test-frontend:
	npm -w apps/frontend run test

# ─── Linting ─────────────────────────────────────────────────────────────────

## lint: Run all linters
lint: lint-api lint-frontend

## lint-api: Run Clippy + fmt check
lint-api:
	cd apps/auth-api && cargo clippy -- -D warnings
	cd apps/auth-api && cargo fmt --check

## lint-frontend: Run svelte-check + eslint
lint-frontend:
	npm -w apps/frontend run check
	npm -w apps/frontend run lint

# ─── Database ─────────────────────────────────────────────────────────────────

## migrate: Run SQLx migrations against local database
migrate:
	cd apps/auth-api && sqlx migrate run --database-url "$$DATABASE_URL"

## db-reset: Drop and recreate local database, then migrate
db-reset:
	cd apps/auth-api && sqlx database reset --database-url "$$DATABASE_URL"

## db-shell: Open psql shell to local database
db-shell:
	docker compose exec postgres psql -U authuser -d authdb

## redis-shell: Open redis-cli to local Redis
redis-shell:
	docker compose exec redis redis-cli

# ─── Kubernetes ───────────────────────────────────────────────────────────────

## k8s-apply-dev: Apply dev overlay to cluster
k8s-apply-dev:
	kubectl apply -k infra/k8s/overlays/dev

## k8s-apply-prod: Apply prod overlay to cluster
k8s-apply-prod:
	kubectl apply -k infra/k8s/overlays/prod

## k8s-diff: Show diff between local manifests and cluster state
k8s-diff:
	kubectl diff -k infra/k8s/overlays/prod

## k8s-observability: Apply observability stack
k8s-observability:
	kubectl apply -k infra/k8s/observability

## argocd-bootstrap: Bootstrap Argo CD app-of-apps
argocd-bootstrap:
	kubectl apply -f infra/argocd/app-of-apps.yaml

# ─── Security ─────────────────────────────────────────────────────────────────

## audit: Run cargo audit + npm audit
audit:
	cd apps/auth-api && cargo audit
	npm audit --workspaces

## scan: Run trivy image scan
scan:
	trivy image auth-api:local
	trivy image frontend:local
