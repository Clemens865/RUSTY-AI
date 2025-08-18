# Contributing to Personal AI Assistant

Thank you for your interest in contributing to the Personal AI Assistant project! This document provides guidelines and instructions for contributing to the codebase.

## Code of Conduct

### Our Pledge

We are committed to making participation in our project a harassment-free experience for everyone, regardless of age, body size, disability, ethnicity, gender identity and expression, level of experience, nationality, personal appearance, race, religion, or sexual identity and orientation.

### Our Standards

Examples of behavior that contributes to creating a positive environment include:

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members

### Unacceptable Behavior

Examples of unacceptable behavior include:

- The use of sexualized language or imagery and unwelcome sexual attention or advances
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information without explicit permission
- Other conduct which could reasonably be considered inappropriate in a professional setting

### Enforcement

Project maintainers are responsible for clarifying the standards of acceptable behavior and are expected to take appropriate and fair corrective action in response to any instances of unacceptable behavior.

## Development Workflow

### Getting Started

1. **Fork the repository** and clone your fork:
   ```bash
   git clone https://github.com/yourusername/personal-ai-assistant.git
   cd personal-ai-assistant
   ```

2. **Set up the development environment**:
   ```bash
   ./setup.sh
   ```

3. **Start the development servers**:
   ```bash
   # Backend
   ./scripts/run_dev.sh

   # Frontend (in a new terminal)
   cd frontend
   npm install
   npm run dev
   ```

### Branch Naming Convention

- `feature/description` - for new features
- `bugfix/description` - for bug fixes
- `docs/description` - for documentation updates
- `refactor/description` - for code refactoring
- `test/description` - for test improvements

### Commit Message Guidelines

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools

**Examples:**
```
feat(voice): add real-time transcription support
fix(api): resolve authentication token expiration issue
docs(readme): update installation instructions
```

## Code Style Guidelines

### Rust Code

We follow the standard Rust formatting and style guidelines:

- **Formatting**: Use `cargo fmt` to format code
- **Linting**: Use `cargo clippy` to check for common mistakes
- **Naming**: Use `snake_case` for variables, functions, and modules
- **Documentation**: Add rustdoc comments for public APIs

```rust
/// Processes a user query and returns a structured response.
///
/// # Arguments
///
/// * `query` - The user's input query
/// * `context` - Additional context for processing
///
/// # Returns
///
/// A `Result` containing the processed response or an error
pub async fn process_query(
    query: &str,
    context: &QueryContext,
) -> Result<QueryResponse, ProcessingError> {
    // Implementation
}
```

### TypeScript/React Code

We use ESLint and Prettier for consistent formatting:

- **Formatting**: Run `npm run format` to format code
- **Linting**: Run `npm run lint` to check for issues
- **Naming**: Use `camelCase` for variables and functions, `PascalCase` for components
- **Components**: Use functional components with hooks

```typescript
interface VoiceChatProps {
  onMessageSent: (message: string) => void;
  isListening: boolean;
}

export const VoiceChat: React.FC<VoiceChatProps> = ({
  onMessageSent,
  isListening,
}) => {
  // Component implementation
};
```

### File Organization

```
src/
├── components/          # Reusable UI components
├── hooks/              # Custom React hooks
├── pages/              # Page components
├── lib/                # Utility functions
├── types/              # TypeScript type definitions
└── config/             # Configuration files
```

## Testing Requirements

### Rust Testing

All Rust code must include appropriate tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_query_success() {
        let query = "What's the weather?";
        let context = QueryContext::default();
        
        let result = process_query(query, &context).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.text.is_empty());
    }
}
```

### Frontend Testing

Use Jest and React Testing Library for frontend tests:

```typescript
import { render, screen, fireEvent } from '@testing-library/react';
import { VoiceChat } from './VoiceChat';

describe('VoiceChat', () => {
  it('calls onMessageSent when message is submitted', () => {
    const mockOnMessageSent = jest.fn();
    
    render(
      <VoiceChat onMessageSent={mockOnMessageSent} isListening={false} />
    );
    
    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'Hello' } });
    fireEvent.submit(input);
    
    expect(mockOnMessageSent).toHaveBeenCalledWith('Hello');
  });
});
```

### Integration Tests

Create integration tests in the `tests/integration/` directory:

```rust
// tests/integration/api_tests.rs
use reqwest;
use serde_json::json;

#[tokio::test]
async fn test_api_health_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/health")
        .send()
        .await
        .unwrap();
    
    assert_eq!(response.status(), 200);
}
```

### Test Coverage

- Maintain at least 80% test coverage for new code
- Run `cargo test` for Rust tests
- Run `npm test` for frontend tests
- Integration tests run with `cargo test --test integration`

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**:
   ```bash
   # Rust tests
   cargo test
   
   # Frontend tests
   cd frontend && npm test
   
   # Integration tests
   cargo test --test integration
   ```

2. **Check code formatting**:
   ```bash
   # Rust
   cargo fmt --check
   cargo clippy
   
   # Frontend
   cd frontend && npm run lint && npm run typecheck
   ```

3. **Update documentation** if needed

4. **Add tests** for new functionality

### Pull Request Template

```markdown
## Description
Brief description of the changes made.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows the style guidelines
- [ ] Self-review completed
- [ ] Comments added for hard-to-understand areas
- [ ] Documentation updated
- [ ] No new warnings introduced
```

### Review Process

1. All PRs require at least one review from a maintainer
2. All CI checks must pass
3. Address review feedback promptly
4. Keep PRs focused and reasonably sized
5. Rebase on main branch before merging

## Development Environment Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 18+ with npm
- Docker and Docker Compose
- PostgreSQL 14+

### Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
# Database
DATABASE_URL=postgresql://username:password@localhost/ai_assistant
TEST_DATABASE_URL=postgresql://username:password@localhost/ai_assistant_test

# API Keys
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key

# Server Configuration
RUST_LOG=debug
SERVER_PORT=8080
FRONTEND_URL=http://localhost:5173
```

### Database Setup

```bash
# Create databases
createdb ai_assistant
createdb ai_assistant_test

# Run migrations
cargo run --bin migrate
```

### Running the Application

```bash
# Development mode
./scripts/run_dev.sh

# Production mode
./scripts/build_release.sh
./target/release/ai-assistant-server
```

## Architecture Guidelines

### Rust Backend Architecture

- Use async/await for I/O operations
- Implement proper error handling with custom error types
- Use dependency injection for testability
- Follow the repository pattern for data access
- Implement proper logging with `tracing`

### Frontend Architecture

- Use React hooks for state management
- Implement proper TypeScript types
- Use React Query for server state
- Follow component composition patterns
- Implement proper error boundaries

### Plugin System

- Plugins must implement the `Plugin` trait
- Use WebAssembly for sandboxed execution
- Implement proper security boundaries
- Document plugin APIs thoroughly

## Getting Help

- **Documentation**: Check the `/docs` directory
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions
- **Discord**: Join our development Discord server

## Recognition

Contributors will be recognized in:
- The project README
- Release notes for their contributions
- Annual contributor appreciation posts

Thank you for contributing to the Personal AI Assistant project!