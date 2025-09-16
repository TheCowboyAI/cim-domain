// Copyright 2025 Cowboy AI, LLC.

//! Ontology → Quality Dimensions adapter (pure, library-level)
//!
//! CIM treats the Conceptual Space (and its metric structure) as a separate
//! domain. Here we only define a pure adapter that maps a domain Ontology to a
//! fixed Quality Schema, producing Quality Vectors per concept. This provides a
//! formal bridge without re-implementing the Conceptual Space.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::domain::semantic_analyzer::DomainOntology;

/// Scale for a quality dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleType {
    Nominal,
    Ordinal,
    Interval,
    Ratio,
}

/// A single quality dimension
#[derive(Debug, Clone)]
pub struct QualityDimension {
    pub id: String,
    pub name: String,
    pub scale: ScaleType,
}

/// A schema of quality dimensions (ordered)
#[derive(Debug, Clone)]
pub struct QualitySchema {
    dims: Vec<QualityDimension>,
    index: BTreeMap<String, usize>,
}

impl QualitySchema {
    pub fn new(dims: Vec<QualityDimension>) -> Self {
        let index = dims
            .iter()
            .enumerate()
            .map(|(i, d)| (d.id.clone(), i))
            .collect();
        Self { dims, index }
    }
    pub fn len(&self) -> usize { self.dims.len() }
    pub fn index_of(&self, id: &str) -> Option<usize> { self.index.get(id).cloned() }
}

/// A value vector over a QualitySchema
#[derive(Debug, Clone, PartialEq)]
pub struct QualityVector {
    pub values: Vec<f64>,
}

impl QualityVector {
    pub fn zero(schema: &QualitySchema) -> Self { Self { values: vec![0.0; schema.len()] } }
    pub fn get(&self, idx: usize) -> f64 { self.values[idx] }
}

/// Pure adapter from Ontology to Quality Vectors under a given schema.
pub trait OntologyQualifier {
    fn qualify(&self, ont: &DomainOntology, concept: &str, schema: &QualitySchema) -> QualityVector;
}

/// A simple, deterministic qualifier based on graph features from the ontology:
/// - relatedness_count: number of outgoing related concepts
/// - isa_depth: depth from the nearest root (shortest path via hierarchy)
/// - part_of_count: number of outgoing part-of edges
#[derive(Debug, Default, Clone)]
pub struct SimpleGraphQualifier;

impl SimpleGraphQualifier {
    fn isa_depth(ont: &DomainOntology, concept: &str) -> f64 {
        // BFS from any root to concept using hierarchy edges
        let mut q: VecDeque<(&str, usize)> = VecDeque::new();
        let mut visited: BTreeSet<&str> = BTreeSet::new();
        for root in ont.roots.iter() { q.push_back((root.as_str(), 0)); }
        while let Some((cur, d)) = q.pop_front() {
            if !visited.insert(cur) { continue; }
            if cur == concept { return d as f64; }
            if let Some(children) = ont.hierarchy.get(cur) {
                for c in children { q.push_back((c.as_str(), d + 1)); }
            }
        }
        // Not reachable from roots
        f64::INFINITY
    }
}

impl OntologyQualifier for SimpleGraphQualifier {
    fn qualify(&self, ont: &DomainOntology, concept: &str, schema: &QualitySchema) -> QualityVector {
        let mut v = QualityVector::zero(schema);
        if let Some(idx) = schema.index_of("relatedness_count") {
            let rel = ont.hierarchy.get(concept).map(|v| v.len()).unwrap_or(0) as f64;
            v.values[idx] = rel;
        }
        if let Some(idx) = schema.index_of("isa_depth") {
            let depth = Self::isa_depth(ont, concept);
            v.values[idx] = if depth.is_finite() { depth } else { 0.0 };
        }
        if let Some(idx) = schema.index_of("part_of_count") {
            // Use reverse edges of hierarchy as a crude proxy (parent count)
            let mut parents = 0usize;
            for (_k, children) in ont.hierarchy.iter() { if children.iter().any(|c| c == concept) { parents += 1; } }
            v.values[idx] = parents as f64;
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::domain::semantic_analyzer::{DomainOntology, OntologyAxiom, AxiomType};

    fn schema() -> QualitySchema {
        QualitySchema::new(vec![
            QualityDimension { id: "relatedness_count".into(), name: "Relatedness Count".into(), scale: ScaleType::Ratio },
            QualityDimension { id: "isa_depth".into(), name: "IsA Depth".into(), scale: ScaleType::Ratio },
            QualityDimension { id: "part_of_count".into(), name: "PartOf Count".into(), scale: ScaleType::Ratio },
        ])
    }

    #[test]
    fn ontology_to_quality_vector_is_deterministic() {
        let mut ont = DomainOntology {
            domain: "finance".into(),
            roots: vec!["thing".into()],
            hierarchy: HashMap::new(),
            axioms: vec![OntologyAxiom { name: "disjoint".into(), axiom_type: AxiomType::Disjoint, concepts: vec!["money".into(), "not_money".into()] }],
        };
        ont.hierarchy.insert("thing".into(), vec!["money".into(), "finance".into()]);
        ont.hierarchy.insert("money".into(), vec!["cash".into(), "deposit".into()]);

        let q = SimpleGraphQualifier::default();
        let s = schema();
        let v = q.qualify(&ont, "money", &s);
        assert_eq!(v.values.len(), 3);
        // relatedness_count should be children count (2)
        assert_eq!(v.get(s.index_of("relatedness_count").unwrap()), 2.0);
        // depth from root thing is 1
        assert_eq!(v.get(s.index_of("isa_depth").unwrap()), 1.0);
    }

    #[test]
    fn adding_related_concepts_increases_relatedness_count() {
        let mut ont = DomainOntology { domain: "finance".into(), roots: vec!["root".into()], hierarchy: HashMap::new(), axioms: vec![] };
        ont.hierarchy.insert("root".into(), vec!["money".into()]);
        ont.hierarchy.insert("money".into(), vec!["cash".into()]);
        let q = SimpleGraphQualifier::default();
        let s = schema();
        let v1 = q.qualify(&ont, "money", &s);
        ont.hierarchy.get_mut("money").unwrap().push("deposit".into());
        let v2 = q.qualify(&ont, "money", &s);
        assert!(v2.get(s.index_of("relatedness_count").unwrap()) > v1.get(s.index_of("relatedness_count").unwrap()));
    }
}
