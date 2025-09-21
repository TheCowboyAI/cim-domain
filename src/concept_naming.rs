// Copyright 2025 Cowboy AI, LLC.

//! Concept Naming — suggest UL concepts for entities from values/features
//!
//! This module provides a pure, minimal facility to suggest a Concept id for a
//! given entity (a group of values) by comparing feature vectors against
//! prototype Quality Vectors under a declared Quality Schema. It does not
//! implement the conceptual space; it only provides the last mile for naming.

use std::collections::BTreeMap;

use crate::ontology_quality::{QualitySchema, QualityVector};

/// Compute cosine similarity between two quality vectors
fn cosine(a: &QualityVector, b: &QualityVector) -> f64 {
    if a.values.len() != b.values.len() || a.values.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for (x, y) in a.values.iter().zip(b.values.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        (dot / (na.sqrt() * nb.sqrt())).clamp(-1.0, 1.0)
    }
}

/// Build a quality vector from sparse features keyed by quality dimension id
pub fn vector_from_features(
    schema: &QualitySchema,
    features: &BTreeMap<String, f64>,
) -> QualityVector {
    let mut v = QualityVector::zero(schema);
    for (id, val) in features {
        if let Some(i) = schema.index_of(id) {
            v.values[i] = *val;
        }
    }
    v
}

/// Suggest top-k concept ids by cosine similarity to prototype vectors
pub fn suggest_by_prototypes(
    schema: &QualitySchema,
    entity_features: &BTreeMap<String, f64>,
    prototypes: &BTreeMap<String, QualityVector>,
    top_k: usize,
) -> Vec<(String, f64)> {
    let v = vector_from_features(schema, entity_features);
    let mut scored: Vec<(String, f64)> = prototypes
        .iter()
        .map(|(cid, proto)| (cid.clone(), cosine(&v, proto)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored.truncate(top_k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology_quality::{QualityDimension, ScaleType};

    fn schema() -> QualitySchema {
        QualitySchema::new(vec![
            QualityDimension {
                id: "has_amount".into(),
                name: "Has Amount".into(),
                scale: ScaleType::Nominal,
            },
            QualityDimension {
                id: "has_party".into(),
                name: "Has Party".into(),
                scale: ScaleType::Nominal,
            },
            QualityDimension {
                id: "has_date".into(),
                name: "Has Date".into(),
                scale: ScaleType::Nominal,
            },
        ])
    }

    #[test]
    fn pick_best_concept_by_similarity() {
        let s = schema();
        let mut prototypes: BTreeMap<String, QualityVector> = BTreeMap::new();
        prototypes.insert(
            "invoice".into(),
            QualityVector {
                values: vec![1.0, 1.0, 1.0],
            },
        );
        prototypes.insert(
            "payment".into(),
            QualityVector {
                values: vec![1.0, 1.0, 0.0],
            },
        );
        prototypes.insert(
            "profile".into(),
            QualityVector {
                values: vec![0.0, 1.0, 0.0],
            },
        );

        let mut feat = BTreeMap::new();
        feat.insert("has_amount".into(), 1.0);
        feat.insert("has_party".into(), 1.0);
        feat.insert("has_date".into(), 0.8);

        let top = suggest_by_prototypes(&s, &feat, &prototypes, 2);
        assert_eq!(top[0].0, "invoice"); // closest to [1,1,1]
        assert!(top[0].1 >= top[1].1);
    }
}
