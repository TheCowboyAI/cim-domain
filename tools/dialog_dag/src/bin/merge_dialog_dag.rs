use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::fs;

fn usage() -> ! {
    eprintln!("Usage: merge_dialog_dag <main_dialog_dag.json> <continuation.json>");
    std::process::exit(2)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        usage();
    }
    let main_path = &args[1];
    let cont_path = &args[2];

    let main_text = fs::read_to_string(main_path)?;
    let cont_text = fs::read_to_string(cont_path)?;

    let mut main_json: Value = serde_json::from_str(&main_text)?;
    let cont_json: Value = serde_json::from_str(&cont_text)?;

    // Work on a cloned events array to avoid long-lived mutable borrows
    let mut main_events: Vec<Value> = main_json["events"].as_array().cloned().unwrap_or_default();
    let mut existing: HashSet<String> = main_events
        .iter()
        .filter_map(|n| n.get("cid").and_then(|c| c.as_str()).map(|s| s.to_string()))
        .collect();

    let mut new_events: Vec<Value> = Vec::new();
    if let Some(cont_events) = cont_json["events"].as_array() {
        for ev in cont_events {
            if let Some(cid) = ev.get("cid").and_then(|c| c.as_str()) {
                if !existing.contains(cid) {
                    new_events.push(ev.clone());
                    existing.insert(cid.to_string());
                }
            }
        }
    }

    // Sort new events by timestamp to keep chronological order
    new_events.sort_by_key(|n| {
        n.get("content")
            .and_then(|c| c.get("timestamp"))
            .and_then(|t| t.as_i64())
            .unwrap_or(0)
    });

    for ev in new_events {
        main_events.push(ev);
    }

    // Merge key insights
    let mut insights: Vec<String> = main_json["key_insights"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let mut insight_set: HashSet<String> = insights.iter().cloned().collect();

    for key in ["key_insights", "key_insights_continued"].iter() {
        if let Some(arr) = cont_json.get(*key).and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    if insight_set.insert(s.to_string()) {
                        insights.push(s.to_string());
                    }
                }
            }
        }
    }

    if !insights.is_empty() {
        main_json["key_insights"] = json!(insights);
    }

    // Write back merged events and update totals
    main_json["events"] = json!(main_events);
    main_json["total_events"] = json!(main_json["events"].as_array().map(|a| a.len()).unwrap_or(0));

    let pretty = serde_json::to_string_pretty(&main_json)?;
    fs::write(main_path, pretty)?;

    Ok(())
}
