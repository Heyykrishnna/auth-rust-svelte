.PHONY: dev dev-frontend dev-api build test lint migrate db-reset k8s-apply k8s-diff argocd-bootstrap clean help

BOLD   := \033[1m
RESET  := \033[0m
GREEN  := \033[32m
YELLOW := \033[33m
CYAN   := \033[36m

help:
	@echo ""
	@echo "$(BOLD)auth-rust-svelte$(RESET) — available targets:"
	@echo ""
	@grep -E '^## ' Makefile | sed 's/## /  $(CYAN)/' | sed 's/:/$(RESET)/'
	@echo ""

dev:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build

dev-d:
	docker compose -f docker-compose.yml -f docker-compose.override.yml up --build -d

dev-frontend:
	npm -w apps/frontend run dev

dev-api:
	cd apps/auth-api && cargo watch -x run

stop:
	docker compose down

clean:
	docker compose down -v --remove-orphans
	rm -rf apps/frontend/.svelte-kit apps/frontend/build
	cd apps/auth-api && cargo clean

build:
	docker build -t auth-api:local apps/auth-api
	docker build -t frontend:local apps/frontend

build-api:
	docker build -t auth-api:local apps/auth-api

build-frontend:
	docker build -t frontend:local apps/frontend

test: test-api test-frontend

test-api:
	cd apps/auth-api && cargo test --all-features

test-frontend:
	npm -w apps/frontend run test

lint: lint-api lint-frontend

lint-api:
	cd apps/auth-api && cargo clippy -- -D warnings
	cd apps/auth-api && cargo fmt --check

lint-frontend:
	npm -w apps/frontend run check
	npm -w apps/frontend run lint

migrate:
	cd apps/auth-api && sqlx migrate run --database-url "$$DATABASE_URL"

db-reset:
	cd apps/auth-api && sqlx database reset --database-url "$$DATABASE_URL"

db-shell:
	docker compose exec postgres psql -U authuser -d authdb

redis-shell:
	docker compose exec redis redis-cli

k8s-apply-dev:
	kubectl apply -k infra/k8s/overlays/dev

k8s-apply-prod:
	kubectl apply -k infra/k8s/overlays/prod

k8s-diff:
	kubectl diff -k infra/k8s/overlays/prod

k8s-observability:
	kubectl apply -k infra/k8s/observability

argocd-bootstrap:
	kubectl apply -f infra/argocd/app-of-apps.yaml

audit:
	cd apps/auth-api && cargo audit
	npm audit --workspaces

scan:
	trivy image auth-api:local
	trivy image frontend:local
