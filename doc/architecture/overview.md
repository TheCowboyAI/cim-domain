<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# CIM Domain Architecture Overview

## Purpose

`cim-domain` is the production library that hosts the domain layer for the Composable Information Machine (CIM). It ships only the building blocks that every CIM domain shares:

- category‑theoretic lens (objects, morphisms, commutation tests)
- subject and domain path algebras for addressing
- core DDD primitives (entities, aggregates, invariants, policies)
- command/query/transaction orchestration with event envelopes
- vector clock and content addressing helpers that remain pure

All transport, storage, or UI concerns live downstream. If a behaviour touches a network, a database, or a UI, it does **not** belong here.

## How the Pieces Fit Together

| Capability | Modules | Highlights |
| --- | --- | --- |
| Domain identity & invariants | `entity`, `domain::value_objects`, `errors` | Phantom‑typed IDs, aggregate roots, invariant checks feed structured `DomainError`s |
| Command & query flow | `cqrs`, `command_handlers`, `query_handlers`, `events` | Pure handlers return acknowledgments, emit `DomainEventEnvelope<E>` with inline or CID payloads |
| Event addressing & routing lens | `subject`, `domain_path`, `ul_classifier`, `ontology_quality` | Free monoids for subject routing, canonical UL paths, quality vectors |
| State machines & orchestration | `state_machine`, `transaction_state`, `saga`, `vector_clock` | Explicit `MealyStateTransitions`, composable sagas, vector clocks for participant ordering |
| Content addressing helpers | `cid`, `object_store` | Deterministic CID generation, bucket logs, index helpers; no persistence performed here |
| Formal domain lens | `category`, `composition`, `formal_domain`, `concept_naming` | Category objects + morphisms kept in sync with `domain-graph.json`; commutation verified in tests |

The library deliberately exposes primitives rather than “finished” application layers. A production domain pulls these modules together, adds domain types in a separate crate, and wires infrastructure around them.

## Domain Authoring Workflow

1. **Model the domain graph**  
   - edit `domain-graph.json` to declare objects, morphisms, and UL surfaces  
   - run `cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin validate_domain_graph` to ensure every morphism is covered by a diagram and the lens commutes  
   - generate UL projection when the graph changes: `cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin ul_projection`

2. **Implement domain types**  
   - create aggregates with `AggregateRoot`/`DomainEntity`
   - encode invariants via `Invariant` implementations and unit tests
   - use `Subject` / `DomainPath` to identify commands, events, read models
   - use `TransactionState` (or your own `MealyStateTransitions`) for lifecycle control

3. **Hook up command/query pipelines**  
   - write handlers using `CommandHandler`/`QueryHandler` traits  
   - return `CommandAcknowledgment` / `QueryResponse` and emit `DomainEventEnvelope`

4. **Document and prove**  
   - update `doc/act/string_diagrams.md` when new diagrams are added  
   - create commutation tests alongside diagrams (see `tests/act_diagram_commutation_tests.rs`)

5. **Quality gates**  
   - `cargo test --all-targets --all-features` (unit, integration, BDD, ACT)  
   - `cargo test --features act_strict -- tests::act` (diagram coverage)  
   - `cargo test -p cim-domain-examples --test bdd_transaction_state` (BDD regression for the sample domain)  
   - `doc/testing/test_report.md` is regenerated from the same `cargo test -- --list` command and must remain current

## Using the Library in a Domain Crate

1. Add `cim-domain` as a dependency.  
2. Define your aggregate boundary using the types re‑exported from `lib.rs`: `AggregateRoot`, `DomainEvent`, `Command`, `Query`, etc.  
3. Pick addressing strategies:  
   ```rust
   use cim_domain::{DomainPath, Subject};
   let command_path = DomainPath::command("billing", "authorize");
   let routing_subject = Subject::from_str("billing.authorize.v1").unwrap();
   ```
4. Implement lifecycle logic with `MealyStateTransitions`. `TransactionState` is a production example you can adapt or extend.  
5. Emit deterministic events: `DomainEventEnvelope::inline` ensures payload metadata and identity travel together. You can switch to CID references later with `with_payload_cid` once infrastructure persists the payloads.
6. Hook projections/read models by implementing `DirectQueryHandler` or a custom `QueryHandler` over your own read storage.

## Tooling at a Glance

| Tool | Location | Purpose |
| --- | --- | --- |
| `validate_domain_graph` | `tools/domain_graph` | Ensures every morphism has a verified diagram and domain identity lines up |
| `ul_projection` | `tools/domain_graph` | Projects the UL vocabulary (`ul-projection.json`) from the domain graph |
| `log_dialog_event` / `log_insight` | `tools/dialog_dag` | Maintains conversational memory (`dialog-dag.json`) with a reproducible CLI |
| ACT tests | `tests/act_*` | Protect diagram coverage, naming policies, and UL alignment |
| BDD harness | `examples/domain_examples/tests/bdd_transaction_state.rs` | Demonstrates how to assert full event streams for state machines |

## What Is **Not** in This Crate

- persistence adapters, repositories, or event stores
- transport, messaging, or subject routing infrastructure
- UI, REST, or gRPC layers
- domain implementations for specific verticals (live in separate crates)

Keeping the library pure makes it easy to reuse across domains and to maintain deterministic, fast test suites.

## Where to Go Next

- **`doc/architecture/state-machine-design.md`** – deep dive into enum‑based state machines using the production transaction state example
- **`doc/architecture/design-patterns.md`** – catalogue of reusable DDD/ACT patterns in this codebase
- **`doc/development/testing.md`** – how we keep tests reproducible and fast, with ACT and BDD gates explained
- **`doc/qa/features/index.yaml`** – source of truth for feature → test/assets mappings
- **`doc/testing/test_report.md`** – up-to-date test inventory regenerated from the latest run
