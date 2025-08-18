#!/bin/bash
set -e

# Personal AI Assistant Backend Entrypoint Script
# This script handles initialization, health checks, and graceful shutdown

# Colors for logging
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Signal handlers for graceful shutdown
cleanup() {
    log_info "Received shutdown signal, cleaning up..."
    
    # Kill background processes
    if [ -n "$SERVER_PID" ]; then
        log_info "Stopping server (PID: $SERVER_PID)..."
        kill -TERM "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    
    log_success "Cleanup completed"
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT SIGQUIT

# Environment variable validation
validate_environment() {
    log_info "Validating environment variables..."
    
    # Required variables
    local required_vars=(
        "DATABASE_URL"
        "SERVER_HOST"
        "SERVER_PORT"
    )
    
    local missing_vars=()
    
    for var in "${required_vars[@]}"; do
        if [ -z "${!var}" ]; then
            missing_vars+=("$var")
        fi
    done
    
    if [ ${#missing_vars[@]} -ne 0 ]; then
        log_error "Missing required environment variables: ${missing_vars[*]}"
        exit 1
    fi
    
    log_success "Environment validation passed"
}

# Database connection check
check_database() {
    log_info "Checking database connection..."
    
    # Extract database components from DATABASE_URL
    if [[ $DATABASE_URL =~ postgresql://([^:]+):([^@]+)@([^:]+):([0-9]+)/(.+) ]]; then
        local db_user="${BASH_REMATCH[1]}"
        local db_host="${BASH_REMATCH[3]}"
        local db_port="${BASH_REMATCH[4]}"
        local db_name="${BASH_REMATCH[5]}"
        
        # Wait for database to be ready
        local max_attempts=30
        local attempt=1
        
        while [ $attempt -le $max_attempts ]; do
            if timeout 5 bash -c "</dev/tcp/$db_host/$db_port" 2>/dev/null; then
                log_success "Database is ready"
                return 0
            fi
            
            log_info "Database not ready, attempt $attempt/$max_attempts..."
            sleep 2
            ((attempt++))
        done
        
        log_error "Database connection failed after $max_attempts attempts"
        exit 1
    else
        log_warn "Unable to parse DATABASE_URL, skipping database check"
    fi
}

# Run database migrations
run_migrations() {
    log_info "Running database migrations..."
    
    # Check if migration binary exists
    if [ -f "./ai-assistant-migrate" ]; then
        ./ai-assistant-migrate up || {
            log_error "Database migration failed"
            exit 1
        }
        log_success "Database migrations completed"
    else
        log_warn "Migration binary not found, skipping migrations"
    fi
}

# Initialize application directories
initialize_directories() {
    log_info "Initializing application directories..."
    
    # Create directories if they don't exist
    mkdir -p data logs storage plugins tmp
    
    # Set proper permissions
    chmod 755 data logs storage plugins tmp
    
    log_success "Directories initialized"
}

# Health check function
health_check() {
    local max_attempts=30
    local attempt=1
    
    log_info "Waiting for server to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -sf "http://${SERVER_HOST}:${SERVER_PORT}/health" >/dev/null 2>&1; then
            log_success "Server health check passed"
            return 0
        fi
        
        log_info "Health check attempt $attempt/$max_attempts..."
        sleep 2
        ((attempt++))
    done
    
    log_error "Server health check failed after $max_attempts attempts"
    return 1
}

# Print system information
print_system_info() {
    log_info "System Information:"
    echo "  - Hostname: $(hostname)"
    echo "  - User: $(whoami)"
    echo "  - Working Directory: $(pwd)"
    echo "  - Rust Version: $(rustc --version 2>/dev/null || echo 'Not available')"
    echo "  - Server: ${SERVER_HOST}:${SERVER_PORT}"
    echo "  - Database: ${DATABASE_URL%%@*}@***"
    echo "  - Log Level: ${RUST_LOG:-info}"
}

# Main execution
main() {
    log_info "Starting Personal AI Assistant Backend..."
    
    # Print system information
    print_system_info
    
    # Validate environment
    validate_environment
    
    # Initialize directories
    initialize_directories
    
    # Check database connection
    if [ "${SKIP_DB_CHECK:-false}" != "true" ]; then
        check_database
    fi
    
    # Run migrations
    if [ "${SKIP_MIGRATIONS:-false}" != "true" ]; then
        run_migrations
    fi
    
    # Start the server
    log_info "Starting server..."
    exec "$@" &
    SERVER_PID=$!
    
    # Wait for server to be ready
    if [ "${SKIP_HEALTH_CHECK:-false}" != "true" ]; then
        if ! health_check; then
            log_error "Server failed to start properly"
            cleanup
            exit 1
        fi
    fi
    
    log_success "Personal AI Assistant Backend started successfully!"
    log_info "Server PID: $SERVER_PID"
    
    # Wait for the server process
    wait "$SERVER_PID"
}

# Handle special commands
case "${1:-}" in
    "migrate")
        log_info "Running database migrations only..."
        validate_environment
        run_migrations
        log_success "Migrations completed"
        exit 0
        ;;
    "health-check")
        log_info "Performing health check..."
        if health_check; then
            log_success "Health check passed"
            exit 0
        else
            log_error "Health check failed"
            exit 1
        fi
        ;;
    "version")
        log_info "Personal AI Assistant Backend"
        ./ai-assistant-server --version 2>/dev/null || echo "Version information not available"
        exit 0
        ;;
    "shell")
        log_info "Starting interactive shell..."
        exec /bin/bash
        ;;
    *)
        # Default: start the server
        main "$@"
        ;;
esac