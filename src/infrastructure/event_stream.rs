// Copyright 2025 Cowboy AI, LLC.

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
        value: serde_json::Value,
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
    async fn save_stream(&self, stream: &EventStream) -> Result<(), EventStreamError>;

    /// Load a saved event stream
    async fn load_stream(&self, stream_id: &EventStreamId)
        -> Result<EventStream, EventStreamError>;

    /// List all saved streams
    async fn list_streams(&self) -> Result<Vec<EventStream>, EventStreamError>;
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

        // Calculate causation chains
        let causation_chains = Self::calculate_causation_chains(events);

        // Calculate CID root
        let cid_root = Self::calculate_cid_root(events);

        EventStreamMetadata {
            event_count: events.len(),
            time_range,
            aggregate_types,
            correlation_ids,
            causation_chains,
            cid_root,
        }
    }

    /// Order events by causation
    pub fn order_by_causation(&mut self) {
        // Build causation map
        let mut causation_map: HashMap<String, Vec<usize>> = HashMap::new();
        let mut roots = Vec::new();

        for (idx, event) in self.events.iter().enumerate() {
            if let Some(causation_id) = event.causation_id() {
                causation_map
                    .entry(causation_id.clone())
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
            visit(
                root_idx,
                &self.events,
                &causation_map,
                &mut visited,
                &mut ordered,
            );
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
        let filtered_events: Vec<StoredEvent> = self
            .events
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
        let mut groups: HashMap<String, Vec<&StoredEvent>> = HashMap::new();

        for event in &self.events {
            if let Some(corr_id) = event.correlation_id() {
                groups.entry(corr_id.clone()).or_default().push(event);
            }
        }

        groups
    }

    /// Calculate causation chains from the events
    fn calculate_causation_chains(events: &[StoredEvent]) -> Vec<CausationChain> {
        let mut chains: Vec<CausationChain> = Vec::new();
        let mut event_to_chain: HashMap<String, usize> = HashMap::new();

        for event in events {
            let event_id = event.event_id.to_string();

            if let Some(causation_id) = event.causation_id() {
                // This event was caused by another event
                if let Some(&chain_idx) = event_to_chain.get(causation_id) {
                    // Add to existing chain
                    chains[chain_idx].chain.push(event_id.clone());
                    event_to_chain.insert(event_id, chain_idx);
                } else {
                    // Start new chain with both the cause and this event
                    let chain = CausationChain {
                        root_event_id: causation_id.clone(),
                        chain: vec![causation_id.clone(), event_id.clone()],
                    };
                    let chain_idx = chains.len();
                    chains.push(chain);
                    event_to_chain.insert(event_id, chain_idx);
                }
            } else {
                // This is a root event, check if it starts any chains
                let has_effects = events.iter().any(|e| {
                    e.causation_id()
                        .map(|cid| cid == &event_id)
                        .unwrap_or(false)
                });

                if has_effects && !event_to_chain.contains_key(&event_id) {
                    // This is a root that causes other events
                    let chain = CausationChain {
                        root_event_id: event_id.clone(),
                        chain: vec![event_id.clone()],
                    };
                    let chain_idx = chains.len();
                    chains.push(chain);
                    event_to_chain.insert(event_id, chain_idx);
                }
            }
        }

        chains
    }

    /// Calculate the CID root for the event stream
    fn calculate_cid_root(_events: &[StoredEvent]) -> Option<String> {
        // For now, return None as CID calculation is complex
        // In a real implementation, this would create a chain of CIDs
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DomainEvent;
    use uuid::Uuid;

    // Test event type
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        id: Uuid,
        name: String,
    }

    impl DomainEvent for TestEvent {
        fn subject(&self) -> String {
            "test.entity.created.v1".to_string()
        }

        fn aggregate_id(&self) -> Uuid {
            self.id
        }

        fn event_type(&self) -> &'static str {
            "TestEvent"
        }
    }

    fn create_test_stored_event(name: &str) -> StoredEvent {
        // Create a workflow started event for testing
        let workflow_id = crate::identifiers::WorkflowId::new();
        let event = crate::domain_events::WorkflowStarted {
            workflow_id,
            definition_id: crate::GraphId::new(),
            initial_state: name.to_string(),
            started_at: Utc::now(),
        };

        StoredEvent {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: workflow_id.to_string(),
            aggregate_type: "TestAggregate".to_string(),
            sequence: 1,
            event: crate::domain_events::DomainEventEnum::WorkflowStarted(event),
            metadata: crate::infrastructure::event_store::EventMetadata {
                correlation_id: Some(Uuid::new_v4().to_string()),
                causation_id: None,
                triggered_by: Some("test".to_string()),
                custom: None,
            },
            stored_at: Utc::now(),
        }
    }

    #[test]
    fn test_event_stream_id() {
        let id1 = EventStreamId::new();
        let id2 = EventStreamId::new();

        assert_ne!(id1, id2);
        assert_eq!(id1, id1.clone());

        let id_str = id1.to_string();
        assert_eq!(id_str.len(), 36); // UUID string length
    }

    #[test]
    fn test_time_range() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);

        let range = TimeRange { start, end };

        assert!(range.end > range.start);
        assert_eq!(range.end - range.start, chrono::Duration::hours(1));
    }

    #[test]
    fn test_event_stream_creation() {
        // Create TestEvent instances
        let test_event1 = TestEvent {
            id: Uuid::new_v4(),
            name: "Test Event 1".to_string(),
        };
        let _test_event2 = TestEvent {
            id: Uuid::new_v4(),
            name: "Test Event 2".to_string(),
        };

        // Verify TestEvent implements DomainEvent correctly
        assert_eq!(test_event1.event_type(), "TestEvent");
        assert_eq!(test_event1.subject(), "test.entity.created.v1");

        let stored1 = create_test_stored_event("event1");
        let stored2 = create_test_stored_event("event2");
        let events = vec![stored1, stored2];

        let stream = EventStream::new(
            "Test Stream".to_string(),
            "A test event stream".to_string(),
            EventQuery::ByAggregateType {
                aggregate_type: "TestAggregate".to_string(),
            },
            events.clone(),
        );

        assert_eq!(stream.name, "Test Stream");
        assert_eq!(stream.description, "A test event stream");
        assert_eq!(stream.events.len(), 2);
        assert_eq!(stream.metadata.event_count, 2);
        assert!(stream.metadata.aggregate_types.contains("TestAggregate"));
    }

    #[test]
    fn test_metadata_calculation() {
        let mut events = Vec::new();
        let base_time = Utc::now();

        for i in 0..5 {
            let mut stored = create_test_stored_event(&format!("event{i}"));
            stored.stored_at = base_time + chrono::Duration::minutes(i as i64);

            // Add correlation ID to some events
            if i < 3 {
                stored.metadata.correlation_id = Some("corr-123".to_string());
            }

            events.push(stored);
        }

        let metadata = EventStream::calculate_metadata(&events);

        assert_eq!(metadata.event_count, 5);
        assert!(metadata.aggregate_types.contains("TestAggregate"));
        assert!(metadata.correlation_ids.len() >= 2); // At least corr-123 and one generated

        let time_range = metadata.time_range.unwrap();
        assert_eq!(time_range.start, base_time);
        assert_eq!(time_range.end, base_time + chrono::Duration::minutes(4));
    }

    #[test]
    fn test_event_filtering() {
        let stored1 = create_test_stored_event("keep");
        let stored2 = create_test_stored_event("filter");
        let stored3 = create_test_stored_event("keep");

        let events = vec![stored1, stored2, stored3];

        let stream = EventStream::new(
            "Original".to_string(),
            "Original stream".to_string(),
            EventQuery::ByAggregateType {
                aggregate_type: "TestAggregate".to_string(),
            },
            events,
        );

        let filtered = stream.filter(|e| e.event_type() == "WorkflowStarted");

        assert_eq!(filtered.events.len(), 3); // All events are WorkflowStarted
        assert!(filtered.name.contains("filtered"));
        assert!(filtered
            .events
            .iter()
            .all(|e| e.event_type() == "WorkflowStarted"));
    }

    #[test]
    fn test_group_by_correlation() {
        let mut events = Vec::new();

        // Create events with different correlation IDs
        for i in 0..6 {
            let mut stored = create_test_stored_event(&format!("event{i}"));
            let corr_id = if i < 3 { "corr-A" } else { "corr-B" };
            stored.metadata.correlation_id = Some(corr_id.to_string());
            events.push(stored);
        }

        let stream = EventStream::new(
            "Grouped".to_string(),
            "Grouped stream".to_string(),
            EventQuery::ByAggregateType {
                aggregate_type: "TestAggregate".to_string(),
            },
            events,
        );

        let groups = stream.group_by_correlation();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("corr-A").unwrap().len(), 3);
        assert_eq!(groups.get("corr-B").unwrap().len(), 3);
    }

    #[test]
    fn test_causation_chains() {
        let mut events = Vec::new();

        // Create a causation chain: event1 -> event2 -> event3
        let mut stored1 = create_test_stored_event("event1");
        stored1.event_id = "event-1".to_string();
        events.push(stored1);

        let mut stored2 = create_test_stored_event("event2");
        stored2.event_id = "event-2".to_string();
        stored2.metadata.causation_id = Some("event-1".to_string());
        events.push(stored2);

        let mut stored3 = create_test_stored_event("event3");
        stored3.event_id = "event-3".to_string();
        stored3.metadata.causation_id = Some("event-2".to_string());
        events.push(stored3);

        // Create an independent event
        let mut stored4 = create_test_stored_event("event4");
        stored4.event_id = "event-4".to_string();
        events.push(stored4);

        let chains = EventStream::calculate_causation_chains(&events);

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].root_event_id, "event-1");
        assert_eq!(chains[0].chain.len(), 3);
        assert_eq!(chains[0].chain, vec!["event-1", "event-2", "event-3"]);
    }

    #[test]
    fn test_order_by_causation() {
        let mut events = Vec::new();

        // Create events in reverse causation order
        let mut stored3 = create_test_stored_event("effect2");
        stored3.event_id = "event-3".to_string();
        stored3.metadata.causation_id = Some("event-2".to_string());
        events.push(stored3);

        let mut stored2 = create_test_stored_event("effect1");
        stored2.event_id = "event-2".to_string();
        stored2.metadata.causation_id = Some("event-1".to_string());
        events.push(stored2);

        let mut stored1 = create_test_stored_event("cause");
        stored1.event_id = "event-1".to_string();
        events.push(stored1);

        let mut stream = EventStream::new(
            "Causal".to_string(),
            "Causal ordering test".to_string(),
            EventQuery::ByAggregateType {
                aggregate_type: "TestAggregate".to_string(),
            },
            events,
        );

        stream.order_by_causation();

        // After ordering, cause should come before effects
        assert_eq!(stream.events[0].event_id, "event-1");
        assert_eq!(stream.events[1].event_id, "event-2");
        assert_eq!(stream.events[2].event_id, "event-3");
    }

    #[test]
    fn test_event_filter_variants() {
        use EventFilter::*;

        let filter1 = EventType("TestEvent".to_string());
        let _filter2 = EventTypes(vec!["Event1".to_string(), "Event2".to_string()]);
        let _filter3 = AggregateId("agg-123".to_string());
        let _filter4 = AggregateType("TestAggregate".to_string());
        let _filter5 = CorrelationId("corr-456".to_string());
        let filter6 = MetadataValue {
            key: "user_id".to_string(),
            value: serde_json::json!("user-789"),
        };

        // Test serialization
        let json1 = serde_json::to_string(&filter1).unwrap();
        assert!(json1.contains("TestEvent"));

        let json6 = serde_json::to_string(&filter6).unwrap();
        assert!(json6.contains("user_id"));
        assert!(json6.contains("user-789"));
    }

    #[test]
    fn test_event_query_variants() {
        let query1 = EventQuery::ByCorrelationId {
            correlation_id: "corr-123".to_string(),
            order: CausationOrder::Causal,
        };

        let query2 = EventQuery::ByTimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now(),
        };

        let query3 = EventQuery::Complex {
            filters: vec![
                EventFilter::EventType("TestEvent".to_string()),
                EventFilter::AggregateType("TestAggregate".to_string()),
            ],
            ordering: EventOrdering::Temporal,
            limit: Some(100),
        };

        // Test serialization
        assert!(serde_json::to_string(&query1).is_ok());
        assert!(serde_json::to_string(&query2).is_ok());
        assert!(serde_json::to_string(&query3).is_ok());
    }

    #[test]
    fn test_window_spec() {
        let time_window = WindowSpec::Time(chrono::Duration::minutes(5));
        let count_window = WindowSpec::Count(100);
        let sliding_window = WindowSpec::SlidingTime {
            size: chrono::Duration::minutes(10),
            slide: chrono::Duration::minutes(1),
        };
        let session_window = WindowSpec::Session(chrono::Duration::seconds(30));

        // Test serialization
        assert!(serde_json::to_string(&time_window).is_ok());
        assert!(serde_json::to_string(&count_window).is_ok());
        assert!(serde_json::to_string(&sliding_window).is_ok());
        assert!(serde_json::to_string(&session_window).is_ok());
    }

    #[test]
    fn test_stream_error_display() {
        let errors = vec![
            EventStreamError::QueryError("Invalid query".to_string()),
            EventStreamError::TransformationError("Transform failed".to_string()),
            EventStreamError::CompositionError("Cannot compose".to_string()),
            EventStreamError::StreamNotFound(EventStreamId::new()),
            EventStreamError::InvalidOperation("Not allowed".to_string()),
            EventStreamError::EventStoreError("Store error".to_string()),
        ];

        for error in errors {
            let error_str = error.to_string();
            assert!(!error_str.is_empty());
        }
    }
}
