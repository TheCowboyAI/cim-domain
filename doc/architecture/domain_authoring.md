<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Authoring a CIM Domain

This playbook captures the exact steps we follow when turning a domain idea into working code using `cim-domain`.

## 1. Start from the Lens

1. Edit `domain-graph.json` and add or adjust objects and morphisms.
2. Run the guard:
   ```bash
   cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin validate_domain_graph
   ```
   Fix any missing diagrams before proceeding.
3. Regenerate the UL projection when identities change:
   ```bash
   cargo run --manifest-path tools/domain_graph/Cargo.toml --quiet --bin ul_projection
   ```

## 2. Lay Out Addressing

- Model command/event/read model identifiers with `DomainPath` helpers.  
- Assign subject routes via `Subject` and `SubjectPattern`.  
- Add feature entries to `doc/qa/features/index.yaml` upfront so QA gates know what to expect.

## 3. Define Aggregates & Entities

- Build aggregates using `AggregateRoot` and `DomainEntity` with phantom typed IDs.
- Encode invariants via the `Invariant` trait and assert them in unit tests (see `tests/domain_invariant_tests.rs`).
- Document the new aggregate in `doc/ul/domain_objects_catalog.md` if it becomes part of the UL.

## 4. Command & Query Handlers

1. Model commands and queries with the re-exported types from `cim_domain::cqrs`.
2. Implement handlers using `CommandHandler` / `QueryHandler`. They should return acknowledgments and use `DomainEventEnvelope` for emitted events.
3. If handlers depend on content addressing, use the in-memory helpers in `cid` and `object_store`—leave persistence for infrastructure crates.

## 5. State Machines

- Use `MealyStateTransitions` for behavioural logic that emits events.  
- Follow the `TransactionState` example: deterministic outputs plus tests and BDD coverage.  
- Update `doc/act/string_diagrams.md` with the commutation law and add a test in `tests/act_*.rs`.

## 6. Prove Behaviour

- Unit tests next to implementation.  
- Integration tests in `tests/` mapped in `doc/qa/features/index.yaml`.  
- BDD scenarios in `examples/domain_examples/tests/` when you need to demonstrate end-to-end flows.

Always rerun and commit the generated test report:
```bash
scripts/update_test_report.sh
```

## 7. Document the Outcome

- Summarise the new behaviour or pattern in `doc/architecture/design-patterns.md`.  
- When a diagram changes, capture the law in `doc/act/string_diagrams.md` and regenerate any SVGs if needed.  
- Update the QA feature index and the UL catalogue so downstream teams see the same vocabulary.

## 8. Ready for Release Checklist

- [ ] `cargo fmt` / `cargo clippy -- -D warnings`  
- [ ] `cargo test --all-targets --all-features`  
- [ ] `cargo test --features act_strict -- tests::act`  
- [ ] BDD suite(s)  
- [ ] `doc/testing/test_report.md` regenerated  
- [ ] QA feature mappings updated  
- [ ] Diagrams validated (`validate_domain_graph`)

Following these steps keeps the code, the diagrams, and the documentation aligned—and lets downstream teams consume the domain without surprises.
