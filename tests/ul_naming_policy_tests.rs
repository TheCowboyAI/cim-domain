use std::fs;

#[test]
fn morphism_names_follow_ul_policy() {
    let raw = fs::read_to_string("domain-graph.json").expect("read domain-graph.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
    let morphs = v["category"]["morphisms"]
        .as_array()
        .expect("morphisms array");

    let mut bad: Vec<String> = vec![];
    for m in morphs {
        let id = m["id"].as_str().unwrap_or("").to_string();
        if id.starts_with("has_")
            || id.starts_with("contains_")
            || id == "is_a"
            || id == "related_to"
        {
            bad.push(id);
        }
    }
    assert!(
        bad.is_empty(),
        "Generic/anti-pattern morphism ids present: {:?}",
        bad
    );

    // Spot-check presence of some preferred names to ensure policy took effect
    let ids: std::collections::HashSet<String> = morphs
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_string())
        .collect();
    for must in [
        "refines_concept",
        "identifies_event",
        "owns_value_collection",
        "scopes_aggregate",
        "correlates_with",
        "was_caused_by",
    ] {
        assert!(ids.contains(must), "expected morphism not found: {}", must);
    }
}
