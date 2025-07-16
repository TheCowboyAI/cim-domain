# CIM Domain

Core Domain-Driven Design (DDD) components for the Composable Information Machine (CIM).

## Overview

The `cim-domain` crate provides the fundamental building blocks for implementing Domain-Driven Design patterns in any CIM implementation. It defines the core entities that are essential for modeling information flows and organizational structures.

## Core Entities

### 1. **People** - Human actors with identity and decision-making capabilities
### 2. **Agents** - Automated actors that execute tasks within bounded capabilities
### 3. **Organizations** - Collective entities that group people and agents
### 4. **Locations** - Physical or logical spaces where activities occur
### 5. **Policies** - Governance rules that control system behavior

## Design Principles

- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Immutability**: Value objects are immutable by design
- **Event Sourcing**: All state changes are captured as domain events
- **Domain Alignment**: Types reflect business concepts, not technical details
- **Composability**: Build complex systems from simple, well-defined components

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
cim-domain = { path = "../cim-domain" }
```

Basic example:

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

## Architecture

The crate follows a modular structure where each core entity has:

- **Aggregate**: The main entity with business logic
- **Commands**: Requests to change state
- **Events**: Records of state changes
- **Value Objects**: Immutable domain concepts
- **Tests**: Comprehensive test coverage

## Documentation

- [User Stories and Acceptance Tests](doc/qa/cim-domain-user-stories.md)
- [Implementation Plan](doc/plan/core-entities-implementation.md)
- [Progress Tracking](doc/progress/progress.json)

## Development Status

Currently implementing Phase 1: People entity with identity management. See [progress.json](doc/progress/progress.json) for detailed status.

## Contributing

This is a foundational crate for CIM implementations. All changes must:

1. Maintain backward compatibility AFTER v0.5.0 (currently v0.1.0)
2. Include comprehensive tests
3. Follow DDD principles
4. Update documentation
5. Pass all quality checks

## License

Part of the CIM ecosystem.

## Status

**Library Status**: ✅ Complete and functional
- Core library builds successfully
- All 135 unit tests pass
- Provides foundational DDD framework for all other domain modules

**Examples Status**: ⚠️ Needs updating
- `simple_example.rs` - ✅ Working example demonstrating core functionality
- Other examples need to be updated to work with the current API
- Many examples depend on types that have been moved to specific domain modules

**Infrastructure**: ✅ Complete
- Event Store integration with NATS JetStream
- Command/Query handlers with proper CQRS separation
- Bevy ECS bridge for visualization
- Event replay and snapshot capabilities

## Features
