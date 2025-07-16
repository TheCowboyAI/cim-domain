# CIM Domain

Core Domain-Driven Design (DDD) components and traits for the Composable Information Machine (CIM).

## Overview

This crate provides the fundamental building blocks for implementing DDD patterns:

- **Component**: Trait for attachable components with type erasure
- **Entity**: Types with identity and lifecycle  
- **Value Objects**: Immutable types defined by their attributes
- **Aggregates**: Consistency boundaries with root entities
- **Domain Events**: Things that happen in the domain
- **Commands**: Requests to change state (return only acknowledgments)
- **Queries**: Requests to read state (return only acknowledgments)
- **State Machines**: Enum-based state management with controlled transitions

## Features

- Event-driven architecture with CQRS pattern
- Content-addressed events with CID chains
- Async event streams using NATS JetStream
- State machine abstractions (Moore and Mealy machines)
- Component system for extensible domain objects
- Full test coverage with examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cim-domain = "0.3.0"
```

## Usage

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

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Examples

```bash
# Basic CQRS pattern demo
cargo run --example cqrs_pattern_demo

# State machine demo
cargo run --example state_machine_demo

# Event sourcing demo
cargo run --example full_event_sourcing_demo
```

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.