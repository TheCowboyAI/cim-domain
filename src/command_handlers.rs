// Copyright 2025 Cowboy AI, LLC.

//! Command handlers for CIM domain aggregates
//!
//! Command handlers process commands, validate business rules, and emit events.
//! They return only acknowledgments, not data - use queries for data retrieval.

use crate::{cqrs::CorrelationId, AggregateRoot, DomainEvent};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Event publisher trait for handlers to emit events
pub trait EventPublisher: Send + Sync {
    /// Publish domain events
    fn publish_events(
        &self,
        events: Vec<Box<dyn DomainEvent>>,
        correlation_id: CorrelationId,
    ) -> Result<(), String>;
}

/// Mock event publisher for testing
#[derive(Clone)]
pub struct MockEventPublisher {
    published_events: Arc<RwLock<Vec<(String, CorrelationId)>>>,
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
    pub fn get_published_events(&self) -> Vec<(String, CorrelationId)> {
        // In core domain we only track event type names here to avoid cloning trait objects.
        self.published_events
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect()
    }

    /// Get a reference to self as Any for downcasting
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl EventPublisher for MockEventPublisher {
    fn publish_events(
        &self,
        events: Vec<Box<dyn DomainEvent>>,
        correlation_id: CorrelationId,
    ) -> Result<(), String> {
        let mut published = self.published_events.write().unwrap();
        for event in events.into_iter() {
            published.push((event.event_type().to_string(), correlation_id));
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
        self.storage
            .write()
            .unwrap()
            .insert(aggregate.id(), aggregate.clone());
        Ok(())
    }
}

// Location Command Handler has been moved to cim-domain-location

// Workflow Command Handler has been moved to cim-domain-workflow

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainEvent;
    use uuid::Uuid;

    #[derive(Debug)]
    struct TestEvent(Uuid);
    impl DomainEvent for TestEvent {
        fn aggregate_id(&self) -> Uuid { self.0 }
        fn event_type(&self) -> &'static str { "TestEvent" }
    }

    #[test]
    fn test_mock_event_publisher_records_events() {
        let publisher = MockEventPublisher::new();
        let correlation = CorrelationId::Single(Uuid::new_v4());
        let events: Vec<Box<dyn DomainEvent>> = vec![
            Box::new(TestEvent(Uuid::new_v4())),
            Box::new(TestEvent(Uuid::new_v4())),
        ];

        publisher.publish_events(events, correlation).unwrap();
        let published = publisher.get_published_events();

        assert_eq!(published.len(), 2);
        for (etype, corr) in published {
            assert_eq!(etype, "TestEvent");
            assert_eq!(corr, correlation);
        }
    }

    #[derive(Clone)]
    struct SimpleAggregate {
        id: crate::entity::EntityId<crate::entity::AggregateMarker>,
        version: u64,
    }
    impl AggregateRoot for SimpleAggregate {
        type Id = crate::entity::EntityId<crate::entity::AggregateMarker>;
        fn id(&self) -> Self::Id { self.id }
        fn version(&self) -> u64 { self.version }
        fn increment_version(&mut self) { self.version += 1; }
    }

    #[test]
    fn test_in_memory_repository_save_and_load() {
        type AId = crate::entity::EntityId<crate::entity::AggregateMarker>;
        let repo: InMemoryRepository<SimpleAggregate> = InMemoryRepository::new();
        let agg = SimpleAggregate { id: AId::new(), version: 0 };

        // Save, then load
        repo.save(&agg).unwrap();
        let loaded = repo.load(agg.id()).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, agg.id);
        assert_eq!(loaded.version, agg.version);
    }
}
