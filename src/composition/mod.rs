//! Composition module for combining domains using category theory
//!
//! This module provides the mechanisms for composing multiple domains
//! into larger structures while preserving their individual properties
//! and maintaining consistency across boundaries.

pub mod domain_composition;
pub mod saga_orchestration;
pub mod topos_structure;
pub mod comprehension_engine;

pub use domain_composition::{DomainComposition, CompositionStrategy};
pub use saga_orchestration::{Saga, SagaStep, SagaState, SagaOrchestrator, RetryPolicy};
pub use topos_structure::{DomainTopos, SubobjectClassifier, InternalLogic};
pub use comprehension_engine::{ComprehensionEngine, Predicate, SubAggregate};