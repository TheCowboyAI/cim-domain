<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# CIM Domain

[![CI](https://github.com/thecowboyai/cim-domain/actions/workflows/ci.yml/badge.svg)](https://github.com/thecowboyai/cim-domain/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/cim-domain.svg)](https://crates.io/crates/cim-domain)
[![Documentation](https://docs.rs/cim-domain/badge.svg)](https://docs.rs/cim-domain)
[![Test Coverage](https://img.shields.io/codecov/c/github/thecowboyai/cim-domain)](https://codecov.io/gh/thecowboyai/cim-domain)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Core Domain-Driven Design (DDD) components and traits for the Composable Information Machine (CIM). This crate is a pure domain library: no persistence, routing, or external I/O.

## Critical Context

**cim-domain is the HEART of CIM** - it defines the foundational domain-driven architecture that all other domains and components depend on. This is our core module that MUST maintain 100% test coverage and production readiness.

## Overview

The `cim-domain` crate provides the fundamental building blocks for implementing Domain-Driven Design patterns in any CIM implementation:

- **Event-driven architecture** foundation with CQRS traits
- **Domain-Driven Design (DDD) primitives** (Aggregates, Commands, Events, Queries)
- **Category Theory-informed** interfaces for inter-domain concepts
- **Pure library** scope suitable for composing domain models

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
- Interfaces for event streams (infrastructure provided downstream)
- State machine abstractions (Moore and Mealy machines)
- Component system for extensible domain objects
- Full test coverage with examples

### Infrastructure Boundary

This crate does not include persistence, routing, or transport. Implement these concerns in downstream crates (e.g., storage, messaging, and subject routing live outside this library).

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
cim-domain = "0.7.8"
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

### Infrastructure Example

Persistence, transport, and routing are intentionally out of scope here.
Compose them in your infrastructure crate and keep domain logic pure.

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
│   └── (infrastructure crates live outside this repo)
├── tests/                  # Integration tests
├── benches/                # Performance benchmarks
└── examples/               # Usage examples
```

## Dialog DAG Tools

This repository maintains a `dialog-dag.json` that captures conversation history as a content-addressed DAG.
To keep the core crate pure, the maintenance utilities live in a separate tools crate:

- Location: `tools/dialog_dag/`
- Purpose: append and merge dialog events, and reindex CIDs using the same content-addressing approach used across CIM (Blake3 → Multihash 0x1e → CIDv1 codec 0x55).

Quickstart:

```
# Recompute proper CIDv1 values for all events and fix parent links
cargo run -p dialog_dag_tools --bin reindex_dialog_cids -- dialog-dag.json

# Append a new event (what_i_did is semicolon-separated)
cargo run -p dialog_dag_tools --bin log_dialog_event -- \
  dialog-dag.json assistant \
  "Short summary of my message" \
  "What I understood" \
  "action1;action2"

# Merge a continuation file into the main DAG (de-dupes by cid)
cargo run -p dialog_dag_tools --bin merge_dialog_dag -- \
  dialog-dag.json continuation.json
```

File shape (simplified):

- `conversation_id`: UUID for the overall dialog
- `events[]`: array of nodes
  - `cid`: CIDv1 (base32) computed from `content`
  - `content`: `{ event_id, type, user_said, i_understood, what_i_did[], parent_cid, timestamp }`
- `total_events`: count of events

Note: These tools are optional and live outside the library boundary; they perform local filesystem updates only.

## Development

### Building

```bash
cargo build
```

### Testing

```bash
# Run all tests (hermetic; no external services)
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test suites
cargo test --lib

# Run benchmarks
cargo bench

# Generate test results for CI/CD dashboard
cargo test -- --format json > test-results.json
```

### Running Examples

```bash
# Simple example demonstrating core functionality
cargo run --example simple_example

# Query handler example
cargo run --example query_handler_example
```

## Current Status

**Library Status**: ✅ Complete and functional (pure library)
- **All tests passing**
- **Zero compilation warnings**
- **Production ready** for composing domains

**Test Coverage**: ✅ Comprehensive
- Hermetic unit and integration tests (no external services)

Infrastructure concerns (persistence, routing, transport) are implemented downstream.

Infrastructure belongs in separate crates; this library provides domain constructs only.

**CI/CD**: ✅ Complete
- GitHub Actions workflow for continuous integration
- Test coverage reporting
- Clippy and formatting checks
- Test results capture for dashboard reporting

## Documentation

- [User Stories and Acceptance Tests](doc/qa/cim-domain-user-stories.md)
- [QA Report](doc/qa/cim-domain-qa-report.md)
- [Component Architecture](doc/design/component-architecture.md)
- [Domain Design Principles](doc/design/domain-design-principles.md)
- [Test Infrastructure Guide](doc/testing/test-infrastructure.md)
 - [Saga Principle: Aggregate-of-Aggregates](doc/architecture/sagas.md)
 - [Transaction State Machine (Mealy)](doc/architecture/transaction_state_machine.md)
 - [Serialization & JSON Schemas (Primitives)](docs/SERIALIZATION_AND_SCHEMAS.md)

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
// Infrastructure crates are not part of this library

### External
- `tokio` - Async runtime
- `serde` - Serialization
- `async-trait` - Async traits
- `thiserror` - Error handling
- `uuid` - Entity IDs
- `chrono` - Timestamps

## License

This project is licensed under the MIT License:

- MIT license ([LICENSE-MIT](LICENSE-MIT))

## Working with this Module

When making changes:
1. **Run tests** - `cargo test`
2. **Check benchmarks** - `cargo bench`
3. **Update docs** - `cargo doc --open`
4. **Run examples** - `cargo run --example <name>` (pure examples only)
5. **Verify downstream** - Test dependent infrastructure crates

This module is the foundation of CIM - treat it with appropriate care and rigor.
### Canonical Value Objects (Invariants)

- `PhysicalAddress`: street, locality, region, optional subregion, country, postal code — treated as a single invariant value.
- `Temperature`: numeric value with a `TemperatureScale` (C/F/K). A number without a scale is not meaningful in domain terms.

These are immutable `ValueObject`s updated by replacement (e.g., `.with_locality(..)` returns a new value). See unit tests and BDD scenarios in `doc/qa/features/value_objects.feature`.
