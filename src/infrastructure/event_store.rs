//! Event store trait and related types

use crate::domain_events::DomainEventEnum;
use crate::events::DomainEvent;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::pin::Pin;
use futures::stream::Stream;
use thiserror::Error;

/// Errors that can occur when working with the event store
#[derive(Debug, Error)]
pub enum EventStoreError {
    /// Failed to connect to the event store
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Failed to serialize or deserialize event data
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Optimistic concurrency check failed
    #[error("Concurrency conflict: expected version {expected}, but current version is {current}")]
    ConcurrencyConflict {
        /// The version that was expected
        expected: u64,
        /// The actual current version
        current: u64
    },

    /// Requested event was not found
    #[error("Event not found: {0}")]
    EventNotFound(String),

    /// Requested stream was not found
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    /// Event data is malformed or invalid
    #[error("Invalid event data: {0}")]
    InvalidEventData(String),

    /// General storage operation failed
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// A stored event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Unique event ID
    pub event_id: String,

    /// Aggregate ID this event belongs to
    pub aggregate_id: String,

    /// Aggregate type (e.g., "Person", "Organization")
    pub aggregate_type: String,

    /// Event sequence number within the aggregate
    pub sequence: u64,

    /// The actual domain event
    pub event: DomainEventEnum,

    /// Event metadata
    pub metadata: EventMetadata,

    /// When the event was stored
    pub stored_at: DateTime<Utc>,
}

impl StoredEvent {
    /// Get the event type as a string
    pub fn event_type(&self) -> &str {
        self.event.event_type()
    }

    /// Get the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.stored_at
    }

    /// Get the correlation ID from metadata
    pub fn correlation_id(&self) -> Option<&String> {
        self.metadata.correlation_id.as_ref()
    }

    /// Get the causation ID from metadata
    pub fn causation_id(&self) -> Option<&String> {
        self.metadata.causation_id.as_ref()
    }
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Correlation ID for tracking related events
    pub correlation_id: Option<String>,

    /// Causation ID - the event that caused this event
    pub causation_id: Option<String>,

    /// User or system that triggered the event
    pub triggered_by: Option<String>,

    /// Additional custom metadata
    pub custom: Option<serde_json::Value>,
}

impl EventMetadata {
    /// Get a custom metadata value by key
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.as_ref()?.get(key)
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            correlation_id: None,
            causation_id: None,
            triggered_by: None,
            custom: None,
        }
    }
}

/// Event store trait for persisting and retrieving events
#[async_trait]
pub trait EventStore: Send + Sync + fmt::Debug {
    /// Append events to the store for a specific aggregate
    async fn append_events(
        &self,
        aggregate_id: &str,
        aggregate_type: &str,
        events: Vec<DomainEventEnum>,
        expected_version: Option<u64>,
        metadata: EventMetadata,
    ) -> Result<(), EventStoreError>;

    /// Get all events for a specific aggregate
    async fn get_events(
        &self,
        aggregate_id: &str,
        from_version: Option<u64>,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;

    /// Get events by type across all aggregates
    async fn get_events_by_type(
        &self,
        event_type: &str,
        limit: usize,
        after: Option<DateTime<Utc>>,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;

    /// Get the current version of an aggregate
    async fn get_aggregate_version(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<u64>, EventStoreError>;

    /// Subscribe to events (returns a stream of events)
    async fn subscribe_to_events(
        &self,
        from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError>;

    /// Subscribe to events for a specific aggregate type
    async fn subscribe_to_aggregate_type(
        &self,
        aggregate_type: &str,
        from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError>;

    /// Stream events by type
    async fn stream_events_by_type(
        &self,
        event_type: &str,
        from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError>;

    /// Stream all events from the store
    async fn stream_all_events(
        &self,
        from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError>;
}

/// Stream of events that implements futures::Stream
pub trait EventStream: Stream<Item = Result<StoredEvent, EventStoreError>> + Send + Unpin {
    /// Acknowledge that an event has been processed
    fn ack(&mut self, event_id: &str) -> Result<(), EventStoreError>;

    /// Close the stream
    fn close(self: Box<Self>) -> Result<(), EventStoreError>;
}

/// Helper type for pinned event streams
pub type PinnedEventStream = Pin<Box<dyn EventStream>>;

/// Event store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStoreStats {
    /// Total number of events
    pub total_events: u64,

    /// Number of unique aggregates
    pub total_aggregates: u64,

    /// Events by aggregate type
    pub events_by_type: std::collections::HashMap<String, u64>,

    /// Storage size in bytes
    pub storage_size_bytes: u64,

    /// Oldest event timestamp
    pub oldest_event: Option<DateTime<Utc>>,

    /// Newest event timestamp
    pub newest_event: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_metadata_default() {
        let metadata = EventMetadata::default();
        assert!(metadata.correlation_id.is_none());
        assert!(metadata.causation_id.is_none());
        assert!(metadata.triggered_by.is_none());
        assert!(metadata.custom.is_none());
    }

    #[test]
    fn test_event_store_error_display() {
        let error = EventStoreError::ConcurrencyConflict {
            expected: 5,
            current: 7,
        };
        let error_str = error.to_string();
        assert!(error_str.contains("expected version 5"));
        assert!(error_str.contains("current version is 7"));
    }
}
