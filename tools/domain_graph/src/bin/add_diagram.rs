use std::env;
use std::fs;

use serde_json::{json, Value};

// Usage: add_diagram [domain-graph.json] --id <diagram_id> --path <svg_path> --describes id1,id2,... [--notes "..."]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut path = "domain-graph.json".to_string();
    if !args.is_empty() && !args[0].starts_with("--") {
        path = args.remove(0);
    }

    let mut id = String::new();
    let mut svg = String::new();
    let mut describes: Vec<String> = vec![];
    let mut notes: Option<String> = None;

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--id" => id = it.next().expect("--id value"),
            "--path" => svg = it.next().expect("--path value"),
            "--describes" => {
                if let Some(csv) = it.next() {
                    describes = csv
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            "--notes" => notes = it.next(),
            other => panic!("unknown arg: {other}"),
        }
    }

    if id.is_empty() || svg.is_empty() || describes.is_empty() {
        eprintln!("Usage: add_diagram [domain-graph.json] --id <diagram_id> --path <svg_path> --describes id1,id2,... [--notes \"...\"]");
        std::process::exit(2);
    }

    if !std::path::Path::new(&svg).exists() {
        eprintln!("warning: diagram path '{svg}' does not exist; continuing anyway");
    }

    let raw = fs::read_to_string(&path)?;
    let mut g: Value = serde_json::from_str(&raw)?;

    // Ensure array exists
    if g["category"]["diagrams"].is_null() {
        g["category"]["diagrams"] = json!([]);
    }
    let mut diagrams = g["category"]["diagrams"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // Replace if exists
    diagrams.retain(|d| d["id"].as_str() != Some(&id));
    let mut d = json!({ "id": id, "commutes": true, "describes": describes, "path": svg });
    if let Some(n) = notes {
        d["notes"] = json!(n);
    }
    diagrams.push(d);
    g["category"]["diagrams"] = json!(diagrams);

    // Update metadata files list
    if g["metadata"]["isomorphic_to"]["string_diagrams"]["files"].is_null() {
        g["metadata"]["isomorphic_to"]["string_diagrams"]["files"] = json!([]);
    }
    let mut files = g["metadata"]["isomorphic_to"]["string_diagrams"]["files"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !files.iter().any(|v| v.as_str() == Some(&svg)) {
        files.push(json!(svg));
    }
    g["metadata"]["isomorphic_to"]["string_diagrams"]["files"] = json!(files);

    // Do not set verified=true automatically; keep diff-first review
    fs::write(&path, serde_json::to_string_pretty(&g)?)?;
    let describes_len = g["category"]["diagrams"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|d| d["describes"].as_array())
        .map(|list| list.len())
        .unwrap_or(0);
    println!("Added diagram '{id}' with {describes_len} describes");
    Ok(())
}
