//! Integration module for dependency injection and cross-domain bridges
//!
//! This module provides the infrastructure for integrating domains
//! through dependency injection and bridge patterns.

pub mod dependency_injection;
pub mod domain_bridge;
pub mod service_registry;
pub mod event_bridge;
pub mod cross_domain_search;
pub mod semantic_search_bridge;

pub use dependency_injection::{DependencyContainer, Injectable, ServiceProvider};
pub use domain_bridge::{DomainBridge, BridgeAdapter, MessageTranslator};
pub use service_registry::{ServiceRegistry, ServiceDescriptor, ServiceLifetime};
pub use event_bridge::{EventBridge, EventRouter, EventTransformer};
pub use cross_domain_search::{CrossDomainSearchEngine, CrossDomainQuery, CrossDomainResult};
pub use semantic_search_bridge::{SemanticSearchBridge, CrossDomainQueryBuilder};