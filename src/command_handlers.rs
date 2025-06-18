//! Command handlers for CIM domain aggregates
//!
//! Command handlers process commands, validate business rules, and emit events.
//! They return only acknowledgments, not data - use queries for data retrieval.

use crate::{
    cqrs::CorrelationId,
    domain_events::DomainEventEnum,
    AggregateRoot,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Event publisher trait for handlers to emit events
pub trait EventPublisher: Send + Sync {
    /// Publish domain events
    fn publish_events(&self, events: Vec<DomainEventEnum>, correlation_id: CorrelationId) -> Result<(), String>;
}

/// Mock event publisher for testing
#[derive(Clone)]
pub struct MockEventPublisher {
    published_events: Arc<RwLock<Vec<(DomainEventEnum, CorrelationId)>>>,
}

impl Default for MockEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEventPublisher {
    /// Create a new mock event publisher for testing
    pub fn new() -> Self {
        Self {
            published_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all published events for verification in tests
    pub fn get_published_events(&self) -> Vec<(DomainEventEnum, CorrelationId)> {
        self.published_events.read().unwrap().clone()
    }

    /// Get a reference to self as Any for downcasting
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl EventPublisher for MockEventPublisher {
    fn publish_events(&self, events: Vec<DomainEventEnum>, correlation_id: CorrelationId) -> Result<(), String> {
        let mut published = self.published_events.write().unwrap();
        for event in events {
            published.push((event, correlation_id.clone()));
        }
        Ok(())
    }
}

/// Repository trait for loading and saving aggregates
pub trait AggregateRepository<A: AggregateRoot>: Send + Sync {
    /// Load aggregate by ID
    fn load(&self, id: A::Id) -> Result<Option<A>, String>;

    /// Save aggregate
    fn save(&self, aggregate: &A) -> Result<(), String>;
}

/// In-memory repository for testing
pub struct InMemoryRepository<A: AggregateRoot + Clone + Send + Sync> {
    storage: Arc<RwLock<HashMap<A::Id, A>>>,
}

impl<A: AggregateRoot + Clone + Send + Sync> Default for InMemoryRepository<A>
where
    A::Id: std::hash::Hash + Eq + Clone,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AggregateRoot + Clone + Send + Sync> InMemoryRepository<A>
where
    A::Id: std::hash::Hash + Eq + Clone,
{
    /// Create a new in-memory repository for testing
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<A: AggregateRoot + Clone + Send + Sync> AggregateRepository<A> for InMemoryRepository<A>
where
    A::Id: std::hash::Hash + Eq + Clone,
{
    fn load(&self, id: A::Id) -> Result<Option<A>, String> {
        Ok(self.storage.read().unwrap().get(&id).cloned())
    }

    fn save(&self, aggregate: &A) -> Result<(), String> {
        self.storage.write().unwrap().insert(aggregate.id(), aggregate.clone());
        Ok(())
    }
}

// Location Command Handler has been moved to cim-domain-location

// Workflow Command Handler has been moved to cim-domain-workflow

#[cfg(test)]
mod tests {
    use super::*;

    // Location command handler tests have been moved to cim-domain-location

    // Workflow command handler tests have been moved to cim-domain-workflow
}
