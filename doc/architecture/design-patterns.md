<!-- Copyright 2025 Cowboy AI, LLC. -->

# Proven Patterns in `cim-domain`

This catalogue records the patterns that the production code base already exercises. Copy them when you build a new domain; extend them only when a new use case demands it.

## 1. Phantom‑Typed Identity

```rust
use cim_domain::EntityId;

struct LedgerMarker;
let ledger_id: EntityId<LedgerMarker> = EntityId::new();
assert_ne!(ledger_id, EntityId::<LedgerMarker>::new());
```

Each aggregate/entity/value object uses a dedicated marker type so IDs cannot be mixed accidentally.

## 2. Deterministic Domain Events

```rust
use cim_domain::{DomainEvent, DomainEventEnvelope, PayloadMetadata, cid::{generate_cid, ContentType}};
use cim_domain::cqrs::{MessageFactory, CommandEnvelope, CommandAcknowledgment, QueryEnvelope};

#[derive(Debug)]
struct LedgerOpened { ledger: EntityId<LedgerMarker> }
impl DomainEvent for LedgerOpened {
    fn aggregate_id(&self) -> uuid::Uuid { self.ledger.into_inner() }
    fn event_type(&self) -> &'static str { "LedgerOpened" }
}

let meta = PayloadMetadata { source: "ledger", version: "v1".into(), properties: Default::default(), payload_type: "".into() };
let env = DomainEventEnvelope::inline(
    MessageFactory::new_event_id(),
    LedgerOpened { ledger: EntityId::new() },
    MessageFactory::new_correlation_id(),
    MessageFactory::new_causation_id(),
    meta,
);
let cid = generate_cid(&[1,2,3], ContentType::Event).unwrap();
let persisted = env.with_payload_cid(cid);
assert!(persisted.payload_cid().is_some());
```

## 3. Subject & Domain Path Algebra

```rust
use cim_domain::{Subject, DomainPath, DomainArtifactKind};

let subject = Subject::from_str("billing.authorize.v1").unwrap();
let command_path = DomainPath::command("billing", "authorize").unwrap();
assert_eq!(command_path.artifact_kind(), Some(DomainArtifactKind::Command));
```

`Subject` and `DomainPath` form free monoids. The commutation tests in `tests/act_diagram_commutation_tests.rs` guarantee associativity and identity.

## 4. Transaction State Machine (Mealy)

```rust
use cim_domain::{TransactionState, TransactionInput};

let mut world = TransactionState::Idle;
let next = world.valid_transitions(&TransactionInput::Start);
assert_eq!(next, vec![TransactionState::Started]);
```

`TransactionState::transition_output` emits typed events. The BDD harness asserts the entire event stream for each scenario, guaranteeing coverage of every transition.

## 5. Vector Clock Coordination

```rust
use cim_domain::vector_clock::{VectorClock, ActorId};

let mut clock = VectorClock::new();
clock.increment(ActorId::new("billing"));
clock.increment(ActorId::new("payments"));
assert!(clock.partial_cmp(&clock).is_some());
```

Vector clocks underpin saga orchestration. They provide deterministic ordering without relying on wall‑clock time.

## 6. Category Lens Verifications

```rust
// tests/act_category_graph_tests.rs
#[test]
fn composition_rules_include_ddd_and_topos_laws() {
    assert!(composition_rules.contains("fold ∘ decide ∘ handle = transition"));
}
```

Every morphism defined in `domain-graph.json` must have a verified diagram and a corresponding test. `cargo test --features act_strict -- tests::act` fails if this alignment breaks.

## 7. Feature Mapping Guard

`tests/act_feature_mapping_strict.rs` keeps the QA matrix honest: every integration test must be mapped to a feature, and every feature must list at least one TDD asset. Update `doc/qa/features/index.yaml` whenever you add a new test or diagram.

## 8. Content Addressing Buckets

```rust
use cim_domain::object_store::{BucketLog, BucketRootKind, BucketEntry};
use cim_domain::cid::{generate_cid, ContentType};

let mut bucket = BucketLog::new(BucketRootKind::Events);
let cid = generate_cid(&[1,2,3], ContentType::Event).unwrap();
let entry = bucket.append(cid, None);
assert_eq!(entry.sequence, 1);
```

The object store types stay in memory; persistence is implemented downstream.

---

Keep the documentation in sync with the code. When you introduce a new pattern, prove it with a test and describe it here using real APIs.

