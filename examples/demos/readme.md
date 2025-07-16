# CIM Domain Demos

This directory contains comprehensive demonstrations of the CIM Domain library's capabilities. Each demo showcases different aspects of the event-sourced, domain-driven architecture.

## Prerequisites

Before running the demos, ensure you have:

1. **NATS Server** running locally on port 4222
   ```bash
   # Using Docker
   docker run -p 4222:4222 -p 8222:8222 nats:latest -js

   # Or using nats-server directly
   nats-server -js
   ```

2. **Rust toolchain** installed (1.75+ recommended)

3. **Build the library** first:
   ```bash
   cd cim-domain
   cargo build
   ```

## Available Demos

### 1. Full Event Sourcing Demo
**File:** `full_event_sourcing_demo.rs`

Demonstrates complete event sourcing workflow with NATS JetStream integration.

**Features:**
- Event store with CID chain integrity
- Command processing and event generation
- Event replay and projection building
- Real-time statistics and monitoring
- Cryptographic verification of event chains

**Run:**
```bash
cargo run --example full_event_sourcing_demo
```

**What you'll see:**
- People, organizations, agents, locations, and policies being created
- Events stored with CID chains for integrity
- Projections built from events
- Complete replay of all events from scratch
- Statistics showing event processing performance

### 2. State Machine Demo
**File:** `state_machine_demo.rs`

Showcases state machine transitions for all domain aggregates.

**Features:**
- Agent lifecycle (Deploy → Active → Suspended → Offline → Decommissioned)
- Policy approval workflow (Draft → Submitted → Approved/Rejected)
- Document processing pipeline (Created → Uploaded → Classified → Processed → Archived)
- Workflow execution states (Created → Running → Completed/Failed)
- Concurrent state transitions

**Run:**
```bash
cargo run --example state_machine_demo
```

**What you'll see:**
- Valid state transitions being executed
- Invalid transitions being rejected
- Business rules enforcement
- Concurrent state machine operations
- Available transitions from each state

### 3. CQRS Pattern Demo
**File:** `cqrs_pattern_demo.rs`

Demonstrates Command Query Responsibility Segregation patterns.

**Features:**
- Command handlers with validation
- Query handlers with optimized read models
- Write model and read model separation
- Eventual consistency demonstration
- Complex query patterns

**Run:**
```bash
cargo run --example cqrs_pattern_demo
```

**What you'll see:**
- Commands being validated and processed
- Events generated from commands
- Read models updated from events
- Complex queries across aggregates
- Eventual consistency in action

## Demo Architecture

All demos follow the CIM architecture principles:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Commands      │────▶│ Command Handler │────▶│     Events      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                          │
                                                          ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    Queries      │◀────│ Query Handler   │◀────│  Projections    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Key Concepts Demonstrated

### 1. Event Sourcing
- All state changes are captured as events
- Events are immutable and append-only
- State can be reconstructed by replaying events
- CID chains ensure cryptographic integrity

### 2. Domain-Driven Design
- Aggregates enforce business invariants
- Value objects ensure data integrity
- Domain events represent business occurrences
- Ubiquitous language throughout

### 3. CQRS (Command Query Responsibility Segregation)
- Commands modify state through aggregates
- Queries read from optimized projections
- Write and read models are separate
- Eventual consistency between models

### 4. State Machines
- Clear state transitions for business processes
- Invalid transitions are prevented
- Business rules enforced at transition time
- Visual representation of allowed states

## Running All Demos

To run all demos in sequence:

```bash
# Run each demo individually
cargo run --example full_event_sourcing_demo
cargo run --example state_machine_demo
cargo run --example cqrs_pattern_demo
```

## Troubleshooting

### NATS Connection Error
If you see "Connection refused" errors:
1. Ensure NATS server is running: `docker ps | grep nats`
2. Check the port: `netstat -an | grep 4222`
3. Try connecting manually: `nats pub test "hello"`

### Build Errors
If demos fail to build:
1. Update dependencies: `cargo update`
2. Clean build: `cargo clean && cargo build`
3. Check Rust version: `rustc --version` (need 1.75+)

### Runtime Errors
If demos crash during execution:
1. Check NATS JetStream is enabled: `nats-server -js`
2. Ensure sufficient memory for event storage
3. Check logs for specific error messages

## Learning Path

We recommend running the demos in this order:

1. **CQRS Pattern Demo** - Understand the basic architecture
2. **State Machine Demo** - See how aggregates manage state
3. **Full Event Sourcing Demo** - Experience the complete system

## Next Steps

After running these demos, you can:

1. **Modify the demos** - Try adding new commands or queries
2. **Build your own** - Create a demo for your specific use case
3. **Integrate with Bevy** - See the main project for ECS integration
4. **Explore the tests** - Look at unit and integration tests for more examples

## Contributing

If you create a useful demo, please consider contributing it back:

1. Follow the existing demo patterns
2. Include comprehensive comments
3. Add error handling and validation
4. Update this README with your demo

## Resources

- [CIM Documentation](../../../doc/)
- [Domain Model](../../src/)
- [Integration Tests](../../tests/)
- [NATS Documentation](https://docs.nats.io/)

---

Happy exploring! These demos showcase the power and flexibility of event-sourced, domain-driven systems with CIM.
