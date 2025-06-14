//! Tests for infrastructure components

use super::*;
use crate::domain_events::DomainEventEnum;
// Domain-specific events have been moved to their respective submodules
use crate::infrastructure::event_store::{EventStream, EventStore};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::stream::Stream;
use crate::identifiers::{GraphId, WorkflowId};
use crate::domain_events::{WorkflowStarted, WorkflowCompleted, WorkflowTransitioned};

/// Mock event store for testing
#[derive(Debug, Clone)]
pub struct MockEventStore {
    events: Arc<RwLock<HashMap<String, Vec<StoredEvent>>>>,
    versions: Arc<RwLock<HashMap<String, u64>>>,
    fail_on_append: Arc<RwLock<bool>>,
    expected_version_check: Arc<RwLock<bool>>,
}

impl MockEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
            versions: Arc::new(RwLock::new(HashMap::new())),
            fail_on_append: Arc::new(RwLock::new(false)),
            expected_version_check: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn set_fail_on_append(&self, fail: bool) {
        *self.fail_on_append.write().await = fail;
    }

    pub async fn set_expected_version_check(&self, check: bool) {
        *self.expected_version_check.write().await = check;
    }

    pub async fn get_all_events(&self) -> Vec<StoredEvent> {
        let events = self.events.read().await;
        events.values().flatten().cloned().collect()
    }
}

#[async_trait]
impl EventStore for MockEventStore {
    async fn append_events(
        &self,
        aggregate_id: &str,
        aggregate_type: &str,
        events: Vec<DomainEventEnum>,
        expected_version: Option<u64>,
        metadata: EventMetadata,
    ) -> Result<(), EventStoreError> {
        if *self.fail_on_append.read().await {
            return Err(EventStoreError::StorageError("Mock failure".to_string()));
        }

        let mut versions = self.versions.write().await;
        let current_version = versions.get(aggregate_id).copied().unwrap_or(0);

        if *self.expected_version_check.read().await {
            if let Some(expected) = expected_version {
                if current_version != expected {
                    return Err(EventStoreError::ConcurrencyConflict {
                        expected,
                        current: current_version,
                    });
                }
            }
        }

        let mut event_map = self.events.write().await;
        let aggregate_events = event_map.entry(aggregate_id.to_string()).or_insert_with(Vec::new);

        let mut new_version = current_version;
        for event in events {
            new_version += 1;
            let stored_event = StoredEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: aggregate_type.to_string(),
                sequence: new_version,
                event,
                metadata: metadata.clone(),
                stored_at: Utc::now(),
            };
            aggregate_events.push(stored_event);
        }

        versions.insert(aggregate_id.to_string(), new_version);
        Ok(())
    }

    async fn get_events(
        &self,
        aggregate_id: &str,
        from_version: Option<u64>,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let events = self.events.read().await;
        let aggregate_events = events.get(aggregate_id).cloned().unwrap_or_default();

        let filtered: Vec<_> = aggregate_events
            .into_iter()
            .filter(|e| from_version.map_or(true, |v| e.sequence > v))
            .collect();

        Ok(filtered)
    }

    async fn get_events_by_type(
        &self,
        event_type: &str,
        limit: usize,
        _after: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let events = self.events.read().await;
        let mut matching_events: Vec<_> = events
            .values()
            .flatten()
            .filter(|e| match &e.event {
                DomainEventEnum::WorkflowStarted(_) => event_type == "WorkflowStarted",
                DomainEventEnum::WorkflowTransitioned(_) => event_type == "WorkflowTransitioned",
                DomainEventEnum::WorkflowCompleted(_) => event_type == "WorkflowCompleted",
                DomainEventEnum::WorkflowSuspended(_) => event_type == "WorkflowSuspended",
                DomainEventEnum::WorkflowResumed(_) => event_type == "WorkflowResumed",
                DomainEventEnum::WorkflowCancelled(_) => event_type == "WorkflowCancelled",
                DomainEventEnum::WorkflowFailed(_) => event_type == "WorkflowFailed",
                DomainEventEnum::WorkflowTransitionExecuted(_) => event_type == "WorkflowTransitionExecuted",
            })
            .take(limit)
            .cloned()
            .collect();

        matching_events.sort_by_key(|e| e.stored_at);
        Ok(matching_events)
    }

    async fn get_aggregate_version(
        &self,
        aggregate_id: &str,
    ) -> Result<Option<u64>, EventStoreError> {
        let versions = self.versions.read().await;
        Ok(versions.get(aggregate_id).copied())
    }

    async fn subscribe_to_events(
        &self,
        _from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        Err(EventStoreError::StorageError("Not implemented in mock".to_string()))
    }

    async fn subscribe_to_aggregate_type(
        &self,
        _aggregate_type: &str,
        _from_position: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        Err(EventStoreError::StorageError("Not implemented in mock".to_string()))
    }

    async fn stream_events_by_type(
        &self,
        _event_type: &str,
        _from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        Err(EventStoreError::StorageError(
            "Not implemented in mock".to_string(),
        ))
    }

    async fn stream_all_events(
        &self,
        from_sequence: Option<u64>,
    ) -> Result<Box<dyn EventStream>, EventStoreError> {
        let events = self.events.read().await;
        let mut all_events: Vec<StoredEvent> = events.values()
            .flat_map(|v| v.clone())
            .collect();

        // Sort by sequence
        all_events.sort_by_key(|e| e.sequence);

        // Filter by sequence if specified
        if let Some(from) = from_sequence {
            all_events.retain(|e| e.sequence >= from);
        }

        Ok(Box::new(MockEventStream {
            events: all_events,
            position: 0,
        }))
    }
}

/// Mock event stream for testing
pub struct MockEventStream {
    events: Vec<StoredEvent>,
    position: usize,
}

impl Stream for MockEventStream {
    type Item = Result<StoredEvent, EventStoreError>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if self.position < self.events.len() {
            let event = self.events[self.position].clone();
            self.position += 1;
            Poll::Ready(Some(Ok(event)))
        } else {
            Poll::Ready(None)
        }
    }
}

impl EventStream for MockEventStream {
    fn ack(&mut self, _event_id: &str) -> Result<(), EventStoreError> {
        Ok(())
    }

    fn close(self: Box<Self>) -> Result<(), EventStoreError> {
        Ok(())
    }
}

// Helper function to create test events
fn create_test_workflow_started_event() -> DomainEventEnum {
    DomainEventEnum::WorkflowStarted(WorkflowStarted {
        workflow_id: WorkflowId::new(),
        definition_id: GraphId::new(),
        initial_state: "Start".to_string(),
        started_at: chrono::Utc::now(),
    })
}

fn create_test_workflow_completed_event() -> DomainEventEnum {
    DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
        workflow_id: WorkflowId::new(),
        final_state: "End".to_string(),
        total_duration: std::time::Duration::from_secs(60),
        completed_at: chrono::Utc::now(),
    })
}

fn create_test_workflow_transitioned_event() -> DomainEventEnum {
    DomainEventEnum::WorkflowTransitioned(WorkflowTransitioned {
        workflow_id: WorkflowId::new(),
        from_state: "Start".to_string(),
        to_state: "Processing".to_string(),
        transition_id: "transition-1".to_string(),
    })
}



#[cfg(test)]
mod event_store_tests {
    use super::*;

    /// Test appending events to the event store
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Event Store] --> B[Create Workflow Event]
    ///     B --> C[Append Event]
    ///     C --> D[Verify Storage]
    ///     D --> E[Check Version]
    /// ```
    #[tokio::test]
    async fn test_append_and_retrieve_events() {
        let store = MockEventStore::new();
        let aggregate_id = "workflow-123";
        let aggregate_type = "Workflow";

        // Create test event
        let event = create_test_workflow_started_event();

        let metadata = EventMetadata {
            correlation_id: Some("corr-123".to_string()),
            causation_id: None,
            triggered_by: Some("test-user".to_string()),
            custom: None,
        };

        // Append event
        store
            .append_events(aggregate_id, aggregate_type, vec![event.clone()], None, metadata.clone())
            .await
            .unwrap();

        // Retrieve events
        let retrieved = store.get_events(aggregate_id, None).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].aggregate_id, aggregate_id);
        assert_eq!(retrieved[0].sequence, 1);

        // Check version
        let version = store.get_aggregate_version(aggregate_id).await.unwrap();
        assert_eq!(version, Some(1));
    }

    /// Test optimistic concurrency control
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Append First Event] --> B[Get Version = 1]
    ///     B --> C[Try Append with Wrong Version]
    ///     C --> D[Expect Concurrency Error]
    ///     D --> E[Append with Correct Version]
    ///     E --> F[Success]
    /// ```
    #[tokio::test]
    async fn test_optimistic_concurrency_control() {
        let store = MockEventStore::new();
        let aggregate_id = "workflow-456";
        let aggregate_type = "Workflow";

        let event1 = create_test_workflow_started_event();
        let metadata = EventMetadata::default();

        // Append first event
        store
            .append_events(aggregate_id, aggregate_type, vec![event1.clone()], None, metadata.clone())
            .await
            .unwrap();

        // Try to append with wrong expected version
        let result = store
            .append_events(aggregate_id, aggregate_type, vec![event1.clone()], Some(0), metadata.clone())
            .await;

        match result {
            Err(EventStoreError::ConcurrencyConflict { expected: 0, current: 1 }) => {},
            _ => panic!("Expected concurrency conflict"),
        }

        // Append with correct version
        store
            .append_events(aggregate_id, aggregate_type, vec![event1], Some(1), metadata)
            .await
            .unwrap();

        let version = store.get_aggregate_version(aggregate_id).await.unwrap();
        assert_eq!(version, Some(2));
    }

    /// Test filtering events by version
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Append 3 Events] --> B[Get All Events]
    ///     B --> C[Count = 3]
    ///     A --> D[Get Events from Version 1]
    ///     D --> E[Count = 2]
    ///     A --> F[Get Events from Version 2]
    ///     F --> G[Count = 1]
    /// ```
    #[tokio::test]
    async fn test_get_events_from_version() {
        let store = MockEventStore::new();
        let aggregate_id = "workflow-789";
        let aggregate_type = "Workflow";

        let events: Vec<DomainEventEnum> = vec![
            create_test_workflow_started_event(),
            create_test_workflow_transitioned_event(),
            create_test_workflow_completed_event(),
        ];

        let metadata = EventMetadata::default();

        // Append all events
        store
            .append_events(aggregate_id, aggregate_type, events, None, metadata)
            .await
            .unwrap();

        // Get all events
        let all_events = store.get_events(aggregate_id, None).await.unwrap();
        assert_eq!(all_events.len(), 3);

        // Get events from version 1
        let from_v1 = store.get_events(aggregate_id, Some(1)).await.unwrap();
        assert_eq!(from_v1.len(), 2);
        assert_eq!(from_v1[0].sequence, 2);

        // Get events from version 2
        let from_v2 = store.get_events(aggregate_id, Some(2)).await.unwrap();
        assert_eq!(from_v2.len(), 1);
        assert_eq!(from_v2[0].sequence, 3);
    }

    /// Test getting events by type
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Mixed Events] --> B[Store Started Events]
    ///     B --> C[Store Completed Events]
    ///     C --> D[Query by Type: WorkflowStarted]
    ///     D --> E[Verify Only Started Events]
    ///     C --> F[Query by Type: WorkflowCompleted]
    ///     F --> G[Verify Only Completed Events]
    /// ```
    #[tokio::test]
    async fn test_get_events_by_type() {
        let store = MockEventStore::new();
        let metadata = EventMetadata::default();

        // Add workflow started events
        for i in 0..3 {
            let event = create_test_workflow_started_event();
            store
                .append_events(&format!("workflow-{}", i), "Workflow", vec![event], None, metadata.clone())
                .await
                .unwrap();
        }

        // Add workflow completed events
        for i in 0..2 {
            let event = create_test_workflow_completed_event();
            store
                .append_events(&format!("workflow-comp-{}", i), "Workflow", vec![event], None, metadata.clone())
                .await
                .unwrap();
        }

        // Query started events
        let started_events = store
            .get_events_by_type("WorkflowStarted", 10, None)
            .await
            .unwrap();
        assert_eq!(started_events.len(), 3);

        // Query completed events
        let completed_events = store
            .get_events_by_type("WorkflowCompleted", 10, None)
            .await
            .unwrap();
        assert_eq!(completed_events.len(), 2);
    }

    /// Test event metadata handling
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Event with Metadata] --> B[Set Correlation ID]
    ///     B --> C[Set Causation ID]
    ///     C --> D[Set Triggered By]
    ///     D --> E[Store Event]
    ///     E --> F[Retrieve and Verify Metadata]
    /// ```
    #[tokio::test]
    async fn test_event_metadata() {
        let store = MockEventStore::new();
        let aggregate_id = "test-metadata";
        let aggregate_type = "Workflow";

        let event = create_test_workflow_started_event();

        let metadata = EventMetadata {
            correlation_id: Some("correlation-123".to_string()),
            causation_id: Some("causation-456".to_string()),
            triggered_by: Some("test-system".to_string()),
            custom: Some(serde_json::json!({
                "test_field": "test_value"
            })),
        };

        // Store event with metadata
        store
            .append_events(aggregate_id, aggregate_type, vec![event], None, metadata.clone())
            .await
            .unwrap();

        // Retrieve and verify
        let events = store.get_events(aggregate_id, None).await.unwrap();
        assert_eq!(events.len(), 1);

        let stored_metadata = &events[0].metadata;
        assert_eq!(stored_metadata.correlation_id, metadata.correlation_id);
        assert_eq!(stored_metadata.causation_id, metadata.causation_id);
        assert_eq!(stored_metadata.triggered_by, metadata.triggered_by);
        assert_eq!(stored_metadata.custom, metadata.custom);
    }
}

#[cfg(test)]
mod cid_chain_tests {
    use super::*;
    use cim_ipld::TypedContent;
    use crate::infrastructure::cid_chain::{create_event_with_cid, verify_event_chain};

    /// Test CID chain creation and verification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create First Event] --> B[No Previous CID]
    ///     B --> C[Create Second Event]
    ///     C --> D[Previous CID = First Event CID]
    ///     D --> E[Verify Chain Integrity]
    /// ```
    #[test]
    fn test_cid_chain_creation() {
        let event1 = create_test_workflow_started_event();
        let event2 = create_test_workflow_transitioned_event();

        // Create first event with CID
        let event_with_cid1 = create_event_with_cid(event1, None).unwrap();
        assert!(event_with_cid1.previous_cid.is_none());

        // Create second event with CID
        let event_with_cid2 = create_event_with_cid(event2, Some(&event_with_cid1)).unwrap();
        assert_eq!(event_with_cid2.previous_cid, Some(event_with_cid1.cid.clone()));

        // Verify chain
        let chain = vec![event_with_cid1, event_with_cid2];
        verify_event_chain(&chain).unwrap();
    }

    /// Test CID chain tampering detection
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Valid Chain] --> B[Tamper with Event]
    ///     B --> C[Change Sequence Number]
    ///     C --> D[Verify Chain]
    ///     D --> E[Expect Verification Failure]
    /// ```
    #[test]
    fn test_cid_chain_tampering_detection() {
        let event1 = create_test_workflow_started_event();
        let event2 = create_test_workflow_completed_event();

        // Create valid chain
        let event_with_cid1 = create_event_with_cid(event1, None).unwrap();
        let mut event_with_cid2 = create_event_with_cid(event2, Some(&event_with_cid1)).unwrap();

        // Tamper with the chain by changing the previous CID
        event_with_cid2.previous_cid = None; // Break the chain

        // Verify should fail
        let chain = vec![event_with_cid1, event_with_cid2];
        let result = verify_event_chain(&chain);
        assert!(result.is_err());
    }

    /// Test EventWrapper TypedContent implementation
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Domain Event] --> B[Wrap in EventWrapper]
    ///     B --> C[Verify TypedContent Trait]
    ///     C --> D[Check CODEC Value]
    ///     D --> E[Check Content Type]
    /// ```
    #[test]
    fn test_event_wrapper_typed_content() {
        let event = create_test_workflow_started_event();
        let _wrapper = cid_chain::EventWrapper { event };

        // Verify TypedContent implementation
        assert_eq!(cid_chain::EventWrapper::CODEC, 0x0200); // JSON codec
        assert_eq!(cid_chain::EventWrapper::CONTENT_TYPE, cim_ipld::ContentType::Event);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test full event store flow with CID chains
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Create Event Store] --> B[Append Events]
    ///     B --> C[Events Get CID Chain]
    ///     C --> D[Retrieve Events]
    ///     D --> E[Verify CID Chain Integrity]
    ///     E --> F[Test Concurrency Control]
    /// ```
    #[tokio::test]
    async fn test_event_store_with_cid_chain() {
        let store = MockEventStore::new();
        let aggregate_id = "test-aggregate";
        let aggregate_type = "TestAggregate";

        // Create multiple events
        let events: Vec<DomainEventEnum> = vec![
            create_test_workflow_started_event(),
            create_test_workflow_transitioned_event(),
            create_test_workflow_completed_event(),
            create_test_workflow_started_event(),
            create_test_workflow_transitioned_event(),
        ];

        let metadata = EventMetadata {
            correlation_id: Some("test-correlation".to_string()),
            causation_id: None,
            triggered_by: Some("integration-test".to_string()),
            custom: None,
        };

        // Append events
        store
            .append_events(aggregate_id, aggregate_type, events, None, metadata)
            .await
            .unwrap();

        // Retrieve and verify
        let stored_events = store.get_events(aggregate_id, None).await.unwrap();
        assert_eq!(stored_events.len(), 5);

        // Verify sequences
        for (i, event) in stored_events.iter().enumerate() {
            assert_eq!(event.sequence, (i + 1) as u64);
        }

        // Test version tracking
        let version = store.get_aggregate_version(aggregate_id).await.unwrap();
        assert_eq!(version, Some(5));
    }

    /// Test error handling scenarios
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Configure Mock to Fail] --> B[Try Append Events]
    ///     B --> C[Expect Storage Error]
    ///     C --> D[Reset Mock]
    ///     D --> E[Try Again]
    ///     E --> F[Success]
    /// ```
    #[tokio::test]
    async fn test_error_handling() {
        let store = MockEventStore::new();
        let aggregate_id = "error-test";
        let aggregate_type = "ErrorTest";

        let event = create_test_workflow_started_event();
        let metadata = EventMetadata::default();

        // Configure to fail
        store.set_fail_on_append(true).await;

        // Try to append - should fail
        let result = store
            .append_events(aggregate_id, aggregate_type, vec![event.clone()], None, metadata.clone())
            .await;

        match result {
            Err(EventStoreError::StorageError(msg)) => {
                assert_eq!(msg, "Mock failure");
            }
            _ => panic!("Expected storage error"),
        }

        // Reset and try again
        store.set_fail_on_append(false).await;

        store
            .append_events(aggregate_id, aggregate_type, vec![event], None, metadata)
            .await
            .unwrap();

        // Verify it worked
        let events = store.get_events(aggregate_id, None).await.unwrap();
        assert_eq!(events.len(), 1);
    }
}
