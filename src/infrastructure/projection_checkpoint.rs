// Copyright 2025 Cowboy AI, LLC.

//! Projection checkpoint storage for fault-tolerant event processing
//!
//! This module provides checkpoint storage to track projection progress,
//! enabling projections to resume from where they left off after failures.

use async_nats::jetstream::kv::Store as KvStore;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;

/// Errors that can occur during checkpoint operations
#[derive(Debug, Error)]
pub enum CheckpointError {
    /// NATS communication error
    #[error("NATS error: {0}")]
    Nats(#[from] async_nats::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Checkpoint not found
    #[error("Checkpoint not found: {0}")]
    NotFound(String),

    /// Invalid checkpoint data format
    #[error("Invalid checkpoint data")]
    InvalidData,
}

/// A checkpoint storing projection progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionCheckpoint {
    /// Unique identifier for the projection
    pub projection_id: String,
    /// Current position in the event stream
    pub position: EventPosition,
    /// When this checkpoint was last updated
    pub last_processed_at: DateTime<Utc>,
    /// Total number of events processed
    pub events_processed: u64,
    /// Number of errors encountered
    pub errors: u32,
    /// Description of the last error (if any)
    pub last_error: Option<String>,
    /// Additional projection-specific metadata
    pub metadata: serde_json::Value,
}

/// Position in an event stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPosition {
    /// Start from the beginning of the stream
    Beginning,
    /// Start from the end of the stream
    End,
    /// Start from a specific sequence number
    Sequence(u64),
    /// Start from a specific timestamp
    Timestamp(DateTime<Utc>),
    /// Start from a specific position in a named stream
    StreamPosition {
        /// ID of the stream
        stream_id: String,
        /// Sequence number within the stream
        sequence: u64,
    },
}

/// Trait for checkpoint storage implementations
#[async_trait::async_trait]
pub trait CheckpointStore: Send + Sync {
    /// Save or update a checkpoint
    async fn save_checkpoint(
        &self,
        checkpoint: &ProjectionCheckpoint,
    ) -> Result<(), CheckpointError>;

    /// Load a checkpoint by projection ID
    async fn load_checkpoint(
        &self,
        projection_id: &str,
    ) -> Result<Option<ProjectionCheckpoint>, CheckpointError>;

    /// Delete a checkpoint
    async fn delete_checkpoint(&self, projection_id: &str) -> Result<(), CheckpointError>;

    /// List all checkpoints
    async fn list_checkpoints(&self) -> Result<Vec<ProjectionCheckpoint>, CheckpointError>;
}

/// JetStream-based checkpoint storage
pub struct JetStreamCheckpointStore {
    kv_store: Arc<KvStore>,
}

impl JetStreamCheckpointStore {
    /// Create a new JetStream checkpoint store
    pub async fn new(
        client: async_nats::Client,
        bucket_name: &str,
    ) -> Result<Self, CheckpointError> {
        let jetstream = async_nats::jetstream::new(client);

        let kv_store = jetstream
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: bucket_name.to_string(),
                description: "Projection checkpoints".to_string(),
                history: 5,
                max_bytes: 10_000_000,
                storage: async_nats::jetstream::stream::StorageType::File,
                ..Default::default()
            })
            .await
            .map_err(|e| {
                CheckpointError::Nats(async_nats::Error::from(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            })?;

        Ok(Self {
            kv_store: Arc::new(kv_store),
        })
    }
}

#[async_trait::async_trait]
impl CheckpointStore for JetStreamCheckpointStore {
    async fn save_checkpoint(
        &self,
        checkpoint: &ProjectionCheckpoint,
    ) -> Result<(), CheckpointError> {
        let key = format!("checkpoint:{}", checkpoint.projection_id);
        let value = serde_json::to_vec(checkpoint)?;

        self.kv_store.put(&key, value.into()).await.map_err(|e| {
            CheckpointError::Nats(async_nats::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        debug!(
            projection_id = %checkpoint.projection_id,
            position = ?checkpoint.position,
            "Saved checkpoint"
        );

        Ok(())
    }

    async fn load_checkpoint(
        &self,
        projection_id: &str,
    ) -> Result<Option<ProjectionCheckpoint>, CheckpointError> {
        let key = format!("checkpoint:{projection_id}");

        match self.kv_store.get(&key).await {
            Ok(Some(entry)) => {
                let checkpoint: ProjectionCheckpoint = serde_json::from_slice(&entry)?;
                Ok(Some(checkpoint))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(CheckpointError::Nats(async_nats::Error::from(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
            ))),
        }
    }

    async fn delete_checkpoint(&self, projection_id: &str) -> Result<(), CheckpointError> {
        let key = format!("checkpoint:{projection_id}");
        self.kv_store.delete(&key).await.map_err(|e| {
            CheckpointError::Nats(async_nats::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;
        Ok(())
    }

    async fn list_checkpoints(&self) -> Result<Vec<ProjectionCheckpoint>, CheckpointError> {
        let mut checkpoints = Vec::new();
        let mut entries = self.kv_store.watch("checkpoint:*").await.map_err(|e| {
            CheckpointError::Nats(async_nats::Error::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        while let Some(entry) = entries.next().await {
            match entry {
                Ok(entry) => {
                    if let Ok(checkpoint) =
                        serde_json::from_slice::<ProjectionCheckpoint>(&entry.value)
                    {
                        checkpoints.push(checkpoint);
                    }
                }
                Err(_) => break,
            }
        }

        Ok(checkpoints)
    }
}

/// In-memory checkpoint storage for testing
pub struct InMemoryCheckpointStore {
    checkpoints: Arc<tokio::sync::RwLock<std::collections::HashMap<String, ProjectionCheckpoint>>>,
}

impl Default for InMemoryCheckpointStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryCheckpointStore {
    /// Create a new in-memory checkpoint store
    pub fn new() -> Self {
        Self {
            checkpoints: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl CheckpointStore for InMemoryCheckpointStore {
    async fn save_checkpoint(
        &self,
        checkpoint: &ProjectionCheckpoint,
    ) -> Result<(), CheckpointError> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.insert(checkpoint.projection_id.clone(), checkpoint.clone());
        Ok(())
    }

    async fn load_checkpoint(
        &self,
        projection_id: &str,
    ) -> Result<Option<ProjectionCheckpoint>, CheckpointError> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints.get(projection_id).cloned())
    }

    async fn delete_checkpoint(&self, projection_id: &str) -> Result<(), CheckpointError> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.remove(projection_id);
        Ok(())
    }

    async fn list_checkpoints(&self) -> Result<Vec<ProjectionCheckpoint>, CheckpointError> {
        let checkpoints = self.checkpoints.read().await;
        Ok(checkpoints.values().cloned().collect())
    }
}

/// Manager for checkpoint operations with convenience methods
pub struct CheckpointManager {
    store: Arc<dyn CheckpointStore>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(store: Arc<dyn CheckpointStore>) -> Self {
        Self { store }
    }

    /// Update projection progress
    pub async fn update_progress(
        &self,
        projection_id: String,
        position: EventPosition,
        events_processed: u64,
    ) -> Result<(), CheckpointError> {
        let checkpoint = ProjectionCheckpoint {
            projection_id,
            position,
            last_processed_at: Utc::now(),
            events_processed,
            errors: 0,
            last_error: None,
            metadata: serde_json::json!({}),
        };

        self.store.save_checkpoint(&checkpoint).await
    }

    /// Record an error for a projection
    pub async fn record_error(
        &self,
        projection_id: &str,
        error: &str,
    ) -> Result<(), CheckpointError> {
        let mut checkpoint = self
            .store
            .load_checkpoint(projection_id)
            .await?
            .ok_or_else(|| CheckpointError::NotFound(projection_id.to_string()))?;

        checkpoint.errors += 1;
        checkpoint.last_error = Some(error.to_string());
        checkpoint.last_processed_at = Utc::now();

        self.store.save_checkpoint(&checkpoint).await
    }

    /// Reset a projection by deleting its checkpoint
    pub async fn reset_projection(&self, projection_id: &str) -> Result<(), CheckpointError> {
        self.store.delete_checkpoint(projection_id).await
    }

    /// Get the current position of a projection
    pub async fn get_position(
        &self,
        projection_id: &str,
    ) -> Result<Option<EventPosition>, CheckpointError> {
        Ok(self
            .store
            .load_checkpoint(projection_id)
            .await?
            .map(|c| c.position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_checkpoint_operations() {
        let store = Arc::new(InMemoryCheckpointStore::new());
        let manager = CheckpointManager::new(store.clone());

        let projection_id = "test-projection";
        manager
            .update_progress(projection_id.to_string(), EventPosition::Sequence(100), 100)
            .await
            .unwrap();

        let checkpoint = store.load_checkpoint(projection_id).await.unwrap().unwrap();
        assert_eq!(checkpoint.events_processed, 100);
        assert_eq!(checkpoint.errors, 0);

        manager
            .record_error(projection_id, "Test error")
            .await
            .unwrap();

        let checkpoint = store.load_checkpoint(projection_id).await.unwrap().unwrap();
        assert_eq!(checkpoint.errors, 1);
        assert_eq!(checkpoint.last_error, Some("Test error".to_string()));

        manager.reset_projection(projection_id).await.unwrap();

        let checkpoint = store.load_checkpoint(projection_id).await.unwrap();
        assert!(checkpoint.is_none());
    }
}
