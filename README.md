# 🤖 RUSTY-AI: Personal AI Assistant

A powerful, extensible AI assistant with integrated Rust backend and React frontend, featuring voice interaction, plugin architecture, and comprehensive task management capabilities.

**Repository**: [github.com/Clemens865/RUSTY-AI](https://github.com/Clemens865/RUSTY-AI) | **Status**: 🟢 Active Development

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)
![React](https://img.shields.io/badge/react-%2320232a.svg?style=for-the-badge&logo=react&logoColor=%2361DAFB)
![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)

## 📊 Current Status

✅ **Backend**: Rust API server with modular architecture  
✅ **Frontend**: React + TypeScript with Tailwind CSS (fully integrated)  
✅ **Database**: SQLite/PostgreSQL with migrations  
✅ **Voice**: STT/TTS pipeline ready for integration  
✅ **Docker**: Complete containerization with docker-compose  
✅ **Plugin System**: WebAssembly-based architecture  

## 🚀 Features

### Core Capabilities
- **🎤 Voice Interaction**: Natural speech-to-text and text-to-speech integration
- **🧩 Plugin System**: WebAssembly-based secure plugin architecture
- **📚 Knowledge Management**: Document storage with semantic search
- **📋 Task Management**: Automated task orchestration and tracking
- **📊 Daily Briefings**: Personalized summaries and insights
- **💬 Conversational AI**: Multi-provider AI integration (OpenAI, Anthropic, Google)

### Technical Highlights
- **High Performance**: Built with Rust for maximum speed and safety
- **Scalable Architecture**: Modular design with microservices support
- **Security First**: Sandboxed plugin execution with resource limits
- **Real-time Features**: WebSocket support for live updates
- **Monitoring**: Comprehensive metrics and health checks
- **Cross-Platform**: Works on Linux, macOS, and Windows

## 📋 Table of Contents

- [Quick Start](#-quick-start)
- [Installation](#-installation)
- [Configuration](#-configuration)
- [Usage](#-usage)
- [Architecture](#-architecture)
- [API Documentation](#-api-documentation)
- [Plugin Development](#-plugin-development)
- [Contributing](#-contributing)
- [License](#-license)

## ⚡ Quick Start

### Prerequisites

- **Rust** 1.75.0 or later
- **Node.js** 18.0.0 or later
- **Docker** (optional, for containerized development)

### One-Command Setup

```bash
# Clone the repository
git clone https://github.com/Clemens865/RUSTY-AI.git
cd RUSTY-AI

# Run the setup script
./setup.sh

# Start development environment
# Run backend on port 8081
cargo run --bin rusty-ai-api

# In another terminal, run frontend on port 5173
cd frontend && npm run dev
```

The setup script will:
- Install required dependencies
- Set up the database
- Configure environment variables
- Start the development servers

Access the application:
- **API**: http://localhost:8080
- **Frontend**: http://localhost:3000
- **API Documentation**: http://localhost:8080/docs

## 🛠 Installation

### Development Setup

1. **Clone and Setup**
   ```bash
   git clone https://github.com/yourusername/rusty-ai.git
   cd rusty-ai
   ./setup.sh
   ```

2. **Manual Installation** (alternative)
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install Node.js dependencies
   cd frontend && npm install && cd ..
   
   # Copy environment configuration
   cp .env.example .env
   
   # Set up database
   make db-setup
   ```

3. **Docker Development**
   ```bash
   # Start all services with Docker
   make docker-dev
   
   # Or manually
   docker-compose up -d
   ```

### Production Deployment

1. **Build Release**
   ```bash
   ./scripts/build_release.sh
   ```

2. **Deploy with Docker**
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
   ```

3. **Manual Deployment**
   ```bash
   # Extract release archive on target server
   tar -xzf rusty-ai-*.tar.gz -C /opt/rusty-ai
   
   # Follow deployment instructions in dist/README.md
   ```

## ⚙️ Configuration

### Environment Variables

Copy `.env.example` to `.env` and configure:

```env
# Database
DATABASE_URL=sqlite:./data/rusty_ai.db

# AI Services
OPENAI_API_KEY=your-api-key
ANTHROPIC_API_KEY=your-api-key

# Security
JWT_SECRET=your-secret-key
SESSION_SECRET=your-session-secret

# Voice Services
AZURE_SPEECH_KEY=your-azure-key
AZURE_SPEECH_REGION=eastus
```

### Configuration Files

- `config/default.toml` - Default application settings
- `config/production.toml` - Production overrides
- `docker-compose.yml` - Development services
- `.env` - Environment-specific variables

### Database Setup

**SQLite** (Default - Development)
```bash
make db-setup
```

**PostgreSQL** (Production)
```bash
# Update DATABASE_URL in .env
DATABASE_URL=postgresql://user:pass@localhost:5432/rusty_ai

# Run migrations
make db-migrate
```

## 🎯 Usage

### Command Line Interface

```bash
# Development
make dev                    # Start development server
make test                   # Run tests
make lint                   # Check code quality

# Database
make db-setup              # Initialize database
make db-migrate            # Run migrations
make db-reset              # Reset database

# Production
make build                 # Build release
make deploy-prod           # Deploy to production

# Utilities
make backup                # Create backup
make logs                  # View logs
make health-check          # Check system health
```

### Web Interface

1. **Dashboard**: Overview of tasks, conversations, and system status
2. **Voice Chat**: Interactive voice conversations with AI
3. **Task Manager**: Create, track, and automate tasks
4. **Knowledge Base**: Upload and search documents
5. **Settings**: Configure plugins, integrations, and preferences

### API Usage

```bash
# Health check
curl http://localhost:8080/health

# Create a task
curl -X POST http://localhost:8080/api/tasks \
  -H "Content-Type: application/json" \
  -d '{"title": "Test Task", "description": "A test task"}'

# Start a conversation
curl -X POST http://localhost:8080/api/conversations \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, how can you help me today?"}'
```

### Voice Commands

- "Create a task to review quarterly reports"
- "What's on my schedule for today?"
- "Search for documents about project management"
- "Generate a daily briefing"

## 🏗 Architecture

### High-Level Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Frontend      │    │   API Gateway   │    │   AI Services   │
│   (React/TS)    │◄──►│   (Rust/Axum)   │◄──►│  (OpenAI, etc)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                       ┌─────────────────┐
                       │   Core Engine   │
                       │                 │
                       │ ┌─────────────┐ │    ┌─────────────────┐
                       │ │ Task Mgmt   │ │◄──►│   Database      │
                       │ └─────────────┘ │    │ (SQLite/Postgres)│
                       │ ┌─────────────┐ │    └─────────────────┘
                       │ │ Plugin Sys  │ │
                       │ └─────────────┘ │    ┌─────────────────┐
                       │ ┌─────────────┐ │◄──►│  Vector Store   │
                       │ │ Voice Proc  │ │    │   (Qdrant)      │
                       │ └─────────────┘ │    └─────────────────┘
                       └─────────────────┘
```

### Components

#### Backend (Rust)
- **API Layer**: REST and WebSocket endpoints
- **Core Engine**: Business logic and orchestration
- **Plugin System**: WebAssembly-based extensions
- **Database Layer**: SQLite/PostgreSQL with migrations
- **Voice Processing**: Speech-to-text and text-to-speech
- **AI Integration**: Multi-provider AI service clients

#### Frontend (TypeScript/React)
- **Dashboard**: Main application interface
- **Voice UI**: Speech interaction components
- **Task Management**: Task creation and tracking
- **Settings**: Configuration and preferences
- **Real-time Updates**: WebSocket integration

#### Infrastructure
- **Database**: SQLite (dev) / PostgreSQL (prod)
- **Vector Store**: Qdrant for semantic search
- **Cache**: Redis for session and data caching
- **Monitoring**: Prometheus + Grafana
- **Deployment**: Docker containers with orchestration

### Plugin Architecture

Plugins are WebAssembly modules that run in a secure sandbox:

```rust
#[async_trait]
pub trait WasmPlugin {
    fn metadata(&self) -> &WasmPluginMetadata;
    async fn initialize(&mut self, config: serde_json::Value) -> Result<()>;
    async fn execute(&self, function: &str, input: &[u8]) -> Result<Vec<u8>>;
    async fn health_check(&self) -> Result<PluginHealth>;
}
```

**Security Features**:
- Memory and CPU limits
- Network and filesystem restrictions
- Capability-based permissions
- Resource monitoring

## 📚 API Documentation

### Authentication

```bash
# Login
POST /api/auth/login
{
  "email": "user@example.com",
  "password": "password"
}

# Response
{
  "token": "jwt-token",
  "user": {...}
}
```

### Tasks

```bash
# Create task
POST /api/tasks
{
  "title": "Task title",
  "description": "Task description",
  "priority": "high",
  "due_date": "2024-12-31T23:59:59Z"
}

# List tasks
GET /api/tasks?status=pending&limit=10

# Update task
PUT /api/tasks/{id}
{
  "status": "completed"
}
```

### Conversations

```bash
# Start conversation
POST /api/conversations
{
  "message": "Hello, AI assistant!"
}

# Send message
POST /api/conversations/{id}/messages
{
  "content": "Follow up message",
  "role": "user"
}

# Get conversation history
GET /api/conversations/{id}/messages
```

### Voice

```bash
# Upload audio for transcription
POST /api/voice/transcribe
Content-Type: multipart/form-data
[audio file]

# Text-to-speech
POST /api/voice/synthesize
{
  "text": "Hello, this is a test",
  "voice": "en-US-AriaNeural"
}
```

See [API.md](API.md) for complete API documentation.

## 🧩 Plugin Development

### Creating a Plugin

1. **Initialize Plugin Project**
   ```bash
   make plugin-new
   # Follow prompts to create plugin template
   ```

2. **Implement Plugin Interface**
   ```rust
   use rusty_ai_plugins::*;
   
   #[export]
   fn execute(input: &str) -> String {
       // Plugin logic here
       "Plugin response".to_string()
   }
   ```

3. **Build and Test**
   ```bash
   # Build plugin to WebAssembly
   cargo build --target wasm32-wasi --release
   
   # Test plugin
   make plugin-test
   ```

4. **Deploy Plugin**
   ```bash
   # Copy .wasm file to plugins directory
   cp target/wasm32-wasi/release/my_plugin.wasm plugins/
   
   # Plugin will be automatically loaded
   ```

### Plugin Examples

- **Weather Plugin**: Get weather information
- **Calendar Plugin**: Manage calendar events
- **Email Plugin**: Send and read emails
- **Analytics Plugin**: Generate reports and insights

## 🧪 Testing

### Running Tests

```bash
# All tests
make test

# Unit tests only
make test-unit

# Integration tests
make test-integration

# With coverage
make test-coverage
```

### Test Categories

- **Unit Tests**: Individual component testing
- **Integration Tests**: End-to-end workflows
- **Plugin Tests**: WebAssembly plugin validation
- **API Tests**: REST endpoint verification
- **Performance Tests**: Load and stress testing

## 🔧 Development

### Prerequisites

- Rust 1.75+ with `cargo-watch` and `sqlx-cli`
- Node.js 18+ with npm
- Docker and Docker Compose
- Git with configured hooks

### Development Workflow

1. **Feature Development**
   ```bash
   # Create feature branch
   git checkout -b feature/new-feature
   
   # Start development server
   make dev
   
   # Make changes and test
   make test
   make lint
   ```

2. **Code Quality**
   ```bash
   # Format code
   make format
   
   # Run linting
   make lint
   
   # Check all quality gates
   make check
   ```

3. **Database Changes**
   ```bash
   # Create migration
   sqlx migrate add migration_name
   
   # Run migrations
   make db-migrate
   ```

### Project Structure

```
rusty-ai/
├── crates/                 # Rust workspace crates
│   ├── api/               # HTTP API server
│   ├── core/              # Core business logic
│   ├── common/            # Shared types and utilities
│   ├── plugins/           # Plugin system
│   ├── voice/             # Voice processing
│   └── knowledge/         # Knowledge management
├── frontend/              # React TypeScript frontend
├── migrations/            # Database migrations
├── config/               # Configuration files
├── scripts/              # Build and deployment scripts
├── docs/                 # Documentation
├── tests/                # Integration tests
└── plugins/              # WebAssembly plugins
```

## 🤝 Contributing

We welcome contributions! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Contribution Guide

1. **Fork and Clone**
   ```bash
   git clone https://github.com/yourusername/rusty-ai.git
   cd rusty-ai
   ```

2. **Set Up Development Environment**
   ```bash
   ./setup.sh
   ```

3. **Create Feature Branch**
   ```bash
   git checkout -b feature/amazing-feature
   ```

4. **Make Changes and Test**
   ```bash
   make check
   make test
   ```

5. **Submit Pull Request**
   - Describe your changes
   - Include tests
   - Update documentation

### Areas for Contribution

- 🐛 **Bug Fixes**: Check open issues
- ✨ **Features**: Propose new capabilities
- 📚 **Documentation**: Improve guides and examples
- 🧩 **Plugins**: Create new plugins
- 🔍 **Testing**: Add test coverage
- 🎨 **UI/UX**: Enhance user interface

## 📈 Roadmap

### Phase 1: Core Foundation ✅
- [x] Basic AI chat interface
- [x] Task management system
- [x] Plugin architecture
- [x] Voice integration
- [x] Database design

### Phase 2: Enhanced Productivity 🚧
- [ ] Advanced task automation
- [ ] Calendar integration
- [ ] Email management
- [ ] Document collaboration
- [ ] Mobile application

### Phase 3: AI-Powered Insights 📋
- [ ] Predictive analytics
- [ ] Habit tracking
- [ ] Goal management
- [ ] Performance metrics
- [ ] Recommendation engine

### Phase 4: Enterprise Features 📋
- [ ] Multi-user support
- [ ] Advanced security
- [ ] API integrations
- [ ] Workflow automation
- [ ] Admin dashboard

## 🔒 Security

### Security Features

- **Authentication**: JWT-based with session management
- **Authorization**: Role-based access control
- **Plugin Sandboxing**: WebAssembly isolation
- **Data Encryption**: At-rest and in-transit
- **Audit Logging**: Comprehensive activity tracking
- **Rate Limiting**: API abuse prevention

### Security Best Practices

- Regular dependency updates
- Secure coding standards
- Penetration testing
- Vulnerability scanning
- Security audits

## 📊 Monitoring

### Health Checks

```bash
# API health
curl http://localhost:8080/health

# Database health
curl http://localhost:8080/health/database

# System metrics
curl http://localhost:9090/metrics
```

### Observability Stack

- **Metrics**: Prometheus + Grafana
- **Tracing**: Jaeger distributed tracing
- **Logging**: Structured JSON logs
- **Alerting**: Webhook-based notifications

## 🚀 Performance

### Benchmarks

- **API Response Time**: < 100ms average
- **Database Queries**: < 50ms for typical operations
- **Plugin Execution**: < 1s for most plugins
- **Voice Processing**: < 3s end-to-end
- **Memory Usage**: < 512MB baseline

### Optimization Features

- Connection pooling
- Query optimization
- Caching strategies
- Async processing
- Resource limits

## 🆘 Troubleshooting

### Common Issues

**Database Connection Failed**
```bash
# Check database status
make db-setup

# Reset database
make db-reset
```

**Plugin Not Loading**
```bash
# Check plugin directory
ls -la plugins/

# Validate plugin
make plugin-test
```

**Voice Services Not Working**
```bash
# Check API keys in .env
grep AZURE_SPEECH .env

# Test voice endpoint
curl -X POST http://localhost:8080/api/voice/test
```

### Getting Help

- 📖 **Documentation**: Check docs/ directory
- 🐛 **Issues**: Open GitHub issue
- 💬 **Discussions**: GitHub discussions
- 📧 **Email**: support@rusty-ai.dev

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Rust Community**: For amazing tooling and libraries
- **WebAssembly**: For secure plugin execution
- **AI Providers**: OpenAI, Anthropic, Google for AI capabilities
- **Open Source**: All the wonderful open source projects we build upon

---

**Built with ❤️ using Rust and TypeScript**

[![GitHub stars](https://img.shields.io/github/stars/yourusername/rusty-ai.svg?style=social&label=Star)](https://github.com/yourusername/rusty-ai)
[![GitHub forks](https://img.shields.io/github/forks/yourusername/rusty-ai.svg?style=social&label=Fork)](https://github.com/yourusername/rusty-ai/fork)
[![GitHub issues](https://img.shields.io/github/issues/yourusername/rusty-ai.svg)](https://github.com/yourusername/rusty-ai/issues)
[![GitHub license](https://img.shields.io/github/license/yourusername/rusty-ai.svg)](https://github.com/yourusername/rusty-ai/blob/main/LICENSE)