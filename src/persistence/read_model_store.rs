// Copyright 2025 Cowboy AI, LLC.

//! Read model storage using NATS KV for optimized queries

use crate::{
    DomainError,
    events::DomainEvent,
};
use async_trait::async_trait;
use async_nats::{Client, jetstream::{self, kv}};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::subject_abstraction::Subject;
use cim_ipld::Cid;

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
    /// CID of the model data
    pub data_cid: Option<Cid>,
    /// Subject for this model
    pub subject: String,
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
    Lagging { behind_by: u64 },
    /// Projection has failed
    Failed,
}

/// A materialized view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedView {
    /// View ID
    pub id: String,
    /// View name
    pub name: String,
    /// Query that defines this view
    pub query: String,
    /// View data
    pub data: serde_json::Value,
    /// Metadata
    pub metadata: ReadModelMetadata,
    /// Projection status
    pub status: ProjectionStatus,
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

/// Read model store errors
#[derive(Debug, thiserror::Error)]
pub enum ReadModelError {
    /// Model not found
    #[error("Read model not found: {0}")]
    NotFound(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Subject error
    #[error("Subject error: {0}")]
    SubjectError(String),
    
    /// NATS error
    #[error("NATS error: {0}")]
    NatsError(String),
}

/// Store for read models using NATS KV
#[async_trait]
pub trait ReadModelStore: Send + Sync {
    /// Save a read model
    async fn save<T: ReadModel>(
        &self,
        model: &T,
        metadata: ReadModelMetadata,
    ) -> Result<(), ReadModelError>;
    
    /// Load a read model by ID
    async fn load<T: ReadModel>(
        &self,
        id: &str,
    ) -> Result<(T, ReadModelMetadata), ReadModelError>;
    
    /// Delete a read model
    async fn delete(&self, model_type: &str, id: &str) -> Result<(), ReadModelError>;
    
    /// Query read models
    async fn query<T: ReadModel>(
        &self,
        pattern: Option<&str>,
        filters: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<(T, ReadModelMetadata)>, ReadModelError>;
    
    /// Get all models of a type
    async fn list<T: ReadModel>(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<(T, ReadModelMetadata)>, ReadModelError>;
    
    /// Create or update a materialized view
    async fn create_view(
        &self,
        view: MaterializedView,
    ) -> Result<(), ReadModelError>;
    
    /// Get a materialized view
    async fn get_view(&self, name: &str) -> Result<MaterializedView, ReadModelError>;
    
    /// List all views
    async fn list_views(&self) -> Result<Vec<String>, ReadModelError>;
    
    /// Update projection status
    async fn update_projection_status(
        &self,
        model_type: &str,
        status: ProjectionStatus,
    ) -> Result<(), ReadModelError>;
}

/// NATS KV-based read model store
pub struct NatsReadModelStore {
    client: Client,
    models_bucket: String,
    views_bucket: String,
    metadata_bucket: String,
    subject_prefix: String,
    cache: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl NatsReadModelStore {
    /// Create a new NATS read model store
    pub async fn new(
        client: Client,
        bucket_prefix: &str,
    ) -> Result<Self, ReadModelError> {
        let js = jetstream::new(client.clone());
        
        // Create KV buckets
        let models_bucket = format!("{}-models", bucket_prefix);
        let views_bucket = format!("{}-views", bucket_prefix);
        let metadata_bucket = format!("{}-metadata", bucket_prefix);
        
        // Create model bucket
        js.create_key_value(kv::Config {
            bucket: models_bucket.clone(),
            storage: jetstream::stream::StorageType::File,
            history: 10,
            ..Default::default()
        })
        .await
        .map_err(|e| ReadModelError::NatsError(e.to_string()))?;
        
        // Create views bucket
        js.create_key_value(kv::Config {
            bucket: views_bucket.clone(),
            storage: jetstream::stream::StorageType::File,
            history: 5,
            ..Default::default()
        })
        .await
        .map_err(|e| ReadModelError::NatsError(e.to_string()))?;
        
        // Create metadata bucket
        js.create_key_value(kv::Config {
            bucket: metadata_bucket.clone(),
            storage: jetstream::stream::StorageType::File,
            history: 10,
            ..Default::default()
        })
        .await
        .map_err(|e| ReadModelError::NatsError(e.to_string()))?;
        
        Ok(Self {
            client,
            models_bucket,
            views_bucket,
            metadata_bucket,
            subject_prefix: format!("{}.readmodels", bucket_prefix),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Build subject for read model
    fn build_subject(&self, model_type: &str, id: &str) -> Result<Subject, ReadModelError> {
        let subject_str = format!("{}.readmodel.{}.{}", 
            self.subject_prefix,
            model_type,
            id
        );
        Subject::new(&subject_str)
            .map_err(|e| ReadModelError::SubjectError(e.to_string()))
    }
    
    /// Get KV key for model
    fn get_model_key(&self, model_type: &str, id: &str) -> String {
        format!("{}.{}", model_type, id)
    }
    
    /// Get KV key for metadata
    fn get_metadata_key(&self, model_type: &str, id: &str) -> String {
        format!("{}.{}.metadata", model_type, id)
    }
}

#[async_trait]
impl ReadModelStore for NatsReadModelStore {
    async fn save<T: ReadModel>(
        &self,
        model: &T,
        metadata: ReadModelMetadata,
    ) -> Result<(), ReadModelError> {
        let js = jetstream::new(self.client.clone());
        
        // Get KV store
        let kv = js
            .get_key_value(&self.models_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Serialize model
        let model_data = serde_json::to_vec(model)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        // Save model data
        let key = self.get_model_key(T::model_type(), model.id());
        kv.put(key, model_data.into())
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Save metadata
        let metadata_kv = js
            .get_key_value(&self.metadata_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let metadata_data = serde_json::to_vec(&metadata)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        let metadata_key = self.get_metadata_key(T::model_type(), model.id());
        metadata_kv.put(metadata_key, metadata_data.into())
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(
            format!("{}:{}", T::model_type(), model.id()),
            serde_json::to_value(model)
                .map_err(|e| ReadModelError::SerializationError(e.to_string()))?
        );
        
        Ok(())
    }
    
    async fn load<T: ReadModel>(
        &self,
        id: &str,
    ) -> Result<(T, ReadModelMetadata), ReadModelError> {
        // Check cache first
        let cache_key = format!("{}:{}", T::model_type(), id);
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                let model: T = serde_json::from_value(cached.clone())
                    .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
                
                // Still need to load metadata
                let js = jetstream::new(self.client.clone());
                let metadata_kv = js
                    .get_key_value(&self.metadata_bucket)
                    .await
                    .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
                
                let metadata_key = self.get_metadata_key(T::model_type(), id);
                if let Some(entry) = metadata_kv.get(&metadata_key)
                    .await
                    .map_err(|e| ReadModelError::StorageError(e.to_string()))?
                {
                    let metadata: ReadModelMetadata = serde_json::from_slice(&entry.value)
                        .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
                    
                    return Ok((model, metadata));
                }
            }
        }
        
        let js = jetstream::new(self.client.clone());
        
        // Load model data
        let kv = js
            .get_key_value(&self.models_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let key = self.get_model_key(T::model_type(), id);
        let entry = kv.get(&key)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?
            .ok_or_else(|| ReadModelError::NotFound(id.to_string()))?;
        
        let model: T = serde_json::from_slice(&entry.value)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        // Load metadata
        let metadata_kv = js
            .get_key_value(&self.metadata_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let metadata_key = self.get_metadata_key(T::model_type(), id);
        let metadata_entry = metadata_kv.get(&metadata_key)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?
            .ok_or_else(|| ReadModelError::NotFound(format!("{} metadata", id)))?;
        
        let metadata: ReadModelMetadata = serde_json::from_slice(&metadata_entry.value)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(
            cache_key,
            serde_json::to_value(&model)
                .map_err(|e| ReadModelError::SerializationError(e.to_string()))?
        );
        
        Ok((model, metadata))
    }
    
    async fn delete(&self, model_type: &str, id: &str) -> Result<(), ReadModelError> {
        let js = jetstream::new(self.client.clone());
        
        // Delete model data
        let kv = js
            .get_key_value(&self.models_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let key = self.get_model_key(model_type, id);
        kv.delete(&key)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Delete metadata
        let metadata_kv = js
            .get_key_value(&self.metadata_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let metadata_key = self.get_metadata_key(model_type, id);
        metadata_kv.delete(&metadata_key)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(&format!("{}:{}", model_type, id));
        
        Ok(())
    }
    
    async fn query<T: ReadModel>(
        &self,
        pattern: Option<&str>,
        filters: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<(T, ReadModelMetadata)>, ReadModelError> {
        let js = jetstream::new(self.client.clone());
        let kv = js
            .get_key_value(&self.models_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        // Get all keys matching the pattern
        let key_pattern = if let Some(p) = pattern {
            p.to_string()
        } else {
            format!("{}.*", T::model_type())
        };
        
        let mut results = Vec::new();
        let mut keys = kv.keys()
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        while let Some(key) = keys.next().await {
            // Check if key matches pattern
            if !key.starts_with(T::model_type()) {
                continue;
            }
            
            // Extract ID from key
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() >= 2 {
                let id = parts[1];
                
                // Load model and metadata
                match self.load::<T>(id).await {
                    Ok((model, metadata)) => {
                        // Apply filters
                        let model_json = serde_json::to_value(&model)
                            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
                        
                        let mut matches = true;
                        for (field, value) in &filters {
                            if let Some(model_value) = model_json.get(field) {
                                if model_value != value {
                                    matches = false;
                                    break;
                                }
                            } else {
                                matches = false;
                                break;
                            }
                        }
                        
                        if matches {
                            results.push((model, metadata));
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        
        Ok(results)
    }
    
    async fn list<T: ReadModel>(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<(T, ReadModelMetadata)>, ReadModelError> {
        self.query::<T>(None, HashMap::new()).await.map(|mut results| {
            // Apply pagination
            let offset = offset.unwrap_or(0);
            let limit = limit.unwrap_or(results.len());
            
            results.drain(offset..).take(limit).collect()
        })
    }
    
    async fn create_view(
        &self,
        view: MaterializedView,
    ) -> Result<(), ReadModelError> {
        let js = jetstream::new(self.client.clone());
        let kv = js
            .get_key_value(&self.views_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let view_data = serde_json::to_vec(&view)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        kv.put(view.name.clone(), view_data.into())
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn get_view(&self, name: &str) -> Result<MaterializedView, ReadModelError> {
        let js = jetstream::new(self.client.clone());
        let kv = js
            .get_key_value(&self.views_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let entry = kv.get(name)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?
            .ok_or_else(|| ReadModelError::NotFound(name.to_string()))?;
        
        let view: MaterializedView = serde_json::from_slice(&entry.value)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        Ok(view)
    }
    
    async fn list_views(&self) -> Result<Vec<String>, ReadModelError> {
        let js = jetstream::new(self.client.clone());
        let kv = js
            .get_key_value(&self.views_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let mut view_names = Vec::new();
        let mut keys = kv.keys()
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        while let Some(key) = keys.next().await {
            view_names.push(key);
        }
        
        Ok(view_names)
    }
    
    async fn update_projection_status(
        &self,
        model_type: &str,
        status: ProjectionStatus,
    ) -> Result<(), ReadModelError> {
        let js = jetstream::new(self.client.clone());
        let kv = js
            .get_key_value(&self.metadata_bucket)
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        let status_key = format!("{}.projection_status", model_type);
        let status_data = serde_json::to_vec(&status)
            .map_err(|e| ReadModelError::SerializationError(e.to_string()))?;
        
        kv.put(status_key, status_data.into())
            .await
            .map_err(|e| ReadModelError::StorageError(e.to_string()))?;
        
        Ok(())
    }
}