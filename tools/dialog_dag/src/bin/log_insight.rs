// Copyright (c) 2025 - Cowboy AI, LLC.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

#[derive(Default)]
struct Args {
    file: String,
    id: String,
    summary: String,
    details: String,
    tags: Vec<String>,
    source_event: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--file" => a.file = it.next().unwrap_or_else(|| "dialog-dag.json".to_string()),
            "--id" => a.id = it.next().unwrap_or_default(),
            "--summary" => a.summary = it.next().unwrap_or_default(),
            "--details" => a.details = it.next().unwrap_or_default(),
            "--tags" => {
                let t = it.next().unwrap_or_default();
                a.tags = t
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "--source" => a.source_event = Some(it.next().unwrap_or_default()),
            _ => {
                // Allow shorthand: file as first arg
                if a.file.is_empty() && Path::new(&arg).exists() {
                    a.file = arg;
                }
            }
        }
    }
    if a.file.is_empty() {
        a.file = "dialog-dag.json".to_string();
    }
    a
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a = parse_args();
    if a.id.is_empty() || a.summary.is_empty() {
        eprintln!("Usage: log_insight --file dialog-dag.json --id <id> --summary <summary> --details <details> --tags tag1,tag2 [--source <cid>]");
        std::process::exit(2);
    }

    let raw = fs::read_to_string(&a.file)?;
    let mut v: Value = serde_json::from_str(&raw)?;
    if v.get("insights").is_none() {
        v["insights"] = json!([]);
    }
    let ins = v["insights"].as_array_mut().unwrap();
    // Dedup by id
    if let Some(existing) = ins
        .iter_mut()
        .find(|x| x.get("id").and_then(|y| y.as_str()) == Some(a.id.as_str()))
    {
        // Update existing
        existing["summary"] = json!(a.summary);
        existing["details"] = json!(a.details);
        existing["tags"] = json!(a.tags);
        if let Some(src) = a.source_event.clone() {
            existing["source_event"] = json!(src);
        }
    } else {
        let mut entry = json!({
            "id": a.id,
            "summary": a.summary,
            "details": a.details,
            "tags": a.tags,
        });
        if let Some(src) = a.source_event {
            entry["source_event"] = json!(src);
        }
        ins.push(entry);
    }
    fs::write(&a.file, serde_json::to_string_pretty(&v)?)?;
    println!("Appended/updated insight '{}' in {}", a.id, a.file);
    Ok(())
}
