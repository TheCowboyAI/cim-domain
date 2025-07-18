// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating event streaming with NATS JetStream
//!
//! This example shows:
//! - Using JetStreamEventStore for event persistence
//! - Publishing domain events
//! - Loading events from the store
//! - Working with event metadata

use cim_domain::{
    // Core types
    EntityId, DomainError, DomainResult,
    markers::AggregateMarker,
    
    // Events
    DomainEventEnum,
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    
    // Infrastructure
    infrastructure::{
        jetstream_event_store::{JetStreamEventStore, JetStreamConfig},
        EventStore,
        event_store::EventMetadata as StoreEventMetadata,
    },
    
    // IDs
    WorkflowId, GraphId,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use futures::StreamExt;

/// Helper function to create event metadata
fn create_metadata() -> StoreEventMetadata {
    StoreEventMetadata {
        correlation_id: Some(Uuid::new_v4().to_string()),
        causation_id: Some(Uuid::new_v4().to_string()),
        triggered_by: Some("user-123".to_string()),
        custom: Some(json!({
            "session_id": "session-456",
            "request_id": Uuid::new_v4().to_string(),
        })),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Event Stream Example");
    println!("===================\n");
    
    // Note: This example requires a running NATS server with JetStream enabled
    // Run: docker run -p 4222:4222 nats:latest -js
    
    // Connect to NATS
    println!("1. Connecting to NATS...");
    let client = async_nats::connect("nats://localhost:4222").await?;
    println!("   ✓ Connected\n");
    
    // Create event store configuration
    let config = JetStreamConfig {
        stream_name: "workflow-events".to_string(),
        stream_subjects: vec!["events.>".to_string()],
        cache_size: 100,
        subject_prefix: "events".to_string(),
    };
    
    // Create event store
    println!("2. Creating event store...");
    let event_store = JetStreamEventStore::new(client, config).await?;
    println!("   ✓ Event store ready\n");
    
    // Create workflow IDs
    let workflow_id = WorkflowId::new();
    let definition_id = GraphId::new();
    let aggregate_id = workflow_id.to_string();
    
    // Create workflow events
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
            input: json!({"action": "submit", "user": "alice"}),
            output: json!({"success": true, "timestamp": Utc::now().to_rfc3339()}),
            executed_at: Utc::now(),
        }),
        DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
            workflow_id: workflow_id.clone(),
            final_state: "approved".to_string(),
            total_duration: std::time::Duration::from_secs(300),
            completed_at: Utc::now(),
        }),
    ];
    
    // Publish events
    println!("3. Publishing events...");
    let metadata = create_metadata();
    
    event_store.append_events(
        &aggregate_id,
        "Workflow",
        events.clone(),
        None, // No expected version for first append
        metadata,
    ).await?;
    
    println!("   ✓ Published {} events", events.len());
    for event in &events {
        println!("     - {}", match event {
            DomainEventEnum::WorkflowStarted(_) => "WorkflowStarted",
            DomainEventEnum::WorkflowTransitionExecuted(_) => "WorkflowTransitionExecuted",
            DomainEventEnum::WorkflowCompleted(_) => "WorkflowCompleted",
            _ => "Other",
        });
    }
    println!();
    
    // Load events for aggregate
    println!("4. Loading events for aggregate...");
    let loaded_events = event_store.get_events(&aggregate_id, None).await?;
    println!("   ✓ Loaded {} events", loaded_events.len());
    
    for (i, stored_event) in loaded_events.iter().enumerate() {
        println!("\n   Event {}:", i + 1);
        println!("     ID: {}", stored_event.event_id);
        println!("     Type: {}", stored_event.event_type());
        println!("     Sequence: {}", stored_event.sequence);
        println!("     Timestamp: {}", stored_event.timestamp());
        
        // Show metadata
        println!("     Metadata:");
        if let Some(corr_id) = &stored_event.metadata.correlation_id {
            println!("       Correlation: {}", &corr_id[..8]);
        }
        if let Some(caus_id) = &stored_event.metadata.causation_id {
            println!("       Causation: {}", &caus_id[..8]);
        }
        if let Some(triggered_by) = &stored_event.metadata.triggered_by {
            println!("       Triggered by: {}", triggered_by);
        }
    }
    
    // Demonstrate version checking
    println!("\n5. Demonstrating optimistic concurrency...");
    let current_version = event_store.get_aggregate_version(&aggregate_id).await?;
    println!("   Current version: {:?}", current_version);
    
    // Try to append with wrong expected version
    let new_event = DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
        workflow_id: workflow_id.clone(),
        from_state: "approved".to_string(),
        to_state: "archived".to_string(),
        input: json!({"reason": "completed"}),
        output: json!({"archived": true}),
        executed_at: Utc::now(),
    });
    
    match event_store.append_events(
        &aggregate_id,
        "Workflow",
        vec![new_event.clone()],
        Some(0), // Wrong version!
        create_metadata(),
    ).await {
        Ok(_) => println!("   ✗ Unexpected success!"),
        Err(e) => println!("   ✓ Expected error: {}", e),
    }
    
    // Append with correct version
    event_store.append_events(
        &aggregate_id,
        "Workflow",
        vec![new_event],
        current_version,
        create_metadata(),
    ).await?;
    println!("   ✓ Successfully appended with correct version");
    
    // Get events by type
    println!("\n6. Getting events by type...");
    let workflow_transitions = event_store.get_events_by_type(
        "WorkflowTransitionExecuted",
        10,
        None,
    ).await?;
    println!("   Found {} WorkflowTransitionExecuted events", workflow_transitions.len());
    
    // Subscribe to events
    println!("\n7. Subscribing to events...");
    let mut stream = event_store.subscribe_to_events(None).await?;
    println!("   ✓ Subscribed to event stream");
    
    // Read a few events from the stream
    println!("   Reading from stream (timeout after 1 second)...");
    let start = std::time::Instant::now();
    let mut count = 0;
    while start.elapsed() < std::time::Duration::from_secs(1) {
        match tokio::time::timeout(
            std::time::Duration::from_millis(100),
            stream.next()
        ).await {
            Ok(Some(Ok(event))) => {
                count += 1;
                println!("     Event {}: {} (seq {})", count, event.event_type(), event.sequence);
                if count >= 3 {
                    break;
                }
            }
            Ok(Some(Err(e))) => {
                println!("     Error reading event: {}", e);
                break;
            }
            Ok(None) => {
                // Stream ended
                break;
            }
            Err(_) => {
                // Timeout, continue
            }
        }
    }
    
    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • JetStream event store setup and configuration");
    println!("  • Publishing domain events with metadata");
    println!("  • Loading events for aggregates");
    println!("  • Optimistic concurrency control");
    println!("  • Getting events by type");
    println!("  • Subscribing to event streams");
    
    Ok(())
} 