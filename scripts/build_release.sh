#!/bin/bash

# Personal AI Assistant - Production Build Script
# This script builds optimized production binaries and assets

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BUILD_DIR="target/release"
DIST_DIR="dist"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

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

check_dependencies() {
    log_info "Checking build dependencies..."
    
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
            cd frontend && npm ci --production && cd ..
        fi
    fi
    
    log_success "Dependencies check completed"
}

clean_build() {
    log_info "Cleaning previous builds..."
    
    # Clean Rust build artifacts
    cargo clean
    
    # Clean frontend build artifacts
    if [[ -d frontend ]]; then
        cd frontend
        rm -rf dist/ build/ .next/ out/
        cd ..
    fi
    
    # Clean distribution directory
    rm -rf "$DIST_DIR"
    
    log_success "Previous builds cleaned"
}

set_build_info() {
    log_info "Setting build information..."
    
    # Create build info for Rust
    cat > src/build_info.rs << EOF
// Auto-generated build information
pub const VERSION: &str = "$VERSION";
pub const BUILD_DATE: &str = "$BUILD_DATE";
pub const GIT_COMMIT: &str = "$GIT_COMMIT";
pub const BUILD_MODE: &str = "release";
EOF

    # Create build info for frontend (if exists)
    if [[ -d frontend ]]; then
        cat > frontend/src/build-info.ts << EOF
// Auto-generated build information
export const buildInfo = {
  version: "$VERSION",
  buildDate: "$BUILD_DATE",
  gitCommit: "$GIT_COMMIT",
  buildMode: "production"
};
EOF
    fi
    
    log_success "Build information set"
}

build_backend() {
    log_info "Building backend (Rust) in release mode..."
    
    # Set optimization flags
    export CARGO_PROFILE_RELEASE_LTO=true
    export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
    export CARGO_PROFILE_RELEASE_STRIP=true
    
    # Build with optimizations
    if cargo build --release; then
        log_success "Backend build completed"
        
        # Display binary size
        if [[ -f "$BUILD_DIR/rusty-ai-api" ]]; then
            local size=$(du -h "$BUILD_DIR/rusty-ai-api" | cut -f1)
            log_info "Binary size: $size"
        fi
    else
        log_error "Backend build failed"
        exit 1
    fi
}

build_frontend() {
    if [[ -d frontend ]]; then
        log_info "Building frontend..."
        
        cd frontend
        
        # Set production environment
        export NODE_ENV=production
        
        # Build frontend
        if npm run build; then
            log_success "Frontend build completed"
            
            # Display bundle size (if available)
            if [[ -d dist ]] || [[ -d build ]]; then
                local dist_dir="dist"
                [[ -d build ]] && dist_dir="build"
                local size=$(du -sh "$dist_dir" | cut -f1)
                log_info "Frontend bundle size: $size"
            fi
        else
            log_error "Frontend build failed"
            cd ..
            exit 1
        fi
        
        cd ..
    else
        log_info "No frontend directory found. Skipping frontend build."
    fi
}

run_tests() {
    log_info "Running tests before packaging..."
    
    # Run Rust tests
    if ! cargo test --release; then
        log_error "Rust tests failed"
        return 1
    fi
    
    # Run frontend tests (if available)
    if [[ -d frontend ]]; then
        cd frontend
        if npm run test:ci 2>/dev/null || npm run test 2>/dev/null; then
            log_success "Frontend tests passed"
        else
            log_warning "Frontend tests failed or not available"
        fi
        cd ..
    fi
    
    log_success "Tests completed"
}

optimize_binaries() {
    log_info "Optimizing binaries..."
    
    # Strip additional symbols if strip is available
    if command -v strip >/dev/null 2>&1; then
        if [[ -f "$BUILD_DIR/rusty-ai-api" ]]; then
            strip "$BUILD_DIR/rusty-ai-api"
            log_info "Stripped debug symbols from binary"
        fi
    fi
    
    # Compress binary if upx is available
    if command -v upx >/dev/null 2>&1; then
        if [[ -f "$BUILD_DIR/rusty-ai-api" ]]; then
            log_info "Compressing binary with UPX..."
            upx --best "$BUILD_DIR/rusty-ai-api" || log_warning "UPX compression failed"
        fi
    fi
    
    log_success "Binary optimization completed"
}

create_distribution() {
    log_info "Creating distribution package..."
    
    # Create distribution directory
    mkdir -p "$DIST_DIR"
    
    # Copy backend binary
    if [[ -f "$BUILD_DIR/rusty-ai-api" ]]; then
        cp "$BUILD_DIR/rusty-ai-api" "$DIST_DIR/"
    fi
    
    # Copy frontend assets
    if [[ -d frontend/dist ]]; then
        cp -r frontend/dist "$DIST_DIR/static"
    elif [[ -d frontend/build ]]; then
        cp -r frontend/build "$DIST_DIR/static"
    fi
    
    # Copy configuration files
    mkdir -p "$DIST_DIR/config"
    cp config/*.toml "$DIST_DIR/config/" 2>/dev/null || true
    
    # Copy migrations
    if [[ -d migrations ]]; then
        cp -r migrations "$DIST_DIR/"
    fi
    
    # Copy plugins directory structure
    if [[ -d plugins ]]; then
        mkdir -p "$DIST_DIR/plugins"
        # Copy only .wasm files and manifests
        find plugins -name "*.wasm" -o -name "*.toml" -o -name "*.json" | while read -r file; do
            cp "$file" "$DIST_DIR/plugins/"
        done
    fi
    
    # Create necessary directories
    mkdir -p "$DIST_DIR"/{data,logs,backups}
    
    # Create systemd service file
    cat > "$DIST_DIR/rusty-ai.service" << EOF
[Unit]
Description=Personal AI Assistant
After=network.target

[Service]
Type=simple
User=rusty-ai
WorkingDirectory=/opt/rusty-ai
ExecStart=/opt/rusty-ai/rusty-ai-api
Restart=always
RestartSec=5
Environment=ENVIRONMENT=production

[Install]
WantedBy=multi-user.target
EOF

    # Create startup script
    cat > "$DIST_DIR/start.sh" << 'EOF'
#!/bin/bash
set -euo pipefail

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   echo "This script should not be run as root for security reasons"
   exit 1
fi

# Set environment
export ENVIRONMENT=production

# Start the application
exec ./rusty-ai-api
EOF
    chmod +x "$DIST_DIR/start.sh"
    
    # Create README for deployment
    cat > "$DIST_DIR/README.md" << EOF
# Personal AI Assistant - Production Build

Version: $VERSION
Build Date: $BUILD_DATE
Git Commit: $GIT_COMMIT

## Installation

1. Copy this directory to your server (e.g., /opt/rusty-ai)
2. Create a rusty-ai user: \`sudo useradd -r -s /bin/false rusty-ai\`
3. Set permissions: \`sudo chown -R rusty-ai:rusty-ai /opt/rusty-ai\`
4. Install systemd service: \`sudo cp rusty-ai.service /etc/systemd/system/\`
5. Enable and start: \`sudo systemctl enable --now rusty-ai\`

## Configuration

- Copy and modify config files in the config/ directory
- Set up environment variables
- Configure database connections
- Set up reverse proxy (nginx recommended)

## Database Setup

1. Create database
2. Run migrations: \`./rusty-ai-api migrate\`

## Monitoring

- Health check: http://localhost:8080/health
- Metrics: http://localhost:9090/metrics
- Logs: journalctl -u rusty-ai -f

## Security

- Run as non-root user
- Use TLS termination at reverse proxy
- Secure database connections
- Regular security updates
EOF

    log_success "Distribution package created in $DIST_DIR/"
}

create_archive() {
    log_info "Creating release archive..."
    
    local archive_name="rusty-ai-$VERSION-$(uname -s)-$(uname -m).tar.gz"
    
    # Create tarball
    tar -czf "$archive_name" -C "$DIST_DIR" .
    
    # Generate checksums
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$archive_name" > "$archive_name.sha256"
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$archive_name" > "$archive_name.sha256"
    fi
    
    local size=$(du -h "$archive_name" | cut -f1)
    log_success "Release archive created: $archive_name ($size)"
}

print_build_summary() {
    echo
    log_success "Build completed successfully!"
    echo
    echo "=========================================="
    echo " 📦 Build Summary"
    echo "=========================================="
    echo "Version: $VERSION"
    echo "Build Date: $BUILD_DATE"
    echo "Git Commit: $GIT_COMMIT"
    echo "Target: $(uname -s)-$(uname -m)"
    echo
    echo "📁 Artifacts:"
    echo "  • Distribution: $DIST_DIR/"
    echo "  • Binary: $BUILD_DIR/rusty-ai-api"
    if [[ -d frontend ]]; then
        echo "  • Frontend: $DIST_DIR/static/"
    fi
    echo
    echo "🚀 Deployment:"
    echo "  • Extract archive to target server"
    echo "  • Follow instructions in $DIST_DIR/README.md"
    echo "  • Configure environment and database"
    echo "  • Start with systemd or Docker"
    echo "=========================================="
    echo
}

main() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Personal AI Assistant Production Build${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo
    
    log_info "Starting production build process..."
    log_info "Version: $VERSION"
    log_info "Target: $(uname -s)-$(uname -m)"
    echo
    
    # Build process
    check_dependencies
    clean_build
    set_build_info
    build_backend
    build_frontend
    
    # Optional steps
    if [[ "${SKIP_TESTS:-}" != "true" ]]; then
        run_tests
    fi
    
    if [[ "${SKIP_OPTIMIZE:-}" != "true" ]]; then
        optimize_binaries
    fi
    
    # Package
    create_distribution
    
    if [[ "${CREATE_ARCHIVE:-true}" == "true" ]]; then
        create_archive
    fi
    
    print_build_summary
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "Personal AI Assistant Production Build Script"
        echo "Usage: $0 [OPTIONS]"
        echo
        echo "Options:"
        echo "  --help, -h        Show this help message"
        echo "  --skip-tests      Skip running tests"
        echo "  --skip-optimize   Skip binary optimization"
        echo "  --no-archive      Don't create release archive"
        echo
        echo "Environment variables:"
        echo "  SKIP_TESTS=true   Skip tests"
        echo "  SKIP_OPTIMIZE=true Skip optimization"
        echo "  CREATE_ARCHIVE=false Don't create archive"
        exit 0
        ;;
    --skip-tests)
        export SKIP_TESTS=true
        ;;
    --skip-optimize)
        export SKIP_OPTIMIZE=true
        ;;
    --no-archive)
        export CREATE_ARCHIVE=false
        ;;
esac

# Run main function
main "$@"