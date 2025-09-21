// Copyright (c) 2025 - Cowboy AI, LLC.

//! Category theory implementation for domain communication
//!
//! This module provides the foundational category theory concepts for
//! structure-preserving communication between domains. Based on Applied
//! Category Theory (ACT) principles, it enables domains to communicate
//! while preserving their internal structures and invariants.

pub mod domain_category;
pub mod functor;
pub mod limits;
pub mod morphism;
pub mod natural_transformation;

pub use domain_category::{DomainCategory, DomainMorphism, DomainObject};
pub use functor::{DomainFunctor, FunctorComposition, FunctorIdentity};
pub use limits::{Colimit, Limit, Pullback, Pushout};
pub use morphism::{Morphism, MorphismComposition, MorphismIdentity};
pub use natural_transformation::{NaturalIsomorphism, NaturalTransformation};
