use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use cim_domain::{classify_object, CoreConceptId};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Obj { id: String, name: String, #[serde(rename = "type")] kind: String }
#[derive(Debug, Deserialize)]
struct Category { objects: Vec<Obj> }
#[derive(Debug, Deserialize)]
struct Graph { category: Category }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Row {
    object_id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    ul_concept_id: String,
    core_concepts: Vec<String>,
}

fn slug(name: &str) -> String {
    name.chars()
        .flat_map(|c| if c.is_ascii_alphanumeric() { Some(c.to_ascii_lowercase()) } else if c.is_whitespace() || c == '<' || c == '>' { Some('_') } else { None })
        .collect::<String>()
}

fn parse_args() -> (String, String, bool) {
    // Accept flags anywhere: --write triggers overwrite mode
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut write = false;
    let mut paths: Vec<String> = Vec::new();
    for a in args {
        if a == "--write" { write = true; } else { paths.push(a); }
    }
    let graph_path = paths.get(0).cloned().unwrap_or_else(|| "domain-graph.json".to_string());
    let out_path = paths.get(1).cloned().unwrap_or_else(|| "ul-projection.json".to_string());
    (graph_path, out_path, write)
}

fn to_map(rows: &[Row]) -> BTreeMap<String, Row> {
    let mut m = BTreeMap::new();
    for r in rows {
        // normalize core concepts ordering for stable diffs
        let mut norm = r.clone();
        let mut cc: Vec<String> = norm.core_concepts.clone();
        cc.sort();
        norm.core_concepts = cc;
        m.insert(norm.object_id.clone(), norm);
    }
    m
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (graph_path, out_path, write) = parse_args();
    let raw = fs::read_to_string(&graph_path)?;
    let g: Graph = serde_json::from_str(&raw)?;

    let mut new_rows: Vec<Row> = Vec::new();
    for o in g.category.objects {
        let core: Vec<String> = classify_object(&o.kind, &o.name)
            .into_iter()
            .map(|c| match c {
                CoreConceptId::Perception => "perception",
                CoreConceptId::Attention => "attention",
                CoreConceptId::Memory => "memory",
                CoreConceptId::Schema => "schema",
                CoreConceptId::ProblemSolving => "problem_solving",
                CoreConceptId::DecisionMaking => "decision_making",
                CoreConceptId::Language => "language",
                CoreConceptId::CognitiveBias => "cognitive_bias",
                CoreConceptId::Metacognition => "metacognition",
                CoreConceptId::CognitiveDevelopment => "cognitive_development",
            }.to_string())
            .collect();
        let ul_id = slug(&o.name);
        new_rows.push(Row { object_id: o.id, name: o.name, kind: o.kind, ul_concept_id: ul_id, core_concepts: core });
    }

    // If not writing, produce a diff against existing file (if present)
    let out_path_obj = Path::new(&out_path);
    if !write {
        let mut added: Vec<Row> = Vec::new();
        let mut removed: Vec<Row> = Vec::new();
        let mut modified: Vec<serde_json::Value> = Vec::new();

        if out_path_obj.exists() {
            let existing_raw = fs::read_to_string(out_path_obj)?;
            let existing_json: serde_json::Value = serde_json::from_str(&existing_raw)?;
            let existing_rows: Vec<Row> = existing_json
                .get("ul_projection")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| serde_json::from_value::<Row>(r.clone()).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let old_map = to_map(&existing_rows);
            let new_map = to_map(&new_rows);

            // Added and modified
            for (id, after) in new_map.iter() {
                match old_map.get(id) {
                    None => added.push(after.clone()),
                    Some(before) => {
                        if before != after {
                            modified.push(json!({ "object_id": id, "before": before, "after": after }));
                        }
                    }
                }
            }
            // Removed
            for (id, before) in old_map.iter() {
                if !new_map.contains_key(id) {
                    removed.push(before.clone());
                }
            }
        } else {
            // No existing file: everything is "added"
            added = new_rows.clone();
        }

        let diff = json!({ "ul_projection_diff": { "added": added, "removed": removed, "modified": modified } });
        let diff_path = out_path_obj.with_extension("diff.json");
        fs::write(&diff_path, serde_json::to_string_pretty(&diff)?)?;
        println!(
            "Diff written to {} (use --write to overwrite {})",
            diff_path.display(),
            out_path_obj.display()
        );
        return Ok(());
    }

    // Write full projection
    let out_rows_json = new_rows
        .into_iter()
        .map(|r| serde_json::to_value(r).expect("row to value"))
        .collect::<Vec<_>>();
    let out = json!({ "ul_projection": out_rows_json });
    fs::write(&out_path, serde_json::to_string_pretty(&out)?)?;
    println!("Wrote {}", Path::new(&out_path).display());
    Ok(())
}
