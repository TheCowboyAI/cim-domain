// Copyright (c) 2025 - Cowboy AI, LLC.

// Verifies that key UL-aligned objects are exported at the crate root
use cim_domain as cd;

#[test]
fn code_exports_match_ul_key_objects() {
    // CQRS envelopes and identity
    let _c: Option<cd::CommandEnvelope<()>> = None;
    let _q: Option<cd::QueryEnvelope<()>> = None;
    // Events
    let _eid: cd::EventId = cd::EventId::new();
    // Read path types
    let _qr = cd::QueryResponse {
        query_id: cd::IdType::nil(),
        correlation_id: cd::CorrelationId::Single(cd::IdType::nil()),
        result: serde_json::json!({}),
    };
    // Projection trait is exported in src/projections/mod.rs tests, but type lives in module
}
