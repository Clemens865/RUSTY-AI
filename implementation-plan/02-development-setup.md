# Development Setup - Environment and Tooling

## Overview

This guide provides comprehensive setup instructions for the Personal AI Assistant development environment, including all required tools, dependencies, and configuration steps.

## Prerequisites

### System Requirements

**Minimum Specifications**:
- CPU: 4 cores, 2.5 GHz
- RAM: 16 GB (32 GB recommended for ML model development)
- Storage: 50 GB free space (SSD recommended)
- GPU: CUDA-compatible GPU with 8GB+ VRAM (optional but recommended)

**Supported Platforms**:
- Linux (Ubuntu 20.04+, Debian 11+, Fedora 35+)
- macOS (12.0+ with Apple Silicon or Intel)
- Windows 10/11 with WSL2

## Core Development Tools

### 1. Rust Toolchain Setup

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install specific Rust version and components
rustup install 1.75.0
rustup default 1.75.0
rustup component add rustfmt clippy rust-analyzer

# Install cross-compilation targets
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Install cargo extensions
cargo install cargo-watch cargo-edit cargo-audit cargo-deny
cargo install cargo-nextest cargo-llvm-cov
cargo install wasm-pack wasmtime-cli
```

### 2. System Dependencies

#### Ubuntu/Debian
```bash
# Essential build tools
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev
sudo apt install -y cmake clang llvm-dev libclang-dev

# Audio processing dependencies
sudo apt install -y libasound2-dev portaudio19-dev

# Optional GPU support (NVIDIA)
sudo apt install -y nvidia-cuda-toolkit nvidia-cuda-dev

# Python for ML model conversion (optional)
sudo apt install -y python3 python3-pip python3-venv
```

#### macOS
```bash
# Install Xcode command line tools
xcode-select --install

# Install Homebrew
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install cmake llvm pkg-config portaudio

# For Apple Silicon Macs
export PATH="/opt/homebrew/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/lib"
export CPPFLAGS="-I/opt/homebrew/include"
```

#### Windows (WSL2)
```bash
# Update WSL2
wsl --update

# Install Ubuntu in WSL2
wsl --install -d Ubuntu-22.04

# Follow Ubuntu setup instructions above
# Install Windows Build Tools
# Visual Studio Build Tools or Visual Studio Community
```

### 3. Database Setup

#### Qdrant Vector Database
```bash
# Using Docker (recommended for development)
docker run -p 6333:6333 -p 6334:6334 \
    -v $(pwd)/qdrant_storage:/qdrant/storage:z \
    qdrant/qdrant

# Or install locally
wget https://github.com/qdrant/qdrant/releases/download/v1.7.4/qdrant-x86_64-unknown-linux-gnu.tar.gz
tar xzf qdrant-x86_64-unknown-linux-gnu.tar.gz
sudo mv qdrant /usr/local/bin/

# Start Qdrant service
qdrant --config-path ./config/production.yaml
```

#### SQLite Setup
```bash
# SQLite is included with most systems
# Verify installation
sqlite3 --version

# Install SQLite development headers (Linux)
sudo apt install -y libsqlite3-dev

# Install DB browser for development (optional)
sudo apt install -y sqlitebrowser
```

## Project Structure Setup

### 1. Initialize Project Repository

```bash
# Clone the project
git clone <repository-url> personal-ai-assistant
cd personal-ai-assistant

# Create project structure
mkdir -p {src/{api,core,plugins,voice},models,data,docs,tests,scripts}
mkdir -p frontend config

# Initialize Rust workspace
cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "crates/core",
    "crates/api", 
    "crates/voice",
    "crates/plugins",
    "crates/ml",
    "examples/*"
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
EOF
```

### 2. Frontend Setup (vox-chic-studio integration)

```bash
# Clone vox-chic-studio
git clone https://github.com/Clemens865/vox-chic-studio.git frontend/vox-chic-studio
cd frontend/vox-chic-studio

# Install Node.js and dependencies
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install project dependencies
npm install

# Configure for Personal AI Assistant integration
cat > .env.local << 'EOF'
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080/api/v1
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
NEXT_PUBLIC_VOICE_ENABLED=true
EOF
```

### 3. ML Models Setup

```bash
# Create models directory structure
mkdir -p models/{whisper,embeddings,chat}

# Download Whisper models
cd models/whisper
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin

# Download embedding models
cd ../embeddings
git lfs install
git clone https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2

# Return to project root
cd ../..
```

## Development Environment Configuration

### 1. Environment Variables Setup

Create `.env` file in project root:
```bash
cat > .env << 'EOF'
# Application Configuration
RUST_LOG=debug
APP_HOST=127.0.0.1
APP_PORT=8080
APP_ENV=development

# Database Configuration
QDRANT_URL=http://localhost:6333
SQLITE_DATABASE_URL=sqlite:./data/assistant.db
ROCKSDB_PATH=./data/cache

# Voice Configuration
WHISPER_MODEL_PATH=./models/whisper/ggml-base.en.bin
ELEVENLABS_API_KEY=your_api_key_here
ELEVENLABS_VOICE_ID=your_voice_id_here

# Security
ENCRYPTION_KEY_PATH=./config/encryption.key
JWT_SECRET=your_jwt_secret_here

# External APIs
GOOGLE_API_KEY=your_google_api_key
OPENAI_API_KEY=your_openai_api_key

# Plugin Configuration
PLUGIN_DIR=./plugins
WASM_CACHE_DIR=./data/wasm_cache
EOF
```

### 2. Logging Configuration

Create `config/logging.yaml`:
```yaml
appenders:
  stdout:
    kind: console
    encoder:
      kind: pattern
      pattern: "{d(%Y-%m-%d %H:%M:%S%.3f)} [{t}] {h({l})} {M} - {m}{n}"
  file:
    kind: file
    path: "logs/assistant.log"
    encoder:
      kind: pattern
      pattern: "{d(%Y-%m-%d %H:%M:%S%.3f)} [{t}] {l} {M} - {m}{n}"

root:
  level: info
  appenders:
    - stdout
    - file

loggers:
  personal_ai_assistant:
    level: debug
    appenders:
      - stdout
    additive: false
```

### 3. IDE Setup

#### Visual Studio Code
```bash
# Install VS Code extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension vadimcn.vscode-lldb
code --install-extension serayuzgur.crates
code --install-extension tamasfe.even-better-toml

# Create workspace settings
mkdir -p .vscode
cat > .vscode/settings.json << 'EOF'
{
    "rust-analyzer.cargo.allFeatures": true,
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.rustfmt.extraArgs": [
        "+nightly"
    ],
    "files.watcherExclude": {
        "**/target/**": true,
        "**/node_modules/**": true
    }
}
EOF

# Create launch configuration
cat > .vscode/launch.json << 'EOF'
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Personal AI Assistant",
            "cargo": {
                "args": [
                    "build",
                    "--bin=personal-ai-assistant"
                ],
                "filter": {
                    "name": "personal-ai-assistant",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}",
            "env": {
                "RUST_LOG": "debug"
            }
        }
    ]
}
EOF
```

## Development Workflow Setup

### 1. Git Configuration

```bash
# Configure Git hooks
mkdir -p .git/hooks

# Pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format code
cargo fmt --all --check
if [ $? -ne 0 ]; then
    echo "Code formatting check failed. Run 'cargo fmt' to fix."
    exit 1
fi

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy check failed."
    exit 1
fi

# Run tests
cargo test --all
if [ $? -ne 0 ]; then
    echo "Tests failed."
    exit 1
fi

echo "All pre-commit checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

### 2. Development Scripts

Create `scripts/dev.sh`:
```bash
#!/bin/bash
# Development script for running the application with hot reload

export RUST_LOG=debug
export RUST_BACKTRACE=1

# Start Qdrant if not running
if ! pgrep -x "qdrant" > /dev/null; then
    echo "Starting Qdrant..."
    qdrant --config-path ./config/qdrant.yaml &
    sleep 2
fi

# Start the application with hot reload
cargo watch -x "run --bin personal-ai-assistant"
```

Create `scripts/test.sh`:
```bash
#!/bin/bash
# Comprehensive testing script

set -e

echo "Running unit tests..."
cargo test --lib

echo "Running integration tests..."
cargo test --test integration

echo "Running benchmarks..."
cargo bench

echo "Checking code coverage..."
cargo llvm-cov --html --output-dir coverage

echo "Security audit..."
cargo audit

echo "License check..."
cargo deny check

echo "All tests completed successfully!"
```

### 3. Docker Development Environment

Create `docker-compose.dev.yml`:
```yaml
version: '3.8'

services:
  qdrant:
    image: qdrant/qdrant:v1.7.4
    ports:
      - "6333:6333"
      - "6334:6334"
    volumes:
      - qdrant_data:/qdrant/storage
    environment:
      QDRANT__SERVICE__GRPC_PORT: 6334

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

  assistant:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "8080:8080"
    volumes:
      - .:/workspace
      - target_cache:/workspace/target
    environment:
      RUST_LOG: debug
      QDRANT_URL: http://qdrant:6333
      REDIS_URL: redis://redis:6379
    depends_on:
      - qdrant
      - redis

volumes:
  qdrant_data:
  redis_data:
  target_cache:
```

## Performance Optimization Setup

### 1. Rust Compiler Optimizations

Create `.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "target-cpu=native"]

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[profile.dev]
debug = true
split-debuginfo = "unpacked"

[profile.release]
debug = false
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.bench]
debug = true
lto = "thin"

[net]
git-fetch-with-cli = true
```

### 2. Memory Profiling Setup

```bash
# Install memory profiling tools
cargo install cargo-profdata
cargo install flamegraph

# Install system profiling tools (Linux)
sudo apt install -y valgrind heaptrack

# Create profiling script
cat > scripts/profile.sh << 'EOF'
#!/bin/bash

echo "Building with debug symbols..."
cargo build --release --bin personal-ai-assistant

echo "Running memory profiler..."
heaptrack target/release/personal-ai-assistant &
PID=$!

sleep 30
kill $PID

echo "Generating flame graph..."
flamegraph -o flamegraph.svg target/release/personal-ai-assistant
EOF
```

## Continuous Integration Setup

### 1. GitHub Actions Workflow

Create `.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      qdrant:
        image: qdrant/qdrant:v1.7.4
        ports:
          - 6333:6333
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Install system dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libasound2-dev portaudio19-dev
    
    - name: Check formatting
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings
    
    - name: Run tests
      run: cargo test --all-features --verbose
      env:
        QDRANT_URL: http://localhost:6333
    
    - name: Run security audit
      run: cargo audit
```

## Troubleshooting Common Issues

### 1. Compilation Issues

**CUDA/GPU Setup**:
```bash
# Verify CUDA installation
nvcc --version
nvidia-smi

# Set CUDA environment variables
export CUDA_ROOT=/usr/local/cuda
export PATH=$CUDA_ROOT/bin:$PATH
export LD_LIBRARY_PATH=$CUDA_ROOT/lib64:$LD_LIBRARY_PATH
```

**Linking Issues on Linux**:
```bash
# Install additional linker
sudo apt install -y lld

# Alternative: use gold linker
sudo apt install -y binutils-gold
```

### 2. Runtime Issues

**Port Conflicts**:
```bash
# Check for port usage
sudo netstat -tulpn | grep :8080
sudo netstat -tulpn | grep :6333

# Kill conflicting processes
sudo fuser -k 8080/tcp
sudo fuser -k 6333/tcp
```

**Permission Issues**:
```bash
# Fix file permissions
chmod +x scripts/*.sh
sudo chown -R $USER:$USER ~/.cargo

# Audio device permissions (Linux)
sudo usermod -a -G audio $USER
```

### 3. Development Environment Verification

Create `scripts/verify-setup.sh`:
```bash
#!/bin/bash

echo "Verifying development environment setup..."

# Check Rust installation
echo "Checking Rust..."
rust_version=$(rustc --version)
echo "✓ Rust: $rust_version"

# Check cargo tools
echo "Checking cargo tools..."
cargo --version && echo "✓ Cargo installed"
cargo-watch --version && echo "✓ cargo-watch installed"

# Check database connections
echo "Checking database connections..."
curl -s http://localhost:6333/health && echo "✓ Qdrant accessible"

# Check audio system
echo "Checking audio system..."
aplay -l &>/dev/null && echo "✓ Audio system available"

# Check GPU (if available)
if command -v nvidia-smi &> /dev/null; then
    nvidia-smi &>/dev/null && echo "✓ NVIDIA GPU available"
fi

echo "Environment verification complete!"
```

## Next Steps

After completing this setup:

1. **Run verification script**: `./scripts/verify-setup.sh`
2. **Start development environment**: `./scripts/dev.sh`
3. **Run initial tests**: `./scripts/test.sh`
4. **Begin Phase 1 implementation**: Follow `03-phase1-mvp.md`

This development environment provides a robust foundation for building the Personal AI Assistant with modern tooling, performance optimization, and comprehensive testing capabilities.