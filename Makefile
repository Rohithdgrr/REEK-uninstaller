.PHONY: help build test clean fmt clippy install run-cli run-tui

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the project
	cargo build --workspace

build-release: ## Build the project in release mode
	cargo build --workspace --release

test: ## Run all tests
	cargo test --workspace --all-features

test-verbose: ## Run tests with verbose output
	cargo test --workspace --all-features -- --nocapture

clean: ## Clean build artifacts
	cargo clean

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clippy: ## Run clippy linter
	cargo clippy --workspace --all-targets --all-features -- -D warnings

install: ## Install the CLI binary
	cargo install --path crates/greek-cli

run-cli: ## Run the CLI application
	cargo run --bin reek

run-tui: ## Run the TUI application
	cargo run --bin reek-tui

check: fmt-check clippy test ## Run all checks (format, clippy, test)

ci: ## Run CI checks
	cargo test --workspace --all-features
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check
