// Copyright 2025 Cowboy AI, LLC.

//! Snapshot store for aggregate state persistence

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use async_nats::jetstream::{self, Context};
use bytes::Bytes;
use std::sync::Arc;
use tracing::{debug, error, info};

/// Errors that can occur during snapshot operations
#[derive(Debug, Error)]
pub enum SnapshotError {
    /// Error from underlying storage system
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Error serializing or deserializing snapshot data
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Requested snapshot was not found
    #[error("Snapshot not found")]
    NotFound,
    
    /// Error from JetStream
    #[error("JetStream error: {0}")]
    JetStreamError(String),
}

/// Snapshot of aggregate state at a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot {
    /// ID of the aggregate this snapshot belongs to
    pub aggregate_id: String,
    /// Type name of the aggregate (e.g., "Person", "Organization")
    pub aggregate_type: String,
    /// Version number of the aggregate when snapshot was taken
    pub version: u64,
    /// Serialized aggregate state data
    pub data: Vec<u8>,
    /// Timestamp when the snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for storing and retrieving aggregate snapshots
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    /// Save a snapshot of aggregate state
    async fn save_snapshot(
        &self,
        aggregate_id: &str,
        snapshot: AggregateSnapshot,
    ) -> Result<(), SnapshotError>;

    /// Get the most recent snapshot for an aggregate
    async fn get_latest_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<AggregateSnapshot>, SnapshotError>;
}

/// JetStream-based implementation of snapshot storage
pub struct JetStreamSnapshotStore {
    jetstream: Arc<Context>,
    bucket_name: String,
}

impl JetStreamSnapshotStore {
    /// Create a new JetStream snapshot store
    pub async fn new(
        jetstream: Arc<Context>,
        bucket_name: String,
    ) -> Result<Self, SnapshotError> {
        // Create or get the key-value bucket for snapshots
        let bucket_config = jetstream::kv::Config {
            bucket: bucket_name.clone(),
            description: "Aggregate snapshots".to_string(),
            max_value_size: 10 * 1024 * 1024, // 10MB max snapshot size
            history: 5, // Keep last 5 snapshots per aggregate
            storage: jetstream::stream::StorageType::File,
            ..Default::default()
        };
        
        jetstream
            .create_key_value(bucket_config)
            .await
            .map_err(|e| SnapshotError::JetStreamError(e.to_string()))?;
        
        info!("JetStream snapshot store initialized with bucket: {}", bucket_name);
        
        Ok(Self {
            jetstream,
            bucket_name,
        })
    }
    
    /// Get a snapshot key for an aggregate
    fn get_snapshot_key(aggregate_type: &str, aggregate_id: &str) -> String {
        format!("{aggregate_type}.{aggregate_id}")
    }
}

#[async_trait]
impl SnapshotStore for JetStreamSnapshotStore {
    async fn save_snapshot(
        &self,
        aggregate_id: &str,
        snapshot: AggregateSnapshot,
    ) -> Result<(), SnapshotError> {
        debug!(
            "Saving snapshot for aggregate {} type {} version {}",
            aggregate_id, snapshot.aggregate_type, snapshot.version
        );
        
        // Get the KV bucket
        let bucket = self
            .jetstream
            .get_key_value(&self.bucket_name)
            .await
            .map_err(|e| SnapshotError::JetStreamError(e.to_string()))?;
        
        // Serialize the snapshot
        let snapshot_bytes = serde_json::to_vec(&snapshot)
            .map_err(|e| SnapshotError::SerializationError(e.to_string()))?;
        
        // Store in JetStream KV
        let key = Self::get_snapshot_key(&snapshot.aggregate_type, aggregate_id);
        bucket
            .put(key, Bytes::from(snapshot_bytes))
            .await
            .map_err(|e| SnapshotError::JetStreamError(e.to_string()))?;
        
        info!(
            "Saved snapshot for aggregate {} at version {}",
            aggregate_id, snapshot.version
        );
        
        Ok(())
    }

    async fn get_latest_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<AggregateSnapshot>, SnapshotError> {
        debug!("Getting latest snapshot for aggregate {}", aggregate_id);
        
        // Get the KV bucket
        let bucket = self
            .jetstream
            .get_key_value(&self.bucket_name)
            .await
            .map_err(|e| SnapshotError::JetStreamError(e.to_string()))?;
        
        // Try to get snapshots for different aggregate types
        // In a real implementation, we might want to track which type an aggregate is
        let aggregate_types = ["Person", "Organization", "Workflow", "Graph", "Document"];
        
        for aggregate_type in &aggregate_types {
            let key = Self::get_snapshot_key(aggregate_type, aggregate_id);
            
            match bucket.get(&key).await {
                Ok(Some(entry)) => {
                    let snapshot: AggregateSnapshot = serde_json::from_slice(&entry)
                        .map_err(|e| SnapshotError::SerializationError(e.to_string()))?;
                    
                    debug!(
                        "Found snapshot for aggregate {} type {} version {}",
                        aggregate_id, aggregate_type, snapshot.version
                    );
                    
                    return Ok(Some(snapshot));
                }
                Ok(None) => continue,
                Err(e) => {
                    error!("Error getting snapshot for key {}: {}", key, e);
                    continue;
                }
            }
        }
        
        debug!("No snapshot found for aggregate {}", aggregate_id);
        Ok(None)
    }
}

/// In-memory snapshot store for testing
pub struct InMemorySnapshotStore {
    snapshots: Arc<tokio::sync::RwLock<std::collections::HashMap<String, AggregateSnapshot>>>,
}

impl Default for InMemorySnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySnapshotStore {
    /// Create a new in-memory snapshot store
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl SnapshotStore for InMemorySnapshotStore {
    async fn save_snapshot(
        &self,
        aggregate_id: &str,
        snapshot: AggregateSnapshot,
    ) -> Result<(), SnapshotError> {
        let mut snapshots = self.snapshots.write().await;
        snapshots.insert(aggregate_id.to_string(), snapshot);
        Ok(())
    }

    async fn get_latest_snapshot(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<AggregateSnapshot>, SnapshotError> {
        let snapshots = self.snapshots.read().await;
        Ok(snapshots.get(aggregate_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_in_memory_snapshot_store() {
        let store = InMemorySnapshotStore::new();
        
        let snapshot = AggregateSnapshot {
            aggregate_id: "test-123".to_string(),
            aggregate_type: "TestAggregate".to_string(),
            version: 5,
            data: vec![1, 2, 3, 4, 5],
            created_at: chrono::Utc::now(),
        };
        
        // Save snapshot
        store.save_snapshot("test-123", snapshot.clone()).await.unwrap();
        
        // Retrieve snapshot
        let retrieved = store.get_latest_snapshot("test-123").await.unwrap();
        assert!(retrieved.is_some());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.aggregate_id, snapshot.aggregate_id);
        assert_eq!(retrieved.version, snapshot.version);
        assert_eq!(retrieved.data, snapshot.data);
        
        // Non-existent aggregate
        let not_found = store.get_latest_snapshot("non-existent").await.unwrap();
        assert!(not_found.is_none());
    }
}