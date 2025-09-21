// Copyright (c) 2025 - Cowboy AI, LLC.

use std::collections::BTreeMap;
use std::fs;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Morphism {
    id: String,
    source: String,
    target: String,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Debug, Deserialize)]
struct Diagram {
    id: String,
    describes: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
struct Category {
    morphisms: Vec<Morphism>,
    diagrams: Vec<Diagram>,
}
#[derive(Debug, Deserialize)]
struct Graph {
    category: Category,
}

fn phrase(id: &str) -> &'static str {
    match id {
        // Concept graph
        "refines_concept" => "refines concept",
        "is_partially_equivalent_to" => "is partially equivalent to",
        // Ownership/containment/scope
        "owns_value_collection" => "owns value collection",
        "scopes_aggregate" => "scopes aggregate",
        "scopes_projection" => "scopes projection",
        "scopes_read_model" => "scopes read model",
        "scopes_event_stream" => "scopes event stream",
        "scopes_command" => "scopes command",
        "scopes_query" => "scopes query",
        "scopes_policy" => "scopes policy",
        "scopes_state_machine" => "scopes state machine",
        "scopes_saga" => "scopes saga",
        // Envelopes/identity
        "identified_by_command_id" => "is identified by command id",
        "identified_by_query_id" => "is identified by query id",
        "encloses_command" => "encloses command",
        "encloses_query" => "encloses query",
        "command_carries_identity" => "carries identity",
        "query_carries_identity" => "carries identity",
        "provides_correlation_id" => "provides correlation id",
        "provides_causation_id" => "provides causation id",
        "provides_command_message_id" => "provides command message id",
        "provides_query_message_id" => "provides query message id",
        "provides_event_id" => "provides event id",
        "identifies_event" => "identifies event",
        "identifies_aggregate" => "identifies aggregate",
        "correlates_with" => "correlates with",
        "was_caused_by" => "was caused by",
        "describes_payload" => "describes payload",
        // Addressing
        "uses_payload_codec" => "uses payload codec",
        "payload_is" => "payload is",
        "annotated_by_metadata" => "is annotated by metadata",
        "domain_cid_defines_node" => "defines node",
        // Streams and pipeline
        "collects_envelope" => "collects envelope",
        "command_correlates_to_event" => "correlates to event",
        "query_correlates_to_event" => "correlates to event",
        "precedes_envelope" => "precedes",
        // Core flow verbs (kept as-is)
        "handled_by" => "is handled by",
        "emits_event" => "emits event",
        "governed_by" => "is governed by",
        "constrained_by_policy" => "is constrained by policy",
        "updates_read_model" => "updates read model",
        "consumes_event" => "consumes event",
        "subscribes_to_stream" => "subscribes to stream",
        "appended_to_stream" => "is appended to stream",
        "wraps_event" => "wraps event",
        "reads_from" => "reads from",
        "responds_with" => "responds with",
        "coordinates" => "coordinates",
        "causes_event" => "causes event",
        "manages_participant" => "manages participant",
        "maintains_vector_clock" => "maintains vector clock",
        // Fallback
        _ => "relates to",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "domain-graph.json".to_string());
    let raw = fs::read_to_string(&path)?;
    let g: Graph = serde_json::from_str(&raw)?;

    // Build index for quick lookup
    let mut by_id: BTreeMap<String, &Morphism> = BTreeMap::new();
    for m in &g.category.morphisms {
        by_id.insert(m.id.clone(), m);
    }

    // Emit grouped by diagram if available
    println!("# UL Narrative\n");
    for d in &g.category.diagrams {
        if let Some(describes) = &d.describes {
            let diag_id = &d.id;
            println!("## Diagram: {diag_id}");
            for id in describes {
                if let Some(m) = by_id.get(id) {
                    let verb = phrase(&m.id);
                    let source = &m.source;
                    let target = &m.target;
                    println!("- {source} {verb} {target}");
                }
            }
            println!();
        }
    }

    // Also print any morphisms not covered by a diagram
    let covered: std::collections::HashSet<&str> = g
        .category
        .diagrams
        .iter()
        .flat_map(|d| {
            d.describes
                .as_ref()
                .map(|v| v.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .unwrap_or_default()
        })
        .collect();
    let mut uncovered: Vec<&Morphism> = g
        .category
        .morphisms
        .iter()
        .filter(|m| !covered.contains(m.id.as_str()))
        .collect();
    if !uncovered.is_empty() {
        println!("## Not Covered by Diagrams:");
        uncovered.sort_by_key(|m| m.id.clone());
        for m in uncovered {
            let verb = phrase(&m.id);
            let source = &m.source;
            let target = &m.target;
            let kind = &m.kind;
            println!("- {source} {verb} {target} (type: {kind})");
        }
    }

    Ok(())
}
