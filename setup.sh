#!/bin/bash

# Personal AI Assistant - Environment Setup Script
# This script sets up the development environment for the Personal AI Assistant

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="Personal AI Assistant"
RUST_VERSION="1.75.0"
NODE_VERSION="18.0.0"

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

check_command() {
    if command -v "$1" >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

install_rust() {
    log_info "Installing Rust..."
    if check_command rustc; then
        local current_version=$(rustc --version | cut -d' ' -f2)
        log_info "Rust is already installed (version: $current_version)"
        if [[ "$current_version" < "$RUST_VERSION" ]]; then
            log_warning "Updating Rust to version $RUST_VERSION or later..."
            rustup update
        fi
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Install required Rust components
    rustup component add clippy rustfmt
    
    # Install additional tools
    if ! check_command cargo-watch; then
        cargo install cargo-watch
    fi
    
    if ! check_command sqlx-cli; then
        cargo install sqlx-cli --features postgres,sqlite
    fi
    
    log_success "Rust environment set up successfully"
}

install_node() {
    log_info "Setting up Node.js environment..."
    
    if check_command node; then
        local current_version=$(node --version | sed 's/v//')
        log_info "Node.js is already installed (version: $current_version)"
        if [[ "$current_version" < "$NODE_VERSION" ]]; then
            log_warning "Node.js version $NODE_VERSION or later is recommended"
        fi
    else
        log_info "Installing Node.js via package manager..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            if check_command brew; then
                brew install node
            else
                log_error "Homebrew not found. Please install Node.js manually from https://nodejs.org/"
                exit 1
            fi
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Install Node.js via NodeSource repository
            curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
            sudo apt-get install -y nodejs
        else
            log_error "Unsupported OS. Please install Node.js manually from https://nodejs.org/"
            exit 1
        fi
    fi
    
    # Install global packages
    npm install -g typescript ts-node @types/node
    
    log_success "Node.js environment set up successfully"
}

install_docker() {
    log_info "Checking Docker installation..."
    
    if check_command docker; then
        log_info "Docker is already installed"
        if ! docker info >/dev/null 2>&1; then
            log_warning "Docker daemon is not running. Please start Docker."
        fi
    else
        log_info "Installing Docker..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            log_info "Please install Docker Desktop from https://www.docker.com/products/docker-desktop/"
        elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
            # Install Docker on Linux
            curl -fsSL https://get.docker.com -o get-docker.sh
            sudo sh get-docker.sh
            sudo usermod -aG docker $USER
            rm get-docker.sh
            log_warning "Please log out and back in for Docker group changes to take effect"
        else
            log_error "Unsupported OS for automatic Docker installation"
            exit 1
        fi
    fi
    
    if check_command docker-compose; then
        log_info "Docker Compose is already installed"
    else
        log_info "Installing Docker Compose..."
        if [[ "$OSTYPE" == "linux-gnu"* ]]; then
            sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
            sudo chmod +x /usr/local/bin/docker-compose
        fi
    fi
    
    log_success "Docker environment set up successfully"
}

setup_directories() {
    log_info "Creating project directories..."
    
    mkdir -p data/{uploads,backups,logs}
    mkdir -p plugins/examples
    mkdir -p scripts/{dev,prod,backup}
    mkdir -p monitoring/{prometheus,grafana/{provisioning,dashboards}}
    mkdir -p nginx/{conf.d,ssl}
    mkdir -p tests/{integration,fixtures}
    
    log_success "Project directories created"
}

setup_environment() {
    log_info "Setting up environment configuration..."
    
    if [[ ! -f .env ]]; then
        if [[ -f .env.example ]]; then
            cp .env.example .env
            log_info "Created .env file from .env.example"
            log_warning "Please update .env file with your actual configuration values"
        else
            log_error ".env.example file not found"
            exit 1
        fi
    else
        log_info ".env file already exists"
    fi
    
    # Generate JWT secret if not set
    if grep -q "your-super-secret-jwt-key-change-this-in-production" .env; then
        local jwt_secret=$(openssl rand -base64 32)
        sed -i.bak "s/your-super-secret-jwt-key-change-this-in-production/$jwt_secret/" .env
        log_info "Generated JWT secret"
    fi
    
    # Generate session secret if not set
    if grep -q "your-session-secret-key-change-this-in-production" .env; then
        local session_secret=$(openssl rand -base64 32)
        sed -i.bak "s/your-session-secret-key-change-this-in-production/$session_secret/" .env
        log_info "Generated session secret"
    fi
    
    log_success "Environment configuration set up"
}

setup_database() {
    log_info "Setting up database..."
    
    if [[ -f .env ]]; then
        source .env
        if [[ "${DATABASE_URL:-}" == *"sqlite"* ]]; then
            log_info "Using SQLite database"
            mkdir -p data
            if check_command sqlx; then
                sqlx database create
                sqlx migrate run
                log_success "SQLite database initialized"
            else
                log_warning "sqlx-cli not found. Run 'cargo install sqlx-cli' to manage database migrations"
            fi
        else
            log_info "Using PostgreSQL database"
            log_info "Make sure PostgreSQL is running and accessible"
            log_info "Run 'make db-setup' to initialize the database"
        fi
    else
        log_warning "No .env file found. Skipping database setup."
    fi
}

install_frontend_deps() {
    log_info "Installing frontend dependencies..."
    
    if [[ -d frontend ]]; then
        cd frontend
        if [[ -f package.json ]]; then
            npm install
            log_success "Frontend dependencies installed"
        else
            log_warning "No package.json found in frontend directory"
        fi
        cd ..
    else
        log_warning "Frontend directory not found"
    fi
}

setup_git_hooks() {
    log_info "Setting up Git hooks..."
    
    if [[ -d .git ]]; then
        # Pre-commit hook
        cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
echo "Running pre-commit checks..."

# Check Rust formatting
if ! cargo fmt -- --check; then
    echo "Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
fi

# Run Rust linting
if ! cargo clippy --all-targets --all-features -- -D warnings; then
    echo "Clippy found issues. Please fix them."
    exit 1
fi

# Check frontend formatting (if exists)
if [[ -d frontend ]]; then
    cd frontend
    if [[ -f package.json ]]; then
        if ! npm run lint:check 2>/dev/null; then
            echo "Frontend linting failed."
            exit 1
        fi
    fi
    cd ..
fi

echo "Pre-commit checks passed!"
EOF
        chmod +x .git/hooks/pre-commit
        
        # Pre-push hook
        cat > .git/hooks/pre-push << 'EOF'
#!/bin/bash
echo "Running pre-push checks..."

# Run tests
if ! cargo test; then
    echo "Tests failed. Push aborted."
    exit 1
fi

echo "Pre-push checks passed!"
EOF
        chmod +x .git/hooks/pre-push
        
        log_success "Git hooks set up successfully"
    else
        log_warning "Not a git repository. Skipping Git hooks setup."
    fi
}

create_monitoring_config() {
    log_info "Creating monitoring configuration..."
    
    # Prometheus configuration
    cat > monitoring/prometheus.yml << 'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  # - "first_rules.yml"
  # - "second_rules.yml"

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'rusty-ai-api'
    static_configs:
      - targets: ['api:9090']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres:5432']
    scrape_interval: 30s
EOF

    # Grafana provisioning
    mkdir -p monitoring/grafana/provisioning/datasources
    cat > monitoring/grafana/provisioning/datasources/prometheus.yml << 'EOF'
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
EOF

    mkdir -p monitoring/grafana/provisioning/dashboards
    cat > monitoring/grafana/provisioning/dashboards/dashboard.yml << 'EOF'
apiVersion: 1

providers:
  - name: 'default'
    orgId: 1
    folder: ''
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    allowUiUpdates: true
    options:
      path: /var/lib/grafana/dashboards
EOF

    log_success "Monitoring configuration created"
}

print_next_steps() {
    log_success "Setup completed successfully!"
    echo
    log_info "Next steps:"
    echo "1. Update the .env file with your API keys and configuration"
    echo "2. Start the development environment:"
    echo "   make dev                 # Start with cargo watch"
    echo "   make docker-dev          # Start with Docker"
    echo "3. Run tests:"
    echo "   make test               # Run all tests"
    echo "4. Build for production:"
    echo "   make build              # Build optimized binary"
    echo "5. Access the application:"
    echo "   API: http://localhost:8080"
    echo "   Frontend: http://localhost:3000"
    echo "   Grafana: http://localhost:3001 (admin/admin)"
    echo
    log_info "For more commands, run: make help"
}

main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  $PROJECT_NAME Setup Script${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo
    
    log_info "Starting environment setup..."
    
    # Check OS
    log_info "Detected OS: $OSTYPE"
    
    # Install dependencies
    install_rust
    install_node
    install_docker
    
    # Setup project
    setup_directories
    setup_environment
    setup_database
    install_frontend_deps
    setup_git_hooks
    create_monitoring_config
    
    print_next_steps
}

# Run main function
main "$@"