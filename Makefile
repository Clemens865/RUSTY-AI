# Personal AI Assistant - Makefile
# This Makefile provides common development tasks and shortcuts

.PHONY: help setup build test lint format clean dev dev-watch docker-dev docker-build docker-stop db-setup db-migrate db-reset backup deploy docs release install check

# Default target
help: ## Show this help message
	@echo "Personal AI Assistant - Available Commands"
	@echo "========================================"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

# =================================
# Environment Setup
# =================================

setup: ## Run the setup script to initialize the development environment
	@echo "Running environment setup..."
	./setup.sh

install: ## Install all dependencies (Rust and Node.js)
	@echo "Installing Rust dependencies..."
	cargo fetch
	@echo "Installing frontend dependencies..."
	cd frontend && npm install
	@echo "Installing development tools..."
	cargo install cargo-watch sqlx-cli --features postgres,sqlite

# =================================
# Development
# =================================

dev: ## Start development server with cargo watch
	@echo "Starting development server with auto-reload..."
	cargo watch -x "run --bin rusty-ai-api"

dev-watch: ## Start development with watch mode for backend and frontend
	@echo "Starting full development environment with auto-reload..."
	@make -j2 dev-backend dev-frontend

dev-backend: ## Start backend development server
	cargo watch -x "run --bin rusty-ai-api"

dev-frontend: ## Start frontend development server
	cd frontend && npm run dev

# =================================
# Building
# =================================

build: ## Build the project in release mode
	@echo "Building project in release mode..."
	cargo build --release

build-debug: ## Build the project in debug mode
	@echo "Building project in debug mode..."
	cargo build

build-frontend: ## Build the frontend
	@echo "Building frontend..."
	cd frontend && npm run build

build-all: build build-frontend ## Build both backend and frontend

# =================================
# Testing
# =================================

test: ## Run all tests
	@echo "Running Rust tests..."
	cargo test
	@echo "Running frontend tests..."
	cd frontend && npm test

test-integration: ## Run integration tests
	@echo "Running integration tests..."
	cargo test --test '*' --features integration-tests

test-unit: ## Run unit tests only
	@echo "Running unit tests..."
	cargo test --lib

test-watch: ## Run tests in watch mode
	@echo "Running tests in watch mode..."
	cargo watch -x test

test-coverage: ## Generate test coverage report
	@echo "Generating test coverage..."
	cargo tarpaulin --out Html --output-dir coverage/

# =================================
# Code Quality
# =================================

lint: ## Run linting for Rust and frontend
	@echo "Running Rust linting..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "Running frontend linting..."
	cd frontend && npm run lint

lint-fix: ## Fix linting issues automatically
	@echo "Fixing Rust linting issues..."
	cargo clippy --all-targets --all-features --fix --allow-dirty
	@echo "Fixing frontend linting issues..."
	cd frontend && npm run lint:fix

format: ## Format code using rustfmt and prettier
	@echo "Formatting Rust code..."
	cargo fmt
	@echo "Formatting frontend code..."
	cd frontend && npm run format

format-check: ## Check code formatting
	@echo "Checking Rust code formatting..."
	cargo fmt -- --check
	@echo "Checking frontend code formatting..."
	cd frontend && npm run format:check

check: ## Run all checks (format, lint, test)
	@echo "Running all checks..."
	@make format-check
	@make lint
	@make test

# =================================
# Database Management
# =================================

db-setup: ## Set up the database
	@echo "Setting up database..."
	sqlx database create
	@make db-migrate

db-migrate: ## Run database migrations
	@echo "Running database migrations..."
	sqlx migrate run

db-migrate-revert: ## Revert last database migration
	@echo "Reverting last migration..."
	sqlx migrate revert

db-reset: ## Reset database (drop and recreate)
	@echo "Resetting database..."
	sqlx database drop -y
	sqlx database create
	sqlx migrate run

db-seed: ## Seed database with sample data
	@echo "Seeding database..."
	cargo run --bin seed-db

# =================================
# Docker Development
# =================================

docker-dev: ## Start development environment with Docker
	@echo "Starting development environment with Docker..."
	docker-compose up -d

docker-dev-build: ## Build and start development environment with Docker
	@echo "Building and starting development environment..."
	docker-compose up -d --build

docker-dev-logs: ## Show logs from Docker development environment
	docker-compose logs -f

docker-dev-stop: ## Stop Docker development environment
	@echo "Stopping development environment..."
	docker-compose down

docker-dev-clean: ## Stop and clean Docker development environment
	@echo "Cleaning development environment..."
	docker-compose down -v --remove-orphans

# =================================
# Docker Production
# =================================

docker-build: ## Build production Docker images
	@echo "Building production Docker images..."
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml build

docker-prod: ## Start production environment with Docker
	@echo "Starting production environment..."
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d

docker-prod-stop: ## Stop production environment
	@echo "Stopping production environment..."
	docker-compose -f docker-compose.yml -f docker-compose.prod.yml down

# =================================
# Utilities
# =================================

clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	cargo clean
	cd frontend && npm run clean
	rm -rf target/
	rm -rf frontend/dist/
	rm -rf frontend/node_modules/.cache/

clean-all: clean ## Clean everything including node_modules
	@echo "Cleaning everything..."
	rm -rf frontend/node_modules/

logs: ## Show application logs
	@echo "Showing application logs..."
	tail -f logs/rusty_ai.log

backup: ## Create database backup
	@echo "Creating database backup..."
	./scripts/backup.sh

# =================================
# Documentation
# =================================

docs: ## Generate documentation
	@echo "Generating Rust documentation..."
	cargo doc --no-deps --open
	@echo "Generating frontend documentation..."
	cd frontend && npm run docs

docs-serve: ## Serve documentation locally
	@echo "Serving documentation..."
	cargo doc --no-deps
	python3 -m http.server 8000 -d target/doc

# =================================
# Deployment
# =================================

deploy-staging: ## Deploy to staging environment
	@echo "Deploying to staging..."
	./scripts/deploy-staging.sh

deploy-prod: ## Deploy to production environment
	@echo "Deploying to production..."
	./scripts/deploy-prod.sh

release: ## Create a new release
	@echo "Creating new release..."
	./scripts/release.sh

# =================================
# Security
# =================================

security-audit: ## Run security audit
	@echo "Running security audit..."
	cargo audit
	cd frontend && npm audit

security-fix: ## Fix security vulnerabilities
	@echo "Fixing security vulnerabilities..."
	cargo audit fix
	cd frontend && npm audit fix

# =================================
# Performance
# =================================

bench: ## Run benchmarks
	@echo "Running benchmarks..."
	cargo bench

profile: ## Profile the application
	@echo "Profiling application..."
	cargo build --release
	valgrind --tool=callgrind target/release/rusty-ai-api

# =================================
# Monitoring
# =================================

monitor: ## Start monitoring stack
	@echo "Starting monitoring stack..."
	docker-compose --profile monitoring up -d

monitor-stop: ## Stop monitoring stack
	@echo "Stopping monitoring stack..."
	docker-compose --profile monitoring down

# =================================
# Plugin Development
# =================================

plugin-new: ## Create a new plugin template
	@echo "Creating new plugin template..."
	./scripts/create-plugin.sh

plugin-build: ## Build all plugins
	@echo "Building all plugins..."
	./scripts/build-plugins.sh

plugin-test: ## Test all plugins
	@echo "Testing all plugins..."
	./scripts/test-plugins.sh

# =================================
# Maintenance
# =================================

update: ## Update all dependencies
	@echo "Updating Rust dependencies..."
	cargo update
	@echo "Updating frontend dependencies..."
	cd frontend && npm update

update-rust: ## Update Rust toolchain
	@echo "Updating Rust toolchain..."
	rustup update

health-check: ## Check system health
	@echo "Running health check..."
	curl -f http://localhost:8080/health || echo "API not running"
	docker-compose ps

# =================================
# CI/CD Simulation
# =================================

ci: ## Simulate CI pipeline locally
	@echo "Running CI pipeline..."
	@make check
	@make test
	@make build
	@echo "CI pipeline completed successfully!"

pre-commit: ## Run pre-commit checks
	@echo "Running pre-commit checks..."
	@make format-check
	@make lint
	@make test-unit

pre-push: ## Run pre-push checks
	@echo "Running pre-push checks..."
	@make test
	@make security-audit

# =================================
# Development Tools
# =================================

tools: ## Install development tools
	@echo "Installing development tools..."
	cargo install cargo-watch cargo-audit cargo-tarpaulin
	rustup component add clippy rustfmt

db-console: ## Open database console
	@echo "Opening database console..."
	sqlx database connect

redis-cli: ## Open Redis CLI
	@echo "Opening Redis CLI..."
	docker-compose exec redis redis-cli

# =================================
# Information
# =================================

info: ## Show project information
	@echo "Project Information"
	@echo "==================="
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "Node version: $$(node --version)"
	@echo "NPM version: $$(npm --version)"
	@echo "Docker version: $$(docker --version)"
	@echo "Docker Compose version: $$(docker-compose --version)"
	@echo ""
	@echo "Project structure:"
	@find . -type f -name "*.rs" | wc -l | xargs echo "Rust files:"
	@find frontend -name "*.tsx" -o -name "*.ts" 2>/dev/null | wc -l | xargs echo "TypeScript files:"
	@echo "Lines of code:"
	@find . -name "*.rs" -exec wc -l {} + | tail -1
	@find frontend -name "*.tsx" -o -name "*.ts" 2>/dev/null -exec wc -l {} + | tail -1

status: ## Show development environment status
	@echo "Development Environment Status"
	@echo "=============================="
	@echo "Backend (Rust):"
	@cargo --version
	@echo ""
	@echo "Frontend (Node.js):"
	@cd frontend && node --version && npm --version
	@echo ""
	@echo "Docker Services:"
	@docker-compose ps 2>/dev/null || echo "Docker Compose not running"
	@echo ""
	@echo "Database:"
	@sqlx database list 2>/dev/null || echo "Database not accessible"