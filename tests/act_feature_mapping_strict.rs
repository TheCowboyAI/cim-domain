#![cfg(feature = "act_strict")]
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct FeatureRec { id: String, kind: String, file: String, tests: Vec<String>, tdd: Vec<String> }
#[derive(Debug)]
struct FeatureIndex { map: HashMap<String, Vec<String>>, recs: Vec<FeatureRec> }

fn load_feature_index(path: &str) -> FeatureIndex {
    let data = fs::read_to_string(path).expect("read feature index");
    let y: serde_yaml::Value = serde_yaml::from_str(&data).expect("valid YAML");
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let mut recs: Vec<FeatureRec> = vec![];
    let empty = Vec::new();
    let feats = y["features"].as_sequence().unwrap_or(&empty);
    for f in feats {
        let empty_tests = Vec::new();
        let tests = f["tests"].as_sequence().unwrap_or(&empty_tests);
        for t in tests {
            if let Some(s) = t.as_str() {
                map.entry(s.to_string()).or_default();
            }
        }
    }
    // Normalize and collect feature records
    for f in feats {
        let id = f["id"].as_str().unwrap_or("").to_string();
        let kind = f["kind"].as_str().unwrap_or("").to_string();
        let file = f["file"].as_str().unwrap_or("").to_string();
        let empty_tests = Vec::new();
        let tests_v = f["tests"].as_sequence().unwrap_or(&empty_tests);
        let mut tests = vec![];
        for t in tests_v { if let Some(s) = t.as_str() { tests.push(s.to_string()); } }
        let empty_tdd = Vec::new();
        let tdd_v = f["tdd"].as_sequence().unwrap_or(&empty_tdd);
        let mut tdd = vec![];
        for t in tdd_v { if let Some(s) = t.as_str() { tdd.push(s.to_string()); } }
        for t in &tests { map.entry(t.clone()).or_default().push(id.clone()); }
        recs.push(FeatureRec { id, kind, file, tests, tdd });
    }
    FeatureIndex { map, recs }
}

fn collect_integration_test_files(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = vec![];
    fn walk(dir: &Path, out: &mut Vec<String>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { walk(&p, out); }
                else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                    // Skip helpers
                    if p.file_name().and_then(|x| x.to_str()) == Some("bdd_support.rs") { continue; }
                    if p.file_name().and_then(|x| x.to_str()) == Some("act_feature_mapping_strict.rs") { continue; }
                    out.push(p.display().to_string());
                }
            }
        }
    }
    walk(root, &mut files);
    files.sort();
    files
}

#[test]
fn every_integration_test_is_mapped_to_a_feature() {
    let idx = load_feature_index("doc/qa/features/index.yaml");
    // scan root tests and examples/domain_examples/tests
    let mut files = vec![];
    files.extend(collect_integration_test_files(Path::new("tests")));
    files.extend(collect_integration_test_files(Path::new("examples/domain_examples/tests")));
    let known: HashSet<&String> = idx.map.keys().collect();
    let mut missing: Vec<String> = vec![];
    for f in files {
        if !known.contains(&f) { missing.push(f); }
    }
    assert!(missing.is_empty(), "Integration tests missing feature mapping: {:?}", missing);
}

#[test]
fn every_feature_has_tdd_and_bdd_assets() {
    let idx = load_feature_index("doc/qa/features/index.yaml");
    let mut errs = vec![];
    for rec in &idx.recs {
        // bdd feature must have a feature file and at least one integration test
        if rec.kind == "bdd" {
            if rec.file.is_empty() || !Path::new(&rec.file).exists() {
                errs.push(format!("feature {} missing BDD file {}", rec.id, rec.file));
            }
            if rec.tests.is_empty() {
                errs.push(format!("feature {} missing BDD tests", rec.id));
            }
        }
        // All features must have at least one TDD asset (unit or integration)
        if rec.tdd.is_empty() {
            errs.push(format!("feature {} missing TDD mapping", rec.id));
        } else {
            for p in &rec.tdd {
                if !Path::new(p).exists() {
                    errs.push(format!("feature {} TDD file missing: {}", rec.id, p));
                }
            }
        }
    }
    assert!(errs.is_empty(), "Feature mapping errors: {:?}", errs);
}
