use std::env;
use std::fs;

use serde_json::{json, Value};

// Usage:
// add_morphism [domain-graph.json] --id <id> --name <Name> --type <Type> --source <Object> --target <Object> [--diagram <diagram_id>]
fn print_usage(program: &str) {
    println!(
        "Usage: {program} [domain-graph.json] --id <id> --name <Name> --type <Type> --source <Object> --target <Object> [--diagram <diagram_id>]"
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv = env::args();
    let program = argv.next().unwrap_or_else(|| "add_morphism".to_string());
    let mut args: Vec<String> = argv.collect();

    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--help" | "-h" | "help"))
    {
        print_usage(&program);
        return Ok(());
    }

    let mut path = "domain-graph.json".to_string();
    if !args.is_empty() && !args[0].starts_with("--") {
        path = args.remove(0);
    }

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
            other => {
                eprintln!("unknown arg: {other}");
                print_usage(&program);
                std::process::exit(2);
            }
        }
    }

    if id.is_empty() || name.is_empty() || typ.is_empty() || source.is_empty() || target.is_empty()
    {
        eprintln!("missing required arguments");
        print_usage(&program);
        std::process::exit(2);
    }

    let raw = fs::read_to_string(&path)?;
    let mut g: Value = serde_json::from_str(&raw)?;
    let morphs = g["category"]["morphisms"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if morphs.iter().any(|m| m["id"].as_str() == Some(&id)) {
        eprintln!("morphism with id '{id}' already exists");
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
    g["category"]["morphisms"]
        .as_array_mut()
        .unwrap()
        .push(new_m);

    let new_morphism_id = g["category"]["morphisms"]
        .as_array()
        .and_then(|arr| arr.last())
        .and_then(|m| m.get("id"))
        .and_then(Value::as_str)
        .expect("morphism id present")
        .to_string();

    // Optionally attach to a diagram's describes list
    if !diagram.is_empty() {
        if let Some(arr) = g["category"]["diagrams"].as_array_mut() {
            if let Some(d) = arr.iter_mut().find(|d| d["id"].as_str() == Some(&diagram)) {
                let desc = d["describes"]
                    .as_array_mut()
                    .expect("diagram.describes is array");
                desc.push(Value::String(new_morphism_id));
            } else {
                eprintln!(
                    "warning: diagram '{diagram}' not found; morphism added but not described"
                );
            }
        }
    }

    fs::write(&path, serde_json::to_string_pretty(&g)?)?;
    println!("Added morphism '{id}' and updated diagram '{diagram}' (if provided)");
    Ok(())
}
