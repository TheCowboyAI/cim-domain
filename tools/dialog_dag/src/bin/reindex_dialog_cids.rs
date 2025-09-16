use cid::Cid;
use multihash::Multihash;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    eprintln!("Usage: reindex_dialog_cids [path=dialog-dag.json]");
    std::process::exit(2)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() >= 2 { args[1].clone() } else { "dialog-dag.json".to_string() };
    if args.len() > 2 { usage(); }

    let p = Path::new(&path);
    let content = fs::read_to_string(p)?;
    let mut dag: Value = serde_json::from_str(&content)?;

    let events = dag["events"].as_array().cloned().unwrap_or_default();
    let mut new_events: Vec<Value> = Vec::with_capacity(events.len());
    let mut prev_cid: Option<String> = None;

    for ev in events {
        let content_v = ev.get("content").cloned().ok_or("missing content")?;
        let mut node: DialogNode = DialogNode {
            cid: String::new(),
            content: serde_json::from_value(content_v)?
        };
        node.content.parent_cid = prev_cid.clone();
        let cid = calculate_cid(&node.content);
        node.cid = cid.clone();
        prev_cid = Some(cid);
        new_events.push(serde_json::to_value(node)?);
    }

    dag["events"] = json!(new_events);
    dag["total_events"] = json!(dag["events"].as_array().map(|a| a.len()).unwrap_or(0));

    fs::write(p, serde_json::to_string_pretty(&dag)?)?;
    Ok(())
}

