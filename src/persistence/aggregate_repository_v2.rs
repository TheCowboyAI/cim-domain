// Copyright 2025 Cowboy AI, LLC.

//! Simplified aggregate repository implementation that avoids complex type dependencies

use crate::{
    DomainEntity,
    events::DomainEvent,
    infrastructure::{
        EventStore, EventStoreError,
        event_store::EventMetadata,
    },
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for persisted aggregates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetadata {
    /// Aggregate ID
    pub aggregate_id: String,
    /// Aggregate type name
    pub aggregate_type: String,
    /// Current version number
    pub version: u64,
    /// Last modification timestamp
    pub last_modified: DateTime<Utc>,
    /// Subject for this aggregate
    pub subject: String,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Options for saving aggregates
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// Expected version for optimistic concurrency control
    pub expected_version: Option<u64>,
    /// Whether to create a snapshot after saving
    pub create_snapshot: bool,
    /// Additional metadata to store
    pub metadata: Option<EventMetadata>,
}

/// Options for loading aggregates
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Specific version to load
    pub version: Option<u64>,
    /// Whether to use snapshots if available
    pub use_snapshot: bool,
    /// Maximum events to replay
    pub max_events: Option<usize>,
}

/// Repository errors
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// Aggregate not found
    #[error("Aggregate not found: {0}")]
    NotFound(String),
    
    /// Version conflict
    #[error("Version conflict: expected {expected}, actual {actual}")]
    VersionConflict { 
        /// The expected version
        expected: u64, 
        /// The actual version found
        actual: u64 
    },
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Event store error
    #[error("Event store error: {0}")]
    EventStoreError(String),
}

impl From<EventStoreError> for RepositoryError {
    fn from(err: EventStoreError) -> Self {
        RepositoryError::EventStoreError(err.to_string())
    }
}

/// Simplified aggregate repository trait that avoids complex type dependencies
#[async_trait]
pub trait AggregateRepository: Send + Sync {
    /// The aggregate type this repository handles
    type Aggregate: DomainEntity + Serialize + for<'de> Deserialize<'de>;
    
    /// Save an aggregate with its events
    async fn save(
        &self,
        aggregate: &Self::Aggregate,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError>;
    
    /// Load an aggregate by ID
    async fn load(
        &self,
        id: &str,
        options: LoadOptions,
    ) -> Result<(Self::Aggregate, AggregateMetadata), RepositoryError>;
    
    /// Check if an aggregate exists
    async fn exists(&self, id: &str) -> Result<bool, RepositoryError>;
    
    /// Delete an aggregate
    async fn delete(&self, id: &str) -> Result<(), RepositoryError>;
}

/// Event-sourced repository implementation
pub struct EventSourcedRepository<T> {
    event_store: Box<dyn EventStore>,
    aggregate_type: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> EventSourcedRepository<T> {
    /// Create a new event-sourced repository
    pub fn new(event_store: Box<dyn EventStore>, aggregate_type: String) -> Self {
        Self {
            event_store,
            aggregate_type,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T> AggregateRepository for EventSourcedRepository<T>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de> + Send + Sync,
    T::IdType: Send + Sync,
{
    type Aggregate = T;
    
    async fn save(
        &self,
        aggregate: &T,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError> {
        let aggregate_id = aggregate.id().to_string();
        
        // For now, we'll skip the event store integration
        // This would need proper DomainEventEnum conversion
        
        Ok(AggregateMetadata {
            aggregate_id: aggregate_id.clone(),
            aggregate_type: self.aggregate_type.clone(),
            version: options.expected_version.unwrap_or(0) + events.len() as u64,
            last_modified: Utc::now(),
            subject: format!("domain.{}.{}", self.aggregate_type, aggregate_id),
            metadata: HashMap::new(),
        })
    }
    
    async fn load(
        &self,
        id: &str,
        _options: LoadOptions,
    ) -> Result<(T, AggregateMetadata), RepositoryError> {
        // This would need proper event replay implementation
        Err(RepositoryError::NotFound(id.to_string()))
    }
    
    async fn exists(&self, id: &str) -> Result<bool, RepositoryError> {
        // Check with event store
        let version = self.event_store
            .get_aggregate_version(id)
            .await?;
        Ok(version.is_some())
    }
    
    async fn delete(&self, _id: &str) -> Result<(), RepositoryError> {
        // Would append a deletion event
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityId;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestAggregate {
        id: EntityId<TestMarker>,
        name: String,
    }
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct TestMarker;
    
    impl DomainEntity for TestAggregate {
        type IdType = TestMarker;
        
        fn id(&self) -> EntityId<Self::IdType> {
            self.id
        }
    }
    
    #[test]
    fn test_save_options_default() {
        let options = SaveOptions::default();
        assert!(options.expected_version.is_none());
        assert!(!options.create_snapshot);
        assert!(options.metadata.is_none());
    }
    
    #[test]
    fn test_aggregate_creation() {
        let id = EntityId::<TestMarker>::new();
        let aggregate = TestAggregate {
            id,
            name: "Test Aggregate".to_string(),
        };
        
        // Verify DomainEntity implementation
        assert_eq!(aggregate.id(), id);
        assert_eq!(aggregate.name, "Test Aggregate");
        
        // Test serialization
        let serialized = serde_json::to_string(&aggregate).unwrap();
        let deserialized: TestAggregate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, aggregate.name);
    }
    
    #[test]
    fn test_aggregate_metadata() {
        let metadata = AggregateMetadata {
            aggregate_id: "test-123".to_string(),
            aggregate_type: "TestAggregate".to_string(),
            version: 5,
            last_modified: Utc::now(),
            subject: "domain.test.123".to_string(),
            metadata: HashMap::new(),
        };
        
        assert_eq!(metadata.aggregate_id, "test-123");
        assert_eq!(metadata.version, 5);
    }
}