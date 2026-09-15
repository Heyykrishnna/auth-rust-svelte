.PHONY: dev dev-web dev-api build test lint migrate db-reset k8s-apply-dev k8s-apply-prod k8s-diff argocd-bootstrap clean help audit scan

BOLD   := \033[1m
RESET  := \033[0m
GREEN  := \033[32m
YELLOW := \033[33m
CYAN   := \033[36m

help:
	@echo ""
	@echo "$(BOLD)secure-auth-platform$(RESET) — available targets:"
	@echo ""
	@grep -E '^## ' Makefile | sed 's/## /  $(CYAN)/' | sed 's/:/$(RESET)/'
	@echo ""

## dev: Start all services via Docker Compose with hot-reload
dev:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build

## dev-d: Start all services in detached mode
dev-d:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build -d

## dev-web: Start SvelteKit frontend locally
dev-web:
	npm -w @secure-auth/web run dev

## dev-api: Start Rust auth API locally with cargo watch
dev-api:
	cd apps/auth-api && cargo watch -x run

## stop: Stop Docker Compose services
stop:
	docker compose down

## clean: Stop containers and clean build artifacts
clean:
	docker compose down -v --remove-orphans
	rm -rf apps/web/.svelte-kit apps/web/build
	cd apps/auth-api && cargo clean

## build: Build Docker images for auth-api and web
build: build-api build-web

## build-api: Build auth-api Docker image
build-api:
	docker build -t secure-auth-api:local -f docker/Dockerfile.api .

## build-web: Build web Docker image
build-web:
	docker build -t secure-auth-web:local -f docker/Dockerfile.web .

## test: Run unit & integration tests across Rust and Web
test: test-api test-web

## test-api: Run Rust tests
test-api:
	cd apps/auth-api && cargo test --all-features

## test-web: Run Vitest frontend tests
test-web:
	npm -w @secure-auth/web run test

## lint: Run formatters and linters across codebase
lint: lint-api lint-web

## lint-api: Run cargo clippy and fmt check
lint-api:
	cd apps/auth-api && cargo clippy -- -D warnings
	cd apps/auth-api && cargo fmt --check

## lint-web: Run svelte-check and eslint
lint-web:
	npm run check
	npm run lint

## migrate: Run database migrations
migrate:
	cd apps/auth-api && sqlx migrate run --database-url "$$DATABASE_URL"

## db-reset: Reset local database
db-reset:
	cd apps/auth-api && sqlx database reset --database-url "$$DATABASE_URL"

## db-shell: Open PostgreSQL interactive shell
db-shell:
	docker compose exec postgres psql -U authuser -d authdb

## redis-shell: Open Redis interactive shell
redis-shell:
	docker compose exec redis redis-cli

## k8s-apply-dev: Apply Kubernetes development overlay
k8s-apply-dev:
	kubectl apply -k infrastructure/kubernetes/dev

## k8s-apply-prod: Apply Kubernetes production overlay
k8s-apply-prod:
	kubectl apply -k infrastructure/kubernetes/prod

## k8s-diff: Diff production manifests against cluster
k8s-diff:
	kubectl diff -k infrastructure/kubernetes/prod

## argocd-bootstrap: Bootstrap Argo CD application of apps
argocd-bootstrap:
	kubectl apply -f infrastructure/kubernetes/argocd/app-of-apps.yaml

## audit: Run dependency security audits
audit:
	cd apps/auth-api && cargo audit
	npm audit --workspaces

## scan: Scan container images with Trivy
scan:
	trivy image secure-auth-api:local
	trivy image secure-auth-web:local
