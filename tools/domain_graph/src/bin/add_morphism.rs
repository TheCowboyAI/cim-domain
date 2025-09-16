use std::env;
use std::fs;

use serde_json::{json, Value};

// Usage:
// add_morphism [domain-graph.json] --id <id> --name <Name> --type <Type> --source <Object> --target <Object> [--diagram <diagram_id>]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let mut path = "domain-graph.json".to_string();
    if !args.is_empty() && !args[0].starts_with("--") { path = args.remove(0); }

    let mut id = String::new();
    let mut name = String::new();
    let mut typ = String::new();
    let mut source = String::new();
    let mut target = String::new();
    let mut diagram = String::new();

    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--id" => id = it.next().expect("--id value"),
            "--name" => name = it.next().expect("--name value"),
            "--type" => typ = it.next().expect("--type value"),
            "--source" => source = it.next().expect("--source value"),
            "--target" => target = it.next().expect("--target value"),
            "--diagram" => diagram = it.next().expect("--diagram value"),
            other => panic!("unknown arg: {}", other),
        }
    }

    if id.is_empty() || name.is_empty() || typ.is_empty() || source.is_empty() || target.is_empty() {
        eprintln!("Usage: add_morphism [domain-graph.json] --id <id> --name <Name> --type <Type> --source <Object> --target <Object> [--diagram <diagram_id>]");
        std::process::exit(2);
    }

    let raw = fs::read_to_string(&path)?;
    let mut g: Value = serde_json::from_str(&raw)?;
    let morphs = g["category"]["morphisms"].as_array().cloned().unwrap_or_default();
    if morphs.iter().any(|m| m["id"].as_str() == Some(&id)) {
        eprintln!("morphism with id '{}' already exists", id);
        std::process::exit(1);
    }

    let new_m = json!({
        "id": id,
        "name": name,
        "source": source,
        "target": target,
        "type": typ,
    });

    // Push morphism
    g["category"]["morphisms"].as_array_mut().unwrap().push(new_m);

    // Optionally attach to a diagram's describes list
    if !diagram.is_empty() {
        if let Some(arr) = g["category"]["diagrams"].as_array_mut() {
            if let Some(d) = arr.iter_mut().find(|d| d["id"].as_str() == Some(&diagram)) {
                let desc = d["describes"].as_array_mut().expect("diagram.describes is array");
                desc.push(Value::String(g["category"]["morphisms"].as_array().unwrap().last().unwrap()["id"].as_str().unwrap().to_string()));
            } else {
                eprintln!("warning: diagram '{}' not found; morphism added but not described", diagram);
            }
        }
    }

    fs::write(&path, serde_json::to_string_pretty(&g)?)?;
    println!("Added morphism '{}' and updated diagram '{}' (if provided)", id, diagram);
    Ok(())
}

