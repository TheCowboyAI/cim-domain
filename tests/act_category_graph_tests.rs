#![cfg(feature = "act_strict")]
use std::collections::HashSet;
use std::fs;
use std::ffi::OsStr;
use std::path::Path;

#[test]
fn graph_diagram_files_exist_and_verified() {
    let data = fs::read_to_string("domain-graph.json").expect("read domain-graph.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("valid JSON");
    let diags = &v["metadata"]["isomorphic_to"]["string_diagrams"]["files"];
    assert!(diags.is_array(), "metadata.isomorphic_to.string_diagrams.files must be an array");
    for p in diags.as_array().unwrap() {
        let path = p.as_str().expect("diagram path string");
        assert!(Path::new(path).exists(), "diagram missing: {}", path);
    }
    let verified = v["metadata"]["isomorphic_to"]["string_diagrams"]["verified"].as_bool();
    assert_eq!(verified, Some(true), "string_diagrams.verified must be true after validation");
}

#[test]
fn every_non_identity_morphism_is_described_by_a_diagram() {
    let data = fs::read_to_string("domain-graph.json").expect("read domain-graph.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("valid JSON");
    let morphs = v["category"]["morphisms"].as_array().expect("category.morphisms array");
    let diagrams = v["category"]["diagrams"].as_array().expect("category.diagrams array");

    let mut described: HashSet<String> = HashSet::new();
    for d in diagrams {
        if let Some(arr) = d["describes"].as_array() {
            for x in arr { if let Some(s) = x.as_str() { described.insert(s.to_string()); } }
        }
    }

    let mut missing: Vec<String> = vec![];
    for m in morphs {
        let id = m["id"].as_str().unwrap_or("");
        let ty = m["type"].as_str().unwrap_or("");
        if ty == "Identity" { continue; }
        if !described.contains(id) { missing.push(id.to_string()); }
    }
    assert!(missing.is_empty(), "Uncovered morphisms (no diagram describes them): {:?}", missing);
}

#[test]
fn composition_rules_include_ddd_and_topos_laws() {
    let data = fs::read_to_string("domain-graph.json").expect("read domain-graph.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("valid JSON");
    let rules = v["category"]["composition_rules"].as_array().expect("composition_rules array");
    let texts: HashSet<String> = rules.iter().map(|r| r["rule"].as_str().unwrap_or("").to_string()).collect();

    for must in [
        "convert(A→B) ∘ convert(B→C) = convert(A→C)",
        "fold(x) ∘ fold(y) = fold(x ∘ y)",
        "P(e2 ∘ e1) = P(e2) ∘ P(e1)",
        "F(id) = id ∧ F(g∘f) = F(g)∘F(f)",
        "χ_{f*m} = χ_m ∘ f",
    ] {
        assert!(texts.contains(must), "missing composition rule: {}", must);
    }
}

#[test]
fn category_objects_include_domain_primitives() {
    let data = fs::read_to_string("domain-graph.json").expect("read domain-graph.json");
    let v: serde_json::Value = serde_json::from_str(&data).expect("valid JSON");
    let objs = v["category"]["objects"].as_array().expect("objects array");
    let names: HashSet<String> = objs.iter().map(|o| o["name"].as_str().unwrap_or("").to_string()).collect();
    for must in [
        "AggregateRoot","DomainEvent","Command","StateMachine","Projection","ReadModel","EventStream","Saga","BoundedContext"
    ] {
        assert!(names.contains(must), "missing object: {}", must);
    }
}

#[test]
fn no_stub_tests_in_tests_folder() {
    // Heuristic check: flag obvious stub patterns in tests source code
    let mut offending: Vec<(String, String)> = vec![];
    fn scan_file(path: &Path, buf: &mut Vec<(String, String)>) {
        if let Ok(s) = fs::read_to_string(path) {
            let patterns = [
                "assert!(true)",
                "assert_eq!(1, 1)",
                "todo!()",
                "unimplemented!()",
            ];
            for p in patterns { if s.contains(p) { buf.push((path.display().to_string(), p.to_string())); } }
        }
    }
    fn walk(dir: &Path, buf: &mut Vec<(String, String)>) {
        if let Ok(read) = fs::read_dir(dir) {
            for e in read.flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, buf); }
                else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                    // Skip this file to avoid false positives from string literals of patterns
                    if p.file_name() == Some(OsStr::new("act_category_graph_tests.rs")) { continue; }
                    scan_file(&p, buf);
                }
            }
        }
    }
    walk(Path::new("tests"), &mut offending);
    assert!(offending.is_empty(), "stub patterns found in tests: {:?}", offending);
}

#[test]
fn no_stub_tests_in_src_test_modules() {
    // Scan src/ for obvious stub patterns in test code. This is heuristic and errs on the side of failing fast.
    let mut offending: Vec<(String, String)> = vec![];
    fn scan_file(path: &Path, buf: &mut Vec<(String, String)>) {
        if let Ok(s) = fs::read_to_string(path) {
            // Skip doc tests embedded in comments; we only scan real Rust files here.
            let patterns = [
                "assert!(true)",
                "assert_eq!(1, 1)",
                "todo!()",
                "unimplemented!()",
            ];
            // Cheap filter: only scan files that contain #[test]
            if !s.contains("#[test]") { return; }
            for p in patterns { if s.contains(p) { buf.push((path.display().to_string(), p.to_string())); } }
        }
    }
    fn walk(dir: &Path, buf: &mut Vec<(String, String)>) {
        if let Ok(read) = fs::read_dir(dir) {
            for e in read.flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, buf); }
                else if p.extension().and_then(|x| x.to_str()) == Some("rs") { scan_file(&p, buf); }
            }
        }
    }
    walk(Path::new("src"), &mut offending);
    assert!(offending.is_empty(), "stub patterns found in src tests: {:?}", offending);
}
