<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Contributing to CIM-Domain

Thank you for your interest in contributing to CIM-Domain! We welcome contributions from the community.

## Code of Conduct

By participating in this project, you agree to abide by our code of conduct: be respectful, constructive, and professional in all interactions.

## How to Contribute

### Reporting Issues

- Check if the issue already exists in the [issue tracker](https://github.com/thecowboyai/cim-domain/issues)
- Provide a clear description of the problem
- Include steps to reproduce the issue
- Include your environment details (OS, Rust version, etc.)

### Submitting Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Run formatting (`cargo fmt`)
7. Run linting (`cargo clippy`)
8. Commit your changes with clear messages
9. Push to your fork
10. Open a pull request with a clear description

### Development Setup

```bash
# Clone the repository
git clone https://github.com/thecowboyai/cim-domain.git
cd cim-domain

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install NATS server for testing
# macOS
brew install nats-server

# Linux
wget https://github.com/nats-io/nats-server/releases/download/v2.10.7/nats-server-v2.10.7-linux-amd64.tar.gz
tar -xzf nats-server-v2.10.7-linux-amd64.tar.gz

# Start NATS with JetStream
nats-server -js

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Coding Standards

- Follow Rust naming conventions
- Write clear, self-documenting code
- Add comments for complex logic
- Keep functions focused and small
- Write comprehensive tests
- Document public APIs

### Testing

- Write unit tests for new functionality
- Add integration tests for cross-module features
- Ensure existing tests continue to pass
- Aim for high test coverage
- Test edge cases and error conditions

### Documentation

- Update documentation for any API changes
- Add examples for new features
- Keep README.md up to date
- Document breaking changes

### Commit Messages

Follow conventional commit format:

```
type(scope): subject

body (optional)

footer (optional)
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Test additions or modifications
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Maintenance tasks

Example:
```
feat(persistence): add event sourcing support

Implements event sourcing with NATS JetStream backend,
including event replay and snapshot capabilities.

Closes #123
```

## Areas for Contribution

- **New Aggregates**: Add domain aggregates for different business contexts
- **Persistence Backends**: Implement new storage backend adapters
- **Performance**: Optimize event processing and state management
- **Documentation**: Improve guides, examples, and API docs
- **Testing**: Increase test coverage and add property-based tests
- **Features**: Implement items from the roadmap

## Questions?

Feel free to open an issue for questions or join our community discussions.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---
Copyright (c) 2025 - Cowboy AI, LLC. 