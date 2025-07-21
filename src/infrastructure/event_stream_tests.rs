// Copyright 2025 Cowboy AI, LLC.

//! Tests for event streams

#[cfg(test)]
mod tests {
    use crate::{
        domain_events::{
            DomainEventEnum, WorkflowCompleted, WorkflowStarted, WorkflowTransitioned,
        },
        identifiers::{GraphId, WorkflowId},
        infrastructure::event_store::EventMetadata,
        infrastructure::{
            event_store::{EventStore, StoredEvent},
            event_stream::{
                CausationOrder, EventOrdering, EventQuery, EventStream, EventStreamError,
                EventStreamId, EventStreamOperations, StreamComposition,
            },
            tests::MockEventStore,
        },
    };
    use async_trait::async_trait;
    use chrono::{Duration, Utc};
    use futures::StreamExt;
    use std::sync::Arc;
    use uuid::Uuid;

    // Mock EventStreamService for testing
    struct EventStreamService {
        event_store: Arc<dyn EventStore>,
        saved_streams:
            Arc<tokio::sync::Mutex<std::collections::HashMap<EventStreamId, EventStream>>>,
    }

    impl EventStreamService {
        fn new(event_store: Arc<dyn EventStore>) -> Self {
            Self {
                event_store,
                saved_streams: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl EventStreamOperations for EventStreamService {
        async fn create_stream(
            &self,
            name: String,
            description: String,
            query: EventQuery,
        ) -> Result<EventStream, EventStreamError> {
            // Query events based on the query type
            let events = match &query {
                EventQuery::ByCorrelationId { correlation_id, .. } => {
                    // Get all events and filter by correlation ID
                    let all_events = self
                        .event_store
                        .stream_all_events(None)
                        .await
                        .map_err(|e| EventStreamError::EventStoreError(e.to_string()))?;

                    let mut filtered_events = vec![];
                    let mut stream = Box::into_pin(all_events);
                    while let Some(result) = stream.next().await {
                        if let Ok(event) = result {
                            if event.correlation_id() == Some(correlation_id) {
                                filtered_events.push(event);
                            }
                        }
                    }
                    filtered_events
                }
                _ => vec![], // Other query types not implemented for testing
            };

            Ok(EventStream::new(name, description, query, events))
        }

        async fn transform_stream(
            &self,
            stream: &EventStream,
            _transformation: crate::infrastructure::event_stream::StreamTransformation,
        ) -> Result<EventStream, EventStreamError> {
            Ok(stream.clone())
        }

        async fn compose_streams(
            &self,
            streams: Vec<EventStream>,
            composition: StreamComposition,
        ) -> Result<EventStream, EventStreamError> {
            if streams.is_empty() {
                return Err(EventStreamError::InvalidOperation(
                    "Cannot compose empty stream list".to_string(),
                ));
            }

            // Simple implementation for testing
            match composition {
                StreamComposition::Union => {
                    let mut all_events = vec![];
                    let mut seen_ids = std::collections::HashSet::new();
                    for stream in streams {
                        for event in stream.events {
                            if seen_ids.insert(event.event_id.clone()) {
                                all_events.push(event);
                            }
                        }
                    }
                    Ok(EventStream::new(
                        "Union".to_string(),
                        "Union of streams".to_string(),
                        EventQuery::Complex {
                            filters: vec![],
                            ordering: EventOrdering::Temporal,
                            limit: None,
                        },
                        all_events,
                    ))
                }
                StreamComposition::Intersection => {
                    if streams.len() < 2 {
                        return Ok(streams.into_iter().next().unwrap());
                    }
                    let first_ids: std::collections::HashSet<_> =
                        streams[0].events.iter().map(|e| &e.event_id).collect();
                    let mut result_events = vec![];
                    for event in &streams[1].events {
                        if first_ids.contains(&event.event_id) {
                            result_events.push(event.clone());
                        }
                    }
                    Ok(EventStream::new(
                        "Intersection".to_string(),
                        "Intersection of streams".to_string(),
                        EventQuery::Complex {
                            filters: vec![],
                            ordering: EventOrdering::Temporal,
                            limit: None,
                        },
                        result_events,
                    ))
                }
                StreamComposition::Difference => {
                    if streams.len() < 2 {
                        return Ok(streams.into_iter().next().unwrap());
                    }
                    let other_ids: std::collections::HashSet<_> =
                        streams[1].events.iter().map(|e| &e.event_id).collect();
                    let mut result_events = vec![];
                    for event in &streams[0].events {
                        if !other_ids.contains(&event.event_id) {
                            result_events.push(event.clone());
                        }
                    }
                    Ok(EventStream::new(
                        "Difference".to_string(),
                        "Difference of streams".to_string(),
                        EventQuery::Complex {
                            filters: vec![],
                            ordering: EventOrdering::Temporal,
                            limit: None,
                        },
                        result_events,
                    ))
                }
                _ => Err(EventStreamError::InvalidOperation(
                    "Unsupported composition".to_string(),
                )),
            }
        }

        async fn save_stream(&self, stream: &EventStream) -> Result<(), EventStreamError> {
            let mut streams = self.saved_streams.lock().await;
            streams.insert(stream.id.clone(), stream.clone());
            Ok(())
        }

        async fn load_stream(
            &self,
            stream_id: &EventStreamId,
        ) -> Result<EventStream, EventStreamError> {
            let streams = self.saved_streams.lock().await;
            streams
                .get(stream_id)
                .cloned()
                .ok_or_else(|| EventStreamError::StreamNotFound(stream_id.clone()))
        }

        async fn list_streams(&self) -> Result<Vec<EventStream>, EventStreamError> {
            let streams = self.saved_streams.lock().await;
            Ok(streams.values().cloned().collect())
        }
    }

    // Helper function to create test events
    fn create_test_stored_event(
        event_type: &str,
        aggregate_id: &str,
        aggregate_type: &str,
        sequence: u64,
        correlation_id: Option<String>,
        causation_id: Option<String>,
    ) -> StoredEvent {
        let event = match event_type {
            "WorkflowStarted" => DomainEventEnum::WorkflowStarted(WorkflowStarted {
                workflow_id: WorkflowId::new(),
                definition_id: GraphId::new(),
                initial_state: "Start".to_string(),
                started_at: Utc::now(),
            }),
            "WorkflowCompleted" => DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
                workflow_id: WorkflowId::new(),
                final_state: "End".to_string(),
                total_duration: std::time::Duration::from_secs(60),
                completed_at: Utc::now(),
            }),
            "WorkflowTransitioned" => DomainEventEnum::WorkflowTransitioned(WorkflowTransitioned {
                workflow_id: WorkflowId::new(),
                from_state: "Start".to_string(),
                to_state: "Processing".to_string(),
                transition_id: "transition-1".to_string(),
            }),
            _ => panic!("Unknown event type"),
        };

        StoredEvent {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: aggregate_type.to_string(),
            sequence,
            event: event.clone(),
            metadata: EventMetadata {
                correlation_id,
                causation_id,
                triggered_by: Some("test".to_string()),
                custom: None,
            },
            stored_at: Utc::now(),
        }
    }

    /// Test creating an event stream from a correlation ID query
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Events with Correlation ID] --> B[Query by Correlation ID]
    ///     B --> C[Verify All Events Retrieved]
    ///     C --> D[Check Metadata Calculated]
    /// ```
    #[tokio::test]
    async fn test_create_stream_by_correlation_id() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventStreamService::new(event_store.clone());

        let correlation_id = Uuid::new_v4().to_string();

        // Add test events with correlation ID
        let metadata = EventMetadata {
            correlation_id: Some(correlation_id.clone()),
            causation_id: None,
            triggered_by: Some("test".to_string()),
            custom: None,
        };

        event_store
            .append_events(
                "workflow-1",
                "Workflow",
                vec![DomainEventEnum::WorkflowStarted(WorkflowStarted {
                    workflow_id: WorkflowId::new(),
                    definition_id: GraphId::new(),
                    initial_state: "Start".to_string(),
                    started_at: Utc::now(),
                })],
                None,
                metadata.clone(),
            )
            .await
            .unwrap();

        event_store
            .append_events(
                "workflow-2",
                "Workflow",
                vec![DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
                    workflow_id: WorkflowId::new(),
                    final_state: "End".to_string(),
                    total_duration: std::time::Duration::from_secs(60),
                    completed_at: Utc::now(),
                })],
                None,
                metadata,
            )
            .await
            .unwrap();

        // Create stream by correlation ID
        let stream = service
            .create_stream(
                "Test Correlation Stream".to_string(),
                "Events with test correlation".to_string(),
                EventQuery::ByCorrelationId {
                    correlation_id: correlation_id.clone(),
                    order: CausationOrder::Temporal,
                },
            )
            .await
            .unwrap();

        // Verify results
        assert_eq!(stream.events.len(), 2);
        assert_eq!(stream.metadata.event_count, 2);
        assert!(stream.metadata.correlation_ids.contains(&correlation_id));
        assert_eq!(stream.metadata.aggregate_types.len(), 1);
        assert!(stream.metadata.aggregate_types.contains("Workflow"));
    }

    /// Test causation ordering of events
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Event Chain] --> B[Event 1: Root]
    ///     B --> C[Event 2: Caused by 1]
    ///     C --> D[Event 3: Caused by 2]
    ///     D --> E[Order by Causation]
    ///     E --> F[Verify Order: 1, 2, 3]
    /// ```
    #[tokio::test]
    async fn test_causation_ordering() {
        let mut events = vec![
            create_test_stored_event(
                "WorkflowStarted",
                "workflow-1",
                "Workflow",
                1,
                Some("corr-1".to_string()),
                None, // Root event
            ),
            create_test_stored_event(
                "WorkflowTransitioned",
                "workflow-1",
                "Workflow",
                2,
                Some("corr-1".to_string()),
                Some("event-1".to_string()), // Caused by first event
            ),
            create_test_stored_event(
                "WorkflowCompleted",
                "workflow-1",
                "Workflow",
                3,
                Some("corr-1".to_string()),
                Some("event-2".to_string()), // Caused by second event
            ),
        ];

        // Set different timestamps
        let base_time = Utc::now();
        events[0].stored_at = base_time - Duration::hours(2);
        events[1].stored_at = base_time - Duration::hours(1);
        events[2].stored_at = base_time;

        // Set proper event IDs for causation chain
        events[0].event_id = "event-1".to_string();
        events[1].event_id = "event-2".to_string();
        events[2].event_id = "event-3".to_string();

        // Shuffle to test ordering
        events.reverse();

        let mut stream = EventStream::new(
            "Test Stream".to_string(),
            "Test".to_string(),
            EventQuery::ByCorrelationId {
                correlation_id: "corr-1".to_string(),
                order: CausationOrder::Causal,
            },
            events,
        );

        stream.order_by_causation();

        // Verify causal order
        assert_eq!(stream.events[0].event_id, "event-1");
        assert_eq!(stream.events[1].event_id, "event-2");
        assert_eq!(stream.events[2].event_id, "event-3");
    }

    /// Test filtering event streams
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Mixed Events] --> B[Filter by Event Type]
    ///     B --> C[Verify Only Matching Events]
    ///     A --> D[Filter by Aggregate Type]
    ///     D --> E[Verify Only Matching Aggregates]
    /// ```
    #[tokio::test]
    async fn test_stream_filtering() {
        let events = vec![
            create_test_stored_event("WorkflowStarted", "workflow-1", "Workflow", 1, None, None),
            create_test_stored_event(
                "WorkflowTransitioned",
                "workflow-1",
                "Workflow",
                2,
                None,
                None,
            ),
            create_test_stored_event("WorkflowStarted", "workflow-2", "Workflow", 1, None, None),
            create_test_stored_event("WorkflowCompleted", "workflow-1", "Workflow", 3, None, None),
        ];

        let stream = EventStream::new(
            "Test Stream".to_string(),
            "Test".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            events,
        );

        // Filter by event type
        let started_stream = stream.filter(|e| e.event_type() == "WorkflowStarted");
        assert_eq!(started_stream.events.len(), 2);

        // Filter by aggregate ID
        let workflow1_stream = stream.filter(|e| e.aggregate_id == "workflow-1");
        assert_eq!(workflow1_stream.events.len(), 3);
        assert_eq!(workflow1_stream.events[0].event_type(), "WorkflowStarted");
    }

    /// Test stream composition operations
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Stream 1] --> B[Create Stream 2]
    ///     B --> C[Union Operation]
    ///     C --> D[Verify Combined Events]
    ///     B --> E[Intersection Operation]
    ///     E --> F[Verify Common Events]
    ///     B --> G[Difference Operation]
    ///     G --> H[Verify Unique Events]
    /// ```
    #[tokio::test]
    async fn test_stream_composition() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventStreamService::new(event_store);

        // Create test events
        let event1 =
            create_test_stored_event("WorkflowStarted", "workflow-1", "Workflow", 1, None, None);
        let event2 = create_test_stored_event(
            "WorkflowTransitioned",
            "workflow-1",
            "Workflow",
            2,
            None,
            None,
        );
        let event3 =
            create_test_stored_event("WorkflowCompleted", "workflow-1", "Workflow", 3, None, None);

        let stream1 = EventStream::new(
            "Stream 1".to_string(),
            "Test".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            vec![event1.clone(), event2.clone()],
        );

        let stream2 = EventStream::new(
            "Stream 2".to_string(),
            "Test".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            vec![event2.clone(), event3.clone()],
        );

        // Test union
        let union = service
            .compose_streams(
                vec![stream1.clone(), stream2.clone()],
                StreamComposition::Union,
            )
            .await
            .unwrap();
        assert_eq!(union.events.len(), 3); // event1, event2, event3 (no duplicates)

        // Test intersection
        let intersection = service
            .compose_streams(
                vec![stream1.clone(), stream2.clone()],
                StreamComposition::Intersection,
            )
            .await
            .unwrap();
        assert_eq!(intersection.events.len(), 1); // Only event2 is in both
        assert_eq!(intersection.events[0].event_id, event2.event_id);

        // Test difference
        let difference = service
            .compose_streams(vec![stream1, stream2], StreamComposition::Difference)
            .await
            .unwrap();
        assert_eq!(difference.events.len(), 1); // Only event1 is unique to stream1
        assert_eq!(difference.events[0].event_id, event1.event_id);
    }

    /// Test saving and loading streams
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Stream] --> B[Save Stream]
    ///     B --> C[List Saved Streams]
    ///     C --> D[Verify Stream Listed]
    ///     B --> E[Load Stream by ID]
    ///     E --> F[Verify Stream Content]
    /// ```
    #[tokio::test]
    async fn test_save_and_load_stream() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventStreamService::new(event_store);

        let events = vec![create_test_stored_event(
            "WorkflowStarted",
            "workflow-1",
            "Workflow",
            1,
            None,
            None,
        )];

        let stream = EventStream::new(
            "Test Stream".to_string(),
            "A test stream".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            events,
        );

        let stream_id = stream.id.clone();

        // Save stream
        service.save_stream(&stream).await.unwrap();

        // List streams
        let saved_streams = service.list_streams().await.unwrap();
        assert_eq!(saved_streams.len(), 1);
        assert_eq!(saved_streams[0].name, "Test Stream");

        // Load stream
        let loaded = service.load_stream(&stream_id).await.unwrap();
        assert_eq!(loaded.name, stream.name);
        assert_eq!(loaded.events.len(), stream.events.len());
        assert_eq!(loaded.id, stream_id);
    }

    /// Test grouping events by correlation
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Events with Different Correlations] --> B[Group by Correlation]
    ///     B --> C[Verify Group 1]
    ///     B --> D[Verify Group 2]
    ///     B --> E[Verify Ungrouped]
    /// ```
    #[test]
    fn test_group_by_correlation() {
        let events = vec![
            create_test_stored_event(
                "WorkflowStarted",
                "workflow-1",
                "Workflow",
                1,
                Some("corr-1".to_string()),
                None,
            ),
            create_test_stored_event(
                "WorkflowTransitioned",
                "workflow-1",
                "Workflow",
                2,
                Some("corr-1".to_string()),
                None,
            ),
            create_test_stored_event(
                "WorkflowStarted",
                "workflow-2",
                "Workflow",
                1,
                Some("corr-2".to_string()),
                None,
            ),
            create_test_stored_event("WorkflowCompleted", "workflow-1", "Workflow", 3, None, None),
        ];

        let stream = EventStream::new(
            "Test Stream".to_string(),
            "Test".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            events,
        );

        let groups = stream.group_by_correlation();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("corr-1").unwrap().len(), 2);
        assert_eq!(groups.get("corr-2").unwrap().len(), 1);
    }

    /// Test metadata calculation
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Events with Time Range] --> B[Calculate Metadata]
    ///     B --> C[Verify Event Count]
    ///     B --> D[Verify Time Range]
    ///     B --> E[Verify Aggregate Types]
    ///     B --> F[Verify Correlation IDs]
    /// ```
    #[test]
    fn test_metadata_calculation() {
        let mut events = vec![
            create_test_stored_event(
                "WorkflowStarted",
                "workflow-1",
                "Workflow",
                1,
                Some("corr-1".to_string()),
                None,
            ),
            create_test_stored_event(
                "WorkflowTransitioned",
                "workflow-1",
                "Workflow",
                2,
                Some("corr-1".to_string()),
                None,
            ),
            create_test_stored_event(
                "WorkflowStarted",
                "workflow-2",
                "Workflow",
                1,
                Some("corr-2".to_string()),
                None,
            ),
        ];

        // Set different timestamps
        let base_time = Utc::now();
        events[0].stored_at = base_time - Duration::hours(2);
        events[1].stored_at = base_time - Duration::hours(1);
        events[2].stored_at = base_time;

        let stream = EventStream::new(
            "Test Stream".to_string(),
            "Test".to_string(),
            EventQuery::Complex {
                filters: vec![],
                ordering: EventOrdering::Temporal,
                limit: None,
            },
            events.clone(),
        );

        // Verify metadata
        assert_eq!(stream.metadata.event_count, 3);
        assert_eq!(stream.metadata.aggregate_types.len(), 1);
        assert!(stream.metadata.aggregate_types.contains("Workflow"));
        assert_eq!(stream.metadata.correlation_ids.len(), 2);
        assert!(stream.metadata.correlation_ids.contains("corr-1"));
        assert!(stream.metadata.correlation_ids.contains("corr-2"));

        // Verify time range
        let time_range = stream.metadata.time_range.unwrap();
        assert_eq!(time_range.start, events[0].stored_at);
        assert_eq!(time_range.end, events[2].stored_at);
    }

    /// Test error handling
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Try Load Non-existent Stream] --> B[Expect StreamNotFound Error]
    ///     C[Try Compose Empty Streams] --> D[Expect InvalidOperation Error]
    /// ```
    #[tokio::test]
    async fn test_error_handling() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventStreamService::new(event_store);

        // Test loading non-existent stream
        let result = service.load_stream(&EventStreamId::new()).await;
        match result {
            Err(EventStreamError::StreamNotFound(_)) => {}
            _ => panic!("Expected StreamNotFound error"),
        }

        // Test composing empty stream list
        let result = service
            .compose_streams(vec![], StreamComposition::Union)
            .await;
        match result {
            Err(EventStreamError::InvalidOperation(_)) => {}
            _ => panic!("Expected InvalidOperation error"),
        }
    }
}
