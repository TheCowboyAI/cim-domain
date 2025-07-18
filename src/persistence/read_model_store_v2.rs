// Copyright 2025 Cowboy AI, LLC.

//! Simplified read model storage using NATS KV

use crate::{
    DomainError,
    events::DomainEvent,
};
use async_nats::{Client, jetstream};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Read model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadModelMetadata {
    /// Read model ID
    pub id: String,
    /// Read model type/name
    pub model_type: String,
    /// Version of the model schema
    pub schema_version: u32,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
    /// Last processed event position
    pub last_event_position: u64,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of a projection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionStatus {
    /// Projection is up to date
    UpToDate,
    /// Projection is being rebuilt
    Rebuilding,
    /// Projection is lagging behind events
    Lagging { 
        /// Number of events the projection is behind
        behind_by: u64 
    },
    /// Projection has failed
    Failed,
}

/// Trait for read models
pub trait ReadModel: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Get the model type name
    fn model_type() -> &'static str;
    
    /// Get the model ID
    fn id(&self) -> &str;
    
    /// Apply an event to update the model
    fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError>;
    
    /// Get the schema version
    fn schema_version() -> u32 {
        1
    }
}

/// Simple read model store using NATS KV
pub struct NatsReadModelStore {
    client: Client,
    bucket_name: String,
    cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl NatsReadModelStore {
    /// Create a new read model store
    pub async fn new(client: Client, bucket_name: String) -> Result<Self, DomainError> {
        let js = jetstream::new(client.clone());
        
        // Create KV bucket for read models
        js.create_key_value(jetstream::kv::Config {
            bucket: bucket_name.clone(),
            storage: jetstream::stream::StorageType::File,
            history: 5,
            ..Default::default()
        })
        .await
        .map_err(|e| DomainError::InvalidOperation {
            reason: format!("Failed to create KV bucket: {}", e),
        })?;
        
        Ok(Self {
            client,
            bucket_name,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Save a read model
    pub async fn save<T: ReadModel>(
        &self,
        model: &T,
        metadata: ReadModelMetadata,
    ) -> Result<(), DomainError> {
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
            })?;
        
        // Create composite value with model and metadata
        let composite = serde_json::json!({
            "model": model,
            "metadata": metadata,
        });
        
        let key = format!("{}.{}", T::model_type(), model.id());
        let data = serde_json::to_vec(&composite)
            .map_err(|e| DomainError::SerializationError(e.to_string()))?;
        
        kv.put(key.clone(), data.into())
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to save read model: {}", e),
            })?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(key, composite);
        
        Ok(())
    }
    
    /// Load a read model
    pub async fn load<T: ReadModel>(
        &self,
        id: &str,
    ) -> Result<Option<(T, ReadModelMetadata)>, DomainError> {
        let key = format!("{}.{}", T::model_type(), id);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&key) {
                let model: T = serde_json::from_value(cached["model"].clone())
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                let metadata: ReadModelMetadata = serde_json::from_value(cached["metadata"].clone())
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                return Ok(Some((model, metadata)));
            }
        }
        
        // Load from KV
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
            })?;
        
        match kv.get(&key).await {
            Ok(Some(entry)) => {
                let composite: serde_json::Value = serde_json::from_slice(&entry)
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                
                let model: T = serde_json::from_value(composite["model"].clone())
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                let metadata: ReadModelMetadata = serde_json::from_value(composite["metadata"].clone())
                    .map_err(|e| DomainError::SerializationError(e.to_string()))?;
                
                // Update cache
                let mut cache = self.cache.write().await;
                cache.insert(key, composite);
                
                Ok(Some((model, metadata)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(DomainError::InvalidOperation {
                reason: format!("Failed to load read model: {}", e),
            }),
        }
    }
    
    /// Delete a read model
    pub async fn delete(&self, model_type: &str, id: &str) -> Result<(), DomainError> {
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
            })?;
        
        let key = format!("{}.{}", model_type, id);
        kv.delete(&key)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to delete read model: {}", e),
            })?;
        
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(&key);
        
        Ok(())
    }
    
    /// Update projection status
    pub async fn update_projection_status(
        &self,
        model_type: &str,
        status: ProjectionStatus,
    ) -> Result<(), DomainError> {
        let js = jetstream::new(self.client.clone());
        let kv = js.get_key_value(&self.bucket_name)
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to get KV store: {}", e),
            })?;
        
        let key = format!("{}.projection_status", model_type);
        let data = serde_json::to_vec(&status)
            .map_err(|e| DomainError::SerializationError(e.to_string()))?;
        
        kv.put(key, data.into())
            .await
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Failed to update projection status: {}", e),
            })?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestReadModel {
        id: String,
        count: u32,
        last_event: Option<String>,
    }
    
    impl ReadModel for TestReadModel {
        fn model_type() -> &'static str {
            "TestReadModel"
        }
        
        fn id(&self) -> &str {
            &self.id
        }
        
        fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError> {
            self.count += 1;
            self.last_event = Some(event.event_type().to_string());
            Ok(())
        }
    }
    
    #[test]
    fn test_read_model_implementation() {
        let model = TestReadModel {
            id: "test-123".to_string(),
            count: 5,
            last_event: Some("EventProcessed".to_string()),
        };
        
        // Verify ReadModel implementation
        assert_eq!(TestReadModel::model_type(), "TestReadModel");
        assert_eq!(model.id(), "test-123");
        assert_eq!(model.count, 5);
        assert_eq!(model.last_event.as_deref(), Some("EventProcessed"));
        
        // Test serialization
        let serialized = serde_json::to_string(&model).unwrap();
        let deserialized: TestReadModel = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.count, model.count);
    }
    
    #[test]
    fn test_read_model_metadata() {
        let metadata = ReadModelMetadata {
            id: "test-123".to_string(),
            model_type: "TestModel".to_string(),
            schema_version: 1,
            last_updated: Utc::now(),
            last_event_position: 42,
            metadata: HashMap::new(),
        };
        
        assert_eq!(metadata.id, "test-123");
        assert_eq!(metadata.schema_version, 1);
        assert_eq!(metadata.last_event_position, 42);
    }
    
    #[test]
    fn test_projection_status() {
        let status1 = ProjectionStatus::UpToDate;
        let status2 = ProjectionStatus::Lagging { behind_by: 10 };
        
        assert_eq!(status1, ProjectionStatus::UpToDate);
        assert_ne!(status1, status2);
        
        if let ProjectionStatus::Lagging { behind_by } = status2 {
            assert_eq!(behind_by, 10);
        }
    }
}