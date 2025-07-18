# CIM Domain

[![Crates.io](https://img.shields.io/crates/v/cim-domain.svg)](https://crates.io/crates/cim-domain)
[![Documentation](https://docs.rs/cim-domain/badge.svg)](https://docs.rs/cim-domain)
[![Test Coverage](https://img.shields.io/codecov/c/github/thecowboyai/cim-domain)](https://codecov.io/gh/thecowboyai/cim-domain)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Core Domain-Driven Design (DDD) components and traits for the Composable Information Machine (CIM).

## Critical Context

**cim-domain is the HEART of CIM** - it defines the foundational domain-driven architecture that all other domains and components depend on. This is our core module that MUST maintain 100% test coverage and production readiness.

## Overview

The `cim-domain` crate provides the fundamental building blocks for implementing Domain-Driven Design patterns in any CIM implementation:

- **Event-driven architecture** foundation with CQRS implementation
- **Domain-Driven Design (DDD) primitives** (Aggregates, Commands, Events, Queries)
- **Category Theory-based** inter-domain communication
- **Event sourcing** with NATS JetStream integration
- **Cross-domain integration** patterns
- **Production-ready infrastructure** for building domains

## Core Components

### DDD Building Blocks

- **Component**: Trait for attachable components with type erasure
- **Entity**: Types with identity and lifecycle  
- **Value Objects**: Immutable types defined by their attributes
- **Aggregates**: Consistency boundaries with root entities
- **Domain Events**: Things that happen in the domain
- **Commands**: Requests to change state (return only acknowledgments)
- **Queries**: Requests to read state (return only acknowledgments)
- **State Machines**: Enum-based state management with controlled transitions

### Architecture Features

- Event-driven architecture with CQRS pattern
- Content-addressed events with CID chains
- Async event streams using NATS JetStream
- State machine abstractions (Moore and Mealy machines)
- Component system for extensible domain objects
- Full test coverage with examples

## Core Entities

The domain model is built around these fundamental entities:

1. **People** - Human actors with identity and decision-making capabilities
2. **Agents** - Automated actors that execute tasks within bounded capabilities
3. **Organizations** - Collective entities that group people and agents
4. **Locations** - Physical or logical spaces where activities occur
5. **Policies** - Governance rules that control system behavior

## Design Principles

- **Event-First Design** - Everything is an event, no CRUD operations
- **CQRS Pattern** - Complete separation of commands and queries
- **Domain Isolation** - No shared state between domains
- **Category Theory** - Functors, morphisms, and natural transformations for domain communication
- **Event Sourcing** - Complete audit trail with time-travel debugging
- **Type Safety** - Leverages Rust's type system for compile-time guarantees
- **Immutability** - Value objects are immutable by design
- **Composability** - Build complex systems from simple, well-defined components

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cim-domain = "0.3.0"
```

## Usage

### Basic Example

```rust
use cim_domain::{Entity, EntityId, DomainEvent, Command};
use serde::{Deserialize, Serialize};

// Define a domain entity
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: EntityId,
    name: String,
    email: String,
}

impl Entity for User {
    fn id(&self) -> EntityId {
        self.id.clone()
    }
}

// Define domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
enum UserEvent {
    Created { id: EntityId, name: String, email: String },
    NameChanged { id: EntityId, new_name: String },
}

impl DomainEvent for UserEvent {
    fn event_type(&self) -> String {
        match self {
            UserEvent::Created { .. } => "UserCreated".to_string(),
            UserEvent::NameChanged { .. } => "UserNameChanged".to_string(),
        }
    }
}

// Define commands
#[derive(Debug, Clone, Serialize, Deserialize)]
enum UserCommand {
    CreateUser { name: String, email: String },
    ChangeName { id: EntityId, new_name: String },
}

impl Command for UserCommand {}
```

### Creating a New Domain

```rust
use cim_domain::{
    person::{PersonId, Person, RegisterPerson},
    DomainResult,
};

// Create a new person
let person_id = PersonId::new();
let command = RegisterPerson {
    id: person_id,
    name: "Alice Smith".to_string(),
    email: "alice@example.com".to_string(),
};

// Process command to get events
let events = Person::handle_command(command)?;
```

## Project Structure

```
cim-domain/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── entity.rs           # Core entity traits
│   ├── commands.rs         # Command definitions
│   ├── events.rs           # Event definitions
│   ├── queries.rs          # Query definitions
│   ├── errors.rs           # Error types
│   ├── cqrs.rs             # CQRS implementation
│   ├── category/           # Category theory
│   ├── domain/             # Domain utilities
│   ├── integration/        # Cross-domain
│   └── state_machine.rs    # State machines
├── crates/
│   ├── cim-component/      # ECS component system
│   ├── cim-ipld/           # Content addressing
│   └── cim-subject/        # NATS subject algebra
└── tests/                  # Integration tests
```

## Development

### Building

```bash
cargo build
```

### Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### Running Examples

```bash
# Basic CQRS pattern demo
cargo run --example cqrs_pattern_demo

# State machine demo
cargo run --example state_machine_demo

# Event sourcing demo
cargo run --example full_event_sourcing_demo

# Simple example demonstrating core functionality
cargo run --example simple_example
```

## Current Status

**Library Status**: ✅ Complete and functional
- **196 tests passing** (100% pass rate)
- **9 doc tests** demonstrating usage
- **Zero compilation warnings** in domain logic
- **Production ready** - Used by 14+ domains

**Infrastructure**: ✅ Complete
- Event Store integration with NATS JetStream
- Command/Query handlers with proper CQRS separation
- Bevy ECS bridge for visualization
- Event replay and snapshot capabilities

**Examples Status**: ⚠️ Needs updating
- `simple_example.rs` - ✅ Working example demonstrating core functionality
- Other examples need to be updated to work with the current API
- Many examples depend on types that have been moved to specific domain modules

## Documentation

- [User Stories and Acceptance Tests](doc/qa/cim-domain-user-stories.md)
- [QA Report](doc/qa/cim-domain-qa-report.md)
- [Component Architecture](doc/design/component-architecture.md)
- [Domain Design Principles](doc/design/domain-design-principles.md)

## Performance Targets

- Event creation: >500,000/sec
- Event publishing: >100,000/sec  
- Command processing: <10ms p99
- Query execution: <5ms p99
- Memory per aggregate: <10KB

## Contributing

This is a foundational crate for CIM implementations. All changes must:

1. Maintain backward compatibility AFTER v0.5.0 (currently v0.3.0)
2. Include comprehensive tests
3. Follow DDD principles
4. Update documentation
5. Pass all quality checks

### Development Rules

- **MANDATORY Test-Driven Development** - Write failing tests first
- **100% test coverage required** - Every public API must be tested
- **Integration tests for cross-domain** - Test domain interactions
- **Doc tests for examples** - Every major component needs usage examples

## Dependencies

### Internal (within cim-domain workspace)
- `cim-component` - ECS component system
- `cim-ipld` - Content addressing with CIDs
- `cim-subject` - NATS subject algebra

### External
- `tokio` - Async runtime
- `serde` - Serialization
- `async-trait` - Async traits
- `thiserror` - Error handling
- `uuid` - Entity IDs
- `chrono` - Timestamps

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Working with this Module

When making changes:
1. **Always run tests first** - `cargo test`
2. **Check benchmarks** - `cargo bench`
3. **Update docs** - `cargo doc --open`
4. **Run examples** - `cargo run --example <name>`
5. **Verify downstream** - Test dependent domains

This module is the foundation of CIM - treat it with appropriate care and rigor.