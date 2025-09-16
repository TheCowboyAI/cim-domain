// Copyright 2025 Cowboy AI, LLC.

//! UL Classifier — map domain objects to core CIM concepts (heuristics)
//!
//! This module provides a pure, deterministic heuristic mapping from domain
//! graph objects (name, type) into sets of core concepts. It is intentionally
//! conservative and meant as a starting point for UL development.

use std::collections::BTreeSet;

use crate::core_concepts::CoreConceptId as C;

/// Classify a domain object (by type/name) into core concepts
pub fn classify_object(object_type: &str, object_name: &str) -> BTreeSet<C> {
    let t = object_type.to_ascii_lowercase();
    let name = object_name.to_ascii_lowercase();
    let mut r: BTreeSet<C> = BTreeSet::new();

    // Baseline by high-level type
    match t.as_str() {
        "valueobject" => { r.insert(C::Memory); r.insert(C::Schema); }
        "entity" => { r.insert(C::Memory); r.insert(C::Schema); r.insert(C::CognitiveDevelopment); }
        "aggregate" => { r.insert(C::ProblemSolving); r.insert(C::DecisionMaking); r.insert(C::Schema); }
        "event" => { r.insert(C::Memory); r.insert(C::Language); }
        "command" => { r.insert(C::Attention); r.insert(C::DecisionMaking); r.insert(C::Language); }
        "concept" => { r.insert(C::Schema); r.insert(C::Language); r.insert(C::Metacognition); }
        "projection" | "readmodel" => { r.insert(C::Memory); r.insert(C::Language); r.insert(C::Schema); }
        "context" => { r.insert(C::Schema); r.insert(C::Language); }
        _ => {}
    }

    // Name-specific tweaks that align to UL (no domain-specifics like Money)
    if name.contains("state_machine") || name == "statemachine" { r.insert(C::ProblemSolving); }
    if name.contains("policy") { r.insert(C::DecisionMaking); r.insert(C::Metacognition); }
    if name.contains("event_stream") { r.insert(C::Memory); r.insert(C::Language); }
    if name.contains("boundedcontext") || name.contains("bounded_context") { r.insert(C::Schema); r.insert(C::Language); }
    if name == "entityid<t>" || name == "queryresponse" { r.insert(C::Memory); r.insert(C::Schema); }
    if name == "conceptgraph" { r.insert(C::Schema); r.insert(C::Language); }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_value_objects_to_memory_schema() {
        let c = classify_object("ValueObject", "Money");
        assert!(c.contains(&C::Memory) && c.contains(&C::Schema));
    }

    #[test]
    fn classify_command_to_attention_and_decision() {
        let c = classify_object("Command", "Command");
        assert!(c.contains(&C::Attention) && c.contains(&C::DecisionMaking));
    }

    #[test]
    fn classify_events_to_memory_language() {
        let c = classify_object("Event", "DomainEvent");
        assert!(c.contains(&C::Memory) && c.contains(&C::Language));
    }
}
