#!/bin/bash

# Personal AI Assistant - Development Server Runner
# This script starts the development environment with proper setup and monitoring

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
API_PORT=8080
FRONTEND_PORT=3000
METRICS_PORT=9090

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

check_dependencies() {
    log_info "Checking dependencies..."
    
    # Check Rust
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    # Check Node.js (if frontend exists)
    if [[ -d frontend ]]; then
        if ! command -v node >/dev/null 2>&1; then
            log_error "Node.js not found. Please install Node.js: https://nodejs.org/"
            exit 1
        fi
        
        if [[ ! -d frontend/node_modules ]]; then
            log_info "Installing frontend dependencies..."
            cd frontend && npm install && cd ..
        fi
    fi
    
    # Check environment file
    if [[ ! -f .env ]]; then
        if [[ -f .env.example ]]; then
            log_info "Creating .env file from .env.example..."
            cp .env.example .env
            log_warning "Please update .env file with your configuration"
        else
            log_error ".env.example file not found"
            exit 1
        fi
    fi
    
    log_success "Dependencies check completed"
}

check_ports() {
    log_info "Checking port availability..."
    
    if check_port $API_PORT; then
        log_error "Port $API_PORT is already in use"
        log_info "Please stop the service using this port or change API_PORT in this script"
        exit 1
    fi
    
    if [[ -d frontend ]] && check_port $FRONTEND_PORT; then
        log_error "Port $FRONTEND_PORT is already in use"
        log_info "Please stop the service using this port or change FRONTEND_PORT in this script"
        exit 1
    fi
    
    log_success "Ports are available"
}

setup_database() {
    log_info "Setting up database..."
    
    # Load environment variables
    if [[ -f .env ]]; then
        source .env
    fi
    
    # Check if sqlx is available
    if command -v sqlx >/dev/null 2>&1; then
        # Create database if it doesn't exist
        if ! sqlx database create 2>/dev/null; then
            log_info "Database already exists or created successfully"
        fi
        
        # Run migrations
        if sqlx migrate run; then
            log_success "Database migrations completed"
        else
            log_warning "Database migrations failed or no migrations to run"
        fi
    else
        log_warning "sqlx-cli not found. Install with: cargo install sqlx-cli"
        log_info "Skipping database setup"
    fi
}

start_backend() {
    log_info "Starting backend server..."
    
    # Check if cargo-watch is available
    if command -v cargo-watch >/dev/null 2>&1; then
        log_info "Starting backend with auto-reload..."
        cargo watch -x "run --bin rusty-ai-api" &
        BACKEND_PID=$!
    else
        log_info "cargo-watch not found. Starting backend without auto-reload..."
        log_warning "Install cargo-watch for auto-reload: cargo install cargo-watch"
        cargo run --bin rusty-ai-api &
        BACKEND_PID=$!
    fi
    
    # Wait for backend to start
    log_info "Waiting for backend to start..."
    for i in {1..30}; do
        if check_port $API_PORT; then
            log_success "Backend started on port $API_PORT"
            return 0
        fi
        sleep 1
    done
    
    log_error "Backend failed to start within 30 seconds"
    return 1
}

start_frontend() {
    if [[ -d frontend ]]; then
        log_info "Starting frontend server..."
        cd frontend
        npm run dev &
        FRONTEND_PID=$!
        cd ..
        
        # Wait for frontend to start
        log_info "Waiting for frontend to start..."
        for i in {1..30}; do
            if check_port $FRONTEND_PORT; then
                log_success "Frontend started on port $FRONTEND_PORT"
                return 0
            fi
            sleep 1
        done
        
        log_error "Frontend failed to start within 30 seconds"
        return 1
    else
        log_info "No frontend directory found. Skipping frontend startup."
        return 0
    fi
}

monitor_health() {
    log_info "Setting up health monitoring..."
    
    while true; do
        sleep 30
        
        # Check backend health
        if ! check_port $API_PORT; then
            log_error "Backend is not responding on port $API_PORT"
        else
            # Try to hit health endpoint
            if curl -sf "http://localhost:$API_PORT/health" >/dev/null 2>&1; then
                log_success "Backend health check passed"
            else
                log_warning "Backend is running but health check failed"
            fi
        fi
        
        # Check frontend health (if running)
        if [[ -n "${FRONTEND_PID:-}" ]] && [[ -d frontend ]]; then
            if ! check_port $FRONTEND_PORT; then
                log_error "Frontend is not responding on port $FRONTEND_PORT"
            fi
        fi
    done
}

cleanup() {
    log_info "Shutting down development servers..."
    
    if [[ -n "${BACKEND_PID:-}" ]]; then
        log_info "Stopping backend server (PID: $BACKEND_PID)..."
        kill $BACKEND_PID 2>/dev/null || true
    fi
    
    if [[ -n "${FRONTEND_PID:-}" ]]; then
        log_info "Stopping frontend server (PID: $FRONTEND_PID)..."
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    
    # Kill any remaining cargo-watch processes
    pkill -f "cargo watch" 2>/dev/null || true
    
    log_success "Development servers stopped"
    exit 0
}

show_info() {
    echo
    log_success "Development environment is running!"
    echo
    echo "=========================================="
    echo " 🚀 Personal AI Assistant - Development"
    echo "=========================================="
    echo
    echo "📊 Services:"
    echo "  • Backend API: http://localhost:$API_PORT"
    echo "  • Health Check: http://localhost:$API_PORT/health"
    echo "  • Metrics: http://localhost:$METRICS_PORT/metrics"
    
    if [[ -d frontend ]]; then
        echo "  • Frontend: http://localhost:$FRONTEND_PORT"
    fi
    
    echo
    echo "📁 Useful endpoints:"
    echo "  • API Documentation: http://localhost:$API_PORT/docs"
    echo "  • Database Admin: http://localhost:5050 (if using pgAdmin)"
    echo "  • Redis Admin: http://localhost:8081 (if using Redis Commander)"
    
    echo
    echo "🔧 Development commands:"
    echo "  • View logs: make logs"
    echo "  • Run tests: make test"
    echo "  • Database console: make db-console"
    echo "  • Check status: make status"
    
    echo
    echo "Press Ctrl+C to stop all services"
    echo "=========================================="
    echo
}

main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Personal AI Assistant Development${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo
    
    # Set up signal handlers
    trap cleanup INT TERM
    
    # Pre-flight checks
    check_dependencies
    check_ports
    setup_database
    
    # Start services
    if start_backend; then
        sleep 2  # Give backend a moment to stabilize
        start_frontend
        
        show_info
        
        # Start health monitoring in background
        monitor_health &
        MONITOR_PID=$!
        
        # Wait for interrupt
        wait
    else
        log_error "Failed to start backend. Exiting."
        cleanup
        exit 1
    fi
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "Personal AI Assistant Development Server"
        echo "Usage: $0 [OPTIONS]"
        echo
        echo "Options:"
        echo "  --help, -h    Show this help message"
        echo "  --no-frontend Skip frontend startup"
        echo "  --port PORT   Override API port (default: $API_PORT)"
        echo
        echo "Environment variables:"
        echo "  API_PORT      Backend API port (default: $API_PORT)"
        echo "  FRONTEND_PORT Frontend port (default: $FRONTEND_PORT)"
        echo "  METRICS_PORT  Metrics port (default: $METRICS_PORT)"
        exit 0
        ;;
    --no-frontend)
        rm -rf frontend
        ;;
    --port)
        if [[ -n "${2:-}" ]]; then
            API_PORT="$2"
            shift
        else
            log_error "--port requires a port number"
            exit 1
        fi
        ;;
esac

# Run main function
main "$@"