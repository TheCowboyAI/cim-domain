<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# Serialization & JSON Schemas (Primitives)

## Principle

- The domain supports serde serialization for all core types.
- JSON Schemas are exposed at the level of primitives (shape-only), while population rules and format constraints are enforced by the full domain objects at runtime.
- No registry/export tooling lives here; downstream generates/export schemas ad‑hoc when needed.

## Primitives with Schemas

- `EntityId<T>` → string UUID (phantom type not encoded)
- `DomainCid` → object with `cid` (string), `content_type` (enum), `context` (optional)
- `IdType` → `Uuid` (message identities are UUID-only)
- `AggregateTransactionId` → `Uuid` (provides correlation IDs)
- `CorrelationId`, `CausationId` → wrappers over `Uuid`
- `VectorClock` → object `{ counters: { actor: integer } }`

Note: Higher‑level aggregates and envelopes can derive schemas, but only the primitives are relied upon for cross‑boundary contracts in this crate. Business rules are validated by domain logic, not the schema.

## How To Generate a Schema (ad‑hoc)

Use `schemars` directly where you need it (e.g., in an app/binary or tests). CIDs are used for payloads only, never for message identities:

```rust
use schemars::schema_for;
use serde_json::to_string_pretty;
use cim_domain::{DomainCid, VectorClock, CorrelationId};

fn main() {
    let cid_schema = schema_for!(DomainCid);
    println!("{}", to_string_pretty(&cid_schema).unwrap());

    let vc_schema = schema_for!(VectorClock);
    println!("{}", to_string_pretty(&vc_schema).unwrap());

    let corr_schema = schema_for!(CorrelationId);
    println!("{}", to_string_pretty(&corr_schema).unwrap());
}
```

This keeps the domain pure and lightweight, while enabling consumers to materialize schemas on demand.
