// Copyright 2025 Cowboy AI, LLC.

//! Simplified repository implementation for NATS persistence

use crate::{entity::EntityId, DomainEntity, DomainError};
use async_nats::{jetstream, Client};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Simplified aggregate metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleAggregateMetadata {
    /// The aggregate's unique identifier
    pub aggregate_id: String,
    /// The type of the aggregate
    pub aggregate_type: String,
    /// The current version number
    pub version: u64,
    /// When the aggregate was last modified
    pub last_modified: DateTime<Utc>,
    /// The NATS subject used for storage
    pub subject: String,
}

/// Simplified repository trait
#[async_trait]
pub trait SimpleRepository<T>: Send + Sync
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de>,
    T::IdType: Send + Sync,
{
    /// Save an aggregate
    async fn save(&self, aggregate: &T) -> Result<SimpleAggregateMetadata, DomainError>;

    /// Load an aggregate
    async fn load(&self, id: &EntityId<T::IdType>) -> Result<Option<T>, DomainError>;

    /// Check if exists
    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, DomainError>;
}

/// NATS-based simple repository
#[derive(Clone)]
pub struct NatsSimpleRepository {
    client: Client,
    bucket_name: String,
    aggregate_type: String,
}

impl NatsSimpleRepository {
    /// Create a new repository
    pub async fn new(
        client: Client,
        bucket_name: String,
        aggregate_type: String,
    ) -> Result<Self, DomainError> {
        // Create KV bucket
        let js = jetstream::new(client.clone());

        js.create_key_value(jetstream::kv::Config {
            bucket: bucket_name.clone(),
            history: 10,
            ..Default::default()
        })
        .await
        .map_err(|e| DomainError::InvalidOperation {
            reason: format!("Failed to create bucket: {e}"),
        })?;

        Ok(Self {
            client,
            bucket_name,
            aggregate_type,
        })
    }

    fn build_key<T: DomainEntity>(&self, id: &EntityId<T::IdType>) -> String {
        format!("{}.{}", self.aggregate_type, id)
    }
}

#[async_trait]
impl<T> SimpleRepository<T> for NatsSimpleRepository
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de> + Send + Sync,
    T::IdType: Send + Sync,
{
    async fn save(&self, aggregate: &T) -> Result<SimpleAggregateMetadata, DomainError> {
        let id = aggregate.id();
        let key = self.build_key::<T>(&id);

        // Serialize aggregate
        let data = serde_json::to_vec(aggregate)
            .map_err(|e| DomainError::SerializationError(e.to_string()))?;

        // Get KV store
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name).await.map_err(|e| {
            DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {e}"),
            }
        })?;

        // Save to KV
        let revision =
            kv.put(key.clone(), data.into())
                .await
                .map_err(|e| DomainError::InvalidOperation {
                    reason: format!("Failed to save: {e}"),
                })?;

        // Build subject
        let subject_str = format!("domain.{}.state.v1", self.aggregate_type);
        use crate::subject_abstraction::SubjectLike;
        let subject = crate::subject_abstraction::Subject::parse(&subject_str).map_err(|e| {
            DomainError::InvalidOperation {
                reason: format!("Failed to build subject: {e}"),
            }
        })?;

        Ok(SimpleAggregateMetadata {
            aggregate_id: id.to_string(),
            aggregate_type: self.aggregate_type.clone(),
            version: revision,
            last_modified: Utc::now(),
            subject: subject.to_string(),
        })
    }

    async fn load(&self, id: &EntityId<T::IdType>) -> Result<Option<T>, DomainError> {
        let key = self.build_key::<T>(id);

        // Get KV store
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name).await.map_err(|e| {
            DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {e}"),
            }
        })?;

        // Load from KV
        match kv.get(&key).await {
            Ok(Some(entry)) => {
                let aggregate: T = serde_json::from_slice(&entry)
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                Ok(Some(aggregate))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(DomainError::InvalidOperation {
                reason: format!("Failed to load: {e}"),
            }),
        }
    }

    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, DomainError> {
        self.load(id).await.map(|opt: Option<T>| opt.is_some())
    }
}
