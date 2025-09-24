.PHONY: help build test run clean setup-db migrate

help: ## Show this help message
	@echo "SnapRAG - Historical Profile Management System"
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the project
	cargo build

build-release: ## Build the project in release mode
	cargo build --release

test: ## Run tests
	cargo test

test-strict: ## Run tests with strict settings (warnings as errors)
	@echo "Running strict tests with warnings treated as errors..."
	@./scripts/run_strict_tests.sh

test-quick: ## Run quick tests (unit tests only)
	cargo test --lib -- --test-threads=1

test-integration: ## Run integration tests only
	cargo test --test "*" -- --test-threads=1


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
