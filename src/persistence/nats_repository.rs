// Copyright 2025 Cowboy AI, LLC.

//! NATS-specific repository implementation using JetStream and cim-subject

use crate::{
    entity::EntityId,
    events::DomainEvent,
    DomainEntity,
    infrastructure::{
        JetStreamEventStore,
        jetstream_event_store::JetStreamConfig,
        SnapshotStore,
    },
    persistence::{
        RepositoryError, AggregateMetadata,
        SaveOptions, LoadOptions, QueryOptions,
        aggregate_repository::{AggregateRepository, BaseAggregateRepository},
    },
};
use async_trait::async_trait;
use async_nats::{Client, jetstream};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::subject_abstraction::{Subject, Pattern};
use cim_ipld::Cid;

/// Configuration for NATS repository
#[derive(Debug, Clone)]
pub struct NatsRepositoryConfig {
    /// Stream name for events
    pub event_stream_name: String,
    /// Stream name for aggregates
    pub aggregate_stream_name: String,
    /// KV bucket for read models
    pub read_model_bucket: String,
    /// KV bucket for snapshots
    pub snapshot_bucket: String,
    /// Subject prefix for events
    pub event_subject_prefix: String,
    /// Subject prefix for aggregates
    pub aggregate_subject_prefix: String,
    /// Cache size for aggregates
    pub cache_size: usize,
    /// Enable content addressing
    pub enable_ipld: bool,
}

impl Default for NatsRepositoryConfig {
    fn default() -> Self {
        Self {
            event_stream_name: "CIM-EVENTS".to_string(),
            aggregate_stream_name: "CIM-AGGREGATES".to_string(),
            read_model_bucket: "cim-read-models".to_string(),
            snapshot_bucket: "cim-snapshots".to_string(),
            event_subject_prefix: "cim.events".to_string(),
            aggregate_subject_prefix: "cim.aggregates".to_string(),
            cache_size: 1000,
            enable_ipld: true,
        }
    }
}

/// NATS repository errors
#[derive(Debug, thiserror::Error)]
pub enum NatsRepositoryError {
    /// NATS error
    #[error("NATS error: {0}")]
    NatsError(String),
    
    /// JetStream error
    #[error("JetStream error: {0}")]
    JetStreamError(String),
    
    /// Subject error
    #[error("Subject error: {0}")]
    SubjectError(String),
    
    /// IPLD error
    #[error("IPLD error: {0}")]
    IpldError(String),
    
    /// Repository error
    #[error("Repository error: {0}")]
    RepositoryError(#[from] RepositoryError),
}

/// NATS-based repository implementation
pub struct NatsRepository<T: DomainEntity> {
    client: Client,
    config: NatsRepositoryConfig,
    event_store: Arc<JetStreamEventStore>,
    snapshot_store: Arc<dyn SnapshotStore>,
    aggregate_type: String,
    content_chains: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: DomainEntity> NatsRepository<T> {
    /// Create a new NATS repository
    pub async fn new(
        client: Client,
        config: NatsRepositoryConfig,
        aggregate_type: String,
    ) -> Result<Self, NatsRepositoryError> {
        // Create event store
        let event_config = JetStreamConfig {
            stream_name: config.event_stream_name.clone(),
            stream_subjects: vec![format!("{}.>", config.event_subject_prefix)],
            cache_size: config.cache_size,
            subject_prefix: config.event_subject_prefix.clone(),
        };
        
        let event_store = Arc::new(
            JetStreamEventStore::new(client.clone(), event_config)
                .await
                .map_err(|e| NatsRepositoryError::JetStreamError(e.to_string()))?
        );
        
        // Create snapshot store
        let js_context = Arc::new(jetstream::new(client.clone()));
        let snapshot_store = Arc::new(
            crate::infrastructure::snapshot_store::JetStreamSnapshotStore::new(
                js_context,
                config.snapshot_bucket.clone(),
            )
            .await
            .map_err(|e| NatsRepositoryError::JetStreamError(e.to_string()))?
        );
        
        // Create aggregate stream
        let js = jetstream::new(client.clone());
        let aggregate_config = jetstream::stream::Config {
            name: config.aggregate_stream_name.clone(),
            subjects: vec![format!("{}.>", config.aggregate_subject_prefix)],
            retention: jetstream::stream::RetentionPolicy::Limits,
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };
        
        js.create_stream(aggregate_config)
            .await
            .map_err(|e| NatsRepositoryError::JetStreamError(e.to_string()))?;
        
        Ok(Self {
            client,
            config,
            event_store,
            snapshot_store,
            aggregate_type,
            content_chains: Arc::new(RwLock::new(HashMap::new())),
            _phantom: std::marker::PhantomData,
        })
    }
    
    /// Build subject for aggregate using cim-subject
    fn build_aggregate_subject(&self, id: &EntityId<T::IdType>, operation: &str) -> Result<Subject, NatsRepositoryError> {
        let subject_str = format!("{}.domain.{}.{}.{}.v1",
            self.config.aggregate_subject_prefix,
            self.aggregate_type,
            id,
            operation
        );
        
        use crate::subject_abstraction::SubjectLike;
        let subject = Subject::parse(&subject_str)
            .map_err(|e| NatsRepositoryError::SubjectError(e.to_string()))?;
        Ok(subject)
    }
    
    /// Build event subject
    fn build_event_subject(&self, event_type: &str, id: &EntityId<T::IdType>) -> Result<Subject, NatsRepositoryError> {
        let subject_str = format!("{}.domain.{}.{}.{}.v1",
            self.config.event_subject_prefix,
            self.aggregate_type,
            id,
            event_type
        );
        
        use crate::subject_abstraction::SubjectLike;
        let subject = Subject::parse(&subject_str)
            .map_err(|e| NatsRepositoryError::SubjectError(e.to_string()))?;
        Ok(subject)
    }
    
    /// Store aggregate state in NATS with IPLD
    async fn store_aggregate_state<U>(
        &self,
        id: &EntityId<T::IdType>,
        aggregate: &U,
        metadata: &AggregateMetadata,
    ) -> Result<Cid, NatsRepositoryError>
    where
        U: Serialize,
    {
        if !self.config.enable_ipld {
            return Ok(Cid::default());
        }
        
        // Serialize aggregate with IPLD
        let content = serde_json::to_vec(aggregate)
            .map_err(|e| NatsRepositoryError::IpldError(e.to_string()))?;
        
        // Store content
        let mut chains = self.content_chains.write().await;
        let chain_key = format!("{}-{}", self.aggregate_type, id);
        chains.insert(chain_key.clone(), content.clone());
        
        // Calculate CID (simplified - in real implementation would use proper IPLD)
        let cid = Cid::default();
        
        // Publish state to NATS
        let subject = self.build_aggregate_subject(id, "state")?;
        let state_message = AggregateStateMessage {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version: metadata.version,
            cid: cid.to_string(),
            subject: subject.to_string(),
            metadata: metadata.clone(),
            chain_head: None,
        };
        
        let payload = serde_json::to_vec(&state_message)
            .map_err(|e| NatsRepositoryError::IpldError(e.to_string()))?;
        
        self.client
            .publish(subject.to_string(), payload.into())
            .await
            .map_err(|e| NatsRepositoryError::NatsError(e.to_string()))?;
        
        Ok(cid)
    }
    
    /// Load aggregate state from NATS
    async fn load_aggregate_state(
        &self,
        id: &EntityId<T::IdType>,
        version: Option<u64>,
    ) -> Result<Option<(Vec<u8>, AggregateMetadata)>, NatsRepositoryError> {
        // Query for aggregate state
        let subject = self.build_aggregate_subject(id, "state")?;
        let pattern = Pattern::parse(&subject.to_string())?;
        
        // Get latest state from JetStream
        let js = jetstream::new(self.client.clone());
        let stream = js
            .get_stream(&self.config.aggregate_stream_name)
            .await
            .map_err(|e| NatsRepositoryError::JetStreamError(e.to_string()))?;
        
        // Get last message for this subject
        let message = stream
            .get_last_raw_message_by_subject(&subject.to_string())
            .await;
        
        match message {
            Ok(raw_msg) => {
            let state_msg: AggregateStateMessage = serde_json::from_slice(&raw_msg.payload)
                .map_err(|e| NatsRepositoryError::IpldError(e.to_string()))?;
            
            // If specific version requested, check it matches
            if let Some(v) = version {
                if state_msg.version != v {
                    return Ok(None);
                }
            }
            
            // Get content from chain if IPLD enabled
            if self.config.enable_ipld && !state_msg.cid.is_empty() {
                let chains = self.content_chains.read().await;
                let chain_key = format!("{}-{}", self.aggregate_type, id);
                
                if let Some(chain) = chains.get(&chain_key) {
                    // Get content by CID
                    let cid = Cid::try_from(state_msg.cid.as_str())
                        .map_err(|e| NatsRepositoryError::IpldError(e.to_string()))?;
                    
                    // In a real implementation, we'd retrieve from IPLD store
                    // For now, we'll use the raw message payload
                    return Ok(Some((raw_msg.payload.to_vec(), state_msg.metadata)));
                }
            }
            
                Ok(Some((raw_msg.payload.to_vec(), state_msg.metadata)))
            }
            Err(_) => Ok(None),
        }
    }
}

/// Message format for aggregate state
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AggregateStateMessage {
    aggregate_id: String,
    aggregate_type: String,
    version: u64,
    cid: String,
    subject: String,
    metadata: AggregateMetadata,
    chain_head: Option<String>,
}

#[async_trait]
impl<T: DomainEntity + Send + Sync + Serialize + for<'de> Deserialize<'de>> AggregateRepository<T> for NatsRepository<T> {
    async fn save(
        &self,
        aggregate: &T,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError> {
        // First save events through event store
        let stream_id = format!("{}-{}", self.aggregate_type, aggregate.id());
        self.event_store
            .append_events(&stream_id, events.clone(), options.expected_version)
            .await?;
        
        // Get current version
        let version = self.event_store
            .get_aggregate_version(&self.aggregate_type, &aggregate.id().to_string())
            .await?;
        
        // Build metadata
        let subject = self.build_aggregate_subject(&aggregate.id(), "state")
            .map_err(|e| RepositoryError::SubjectError(e.to_string()))?;
        
        let mut metadata = AggregateMetadata {
            aggregate_id: aggregate.id().to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version,
            last_modified: Utc::now(),
            state_cid: Cid::default(),
            subject: subject.to_string(),
            metadata: options.metadata,
        };
        
        // Store aggregate state and get CID
        let cid = self.store_aggregate_state(&aggregate.id(), aggregate, &metadata)
            .await
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?;
        
        metadata.state_cid = cid;
        
        // Create snapshot if requested
        if options.create_snapshot {
            self.create_snapshot(&aggregate.id(), aggregate).await?;
        }
        
        Ok(metadata)
    }
    
    async fn load(
        &self,
        id: &EntityId<T::IdType>,
        options: LoadOptions,
    ) -> Result<(T, AggregateMetadata), RepositoryError> {
        // Try to load from aggregate state first
        if let Some((data, metadata)) = self.load_aggregate_state(id, options.version)
            .await
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
        {
            let aggregate: T = serde_json::from_slice(&data)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
            
            return Ok((aggregate, metadata));
        }
        
        // Fall back to event rebuilding
        let base_repo = BaseAggregateRepository::new(
            Box::new(self.event_store.as_ref().clone()) as Box<dyn crate::infrastructure::EventStore>,
            Box::new(self.snapshot_store.as_ref().clone()) as Box<dyn SnapshotStore>,
            self.aggregate_type.clone(),
        );
        
        base_repo.load(id, options).await
    }
    
    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, RepositoryError> {
        // Check if aggregate state exists
        if let Some(_) = self.load_aggregate_state(id, None)
            .await
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
        {
            return Ok(true);
        }
        
        // Fall back to checking events
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        let events = self.event_store
            .get_events(&stream_id, 0, Some(1))
            .await?;
        
        Ok(!events.is_empty())
    }
    
    async fn delete(&self, id: &EntityId<T::IdType>) -> Result<(), RepositoryError> {
        // Append a deleted event
        let deleted_event = DeletedEvent {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            deleted_at: Utc::now(),
        };
        
        let stream_id = format!("{}-{}", self.aggregate_type, id);
        self.event_store
            .append_events(&stream_id, vec![Box::new(deleted_event)], None)
            .await?;
        
        Ok(())
    }
    
    async fn query(
        &self,
        options: QueryOptions,
    ) -> Result<Vec<AggregateMetadata>, RepositoryError> {
        // Build query pattern
        let pattern = if let Some(ref pattern_str) = options.subject_pattern {
            Pattern::parse(pattern_str)
                .map_err(|e| RepositoryError::SubjectError(e.to_string()))?
        } else {
            // Default pattern for aggregate type
            let pattern_str = format!("{}.domain.{}.>", 
                self.config.aggregate_subject_prefix,
                options.aggregate_type.as_ref().unwrap_or(&self.aggregate_type)
            );
            Pattern::parse(&pattern_str)
                .map_err(|e| RepositoryError::SubjectError(e.to_string()))?
        };
        
        // Query JetStream for matching subjects
        // This is a simplified implementation
        // In production, you'd want more sophisticated querying
        let results = Vec::new();
        
        Ok(results)
    }
    
    async fn get_history(
        &self,
        id: &EntityId<T>,
        from_version: Option<u64>,
        to_version: Option<u64>,
    ) -> Result<Vec<crate::infrastructure::StoredEvent>, RepositoryError> {
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
        // Try to load specific version from state
        if let Some((data, mut metadata)) = self.load_aggregate_state(id, Some(version))
            .await
            .map_err(|e| RepositoryError::StorageError(e.to_string()))?
        {
            metadata.version = version;
            let aggregate: T = serde_json::from_slice(&data)
                .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
            
            return Ok((aggregate, metadata));
        }
        
        // Fall back to base implementation
        let base_repo = BaseAggregateRepository::new(
            Box::new(self.event_store.clone()),
            self.snapshot_store.clone(),
            self.aggregate_type.clone(),
        );
        
        base_repo.get_at_version(id, version).await
    }
    
    async fn create_snapshot(
        &self,
        id: &EntityId<T>,
        aggregate: &T,
    ) -> Result<Cid, RepositoryError> {
        // Serialize aggregate
        let serialized = serde_json::to_vec(aggregate)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
        
        // Get current version
        let version = self.event_store
            .get_aggregate_version(&self.aggregate_type, &id.to_string())
            .await?;
        
        // Create snapshot
        let snapshot = crate::infrastructure::AggregateSnapshot {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version,
            state: serialized.clone(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        
        self.snapshot_store
            .save_snapshot(snapshot)
            .await?;
        
        // Store snapshot content if IPLD enabled
        if self.config.enable_ipld {
            let mut chains = self.content_chains.write().await;
            let chain_key = format!("{}-{}-snapshot-v{}", self.aggregate_type, id, version);
            chains.insert(chain_key, serialized);
            
            // In real implementation, would calculate proper CID
            Ok(Cid::default())
        } else {
            Ok(Cid::default())
        }
    }
}

/// Event for marking aggregates as deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeletedEvent {
    aggregate_id: String,
    aggregate_type: String,
    deleted_at: DateTime<Utc>,
}

impl DomainEvent for DeletedEvent {
    fn event_type(&self) -> &'static str {
        "AggregateDeleted"
    }
    
    fn aggregate_id(&self) -> Uuid {
        Uuid::parse_str(&self.aggregate_id).unwrap_or_default()
    }
    
    fn subject(&self) -> String {
        format!("{}.deleted.v1", self.aggregate_type)
    }
}