<!-- Copyright 2025 Cowboy AI, LLC. -->

# Core Aggregates (Architecture)

This document is domain‑neutral. It describes how aggregates are modeled in the CIM Domain core library without any contrived domain examples.

## Aggregate boundary

- AggregateRoot: the consistency boundary; exposes command handlers and emits domain events.
- DomainEntity: entities inside the boundary; identified by typed `EntityId<T>`.
- ValueObject: immutable values that compose entities and invariants.
- Invariants: validated within the boundary before events are emitted.
- State machine: governs valid transitions of aggregate state.

## Minimal structure (generic)

```rust
use cim_domain::{AggregateRoot, DomainEvent, EntityId};

// Aggregate marker
struct A;

#[derive(Debug, Clone)]
struct Created { id: EntityId<A> }
impl DomainEvent for Created {
    fn aggregate_id(&self) -> uuid::Uuid { *self.id.as_uuid() }
    fn event_type(&self) -> &'static str { "Created" }
}

struct ARoot {
    id: EntityId<A>,
}
impl ARoot {
    fn handle_create(&mut self) -> Vec<Box<dyn DomainEvent>> {
        vec![Box::new(Created { id: self.id })]
    }
}
```

## Architectural rules

- Commands acknowledge (Accepted/Rejected); results are events.
- Events are wrapped by `DomainEventEnvelope` with payload `Either<DomainCid, E>` and appended to an `EventStream`.
- Projections subscribe to streams and update `ReadModel`s.
- Queries read from `ReadModel`s and return a `QueryResponse`.
- Transport/persistence live downstream; this crate stays pure.

## Recommended layout

- Group ValueObjects with their DomainEntities.
- Keep the AggregateRoot small; prefer pure functions over ValueObjects.
- Use state machines to encode allowed transitions.
- Validate invariants at command handling boundary.

## Related components

- Saga: coordinates multiple aggregates and maintains a `VectorClock` for causal ordering.
- BoundedContext: logical container that owns aggregates, projections, read models, and streams.

This document intentionally avoids domain examples. Concrete aggregates belong downstream, not in this core library.

