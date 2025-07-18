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

pub mod dependency_injection;
pub mod domain_bridge;
pub mod service_registry;
pub mod event_bridge;
pub mod cross_domain_search;
pub mod semantic_search_bridge;
/// Aggregate event routing for cross-aggregate communication
pub mod aggregate_event_router;

#[cfg(test)]
mod tests;

pub use dependency_injection::{DependencyContainer, ServiceProvider, ContainerBuilder};
pub use domain_bridge::{DomainBridge, BridgeAdapter, BridgeRegistry, MessageTranslator, PropertyBasedTranslator, SerializedCommand, TranslationContext};
pub use service_registry::{ServiceRegistry, ServiceDescriptor, ServiceLifetime};
pub use event_bridge::{EventBridge, EventRouter, EventTransformer};
pub use cross_domain_search::{CrossDomainSearchEngine, CrossDomainQuery, CrossDomainResult};
pub use semantic_search_bridge::{SemanticSearchBridge, CrossDomainQueryBuilder};
pub use aggregate_event_router::{AggregateEventRouter, AggregateEventHandler};

// Re-export saga orchestration from infrastructure
pub use crate::infrastructure::saga::{
    SagaCoordinator,
    SagaDefinition,
    ProcessManager,
    ProcessPolicy,
};