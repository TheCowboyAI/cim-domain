# Domain Objects — Incremental TODOs (Proof‑First)

Guiding loop: target one thing → make it work → improve it → prove it.

## Priority 1 — Envelopes & Identity
- Prove correlation/causation flow:
  - Test: CommandEnvelope.identity propagates to DomainEventEnvelope (correlation id preserved; causation id references the causing command).
  - Diagram: identity_envelope.svg covers correlates_to (done). Ensure code examples/docstrings align.
- Prove EventId ordering:
  - Property test: sequential EventId::new() is non‑decreasing.
  - Note: UUID v7 monotonicity guarantees depend on clock; test accordingly (allow equality within same tick).
- Prove DomainEventEnvelope fields:
  - Unit tests asserting presence of event_id, aggregate_id, correlation_id, causation_id, payload_metadata; with_payload_cid swaps inline → CID.

## Priority 2 — Content Addressing
- Prove DomainNode semantics:
  - Test: payload codec annotation retained; root CID changes when metadata changes; payload_is references IPLD.Cid.
  - Diagram: addressing.svg (done).
- Prove CID chain helper (if used):
  - Test: previous matches prior current; genesis detection.

## Priority 3 — Read Path
- Prove Projection contract:
  - Async tests for handle_event increments state and clear resets.
- Prove Query path:
  - Unit tests for QueryResponse; optional example showing reads_from/responds_with lifecycle (pure, no IO).

## Priority 4 — Collections
- Prove ValueCollection monoid laws:
  - Property tests (Vec concat, BTreeSet union) — done in tests/value_collection_monoid_tests.rs.
  - Add doc cross‑refs from catalog to tests.

## Priority 5 — Lens Mapping & UL
- Document mapping: StateMachine (lens) ↔ MealyStateMachine (code) in catalog and act rules.
- UL projection:
  - Generate diff (done). On approval, write updates to include new objects (Policy, Saga, ValueCollection, etc.).

## Governance
- Keep code authoritative for now; propose graph/doc edits as PRs referencing tests/diagrams.
- Log one consolidated dialog event per iteration and update insights when policy changes.

