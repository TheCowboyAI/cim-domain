<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Saga Principle: Aggregate-of-Aggregates

## Overview

In CIM, a Saga is modeled exactly like an Aggregate, except its "entities" are other Aggregates. A Saga’s AggregateRoot is itself an Aggregate (the coordinator) and the Saga composes participant Aggregates that may belong to different bounded contexts or domains. This keeps Sagas in the pure domain layer with clear boundaries and no infrastructure concerns.

## Model

- Root: `Participant` holding the root `AggregateId` (and optional domain label)
- Participants: `Vec<Participant>` referencing other Aggregates
- Causality: `VectorClock` for ordering across root and participants
- No time generation: call sites provide any physical time if needed downstream

```rust
use cim_domain::{Saga, Participant, VectorClock, ClockCmp, AggregateId};

let root = Participant { id: AggregateId::new(), domain: Some("domain-a".into()) };
let saga = Saga::new(root)
    .with_participant(Participant { id: AggregateId::new(), domain: Some("domain-b".into()) })
    .with_participant(Participant { id: AggregateId::new(), domain: Some("domain-c".into()) });
```

## Causality & Ordering

Vector clocks provide a partial order between the Saga root and participant Aggregates without wall‑clock time:

- `tick(actor)` produces a new Saga with the vector clock incremented for `actor`
- `merge_clock` merges remote clocks (element‑wise maxima)
- `order(other)` returns `ClockCmp` (Equal | Before | After | Concurrent)

```rust
let s1 = saga.tick("root");
let s2 = saga; // unchanged
assert!(matches!(s1.order(&s2), ClockCmp::After));
```

## FP Boundaries

- Pure data only: no subject/routing, no persistence, no time generation
- Any physical time (e.g., HLC) is consumed as input in downstream crates
- Effects live behind ports/adapters outside the domain

This principle preserves referential transparency, enables deterministic testing, and cleanly separates domain logic from infrastructure.
