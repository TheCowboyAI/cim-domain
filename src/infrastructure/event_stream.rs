//! Event streams as first-class domain objects

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

use crate::infrastructure::event_store::StoredEvent;

/// Unique identifier for an event stream
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventStreamId(pub Uuid);

impl EventStreamId {
    /// Create a new unique event stream identifier
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventStreamId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventStreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a time range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time of the range (inclusive)
    pub start: DateTime<Utc>,
    /// End time of the range (inclusive)
    pub end: DateTime<Utc>,
}

/// Represents a causation chain of events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausationChain {
    /// ID of the root event that started this chain
    pub root_event_id: String,
    /// Ordered list of event IDs in the causation chain
    pub chain: Vec<String>,
}

/// Metadata about an event stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamMetadata {
    /// Total number of events in the stream
    pub event_count: usize,
    /// Time range covered by events in the stream
    pub time_range: Option<TimeRange>,
    /// Set of aggregate types present in the stream
    pub aggregate_types: HashSet<String>,
    /// Set of correlation IDs present in the stream
    pub correlation_ids: HashSet<String>,
    /// Causation chains identified in the stream
    pub causation_chains: Vec<CausationChain>,
    /// Root CID for cryptographic verification
    pub cid_root: Option<String>,
}

/// A first-class event stream object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStream {
    /// Unique identifier for this stream
    pub id: EventStreamId,
    /// Human-readable name for the stream
    pub name: String,
    /// Description of what this stream represents
    pub description: String,
    /// Query that defines which events belong to this stream
    pub query: EventQuery,
    /// Events contained in this stream
    pub events: Vec<StoredEvent>,
    /// Metadata about the stream contents
    pub metadata: EventStreamMetadata,
    /// When this stream was created
    pub created_at: DateTime<Utc>,
    /// When this stream was last updated
    pub updated_at: DateTime<Utc>,
}

/// Defines how to order events by causation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CausationOrder {
    /// Order by causation chain (cause before effect)
    Causal,
    /// Order by timestamp
    Temporal,
    /// Order by aggregate and sequence
    AggregateSequence,
}

/// Event filter criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Filter by event type
    EventType(String),
    /// Filter by multiple event types
    EventTypes(Vec<String>),
    /// Filter by aggregate ID
    AggregateId(String),
    /// Filter by aggregate type
    AggregateType(String),
    /// Filter by multiple aggregate types
    AggregateTypes(Vec<String>),
    /// Filter by correlation ID
    CorrelationId(String),
    /// Filter by metadata key-value
    MetadataValue {
        /// Metadata key to filter by
        key: String,
        /// Expected value for the metadata key
        value: serde_json::Value
    },
}

/// How to order events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventOrdering {
    /// Order by timestamp
    Temporal,
    /// Order by causation relationships
    Causal,
    /// Order by aggregate ID and sequence number
    AggregateSequence,
}

/// Query for retrieving events from the store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventQuery {
    /// Get all events for a specific correlation ID
    ByCorrelationId {
        /// The correlation ID to search for
        correlation_id: String,
        /// How to order the results
        order: CausationOrder,
    },

    /// Get events within a time range
    ByTimeRange {
        /// Start time (inclusive)
        start: DateTime<Utc>,
        /// End time (inclusive)
        end: DateTime<Utc>,
    },

    /// Get events by aggregate type
    ByAggregateType {
        /// Type of aggregate to filter by
        aggregate_type: String,
    },

    /// Complex query with multiple filters
    Complex {
        /// List of filters to apply
        filters: Vec<EventFilter>,
        /// How to order the results
        ordering: EventOrdering,
        /// Maximum number of events to return
        limit: Option<usize>,
    },

    /// Get events that form a specific workflow execution
    ByWorkflowExecution {
        /// Workflow instance identifier
        instance_id: String,
        /// Set of correlation IDs involved in the workflow
        correlation_ids: HashSet<String>,
    },
}

/// Grouping criteria for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupingCriteria {
    /// Group by aggregate type
    AggregateType,
    /// Group by correlation ID
    CorrelationId,
    /// Group by time windows of specified duration
    TimeWindow(chrono::Duration),
    /// Group by event type
    EventType,
}

/// Window specification for event windowing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowSpec {
    /// Fixed time windows
    Time(chrono::Duration),
    /// Fixed count windows
    Count(usize),
    /// Sliding time windows
    SlidingTime {
        /// Size of the window
        size: chrono::Duration,
        /// How much the window slides
        slide: chrono::Duration,
    },
    /// Session windows with gap
    Session(chrono::Duration),
}

/// Stream transformation operations
#[derive(Debug, Clone)]
pub enum StreamTransformation {
    /// Filter events
    Filter(EventFilter),

    /// Group events by criteria
    GroupBy(GroupingCriteria),

    /// Window events by time or count
    Window(WindowSpec),
}

/// How to resolve conflicts when merging streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep the first event
    KeepFirst,
    /// Keep the last event
    KeepLast,
    /// Keep all events
    KeepAll,
    /// Custom resolution function
    Custom(String),
}

/// Stream composition operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamComposition {
    /// Union of all events
    Union,

    /// Intersection of events
    Intersection,

    /// Events in first stream but not in others
    Difference,

    /// Merge with conflict resolution
    Merge(ConflictResolution),
}

/// Errors that can occur with event streams
#[derive(Debug, Error)]
pub enum EventStreamError {
    /// Query execution failed
    #[error("Query execution failed: {0}")]
    QueryError(String),

    /// Stream transformation failed
    #[error("Transformation failed: {0}")]
    TransformationError(String),

    /// Stream composition failed
    #[error("Composition failed: {0}")]
    CompositionError(String),

    /// Requested stream was not found
    #[error("Stream not found: {0}")]
    StreamNotFound(EventStreamId),

    /// Invalid operation on stream
    #[error("Invalid stream operation: {0}")]
    InvalidOperation(String),

    /// Error from underlying event store
    #[error("Event store error: {0}")]
    EventStoreError(String),
}

/// Operations on event streams
#[async_trait]
pub trait EventStreamOperations: Send + Sync {
    /// Create a new event stream from a query
    async fn create_stream(
        &self,
        name: String,
        description: String,
        query: EventQuery,
    ) -> Result<EventStream, EventStreamError>;

    /// Transform an event stream
    async fn transform_stream(
        &self,
        stream: &EventStream,
        transformation: StreamTransformation,
    ) -> Result<EventStream, EventStreamError>;

    /// Compose multiple streams
    async fn compose_streams(
        &self,
        streams: Vec<EventStream>,
        composition: StreamComposition,
    ) -> Result<EventStream, EventStreamError>;

    /// Save an event stream for later use
    async fn save_stream(
        &self,
        stream: &EventStream,
    ) -> Result<(), EventStreamError>;

    /// Load a saved event stream
    async fn load_stream(
        &self,
        stream_id: &EventStreamId,
    ) -> Result<EventStream, EventStreamError>;

    /// List all saved streams
    async fn list_streams(
        &self,
    ) -> Result<Vec<EventStream>, EventStreamError>;
}

impl EventStream {
    /// Create a new event stream
    pub fn new(
        name: String,
        description: String,
        query: EventQuery,
        events: Vec<StoredEvent>,
    ) -> Self {
        let metadata = Self::calculate_metadata(&events);

        Self {
            id: EventStreamId::new(),
            name,
            description,
            query,
            events,
            metadata,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Calculate metadata from events
    fn calculate_metadata(events: &[StoredEvent]) -> EventStreamMetadata {
        let mut aggregate_types = HashSet::new();
        let mut correlation_ids = HashSet::new();
        let mut min_time = None;
        let mut max_time = None;

        for event in events {
            aggregate_types.insert(event.aggregate_type.clone());

            if let Some(corr_id) = event.correlation_id() {
                correlation_ids.insert(corr_id.clone());
            }

            let timestamp = event.timestamp();
            match min_time {
                None => min_time = Some(timestamp),
                Some(t) if timestamp < t => min_time = Some(timestamp),
                _ => {}
            }

            match max_time {
                None => max_time = Some(timestamp),
                Some(t) if timestamp > t => max_time = Some(timestamp),
                _ => {}
            }
        }

        let time_range = match (min_time, max_time) {
            (Some(start), Some(end)) => Some(TimeRange { start, end }),
            _ => None,
        };

        EventStreamMetadata {
            event_count: events.len(),
            time_range,
            aggregate_types,
            correlation_ids,
            causation_chains: Vec::new(), // TODO: Calculate causation chains
            cid_root: None, // TODO: Calculate CID root
        }
    }

    /// Order events by causation
    pub fn order_by_causation(&mut self) {
        // Build causation map
        let mut causation_map: HashMap<String, Vec<usize>> = HashMap::new();
        let mut roots = Vec::new();

        for (idx, event) in self.events.iter().enumerate() {
            if let Some(causation_id) = event.causation_id() {
                causation_map.entry(causation_id.clone())
                    .or_default()
                    .push(idx);
            } else {
                roots.push(idx);
            }
        }

        // Topological sort
        let mut ordered = Vec::new();
        let mut visited = HashSet::new();

        fn visit(
            idx: usize,
            events: &[StoredEvent],
            causation_map: &HashMap<String, Vec<usize>>,
            visited: &mut HashSet<usize>,
            ordered: &mut Vec<StoredEvent>,
        ) {
            if visited.contains(&idx) {
                return;
            }

            visited.insert(idx);

            // Visit all events caused by this one
            if let Some(caused) = causation_map.get(&events[idx].event_id) {
                for &caused_idx in caused {
                    visit(caused_idx, events, causation_map, visited, ordered);
                }
            }

            ordered.push(events[idx].clone());
        }

        // Start from roots
        for &root_idx in &roots {
            visit(root_idx, &self.events, &causation_map, &mut visited, &mut ordered);
        }

        // Add any remaining events (cycles or disconnected)
        for (idx, event) in self.events.iter().enumerate() {
            if !visited.contains(&idx) {
                ordered.push(event.clone());
            }
        }

        // Reverse to get cause-before-effect order
        ordered.reverse();
        self.events = ordered;
    }

    /// Filter events by predicate
    pub fn filter<F>(&self, predicate: F) -> Self
    where
        F: Fn(&StoredEvent) -> bool,
    {
        let filtered_events: Vec<StoredEvent> = self.events
            .iter()
            .filter(|e| predicate(e))
            .cloned()
            .collect();

        Self::new(
            format!("{} (filtered)", self.name),
            self.description.clone(),
            self.query.clone(),
            filtered_events,
        )
    }

    /// Group events by correlation ID
    pub fn group_by_correlation(&self) -> HashMap<String, Vec<&StoredEvent>> {
        let mut groups = HashMap::new();

        for event in &self.events {
            if let Some(corr_id) = event.correlation_id() {
                groups.entry(corr_id.clone())
                    .or_insert_with(Vec::new)
                    .push(event);
            }
        }

        groups
    }
}
