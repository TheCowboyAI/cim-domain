//! NATS KV-based repository implementation with proper type handling

use crate::{
    DomainEntity,
    DomainError,
    persistence::{
        SimpleRepository, SimpleAggregateMetadata,
    },
};
use async_trait::async_trait;
use async_nats::{Client, jetstream};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Configuration for NATS KV repository
#[derive(Debug, Clone)]
pub struct NatsKvConfig {
    /// KV bucket name
    pub bucket_name: String,
    /// Aggregate type name
    pub aggregate_type: String,
    /// History depth for KV entries
    pub history: i64,
    /// TTL for entries (0 = no expiry)
    pub ttl_seconds: u64,
}

impl Default for NatsKvConfig {
    fn default() -> Self {
        Self {
            bucket_name: "aggregates".to_string(),
            aggregate_type: "Unknown".to_string(),
            history: 10,
            ttl_seconds: 0,
        }
    }
}

/// NATS KV-based repository with improved type handling
pub struct NatsKvRepository<T> {
    client: Client,
    config: NatsKvConfig,
    _phantom: PhantomData<T>,
}

impl<T> NatsKvRepository<T> {
    /// Create a new NATS KV repository
    pub async fn new(client: Client, config: NatsKvConfig) -> Result<Self, DomainError> {
        // Create or get the KV bucket
        let js = jetstream::new(client.clone());
        
        let kv_config = jetstream::kv::Config {
            bucket: config.bucket_name.clone(),
            history: config.history,
            max_age: if config.ttl_seconds > 0 {
                std::time::Duration::from_secs(config.ttl_seconds)
            } else {
                std::time::Duration::from_secs(365 * 24 * 60 * 60) // 1 year default
            },
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };
        
        js.create_key_value(kv_config)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to create KV bucket: {}", e),
            })?;
        
        Ok(Self {
            client,
            config,
            _phantom: PhantomData,
        })
    }
    
    /// Build a key for the aggregate
    fn build_key(&self, id: &str) -> String {
        format!("{}.{}", self.config.aggregate_type, id)
    }
    
    /// Build a subject for the aggregate
    fn build_subject(&self, id: &str) -> String {
        format!("kv.{}.{}.{}", self.config.bucket_name, self.config.aggregate_type, id)
    }
}

#[async_trait]
impl<T> SimpleRepository<T> for NatsKvRepository<T>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de> + Send + Sync,
    T::IdType: Send + Sync,
{
    async fn save(&self, aggregate: &T) -> Result<SimpleAggregateMetadata, DomainError> {
        let id = aggregate.id().to_string();
        let key = self.build_key(&id);
        
        // Serialize the aggregate
        let data = serde_json::to_vec(aggregate)
            .map_err(|e| DomainError::SerializationError(e.to_string()))?;
        
        // Get the KV store
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.config.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
            })?;
        
        // Save to KV
        let revision = kv.put(key, data.into())
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to save aggregate: {}", e),
            })?;
        
        Ok(SimpleAggregateMetadata {
            aggregate_id: id.clone(),
            aggregate_type: self.config.aggregate_type.clone(),
            version: revision,
            last_modified: Utc::now(),
            subject: self.build_subject(&id),
        })
    }
    
    async fn load(&self, id: &crate::EntityId<T::IdType>) -> Result<Option<T>, DomainError> {
        let key = self.build_key(&id.to_string());
        
        // Get the KV store
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.config.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
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
                reason: format!("Failed to load aggregate: {}", e),
            }),
        }
    }
    
    async fn exists(&self, id: &crate::EntityId<T::IdType>) -> Result<bool, DomainError> {
        self.load(id).await.map(|opt| opt.is_some())
    }
}

/// Builder for NatsKvRepository
pub struct NatsKvRepositoryBuilder<T> {
    client: Option<Client>,
    config: NatsKvConfig,
    _phantom: PhantomData<T>,
}

impl<T> NatsKvRepositoryBuilder<T> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            client: None,
            config: NatsKvConfig::default(),
            _phantom: PhantomData,
        }
    }
    
    /// Set the NATS client
    pub fn client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }
    
    /// Set the bucket name
    pub fn bucket_name(mut self, name: impl Into<String>) -> Self {
        self.config.bucket_name = name.into();
        self
    }
    
    /// Set the aggregate type
    pub fn aggregate_type(mut self, type_name: impl Into<String>) -> Self {
        self.config.aggregate_type = type_name.into();
        self
    }
    
    /// Set the history depth
    pub fn history(mut self, history: i64) -> Self {
        self.config.history = history;
        self
    }
    
    /// Set the TTL in seconds
    pub fn ttl_seconds(mut self, ttl: u64) -> Self {
        self.config.ttl_seconds = ttl;
        self
    }
    
    /// Build the repository
    pub async fn build(self) -> Result<NatsKvRepository<T>, DomainError> {
        let client = self.client.ok_or_else(|| DomainError::InvalidOperation {
            reason: "NATS client not provided".to_string(),
        })?;
        
        NatsKvRepository::new(client, self.config).await
    }
}

impl<T> Default for NatsKvRepositoryBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = NatsKvConfig::default();
        assert_eq!(config.bucket_name, "aggregates");
        assert_eq!(config.aggregate_type, "Unknown");
        assert_eq!(config.history, 10);
        assert_eq!(config.ttl_seconds, 0);
    }
    
    #[test]
    fn test_builder() {
        let builder = NatsKvRepositoryBuilder::<()>::new()
            .bucket_name("test-bucket")
            .aggregate_type("TestAggregate")
            .history(20)
            .ttl_seconds(3600);
        
        assert_eq!(builder.config.bucket_name, "test-bucket");
        assert_eq!(builder.config.aggregate_type, "TestAggregate");
        assert_eq!(builder.config.history, 20);
        assert_eq!(builder.config.ttl_seconds, 3600);
    }
}