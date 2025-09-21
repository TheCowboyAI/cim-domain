use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "domain-graph.json".to_string());
    let mut v: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;

    // Basic structure checks
    let category = v
        .get_mut("category")
        .ok_or("missing category in domain-graph.json")?;

    let morphisms = category
        .get("morphisms")
        .and_then(|m| m.as_array())
        .ok_or("category.morphisms must be an array")?;

    let diagrams = category
        .get("diagrams")
        .and_then(|d| d.as_array())
        .ok_or("category.diagrams must be an array")?;

    // Collect diagram-described ids
    let mut described: HashSet<String> = HashSet::new();
    for d in diagrams {
        if let Some(arr) = d.get("describes").and_then(|a| a.as_array()) {
            for x in arr {
                if let Some(s) = x.as_str() {
                    described.insert(s.to_string());
                }
            }
        }
        // Validate diagram path exists
        if let Some(p) = d.get("path").and_then(|p| p.as_str()) {
            if !Path::new(p).exists() {
                eprintln!("Diagram path missing: {p}");
                return Err("missing diagram file".into());
            }
        }
    }

    // Check coverage for non-identity morphisms
    let mut uncovered: Vec<String> = Vec::new();
    for m in morphisms {
        let id = m
            .get("id")
            .and_then(|s| s.as_str())
            .ok_or("morphism without id")?;
        let ty = m.get("type").and_then(|s| s.as_str()).unwrap_or("");
        if ty == "Identity" {
            continue;
        }
        if !described.contains(id) {
            uncovered.push(id.to_string());
        }
    }

    if !uncovered.is_empty() {
        eprintln!("Uncovered morphisms (no diagram describes them):");
        for id in &uncovered {
            eprintln!("  - {id}");
        }
        // Ensure verified=false
        if let Some(meta) = v.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            if let Some(iso) = meta
                .get_mut("isomorphic_to")
                .and_then(|x| x.as_object_mut())
            {
                if let Some(diag) = iso
                    .get_mut("string_diagrams")
                    .and_then(|x| x.as_object_mut())
                {
                    diag.insert("verified".to_string(), Value::Bool(false));
                }
            }
        }
        // Write back but fail
        fs::write(&path, serde_json::to_string_pretty(&v)?)?;
        return Err("diagram coverage check failed".into());
    }

    // All covered: set verified=true
    if let Some(meta) = v.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        if let Some(iso) = meta
            .get_mut("isomorphic_to")
            .and_then(|x| x.as_object_mut())
        {
            if let Some(diag) = iso
                .get_mut("string_diagrams")
                .and_then(|x| x.as_object_mut())
            {
                diag.insert("verified".to_string(), Value::Bool(true));
            }
        }
    }
    fs::write(&path, serde_json::to_string_pretty(&v)?)?;
    println!("OK: all non-identity morphisms are covered by diagrams; set verified=true");
    Ok(())
}
