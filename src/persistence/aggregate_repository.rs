//! Aggregate repository for persistence using NATS JetStream

use crate::{
    entity::EntityId,
    events::DomainEvent,
    domain_events::DomainEventEnum,
    DomainEntity,
    infrastructure::{
        EventStore, EventStoreError, StoredEvent,
        event_store::EventMetadata,
        SnapshotStore, SnapshotError, AggregateSnapshot,
    },
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use cim_subject::{Subject, SubjectBuilder};
use cim_ipld::Cid;

/// Metadata for persisted aggregates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetadata {
    /// The aggregate ID
    pub aggregate_id: String,
    /// The aggregate type
    pub aggregate_type: String,
    /// Current version number
    pub version: u64,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
    /// CID of the aggregate state
    pub state_cid: Cid,
    /// Subject path for this aggregate
    pub subject: String,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Options for saving aggregates
#[derive(Debug, Clone, Default)]
pub struct SaveOptions {
    /// Expected version for optimistic concurrency
    pub expected_version: Option<u64>,
    /// Whether to create a snapshot
    pub create_snapshot: bool,
    /// Custom metadata to attach
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Options for loading aggregates
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Load from a specific version
    pub version: Option<u64>,
    /// Load from a specific point in time
    pub as_of: Option<DateTime<Utc>>,
    /// Include event history
    pub include_events: bool,
    /// Use snapshot if available
    pub use_snapshot: bool,
}

/// Options for querying aggregates
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Subject pattern for filtering
    pub subject_pattern: Option<String>,
    /// Filter by aggregate type
    pub aggregate_type: Option<String>,
    /// Filter by tags
    pub tags: Vec<String>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Sort order
    pub sort_by: Option<String>,
}

/// Repository errors
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// Aggregate not found
    #[error("Aggregate not found: {0}")]
    NotFound(String),
    
    /// Version conflict
    #[error("Version conflict: expected {expected}, found {actual}")]
    VersionConflict { expected: u64, actual: u64 },
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Subject error
    #[error("Subject error: {0}")]
    SubjectError(String),
    
    /// IPLD error
    #[error("IPLD error: {0}")]
    IpldError(String),
}

impl From<EventStoreError> for RepositoryError {
    fn from(err: EventStoreError) -> Self {
        RepositoryError::StorageError(err.to_string())
    }
}

impl From<SnapshotError> for RepositoryError {
    fn from(err: SnapshotError) -> Self {
        RepositoryError::StorageError(err.to_string())
    }
}

/// Trait for aggregate persistence
#[async_trait]
pub trait AggregateRepository<T: DomainEntity>: Send + Sync {
    /// Save an aggregate
    async fn save(
        &self,
        aggregate: &T,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError>;
    
    /// Load an aggregate by ID
    async fn load(
        &self,
        id: &EntityId<T::IdType>,
        options: LoadOptions,
    ) -> Result<(T, AggregateMetadata), RepositoryError>;
    
    /// Check if an aggregate exists
    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, RepositoryError>;
    
    /// Delete an aggregate
    async fn delete(&self, id: &EntityId<T::IdType>) -> Result<(), RepositoryError>;
    
    /// Query aggregates
    async fn query(
        &self,
        options: QueryOptions,
    ) -> Result<Vec<AggregateMetadata>, RepositoryError>;
    
    /// Get aggregate history
    async fn get_history(
        &self,
        id: &EntityId<T>,
        from_version: Option<u64>,
        to_version: Option<u64>,
    ) -> Result<Vec<StoredEvent>, RepositoryError>;
    
    /// Get aggregate at a specific version
    async fn get_at_version(
        &self,
        id: &EntityId<T>,
        version: u64,
    ) -> Result<(T, AggregateMetadata), RepositoryError>;
    
    /// Create a snapshot of the aggregate
    async fn create_snapshot(
        &self,
        id: &EntityId<T>,
        aggregate: &T,
    ) -> Result<Cid, RepositoryError>;
}

/// Base repository implementation using event store and snapshot store
pub struct BaseAggregateRepository<T: DomainEntity> {
    event_store: Box<dyn EventStore>,
    snapshot_store: Box<dyn SnapshotStore>,
    aggregate_type: String,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DomainEntity> BaseAggregateRepository<T> {
    /// Create a new repository
    pub fn new(
        event_store: Box<dyn EventStore>,
        snapshot_store: Box<dyn SnapshotStore>,
        aggregate_type: String,
    ) -> Self {
        Self {
            event_store,
            snapshot_store,
            aggregate_type,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Build subject for aggregate
    fn build_subject(&self, id: &EntityId<T::IdType>) -> Result<Subject, RepositoryError> {
        let subject = SubjectBuilder::new()
            .context("domain")
            .aggregate(&self.aggregate_type)
            .build()
            .map_err(|e| RepositoryError::SubjectError(e.to_string()))?;
        
        Ok(subject)
    }
    
    /// Rebuild aggregate from events
    async fn rebuild_from_events(
        &self,
        _id: &EntityId<T>,
        _events: Vec<StoredEvent>,
    ) -> Result<T, RepositoryError> {
        // This would need to be implemented based on your aggregate's apply_event method
        // For now, returning a placeholder error
        Err(RepositoryError::NotFound(
            "Aggregate rebuilding not implemented".to_string()
        ))
    }
}

#[async_trait]
impl<T: DomainEntity + Send + Sync> AggregateRepository<T> for BaseAggregateRepository<T> {
    async fn save(
        &self,
        aggregate: &T,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError> {
        let id = aggregate.id();
        let subject = self.build_subject(&id)?;
        
        // Check version if specified
        if let Some(expected) = options.expected_version {
            let current = self.event_store
                .get_aggregate_version(&id.to_string())
                .await?;
                
            if current != Some(expected) {
                return Err(RepositoryError::VersionConflict {
                    expected,
                    actual: current.unwrap_or(0),
                });
            }
        }
        
        // Append events
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        
        // Convert events to DomainEventEnum (placeholder - needs proper implementation)
        let domain_events: Vec<DomainEventEnum> = vec![];
        
        self.event_store
            .append_events(
                &stream_id,
                &self.aggregate_type,
                domain_events,
                options.expected_version,
                options.metadata.unwrap_or_default(),
            )
            .await?;
        
        // Create snapshot if requested
        let state_cid = if options.create_snapshot {
            self.create_snapshot(&id, aggregate).await?
        } else {
            // Generate CID for current state
            Cid::default() // Placeholder
        };
        
        // Get updated version
        let version = self.event_store
            .get_aggregate_version(&self.aggregate_type, &id.to_string())
            .await?;
        
        Ok(AggregateMetadata {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version,
            last_modified: Utc::now(),
            state_cid,
            subject: subject.to_string(),
            metadata: options.metadata,
        })
    }
    
    async fn load(
        &self,
        id: &EntityId<T::IdType>,
        options: LoadOptions,
    ) -> Result<(T, AggregateMetadata), RepositoryError> {
        let subject = self.build_subject(id)?;
        
        // Try to load from snapshot first if requested
        if options.use_snapshot {
            if let Ok(snapshot) = self.snapshot_store
                .get_latest_snapshot(&self.aggregate_type, &id.to_string())
                .await
            {
                // Deserialize from snapshot
                // This would need proper implementation
                return Err(RepositoryError::NotFound(
                    "Snapshot deserialization not implemented".to_string()
                ));
            }
        }
        
        // Load events
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        let events = self.event_store
            .get_events(&stream_id, 0, None)
            .await?;
        
        if events.is_empty() {
            return Err(RepositoryError::NotFound(id.to_string()));
        }
        
        // Rebuild aggregate from events
        let aggregate = self.rebuild_from_events(id, events.clone()).await?;
        
        // Build metadata
        let version = events.len() as u64;
        let last_event = events.last().unwrap();
        
        Ok((aggregate, AggregateMetadata {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version,
            last_modified: last_event.timestamp,
            state_cid: Cid::default(), // Placeholder
            subject: subject.to_string(),
            metadata: HashMap::new(),
        }))
    }
    
    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, RepositoryError> {
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        let events = self.event_store
            .get_events(&stream_id, 0, Some(1))
            .await?;
        
        Ok(!events.is_empty())
    }
    
    async fn delete(&self, id: &EntityId<T::IdType>) -> Result<(), RepositoryError> {
        // In event sourcing, we typically don't delete events
        // Instead, we might append a "Deleted" event
        Err(RepositoryError::StorageError(
            "Delete operation not supported in event sourcing".to_string()
        ))
    }
    
    async fn query(
        &self,
        options: QueryOptions,
    ) -> Result<Vec<AggregateMetadata>, RepositoryError> {
        // This would need to be implemented with proper querying logic
        // For now, returning empty results
        Ok(vec![])
    }
    
    async fn get_history(
        &self,
        id: &EntityId<T>,
        from_version: Option<u64>,
        to_version: Option<u64>,
    ) -> Result<Vec<StoredEvent>, RepositoryError> {
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        let from = from_version.unwrap_or(0);
        let limit = to_version.map(|to| (to - from) as usize);
        
        self.event_store
            .get_events(&stream_id, from, limit)
            .await
            .map_err(|e| e.into())
    }
    
    async fn get_at_version(
        &self,
        id: &EntityId<T>,
        version: u64,
    ) -> Result<(T, AggregateMetadata), RepositoryError> {
        let events = self.get_history(id, Some(0), Some(version)).await?;
        
        if events.is_empty() {
            return Err(RepositoryError::NotFound(id.to_string()));
        }
        
        let aggregate = self.rebuild_from_events(id, events.clone()).await?;
        let subject = self.build_subject(id)?;
        
        Ok((aggregate, AggregateMetadata {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version,
            last_modified: events.last().unwrap().timestamp,
            state_cid: Cid::default(),
            subject: subject.to_string(),
            metadata: HashMap::new(),
        }))
    }
    
    async fn create_snapshot(
        &self,
        id: &EntityId<T>,
        aggregate: &T,
    ) -> Result<Cid, RepositoryError> {
        // Serialize aggregate to IPLD
        let serialized = serde_json::to_vec(aggregate)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
        
        // Create snapshot
        let snapshot = AggregateSnapshot {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version: 0, // Would need to get actual version
            state: serialized,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        
        self.snapshot_store
            .save_snapshot(snapshot)
            .await
            .map_err(|e| e.into())?;
        
        // Return CID (placeholder for now)
        Ok(Cid::default())
    }
}