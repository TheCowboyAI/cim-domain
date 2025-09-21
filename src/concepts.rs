// Copyright 2025 Cowboy AI, LLC.

//! Concepts and Concept Graph
//!
//! Concepts capture the Ubiquitous Language of a domain. They are syntactic
//! groupings that convey meaning (semantics) and connect domain ideas such as
//! Money, Finance, Trade, and ResourceValue. Concepts are not runtime storage
//! concerns; they are pure, in‑memory structures that classify and relate
//! domain notions for analysis, documentation, and verification.
//!
//! A Concept can be associated to concrete domain primitives (e.g., Value
//! Objects) via the `HasConcept` trait, allowing library types to declare their
//! conceptual identity without additional dependencies.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::formal_domain::DomainConcept;

/// Relationship type between concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConceptRelationshipType {
    /// Is-a (taxonomy) relationship
    IsA,
    /// Part-of (mereology) relationship
    PartOf,
    /// General relatedness (association)
    RelatedTo,
    /// Same-as (equivalence)
    SameAs,
    /// Opposite-of (antonym)
    OppositeOf,
    /// Depends-on (dependency)
    DependsOn,
}

/// A Concept in the Ubiquitous Language
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Concept {
    /// Stable identifier (e.g., "money", "finance")
    pub id: String,
    /// Human readable name (e.g., "Money")
    pub name: String,
    /// Optional description/definition
    pub description: Option<String>,
    /// Synonyms/aliases
    pub synonyms: BTreeSet<String>,
    /// Ontology tags (e.g., Finance, Trade)
    pub tags: BTreeSet<String>,
}

impl Concept {
    /// Create a new concept with an id and name
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            synonyms: BTreeSet::new(),
            tags: BTreeSet::new(),
        }
    }

    /// Set/replace description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    /// Add a synonym
    pub fn add_synonym(mut self, syn: impl Into<String>) -> Self {
        self.synonyms.insert(syn.into());
        self
    }
    /// Add a tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }
}

impl DomainConcept for Concept {}

/// Relationship edge between two concepts
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConceptEdge {
    /// Source concept id
    pub from: String,
    /// Target concept id
    pub to: String,
    /// Relationship type
    pub rel: ConceptRelationshipType,
}

/// In‑memory Concept Graph (pure graph of Concepts + Relationships)
#[derive(Debug, Default, Clone)]
pub struct ConceptGraph {
    nodes: BTreeMap<String, Concept>,
    edges: BTreeSet<ConceptEdge>,
}

impl ConceptGraph {
    /// Create empty concept graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeSet::new(),
        }
    }

    /// Insert or update a concept
    pub fn upsert_concept(&mut self, c: Concept) {
        self.nodes.insert(c.id.clone(), c);
    }

    /// Get a concept by id
    pub fn concept(&self, id: &str) -> Option<&Concept> {
        self.nodes.get(id)
    }

    /// Add a relationship edge
    pub fn relate(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
        rel: ConceptRelationshipType,
    ) {
        self.edges.insert(ConceptEdge {
            from: from.into(),
            to: to.into(),
            rel,
        });
    }

    /// Outgoing neighbors by relationship type
    pub fn neighbors(&self, id: &str, rel: ConceptRelationshipType) -> Vec<&Concept> {
        self.edges
            .iter()
            .filter(|e| e.from == id && e.rel == rel)
            .filter_map(|e| self.nodes.get(&e.to))
            .collect()
    }

    /// Check if a path exists from `from` to `to` following any relationship types
    pub fn path_exists(&self, from: &str, to: &str, max_depth: usize) -> bool {
        if from == to {
            return true;
        }
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut q: VecDeque<(String, usize)> = VecDeque::new();
        q.push_back((from.to_string(), 0));
        while let Some((cur, d)) = q.pop_front() {
            if d >= max_depth {
                continue;
            }
            if !visited.insert(cur.clone()) {
                continue;
            }
            for e in self.edges.iter().filter(|e| e.from == cur) {
                if e.to == to {
                    return true;
                }
                q.push_back((e.to.clone(), d + 1));
            }
        }
        false
    }
}

/// Marker trait for types that declare their Concept id in the Ubiquitous Language
pub trait HasConcept {
    /// Stable Concept id (e.g., "money")
    fn concept_id() -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concept_graph_relationships_and_paths() {
        let mut cs = ConceptGraph::new();
        cs.upsert_concept(Concept::new("domain", "Domain"));
        cs.upsert_concept(Concept::new("schema", "Schema"));
        cs.upsert_concept(Concept::new("language", "Language"));
        cs.upsert_concept(Concept::new("memory", "Memory"));

        cs.relate("domain", "schema", ConceptRelationshipType::RelatedTo);
        cs.relate("domain", "language", ConceptRelationshipType::RelatedTo);
        cs.relate("domain", "memory", ConceptRelationshipType::RelatedTo);

        let neighbors = cs.neighbors("domain", ConceptRelationshipType::RelatedTo);
        let names: BTreeSet<_> = neighbors.iter().map(|c| c.id.as_str()).collect();
        assert!(names.contains("schema") && names.contains("language") && names.contains("memory"));

        assert!(cs.path_exists("domain", "schema", 1));
        assert!(!cs.path_exists("schema", "domain", 1));
    }
}
