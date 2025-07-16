//! Category theory implementation for domain communication
//!
//! This module provides the foundational category theory concepts for
//! structure-preserving communication between domains. Based on Applied
//! Category Theory (ACT) principles, it enables domains to communicate
//! while preserving their internal structures and invariants.

pub mod domain_category;
pub mod morphism;
pub mod functor;
pub mod natural_transformation;
pub mod limits;

pub use domain_category::{DomainCategory, DomainObject, DomainMorphism};
pub use morphism::{Morphism, MorphismComposition, MorphismIdentity};
pub use functor::{DomainFunctor, FunctorComposition, FunctorIdentity};
pub use natural_transformation::{NaturalTransformation, NaturalIsomorphism};
pub use limits::{Limit, Colimit, Pullback, Pushout};