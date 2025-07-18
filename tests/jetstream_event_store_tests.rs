// Copyright 2025 Cowboy AI, LLC.

//! Integration tests for JetStreamEventStore
//!
//! These tests require a running NATS server with JetStream enabled.
//! Run with: `nats-server -js`

use cim_domain::infrastructure::{
    EventStore, EventStoreError, EventMetadata,
    JetStreamEventStore,
    NatsClient, NatsConfig,
    jetstream_event_store::JetStreamConfig,
};
use cim_domain::{
    DomainEventEnum,
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    WorkflowSuspended, WorkflowResumed, WorkflowCancelled,
    WorkflowId, GraphId,
};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use chrono::{self, Utc};
use serde_json::json;

/// Helper to check if NATS is available
async fn nats_available() -> bool {
    let config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        ..Default::default()
    };

    match NatsClient::connect(config).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Create a test event store with a unique stream name
async fn create_test_event_store(test_name: &str) -> Result<JetStreamEventStore, EventStoreError> {
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        ..Default::default()
    };

    let client = NatsClient::connect(nats_config)
        .await
        .map_err(|e| EventStoreError::ConnectionError(e.to_string()))?;

    let config = JetStreamConfig {
        stream_name: format!("TEST-EVENTS-{}", test_name.to_uppercase()),
        stream_subjects: vec![format!("test.{test_name}.events.>")],
        cache_size: 10_000,
        subject_prefix: format!("test.{test_name}"),
    };

    JetStreamEventStore::new(client.client().clone(), config).await
}

/// Test basic event append and retrieval
///
/// ```mermaid
/// graph TD
///     A[Connect to NATS] --> B[Create JetStream Store]
///     B --> C[Append Events]
///     C --> D[Retrieve Events]
///     D --> E[Verify CID Chain]
///     E --> F[Verify Event Content]
/// ```
#[tokio::test]
async fn test_jetstream_append_and_retrieve() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("append-retrieve").await.unwrap();
    let aggregate_id = "workflow-123";
    let aggregate_type = "Workflow";

    // Create test events
    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    let events = vec![
        DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id: workflow_id.clone(),
            definition_id: definition_id.clone(),
            initial_state: "draft".to_string(),
            started_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: workflow_id.clone(),
            from_state: "draft".to_string(),
            to_state: "submitted".to_string(),
            input: json!({"action": "submit"}),
            output: json!({"success": true}),
            executed_at: Utc::now(),
        }),
    ];

    let metadata = EventMetadata {
        correlation_id: Some("test-correlation-123".to_string()),
        causation_id: None,
        triggered_by: Some("test-user".to_string()),
        custom: Some(serde_json::json!({
            "test": true,
            "source": "integration-test"
        })),
    };

    // Append events
    store
        .append_events(aggregate_id, aggregate_type, events.clone(), None, metadata.clone())
        .await
        .unwrap();

    // Wait a bit for JetStream to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Retrieve events
    let retrieved = store.get_events(aggregate_id, None).await.unwrap();
    assert_eq!(retrieved.len(), 2);

    // Verify content
    assert_eq!(retrieved[0].aggregate_id, aggregate_id);
    assert_eq!(retrieved[0].aggregate_type, aggregate_type);
    assert_eq!(retrieved[0].sequence, 1);
    assert_eq!(retrieved[1].sequence, 2);

    // Verify metadata preserved
    assert_eq!(retrieved[0].metadata.correlation_id, metadata.correlation_id);
    assert_eq!(retrieved[0].metadata.triggered_by, metadata.triggered_by);
}

/// Test optimistic concurrency control with JetStream
///
/// ```mermaid
/// graph TD
///     A[Append Initial Events] --> B[Get Current Version]
///     B --> C[Try Append with Old Version]
///     C --> D[Expect Concurrency Error]
///     D --> E[Append with Correct Version]
///     E --> F[Success]
/// ```
#[tokio::test]
async fn test_jetstream_concurrency_control() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("concurrency").await.unwrap();
    let aggregate_id = "workflow-456";
    let aggregate_type = "Workflow";

    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    let event1 = DomainEventEnum::WorkflowStarted(WorkflowStarted {
        workflow_id: workflow_id.clone(),
        definition_id: definition_id.clone(),
        initial_state: "initial".to_string(),
        started_at: Utc::now(),
    });

    let event2 = DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
        workflow_id: workflow_id.clone(),
        final_state: "completed".to_string(),
        total_duration: Duration::from_secs(300),
        completed_at: Utc::now(),
    });

    let metadata = EventMetadata::default();

    // Append first event
    store
        .append_events(aggregate_id, aggregate_type, vec![event1], None, metadata.clone())
        .await
        .unwrap();

    // Get current version
    let version = store.get_aggregate_version(aggregate_id).await.unwrap();
    assert_eq!(version, Some(1));

    // Try to append with wrong version
    let result = store
        .append_events(aggregate_id, aggregate_type, vec![event2.clone()], Some(0), metadata.clone())
        .await;

    match result {
        Err(EventStoreError::ConcurrencyConflict { expected: 0, current: 1 }) => {},
        _ => panic!("Expected concurrency conflict"),
    }

    // Append with correct version
    store
        .append_events(aggregate_id, aggregate_type, vec![event2], Some(1), metadata)
        .await
        .unwrap();

    let final_version = store.get_aggregate_version(aggregate_id).await.unwrap();
    assert_eq!(final_version, Some(2));
}

/// Test CID chain verification with JetStream
///
/// ```mermaid
/// graph TD
///     A[Create Event Store with CID Verification] --> B[Append Multiple Events]
///     B --> C[Each Event Gets CID]
///     C --> D[Retrieve Events]
///     D --> E[CID Chain Verified Automatically]
///     E --> F[Test Chain Integrity]
/// ```
#[tokio::test]
async fn test_jetstream_cid_chain_verification() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("cid-chain").await.unwrap();
    let aggregate_id = "workflow-789";
    let aggregate_type = "Workflow";

    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    // Create a series of events
    let events: Vec<DomainEventEnum> = vec![
        DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id: workflow_id.clone(),
            definition_id: definition_id.clone(),
            initial_state: "initial".to_string(),
            started_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: workflow_id.clone(),
            from_state: "initial".to_string(),
            to_state: "processing".to_string(),
            input: json!({"step": 1}),
            output: json!({"processed": true}),
            executed_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowSuspended(WorkflowSuspended {
            workflow_id: workflow_id.clone(),
            current_state: "processing".to_string(),
            reason: "Manual suspension".to_string(),
            suspended_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowResumed(WorkflowResumed {
            workflow_id: workflow_id.clone(),
            current_state: "processing".to_string(),
            resumed_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
            workflow_id: workflow_id.clone(),
            final_state: "completed".to_string(),
            total_duration: Duration::from_secs(600),
            completed_at: Utc::now(),
        }),
    ];

    let metadata = EventMetadata {
        correlation_id: Some("cid-test".to_string()),
        ..Default::default()
    };

    // Append all events
    store
        .append_events(aggregate_id, aggregate_type, events, None, metadata)
        .await
        .unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Retrieve events - CID chain verification happens automatically
    let retrieved = store.get_events(aggregate_id, None).await.unwrap();
    assert_eq!(retrieved.len(), 5);

    // Verify sequences are correct
    for (i, event) in retrieved.iter().enumerate() {
        assert_eq!(event.sequence, (i + 1) as u64);
    }
}

/// Test event filtering by version
///
/// ```mermaid
/// graph TD
///     A[Append 10 Events] --> B[Get All Events]
///     B --> C[Count = 10]
///     A --> D[Get Events from Version 5]
///     D --> E[Count = 5]
///     A --> F[Get Events from Version 8]
///     F --> G[Count = 2]
/// ```
#[tokio::test]
async fn test_jetstream_event_filtering() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("filtering").await.unwrap();
    let aggregate_id = "filter-test";
    let aggregate_type = "Workflow";

    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    // Create many events
    let events: Vec<DomainEventEnum> = (0..10)
        .map(|i| {
            DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
                workflow_id: workflow_id.clone(),
                from_state: format!("state-{}", i),
                to_state: format!("state-{}", i + 1),
                input: json!({"step": i}),
                output: json!({"result": format!("step-{}-complete", i)}),
                executed_at: Utc::now(),
            })
        })
        .collect();

    let metadata = EventMetadata::default();

    // Append in batches to test multiple appends
    for chunk in events.chunks(3) {
        store
            .append_events(aggregate_id, aggregate_type, chunk.to_vec(), None, metadata.clone())
            .await
            .unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Get all events
    let all_events = store.get_events(aggregate_id, None).await.unwrap();
    println!("DEBUG: Retrieved {} events for aggregate {}", all_events.len(), aggregate_id);
    for (i, event) in all_events.iter().enumerate() {
        println!("  Event {}: aggregate_id={}, sequence={}", i, event.aggregate_id, event.sequence);
    }
    assert_eq!(all_events.len(), 10);

    // Get events from version 5
    let from_v5 = store.get_events(aggregate_id, Some(5)).await.unwrap();
    assert_eq!(from_v5.len(), 5);
    assert_eq!(from_v5[0].sequence, 6);

    // Get events from version 8
    let from_v8 = store.get_events(aggregate_id, Some(8)).await.unwrap();
    assert_eq!(from_v8.len(), 2);
    assert_eq!(from_v8[0].sequence, 9);
}

/// Test multiple aggregates in same stream
///
/// ```mermaid
/// graph TD
///     A[Create Event Store] --> B[Append Events for Aggregate 1]
///     B --> C[Append Events for Aggregate 2]
///     C --> D[Append Events for Aggregate 3]
///     D --> E[Retrieve Each Aggregate's Events]
///     E --> F[Verify Isolation]
/// ```
#[tokio::test]
async fn test_jetstream_multiple_aggregates() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("multi-aggregate").await.unwrap();
    let metadata = EventMetadata::default();

    // Create events for different aggregates
    let aggregates = vec![
        ("workflow-1", "Workflow"),
        ("workflow-2", "Workflow"),
        ("workflow-3", "Workflow"),
    ];

    for (agg_id, agg_type) in &aggregates {
        let workflow_id = WorkflowId::new();
        let definition_id = GraphId::new();
        
        let events = vec![
            DomainEventEnum::WorkflowStarted(WorkflowStarted {
                workflow_id: workflow_id.clone(),
                definition_id: definition_id.clone(),
                initial_state: format!("initial-{agg_id}"),
                started_at: Utc::now(),
            }),
        ];

        store
            .append_events(agg_id, agg_type, events, None, metadata.clone())
            .await
            .unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify each aggregate has its own events
    for (agg_id, _) in &aggregates {
        let events = store.get_events(agg_id, None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].aggregate_id, *agg_id);
        assert_eq!(events[0].sequence, 1);
    }
}

/// Test cache behavior
///
/// ```mermaid
/// graph TD
///     A[Append Events] --> B[First Retrieval - From JetStream]
///     B --> C[Second Retrieval - From Cache]
///     C --> D[Append More Events]
///     D --> E[Cache Updated]
///     E --> F[Verify All Events Present]
/// ```
#[tokio::test]
async fn test_jetstream_cache_behavior() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS server not available");
        return;
    }

    let store = create_test_event_store("cache").await.unwrap();
    let aggregate_id = "cache-test";
    let aggregate_type = "Workflow";
    let metadata = EventMetadata::default();

    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    
    // Initial events
    let event1 = DomainEventEnum::WorkflowStarted(WorkflowStarted {
        workflow_id: workflow_id.clone(),
        definition_id: definition_id.clone(),
        initial_state: "initial".to_string(),
        started_at: Utc::now(),
    });

    store
        .append_events(aggregate_id, aggregate_type, vec![event1], None, metadata.clone())
        .await
        .unwrap();

    // First retrieval - populates cache
    let first_retrieval = store.get_events(aggregate_id, None).await.unwrap();
    assert_eq!(first_retrieval.len(), 1);

    // Second retrieval - should use cache
    let second_retrieval = store.get_events(aggregate_id, None).await.unwrap();
    assert_eq!(second_retrieval.len(), 1);

    // Add more events
    let event2 = DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
        workflow_id: workflow_id.clone(),
        from_state: "initial".to_string(),
        to_state: "processing".to_string(),
        input: json!({"action": "process"}),
        output: json!({"status": "started"}),
        executed_at: Utc::now(),
    });

    store
        .append_events(aggregate_id, aggregate_type, vec![event2], Some(1), metadata)
        .await
        .unwrap();

    // Retrieve again - cache should be updated
    let final_retrieval = store.get_events(aggregate_id, None).await.unwrap();
    assert_eq!(final_retrieval.len(), 2);
}
