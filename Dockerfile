# Multi-stage Docker build for Personal AI Assistant Rust Backend
# This Dockerfile creates an optimized production image with minimal attack surface

# ================================
# Stage 1: Build environment
# ================================
FROM rust:1.75-slim-bullseye AS builder

# Install system dependencies required for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    libsqlite3-dev \
    build-essential \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for building
RUN useradd -m -u 1001 builder
USER builder
WORKDIR /home/builder

# Set environment variables for optimal builds
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ENV CARGO_TARGET_DIR=/home/builder/target
ENV RUSTFLAGS="-C target-cpu=native"

# Copy dependency manifests first for better caching
COPY --chown=builder:builder Cargo.toml Cargo.lock ./
COPY --chown=builder:builder crates/ ./crates/

# Create a dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached unless dependencies change)
RUN cargo build --release --locked
RUN rm -rf src

# Copy the actual source code
COPY --chown=builder:builder src/ ./src/
COPY --chown=builder:builder migrations/ ./migrations/

# Build the actual application
RUN cargo build --release --locked --bin ai-assistant-server

# ================================
# Stage 2: Runtime environment
# ================================
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libpq5 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create a non-root user for running the application
RUN useradd -m -u 1001 -s /bin/bash appuser

# Create necessary directories
RUN mkdir -p /app/data /app/logs /app/storage /app/plugins \
    && chown -R appuser:appuser /app

# Copy the compiled binary from builder stage
COPY --from=builder --chown=appuser:appuser /home/builder/target/release/ai-assistant-server /app/ai-assistant-server

# Copy configuration files
COPY --chown=appuser:appuser config/ /app/config/
COPY --chown=appuser:appuser migrations/ /app/migrations/

# Copy startup script
COPY --chown=appuser:appuser docker/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Switch to non-root user
USER appuser
WORKDIR /app

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=postgresql://postgres:password@db:5432/ai_assistant
ENV REDIS_URL=redis://redis:6379
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080

# Expose the application port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Use entrypoint script for proper signal handling
ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["./ai-assistant-server"]

# ================================
# Stage 3: Development environment (optional)
# ================================
FROM rust:1.75-slim-bullseye AS development

# Install development dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    libsqlite3-dev \
    build-essential \
    curl \
    ca-certificates \
    git \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Install cargo tools for development
RUN cargo install cargo-watch cargo-edit sqlx-cli

# Create development user
RUN useradd -m -u 1001 -s /bin/bash developer
USER developer
WORKDIR /workspace

# Set development environment variables
ENV RUST_LOG=debug
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Expose ports for development
EXPOSE 8080 9229

# Default command for development
CMD ["cargo", "run", "--bin", "ai-assistant-server"]

# ================================
# Labels for metadata
# ================================
LABEL maintainer="Personal AI Assistant Team"
LABEL version="1.0.0"
LABEL description="Personal AI Assistant Rust Backend"
LABEL org.opencontainers.image.title="Personal AI Assistant Backend"
LABEL org.opencontainers.image.description="Rust-based backend for Personal AI Assistant"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.vendor="Personal AI Assistant"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/yourusername/personal-ai-assistant"