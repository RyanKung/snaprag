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


run: ## Run the application
	cargo run --bin snaprag

run-example: ## Run the basic usage example
	cargo run --example basic_usage

check-config: ## Check configuration file
	cargo run --bin check_config

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
