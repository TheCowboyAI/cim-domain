// Copyright (c) 2025 - Cowboy AI, LLC.

//! Core CIM Cognitive Concepts (immutable, foundational)
//!
//! These 10 concepts ground the Ubiquitous Language of any domain modeled in
//! CIM. Domain concepts (types, objects, operations) should map to one or more
//! of these.

use crate::concepts::Concept;

/// The immutable set of core cognitive concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CoreConceptId {
    /// How agents gather sensory input from the environment.
    Perception,
    /// How agents focus mental resources on relevant signals.
    Attention,
    /// How agents retain and recall information across time.
    Memory,
    /// How agents organize knowledge structures and patterns.
    Schema,
    /// How agents reason through challenges to reach outcomes.
    ProblemSolving,
    /// How agents weigh alternatives and select a course of action.
    DecisionMaking,
    /// How agents encode, decode, and exchange meaning.
    Language,
    /// How agents' judgments are shaped by systematic thinking shortcuts.
    CognitiveBias,
    /// How agents reflect on and regulate their own cognition.
    Metacognition,
    /// How agents' cognitive abilities evolve and mature over time.
    CognitiveDevelopment,
}

impl CoreConceptId {
    /// Stable id
    pub fn id(&self) -> &'static str {
        match self {
            CoreConceptId::Perception => "perception",
            CoreConceptId::Attention => "attention",
            CoreConceptId::Memory => "memory",
            CoreConceptId::Schema => "schema",
            CoreConceptId::ProblemSolving => "problem_solving",
            CoreConceptId::DecisionMaking => "decision_making",
            CoreConceptId::Language => "language",
            CoreConceptId::CognitiveBias => "cognitive_bias",
            CoreConceptId::Metacognition => "metacognition",
            CoreConceptId::CognitiveDevelopment => "cognitive_development",
        }
    }

    /// Human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            CoreConceptId::Perception => "Perception",
            CoreConceptId::Attention => "Attention",
            CoreConceptId::Memory => "Memory",
            CoreConceptId::Schema => "Schema",
            CoreConceptId::ProblemSolving => "Problem Solving",
            CoreConceptId::DecisionMaking => "Decision Making",
            CoreConceptId::Language => "Language",
            CoreConceptId::CognitiveBias => "Cognitive Bias",
            CoreConceptId::Metacognition => "Metacognition",
            CoreConceptId::CognitiveDevelopment => "Cognitive Development",
        }
    }
}

/// Return all core concepts as Concept values (for UL seeding or docs)
pub fn core_concepts() -> Vec<Concept> {
    use CoreConceptId::*;
    let mut v = Vec::new();
    for id in [
        Perception,
        Attention,
        Memory,
        Schema,
        ProblemSolving,
        DecisionMaking,
        Language,
        CognitiveBias,
        Metacognition,
        CognitiveDevelopment,
    ] {
        v.push(Concept::new(id.id(), id.name()));
    }
    v
}
