// Copyright (c) 2025 - Cowboy AI, LLC.

use chrono::Utc;
use cid::Cid;
use multihash::Multihash;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct DialogEventContent {
    event_id: String,
    #[serde(rename = "type")]
    kind: String,
    user_said: String,
    i_understood: String,
    what_i_did: Vec<String>,
    parent_cid: Option<String>,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DialogNode {
    cid: String,
    content: DialogEventContent,
}

fn calculate_cid(content: &DialogEventContent) -> String {
    let json_bytes = serde_json::to_vec(content).expect("serialize content");
    let hash = blake3::hash(&json_bytes);
    let mh = Multihash::wrap(0x1e, hash.as_bytes()).expect("wrap blake3");
    let cid = Cid::new_v1(0x55, mh);
    cid.to_string()
}

fn usage() -> ! {
    eprintln!(
        "Usage: log_dialog_event [path=dialog-dag.json] <type> <user_said> <i_understood> <what_i_did_semi_colon_separated> [parent_cid]"
    );
    std::process::exit(2)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let (path, i) = if args.len() >= 6 {
        // With explicit path
        (args[1].clone(), 2)
    } else if args.len() >= 5 {
        ("dialog-dag.json".to_string(), 1)
    } else {
        usage()
    };

    let kind = args[i].clone();
    let user_said = args[i + 1].clone();
    let i_understood = args[i + 2].clone();
    let did_raw = args[i + 3].clone();
    let parent_override = args.get(i + 4).cloned();

    let what_i_did: Vec<String> = did_raw
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Load existing JSON
    let p = Path::new(&path);
    let content = fs::read_to_string(p)?;
    let mut dag: Value = serde_json::from_str(&content)?;

    // Determine parent_cid (last event) unless overridden
    let parent_cid = parent_override.or_else(|| {
        dag["events"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|n| n.get("cid"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    });

    let content = DialogEventContent {
        event_id: Uuid::new_v4().to_string(),
        kind,
        user_said,
        i_understood,
        what_i_did,
        parent_cid,
        timestamp: Utc::now().timestamp(),
    };

    let cid = calculate_cid(&content);
    let node = DialogNode { cid, content };

    // Append
    if let Some(events) = dag["events"].as_array_mut() {
        events.push(serde_json::to_value(node)?);
        dag["total_events"] = json!(events.len());
    }

    // Persist
    let pretty = serde_json::to_string_pretty(&dag)?;
    fs::write(p, pretty)?;

    Ok(())
}
