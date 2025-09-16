<!-- Copyright 2025 Cowboy AI, LLC. -->

# Core Functional & Reactive Patterns (Domain‑Neutral)

This document outlines the core patterns used in the CIM Domain core library without domain‑specific examples. Infrastructure (transport/persistence) and concrete domains live downstream.

## Principles

- Event‑first, pure functions; immutable values.
- Make invalid states unrepresentable (enums/newtypes).
- Explicit state machines for allowed transitions.
- Effects at boundaries via traits; implementations downstream.

## Identity & Types

```rust
use cim_domain::EntityId;
struct A;
let id: EntityId<A> = EntityId::new();
```

## Events & Envelopes

```rust
use cim_domain::{DomainEvent, DomainEventEnvelope, PayloadMetadata, Either, DomainCid};

#[derive(Debug, Clone)]
struct Created { /* fields */ }
impl DomainEvent for Created {
    fn aggregate_id(&self) -> uuid::Uuid { /* ... */ unimplemented!() }
    fn event_type(&self) -> &'static str { "Created" }
}

// Inline → ByCid
let meta = PayloadMetadata { source: "core".into(), version: "v1".into(), properties: Default::default() };
let env = DomainEventEnvelope::inline(Default::default(), Created { /* .. */ }, Default::default(), Default::default(), meta);
let cid: DomainCid = cim_domain::generate_cid(&42u8, cim_domain::ContentType::Event).unwrap();
let env = env.with_payload_cid(cid);
assert!(env.payload_cid().is_some());
```

## State Machines (generic)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum Lifecycle { Init, Active, Closed }
// Implement cim_domain::state_machine::{State, StateTransitions}
```

## Projections & Read Models

- Projections subscribe to event streams and update read models.
- Queries read from read models and return `QueryResponse`.

## Sagas

- Coordinate multiple aggregates; maintain `VectorClock` for causal ordering across participants.

This document is intentionally concise and domain‑neutral to keep the core library focused and pure.

