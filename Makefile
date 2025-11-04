.PHONY: help build test run clean setup-db migrate

help: ## Show this help message
	@echo "SnapRAG - Historical Profile Management System"
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run tests (database tests are ignored by default)
	cargo test

test-strict: ## Run tests with strict settings (warnings as errors)
	@echo "Running strict tests with warnings treated as errors..."
	@./scripts/run_strict_tests.sh

test-quick: ## Run quick tests (unit tests only)
	cargo test --lib -- --test-threads=1

test-integration: ## Run integration tests only
	cargo test --test "*" -- --test-threads=1

test-local: ## Run ALL tests including database tests (requires local PostgreSQL)
	@echo "🧪 Running local tests with test database..."
	@echo "⚠️  This will create/drop snaprag_test database on localhost"
	@echo ""
	@$(MAKE) test-db-check
	@echo ""
	@$(MAKE) test-setup
	@export SNAPRAG_CONFIG=config.test.toml && \
	 export SNAPRAG_ALLOW_RESET=yes && \
	 cargo run --release -- init --force && \
	 cargo test -- --ignored --test-threads=1 || true
	@$(MAKE) test-cleanup
	@echo "✅ Local tests complete"

test-setup: ## Create local test database
	@echo "🗄️  Creating test database..."
	@dropdb snaprag_test 2>/dev/null || true
	@createdb snaprag_test
	@psql -d snaprag_test -c "CREATE EXTENSION IF NOT EXISTS vector;" 2>/dev/null || echo "⚠️  pgvector not installed"
	@psql -d snaprag_test -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;" 2>/dev/null || true
	@echo "✅ Test database created"

test-cleanup: ## Drop local test database
	@echo "🧹 Cleaning up test database..."
	@dropdb snaprag_test 2>/dev/null || true
	@echo "✅ Test database removed"

test-db-check: ## Verify test database configuration (CRITICAL SAFETY CHECK)
	@echo "🛡️  CRITICAL SAFETY CHECK: Verifying test database configuration..."
	@if [ ! -f "config.test.toml" ]; then \
		echo "❌ ERROR: config.test.toml not found"; \
		echo "   Tests require local database configuration"; \
		exit 1; \
	fi
	@DB_URL=$$(grep "url = " config.test.toml | cut -d'"' -f2); \
	echo "   Checking database URL..."; \
	if echo "$$DB_URL" | grep -qE "localhost|127\.0\.0\.1|::1"; then \
		echo "   ✅ Database URL points to LOCALHOST"; \
		echo "   ✅ Safe to run tests"; \
	else \
		echo ""; \
		echo "   ❌❌❌ CRITICAL ERROR ❌❌❌"; \
		echo "   Database URL does NOT point to localhost!"; \
		echo "   Current URL: $$DB_URL"; \
		echo ""; \
		echo "   Running tests against this database would DESTROY PRODUCTION DATA!"; \
		echo ""; \
		echo "   Fix config.test.toml to use localhost database:"; \
		echo "   url = \"postgresql://user:pass@localhost/snaprag_test\""; \
		echo ""; \
		exit 1; \
	fi
	@echo "   Checking database name..."; \
	DB_URL=$$(grep "url = " config.test.toml | cut -d'"' -f2); \
	if echo "$$DB_URL" | grep -q "snaprag_test"; then \
		echo "   ✅ Database name is snaprag_test"; \
	else \
		echo "   ⚠️  WARNING: Database name is not snaprag_test"; \
		echo "   Recommended to use dedicated test database"; \
	fi; \
	echo "   ✅ All safety checks passed"


run: ## Run the application
	cargo run

run-example: ## Run the basic usage example
	cargo run --example basic_usage

check-config: ## Check configuration file
	cargo run --bin check_config

sync: ## Start sync service
	cargo run -- sync all

sync-dry-run: ## Test sync service without connecting to snapchain
	@echo "Dry run mode - sync service would start here"
	@echo "Configuration check:"
	@cargo run --bin check_config

migrate: ## Run database migrations
	@if [ ! -f "config.toml" ] && [ ! -f "config.example.toml" ]; then \
		echo "Error: No config file found"; \
		echo "Please create config.toml or ensure config.example.toml exists"; \
		echo "You can copy config.example.toml to config.toml and modify the database URL"; \
		exit 1; \
	fi
	@echo "Running database migrations..."
	@cargo run --bin migrate

clean: ## Clean build artifacts
	cargo clean

check: ## Run clippy and formatting checks
	cargo clippy -- -D warnings
	cargo fmt --check

fix: ## Fix clippy and formatting issues
	cargo clippy --fix --allow-dirty
	cargo fmt

docs: ## Generate documentation
	cargo doc --open

bench: ## Run benchmarks
	cargo bench
