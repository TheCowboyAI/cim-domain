use std::fs;

use serde::Deserialize;

#[derive(Deserialize)]
struct Insight {
    id: String,
    summary: String,
    details: String,
}

#[derive(Deserialize)]
struct DialogDag {
    insights: Option<Vec<serde_json::Value>>,
}

#[test]
fn insights_exist_and_have_purpose() {
    let raw = fs::read_to_string("dialog-dag.json").expect("read dialog-dag.json");
    let v: DialogDag = serde_json::from_str(&raw).expect("parse dialog-dag.json");
    let Some(list) = v.insights.as_ref() else {
        panic!("insights[] missing in dialog-dag.json");
    };
    assert!(!list.is_empty(), "insights[] must not be empty");

    // Ensure purpose insight exists
    let mut has_purpose = false;
    for item in list {
        if let Some(id) = item.get("id").and_then(|s| s.as_str()) {
            if id == "purpose" {
                has_purpose = true;
            }
        }
        // All insights must have id/summary/details
        let ins: Insight =
            serde_json::from_value(item.clone()).expect("insight must have id/summary/details");
        assert!(!ins.id.is_empty(), "insight id must be non-empty");
        assert!(!ins.summary.is_empty(), "insight summary must be populated");
        assert!(!ins.details.is_empty(), "insight details must be populated");
    }
    assert!(has_purpose, "insights must include a 'purpose' entry");
}
