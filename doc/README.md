<!-- Copyright 2025 Cowboy AI, LLC. -->

# CIM Domain Documentation

Welcome to the comprehensive documentation for the CIM Domain - the foundational Domain-Driven Design framework for the Composable Information Machine.

## Quick Links

- [Architecture Overview](architecture/overview.md) - System design and principles
- [Implementation Status](development/implementation-status.md) - Current progress and roadmap
- [User Stories](quality/user-stories.md) - Requirements and acceptance criteria

## Documentation Structure

### 📐 Architecture

Core architectural documentation explaining the design and structure of CIM Domain.

- **[Overview](architecture/overview.md)** - High-level system architecture and design principles
- **[Aggregates](architecture/aggregates.md)** - Detailed documentation of all domain aggregates
- **[Design Patterns](architecture/design-patterns.md)** - Core patterns and anti-patterns
- **[Components](architecture/components.md)** - Component system and composition patterns

### 🛠️ Development

Implementation details and development guidelines.

- **[Implementation Status](development/implementation-status.md)** - Current progress tracking
- **[Testing](development/testing.md)** - Testing strategy and coverage reports
- **[API Reference](development/api-reference.md)** - Public API documentation

### ✅ Quality

Quality assurance and requirements documentation.

- **[User Stories](quality/user-stories.md)** - Comprehensive user stories and acceptance tests
- **[QA Reports](quality/qa-reports.md)** - Quality assurance findings and recommendations

## Key Concepts

### Core Entities

The domain model is built around five fundamental entities:

1. **People** - Human actors with identity and decision-making capabilities
2. **Agents** - Automated actors that execute tasks within bounded capabilities
3. **Organizations** - Collective entities that group people and agents
4. **Locations** - Physical or logical spaces where activities occur
5. **Policies** - Governance rules that control system behavior

### Architecture Principles

- **Event-Driven** - All state changes are events
- **CQRS** - Complete separation of commands and queries
- **Domain Isolation** - No shared state between domains
- **Category Theory** - Mathematical foundation for composition
- **Type Safety** - Compile-time guarantees through Rust's type system

## Getting Started

1. Read the [Architecture Overview](architecture/overview.md) to understand the system design
2. Review [Aggregates](architecture/aggregates.md) to understand domain entities
3. Check [Implementation Status](development/implementation-status.md) for current progress
4. See [User Stories](quality/user-stories.md) for usage examples

## Navigation

```
doc/
├── README.md                    # This file
├── architecture/               # System design and architecture
│   ├── overview.md            # High-level architecture
│   ├── aggregates.md          # Domain aggregate documentation
│   ├── design-patterns.md     # Patterns and principles
│   └── components.md          # Component system
├── development/               # Implementation details
│   ├── implementation-status.md # Progress tracking
│   ├── testing.md             # Test documentation
│   └── api-reference.md       # API documentation
└── quality/                   # QA and requirements
    ├── user-stories.md        # User stories and acceptance tests
    └── qa-reports.md          # Quality findings
```

## Contributing

See the main [README](../README.md) for contribution guidelines. All documentation updates should:

1. Follow the established structure
2. Include examples where appropriate
3. Be reviewed for technical accuracy
4. Maintain consistency with code

## Status

- **Core Library**: ✅ Production ready (196 tests passing)
- **Documentation**: 📝 Consolidated and organized
- **Examples**: ⚠️ Some need updating for current API

Last updated: 2025-01-16