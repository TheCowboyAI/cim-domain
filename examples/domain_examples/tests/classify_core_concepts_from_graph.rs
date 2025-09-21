// Copyright (c) 2025 - Cowboy AI, LLC.

use cim_domain::{classify_object, CoreConceptId as C};
use serde::Deserialize;

#[derive(Deserialize)]
struct Obj {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Category {
    objects: Vec<Obj>,
}

#[derive(Deserialize)]
struct Graph {
    category: Category,
}

#[test]
fn classify_objects_in_domain_graph_to_core_concepts() {
    let raw = include_str!("../../../domain-graph.json");
    let g: Graph = serde_json::from_str(raw).expect("parse domain-graph.json");
    // Spot check a few objects
    let mut query_response = None;
    let mut cmd = None;
    let mut event = None;
    let mut read_model = None;
    for o in g.category.objects {
        match o.name.as_str() {
            "QueryResponse" => query_response = Some(o.kind.clone()),
            "Command" => cmd = Some(o.kind.clone()),
            "DomainEvent" => event = Some(o.kind.clone()),
            "ReadModel" => read_model = Some(o.kind.clone()),
            _ => {}
        }
    }
    let c_qr = classify_object(query_response.as_deref().unwrap(), "QueryResponse");
    assert!(c_qr.contains(&C::Memory) && c_qr.contains(&C::Schema));

    let c_cmd = classify_object(cmd.as_deref().unwrap(), "Command");
    assert!(c_cmd.contains(&C::Attention) && c_cmd.contains(&C::DecisionMaking));

    let c_evt = classify_object(event.as_deref().unwrap(), "DomainEvent");
    assert!(c_evt.contains(&C::Memory) && c_evt.contains(&C::Language));

    let c_rm = classify_object(read_model.as_deref().unwrap(), "ReadModel");
    assert!(c_rm.contains(&C::Memory) && c_rm.contains(&C::Schema));
}
