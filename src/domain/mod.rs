// Copyright 2025 Cowboy AI, LLC.

//! Cross-domain invariants and logic
//!
//! This module implements the mechanisms for maintaining invariants
//! across domain boundaries and enforcing cross-domain business rules.

pub mod cross_domain_rules;
pub mod domain_predicates;
pub mod invariants;
pub mod semantic_analyzer;
pub mod value_objects;
pub mod org_chart;

pub use cross_domain_rules::{CrossDomainRule, RuleEngine, RuleEvaluationResult};
pub use domain_predicates::{DomainPredicate, PredicateEvaluator};
pub use invariants::{DomainInvariant, InvariantChecker, InvariantViolation};
pub use semantic_analyzer::{ConceptAlignment, SemanticAnalyzer, SemanticDistance};
pub use value_objects::{
    ConversionRate, Currency, CurrencyType, Money, PhysicalAddress, RateProvider, Temperature,
    TemperatureScale,
};
pub use org_chart::{
    OrganizationMarker, LocationMarker, PersonMarker, PolicyVO, ProjectAggregate, ProjectCommand,
    ProjectEvent, ProjectMarker, ProjectState,
};
