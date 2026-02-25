# AgentMesh — Build Orchestration
# Usage: make <target>

.PHONY: help setup dev test lint build clean fmt check docker-up docker-down

CARGO := cargo
PNPM := pnpm
RUST_LOG ?= info

# ── Help ──────────────────────────────────────────────────────────────────────
help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# ── Setup ─────────────────────────────────────────────────────────────────────
setup: ## First-time project setup
	@echo "==> Installing Rust toolchain..."
	rustup update stable
	rustup component add clippy rustfmt
	@echo "==> Installing dashboard dependencies..."
	cd dashboard && $(PNPM) install
	@echo "==> Setup complete. Run 'make dev' to start development servers."

# ── Development ───────────────────────────────────────────────────────────────
dev: ## Start proxy + dashboard in watch mode
	@echo "==> Starting development servers..."
	$(CARGO) watch -x "run --bin agentmesh" &
	cd dashboard && $(PNPM) dev

dev-proxy: ## Start only the proxy in watch mode
	RUST_LOG=$(RUST_LOG) $(CARGO) watch -x "run --bin agentmesh"

dev-dashboard: ## Start only the dashboard
	cd dashboard && $(PNPM) dev

# ── Build ─────────────────────────────────────────────────────────────────────
build: ## Build all crates in release mode
	$(CARGO) build --release --workspace

build-proxy: ## Build only the proxy binary
	$(CARGO) build --release -p agentmesh-proxy

build-dashboard: ## Build dashboard for production
	cd dashboard && $(PNPM) build

# ── Testing ───────────────────────────────────────────────────────────────────
test: ## Run all Rust tests
	$(CARGO) test --workspace

test-proxy: ## Run proxy tests only
	$(CARGO) test -p agentmesh-proxy

test-integration: ## Run integration tests (requires running postgres)
	$(CARGO) test --test '*' -- --test-threads=1

test-dashboard: ## Run dashboard tests
	cd dashboard && $(PNPM) test

# ── Lint & Format ─────────────────────────────────────────────────────────────
lint: ## Run clippy on all crates
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

fmt: ## Format all Rust code
	$(CARGO) fmt --all

fmt-check: ## Check formatting without modifying files
	$(CARGO) fmt --all -- --check

check: ## Run cargo check (fast compile check)
	$(CARGO) check --workspace

# ── Database ──────────────────────────────────────────────────────────────────
migrate: ## Run database migrations
	@echo "==> Running migrations..."
	@for f in migrations/*.sql; do \
		echo "  Applying $$f..."; \
		psql "$$DATABASE_URL" -f "$$f"; \
	done

db-reset: ## Drop and recreate the database
	@echo "==> Resetting database..."
	psql "$$DATABASE_URL" -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
	$(MAKE) migrate

# ── Docker ────────────────────────────────────────────────────────────────────
docker-up: ## Start all Docker services
	docker compose -f docker/docker-compose.yml up -d

docker-down: ## Stop all Docker services
	docker compose -f docker/docker-compose.yml down

docker-build: ## Build Docker images
	docker compose -f docker/docker-compose.yml build

docker-logs: ## Tail Docker logs
	docker compose -f docker/docker-compose.yml logs -f

# ── Clean ─────────────────────────────────────────────────────────────────────
clean: ## Remove build artifacts
	$(CARGO) clean
	cd dashboard && rm -rf dist node_modules

# ── CI ────────────────────────────────────────────────────────────────────────
ci: fmt-check lint test build ## Run full CI pipeline (format check, lint, test, build)
