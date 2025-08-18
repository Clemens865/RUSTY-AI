# Multi-stage Docker build for Personal AI Assistant Frontend
# This Dockerfile creates an optimized production image for the React/Vite frontend

# ================================
# Stage 1: Build environment
# ================================
FROM node:18-alpine AS builder

# Set working directory
WORKDIR /app

# Install build dependencies
RUN apk add --no-cache \
    git \
    python3 \
    make \
    g++

# Create non-root user for building
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001 -G nodejs

# Copy package files for dependency installation
COPY package*.json ./
COPY bun.lockb ./

# Install dependencies
# Use npm ci for faster, reliable builds in production
RUN npm ci --only=production --silent

# Copy source code
COPY --chown=nextjs:nodejs . .

# Set environment variables for build
ENV NODE_ENV=production
ENV VITE_APP_NAME="Personal AI Assistant"
ENV VITE_APP_VERSION="1.0.0"

# Build the application
RUN npm run build

# Verify build output
RUN ls -la dist/

# ================================
# Stage 2: Production runtime with Nginx
# ================================
FROM nginx:1.24-alpine AS production

# Install runtime dependencies
RUN apk add --no-cache \
    curl \
    ca-certificates \
    tzdata

# Create non-root user
RUN addgroup -g 1001 -S nginx && \
    adduser -S appuser -u 1001 -G nginx

# Copy built application from builder stage
COPY --from=builder --chown=appuser:nginx /app/dist /usr/share/nginx/html

# Copy custom nginx configuration
COPY --chown=appuser:nginx docker/nginx.conf /etc/nginx/nginx.conf
COPY --chown=appuser:nginx docker/default.conf /etc/nginx/conf.d/default.conf

# Copy security headers configuration
COPY --chown=appuser:nginx docker/security-headers.conf /etc/nginx/conf.d/security-headers.conf

# Create nginx cache and log directories
RUN mkdir -p /var/cache/nginx /var/log/nginx /var/run && \
    chown -R appuser:nginx /var/cache/nginx /var/log/nginx /var/run

# Switch to non-root user
USER appuser

# Expose port 80
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost/ || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]

# ================================
# Stage 3: Development environment
# ================================
FROM node:18-alpine AS development

# Install development dependencies
RUN apk add --no-cache \
    git \
    python3 \
    make \
    g++ \
    curl

# Create development user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S developer -u 1001 -G nodejs

# Set working directory
WORKDIR /app

# Change ownership of working directory
RUN chown developer:nodejs /app

# Switch to development user
USER developer

# Copy package files
COPY --chown=developer:nodejs package*.json ./
COPY --chown=developer:nodejs bun.lockb ./

# Install all dependencies (including dev dependencies)
RUN npm install

# Set development environment variables
ENV NODE_ENV=development
ENV VITE_API_BASE_URL=http://localhost:8080
ENV VITE_WS_BASE_URL=ws://localhost:8080
ENV VITE_ENABLE_DEV_TOOLS=true
ENV VITE_ENABLE_CONSOLE_LOGS=true

# Expose development server port
EXPOSE 5173

# Default command for development
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

# ================================
# Stage 4: Testing environment
# ================================
FROM node:18-alpine AS testing

# Install testing dependencies
RUN apk add --no-cache \
    git \
    python3 \
    make \
    g++ \
    chromium \
    curl

# Set Puppeteer environment variables
ENV PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium-browser

# Create test user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S tester -u 1001 -G nodejs

# Set working directory
WORKDIR /app

# Change ownership of working directory
RUN chown tester:nodejs /app

# Switch to test user
USER tester

# Copy package files
COPY --chown=tester:nodejs package*.json ./
COPY --chown=tester:nodejs bun.lockb ./

# Install dependencies
RUN npm ci

# Copy source code
COPY --chown=tester:nodejs . .

# Set test environment variables
ENV NODE_ENV=test
ENV CI=true

# Default command for testing
CMD ["npm", "test"]

# ================================
# Labels for metadata
# ================================
LABEL maintainer="Personal AI Assistant Team"
LABEL version="1.0.0"
LABEL description="Personal AI Assistant React Frontend"
LABEL org.opencontainers.image.title="Personal AI Assistant Frontend"
LABEL org.opencontainers.image.description="React/Vite frontend for Personal AI Assistant"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.vendor="Personal AI Assistant"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.source="https://github.com/yourusername/personal-ai-assistant"