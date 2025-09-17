use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
struct Obj { id: String, name: String, #[serde(rename = "type")] kind: String }
#[derive(Debug, Deserialize, Clone)]
struct Morph { id: String, name: String, source: String, target: String, #[serde(rename = "type")] kind: String }
#[derive(Debug, Deserialize)]
struct Diagram { id: String, describes: Option<Vec<String>> }
#[derive(Debug, Deserialize)]
struct Category { objects: Vec<Obj>, morphisms: Vec<Morph>, diagrams: Vec<Diagram> }
#[derive(Debug, Deserialize)]
struct Graph { category: Category }

fn parse_args() -> (String, Option<String>, Option<BTreeSet<String>>, Option<String>) {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut path = "domain-graph.json".to_string();
    if !args.is_empty() && !args[0].starts_with("--") { path = args.remove(0); }
    let mut diagram: Option<String> = None;
    let mut include: Option<BTreeSet<String>> = None;
    let mut out: Option<String> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--diagram" => diagram = it.next(),
            "--include" => {
                if let Some(csv) = it.next() {
                    let set = csv.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                    include = Some(set);
                }
            }
            "--out" => out = it.next(),
            other => panic!("unknown arg: {}", other),
        }
    }
    (path, diagram, include, out)
}

fn edge_style(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "Identity" => ("#0ea5e9", "dashed"),
        "Envelope" => ("#7c3aed", "solid"),
        "ProjectionUpdate" => ("#8b5cf6", "solid"),
        "Consumption" | "Subscription" => ("#10b981", "solid"),
        "Causation" | "Temporal" => ("#ef4444", "dotted"),
        "Containment" => ("#475569", "dashed"),
        _ => ("#374151", "solid"),
    }
}

fn node_style(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "Aggregate" | "Trait" => ("#0ea5e9", "lightblue"),
        "Entity" => ("#2563eb", "#eff6ff"),
        "ValueObject" => ("#6366f1", "#eef2ff"),
        "Command" | "Query" => ("#f59e0b", "#fffbeb"),
        "Event" | "Concept" => ("#7c3aed", "#f5f3ff"),
        "Projection" | "ReadModel" => ("#8b5cf6", "#faf5ff"),
        _ => ("#374151", "#ffffff"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (path, diagram, include, out) = parse_args();
    let raw = fs::read_to_string(&path)?;
    let g: Graph = serde_json::from_str(&raw)?;

    // Filter morphisms
    let mut morphs: Vec<Morph> = g.category.morphisms.clone();
    if let Some(did) = diagram.as_ref() {
        if let Some(d) = g.category.diagrams.iter().find(|d| &d.id == did) {
            let ids: BTreeSet<String> = d
                .describes
                .as_ref()
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default();
            morphs.retain(|m| ids.contains(&m.id));
        }
    }
    if let Some(inc) = include.as_ref() {
        morphs.retain(|m| inc.contains(&m.id));
    }

    // Build object map and mark used nodes
    let mut used: BTreeSet<String> = BTreeSet::new();
    for m in &morphs {
        used.insert(m.source.clone());
        used.insert(m.target.clone());
    }
    let mut objs: BTreeMap<String, Obj> = BTreeMap::new();
    for o in &g.category.objects {
        if used.contains(&o.id) { objs.insert(o.id.clone(), o.clone()); }
    }

    let mut dot = String::new();
    dot.push_str("digraph UL {\n  rankdir=LR;\n  node [shape=box, style=filled, fontname=Helvetica];\n\n");

    // Nodes
    for (id, o) in &objs {
        let (stroke, fill) = node_style(&o.kind);
        dot.push_str(&format!("  \"{}\" [label=\"{}\n({})\", color=\"{}\", fillcolor=\"{}\"];\n", id, o.name, o.kind, stroke, fill));
    }
    dot.push('\n');

    // Edges
    for m in &morphs {
        let (color, style) = edge_style(&m.kind);
        // use id as label (directional verb)
        dot.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\", color=\"{}\", style=\"{}\"];\n",
            m.source, m.target, m.id, color, style
        ));
    }

    dot.push_str("}\n");

    if let Some(path) = out {
        fs::write(path, dot)?;
    } else {
        print!("{}", dot);
    }
    Ok(())
}

