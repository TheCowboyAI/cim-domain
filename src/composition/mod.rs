// Copyright 2025 Cowboy AI, LLC.

//! Composition module for combining domains using category theory
//!
//! This module provides the mechanisms for composing multiple domains
//! into larger structures while preserving their individual properties
//! and maintaining consistency across boundaries.

pub mod comprehension_engine;
pub mod domain_composition;
pub mod topos_structure;

pub use comprehension_engine::{ComprehensionEngine, Predicate, SubAggregate};
pub use domain_composition::{CompositionStrategy, DomainComposition};
// Saga orchestration has been moved downstream (workflows live outside core)
pub use topos_structure::{DomainTopos, InternalLogic, SubobjectClassifier};
