//! Snapshot store for aggregate state persistence

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    // TODO: Implement JetStream-based snapshot storage
}

impl JetStreamSnapshotStore {
    /// Create a new JetStream snapshot store
    pub fn new() -> Self {
        Self {}
    }
}
