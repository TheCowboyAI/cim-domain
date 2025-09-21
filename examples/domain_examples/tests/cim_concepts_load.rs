// Copyright (c) 2025 - Cowboy AI, LLC.

use std::collections::BTreeSet;

use cim_domain::concepts::{Concept, ConceptGraph, ConceptRelationshipType};

#[derive(Debug, serde::Deserialize)]
struct ConceptYaml {
    id: String,
    name: String,
    description: Option<String>,
    #[serde(default)]
    synonyms: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct RelationYaml {
    from: String,
    to: String,
    rel: String,
}

#[derive(Debug, serde::Deserialize)]
struct ConceptSeed {
    concepts: Vec<ConceptYaml>,
    relations: Vec<RelationYaml>,
}

fn parse_rel(s: &str) -> ConceptRelationshipType {
    match s {
        "IsA" => ConceptRelationshipType::IsA,
        "PartOf" => ConceptRelationshipType::PartOf,
        "RelatedTo" => ConceptRelationshipType::RelatedTo,
        "SameAs" => ConceptRelationshipType::SameAs,
        "OppositeOf" => ConceptRelationshipType::OppositeOf,
        "DependsOn" => ConceptRelationshipType::DependsOn,
        _ => ConceptRelationshipType::RelatedTo,
    }
}

#[test]
fn load_cim_core_concepts_yaml() {
    let raw = include_str!("../concepts/cim_core_concepts.yaml");
    let seed: ConceptSeed = serde_yaml::from_str(raw).expect("valid yaml");

    let mut space = ConceptGraph::new();
    for c in seed.concepts {
        let mut syns: BTreeSet<String> = BTreeSet::new();
        syns.extend(c.synonyms.into_iter());
        let mut tags: BTreeSet<String> = BTreeSet::new();
        tags.extend(c.tags.into_iter());
        let concept = Concept {
            id: c.id,
            name: c.name,
            description: c.description,
            synonyms: syns,
            tags,
        };
        space.upsert_concept(concept);
    }
    for r in seed.relations {
        space.relate(r.from, r.to, parse_rel(&r.rel));
    }

    // Expect 10 core nodes
    let ids = [
        "perception",
        "attention",
        "memory",
        "schema",
        "problem_solving",
        "decision_making",
        "language",
        "cognitive_bias",
        "metacognition",
        "cognitive_development",
    ];
    for id in ids {
        assert!(space.concept(id).is_some(), "missing concept {id}");
    }

    // Some relationship checks
    let neigh = space.neighbors("problem_solving", ConceptRelationshipType::RelatedTo);
    let set: BTreeSet<_> = neigh.iter().map(|c| c.id.as_str()).collect();
    assert!(set.contains("decision_making"));

    // Path checks (length 1)
    assert!(space.path_exists("perception", "attention", 1));
    assert!(space.path_exists("memory", "schema", 1));
}
