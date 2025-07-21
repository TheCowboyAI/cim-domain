// Copyright 2025 Cowboy AI, LLC.

//! Integration module for dependency injection and cross-domain bridges
//!
//! This module provides the infrastructure for integrating domains
//! through dependency injection and bridge patterns.
//!
//! Key features:
//! - Dependency injection container
//! - Service registry and lifecycle management
//! - Domain bridges for cross-domain communication
//! - Event routing and transformation
//! - Cross-domain search using category theory
//! - Semantic search integration
//! - Aggregate event routing for consistency
//! - Saga orchestration (via infrastructure::saga)

/// Aggregate event routing for cross-aggregate communication
pub mod aggregate_event_router;
pub mod cross_domain_search;
pub mod dependency_injection;
pub mod domain_bridge;
pub mod event_bridge;
pub mod semantic_search_bridge;
pub mod service_registry;

#[cfg(test)]
mod tests;

pub use aggregate_event_router::{AggregateEventHandler, AggregateEventRouter};
pub use cross_domain_search::{CrossDomainQuery, CrossDomainResult, CrossDomainSearchEngine};
pub use dependency_injection::{ContainerBuilder, DependencyContainer, ServiceProvider};
pub use domain_bridge::{
    BridgeAdapter, BridgeRegistry, DomainBridge, MessageTranslator, PropertyBasedTranslator,
    SerializedCommand, TranslationContext,
};
pub use event_bridge::{EventBridge, EventRouter, EventTransformer};
pub use semantic_search_bridge::{CrossDomainQueryBuilder, SemanticSearchBridge};
pub use service_registry::{ServiceDescriptor, ServiceLifetime, ServiceRegistry};

// Re-export saga orchestration from infrastructure
pub use crate::infrastructure::saga::{
    ProcessManager, ProcessPolicy, SagaCoordinator, SagaDefinition,
};
