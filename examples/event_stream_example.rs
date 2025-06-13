//! Example demonstrating event streams as first-class objects

use chrono::Utc;
use cim_domain::infrastructure::{
    NatsClient, NatsConfig, JetStreamEventStore, JetStreamConfig,
    EventStreamService, EventStreamOperations, EventQuery, CausationOrder,
    EventFilter, EventOrdering, StreamTransformation, StreamComposition,
};
use cim_domain::command_handlers::{
    PersonCommandHandler, OrganizationCommandHandler, WorkflowCommandHandler,
    EventPublisher, AggregateRepository, InMemoryRepository,
};
use cim_domain::commands::{
    PersonCommand, OrganizationCommand, WorkflowCommand,
};
use cim_domain::value_objects::{PersonName, Address, OrganizationType};
use cim_domain::workflow::{SimpleState, SimpleTransition};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Event Streams as First-Class Objects Demo ===\n");

    // Setup infrastructure
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        ..Default::default()
    };

    let nats_client = NatsClient::connect(nats_config).await?;

    let jetstream_config = JetStreamConfig {
        stream_name: "EVENT_STREAM_DEMO".to_string(),
        stream_subjects: vec!["demo.stream.events.>".to_string()],
        cache_size: 100,
        subject_prefix: "demo.stream.events".to_string(),
    };

    let event_store = Arc::new(
        JetStreamEventStore::new(nats_client.client().clone(), jetstream_config).await?
    );

    // Create event stream service
    let stream_service = EventStreamService::new(event_store.clone());

    // Create some test data with correlation
    let correlation_id = Uuid::new_v4().to_string();
    println!("Creating test data with correlation ID: {}", correlation_id);

    // Create repositories and handlers
    let person_repo = Arc::new(InMemoryRepository::new());
    let org_repo = Arc::new(InMemoryRepository::new());
    let workflow_repo = Arc::new(InMemoryRepository::new());

    let mut person_handler = PersonCommandHandler::new(
        person_repo.clone(),
        event_store.clone(),
    );

    let mut org_handler = OrganizationCommandHandler::new(
        org_repo.clone(),
        event_store.clone(),
    );

    let mut workflow_handler = WorkflowCommandHandler::new(
        workflow_repo.clone(),
        event_store.clone(),
    );

    // Create a person
    let person_id = Uuid::new_v4().to_string();
    person_handler.handle(PersonCommand::RegisterPerson {
        person_id: person_id.clone(),
        name: PersonName {
            given_name: "Alice".to_string(),
            family_name: "Smith".to_string(),
            middle_names: vec![],
        },
        email: "alice@example.com".to_string(),
        correlation_id: Some(correlation_id.clone()),
    }).await?;

    // Create an organization
    let org_id = Uuid::new_v4().to_string();
    org_handler.handle(OrganizationCommand::CreateOrganization {
        organization_id: org_id.clone(),
        name: "Tech Corp".to_string(),
        org_type: OrganizationType::Company,
        address: Address {
            street1: "123 Tech St".to_string(),
            street2: None,
            locality: "San Francisco".to_string(),
            region: "CA".to_string(),
            postal_code: "94105".to_string(),
            country: "USA".to_string(),
        },
        correlation_id: Some(correlation_id.clone()),
    }).await?;

    // Add person to organization
    person_handler.handle(PersonCommand::JoinOrganization {
        person_id: person_id.clone(),
        organization_id: org_id.clone(),
        role: "Engineer".to_string(),
        correlation_id: Some(correlation_id.clone()),
    }).await?;

    // Create a workflow
    let workflow_id = Uuid::new_v4().to_string();
    let instance_id = Uuid::new_v4().to_string();

    workflow_handler.handle(WorkflowCommand::StartWorkflow {
        workflow_id: workflow_id.clone(),
        instance_id: instance_id.clone(),
        initial_state: Box::new(SimpleState {
            id: "draft".to_string(),
            name: "Draft".to_string(),
            is_terminal: false,
        }),
        context: HashMap::from([
            ("person_id".to_string(), person_id.clone()),
            ("org_id".to_string(), org_id.clone()),
        ]),
        correlation_id: Some(correlation_id.clone()),
    }).await?;

    // Execute workflow transition
    workflow_handler.handle(WorkflowCommand::ExecuteTransition {
        instance_id: instance_id.clone(),
        transition: Box::new(SimpleTransition {
            id: "submit".to_string(),
            name: "Submit for Review".to_string(),
            from_state: "draft".to_string(),
            to_state: "review".to_string(),
            guard: None,
        }),
        input: HashMap::new(),
        correlation_id: Some(correlation_id.clone()),
    }).await?;

    // Wait a bit for events to be persisted
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    println!("\n=== Demonstrating Event Stream Queries ===\n");

    // 1. Query by correlation ID with causal ordering
    println!("1. Creating event stream by correlation ID (causal order):");
    let correlation_stream = stream_service.create_stream(
        "User Onboarding Flow".to_string(),
        "All events related to user onboarding process".to_string(),
        EventQuery::ByCorrelationId {
            correlation_id: correlation_id.clone(),
            order: CausationOrder::Causal,
        },
    ).await?;

    println!("   Found {} events in correlation stream", correlation_stream.events.len());
    for event in &correlation_stream.events {
        println!("   - {} [{}] at {:?}",
            event.event_type,
            event.aggregate_type,
            event.timestamp
        );
    }

    // 2. Query by time range
    println!("\n2. Creating event stream by time range:");
    let time_stream = stream_service.create_stream(
        "Recent Activity".to_string(),
        "Events from the last hour".to_string(),
        EventQuery::ByTimeRange {
            start: Utc::now() - chrono::Duration::hours(1),
            end: Utc::now(),
        },
    ).await?;

    println!("   Found {} events in time range", time_stream.events.len());

    // 3. Query by aggregate type
    println!("\n3. Creating event stream by aggregate type:");
    let person_stream = stream_service.create_stream(
        "Person Events".to_string(),
        "All events related to people".to_string(),
        EventQuery::ByAggregateType {
            aggregate_type: "Person".to_string(),
        },
    ).await?;

    println!("   Found {} person events", person_stream.events.len());

    // 4. Complex query with filters
    println!("\n4. Creating event stream with complex filters:");
    let complex_stream = stream_service.create_stream(
        "Workflow State Changes".to_string(),
        "All workflow transition events".to_string(),
        EventQuery::Complex {
            filters: vec![
                EventFilter::AggregateType("Workflow".to_string()),
                EventFilter::EventTypes(vec![
                    "WorkflowStarted".to_string(),
                    "WorkflowTransitionExecuted".to_string(),
                ]),
            ],
            ordering: EventOrdering::Temporal,
            limit: Some(10),
        },
    ).await?;

    println!("   Found {} workflow events", complex_stream.events.len());

    // 5. Transform streams
    println!("\n5. Transforming event streams:");
    let filtered_stream = stream_service.transform_stream(
        &correlation_stream,
        StreamTransformation::Filter(
            EventFilter::EventType("PersonRegistered".to_string())
        ),
    ).await?;

    println!("   Filtered stream has {} events", filtered_stream.events.len());

    // 6. Compose streams
    println!("\n6. Composing multiple streams:");
    let composed_stream = stream_service.compose_streams(
        vec![person_stream, complex_stream],
        StreamComposition::Union,
    ).await?;

    println!("   Composed stream has {} events", composed_stream.events.len());

    // 7. Save and load streams
    println!("\n7. Saving and loading streams:");
    stream_service.save_stream(&correlation_stream).await?;

    let saved_streams = stream_service.list_streams().await?;
    println!("   Saved {} streams", saved_streams.len());

    let loaded_stream = stream_service.load_stream(&correlation_stream.id).await?;
    println!("   Loaded stream '{}' with {} events",
        loaded_stream.name,
        loaded_stream.events.len()
    );

    // 8. Demonstrate causation ordering
    println!("\n8. Demonstrating causation ordering:");
    let mut causal_stream = correlation_stream.clone();
    causal_stream.order_by_causation();

    println!("   Events in causal order:");
    for (i, event) in causal_stream.events.iter().enumerate() {
        let causation_info = if let Some(cid) = &event.causation_id {
            format!("caused by {}", &cid[..8])
        } else {
            "root event".to_string()
        };
        println!("   {}. {} - {}", i + 1, event.event_type, causation_info);
    }

    // 9. Group by correlation
    println!("\n9. Grouping events by correlation:");
    let groups = correlation_stream.group_by_correlation();
    for (corr_id, events) in groups {
        println!("   Correlation {}: {} events", &corr_id[..8], events.len());
    }

    println!("\n=== Event Stream Metadata ===");
    println!("Stream: {}", correlation_stream.name);
    println!("Description: {}", correlation_stream.description);
    println!("Event count: {}", correlation_stream.metadata.event_count);
    println!("Aggregate types: {:?}", correlation_stream.metadata.aggregate_types);
    if let Some(time_range) = &correlation_stream.metadata.time_range {
        println!("Time range: {} to {}", time_range.start, time_range.end);
    }

    println!("\n=== Demo Complete ===");
    println!("\nEvent streams enable powerful analysis capabilities:");
    println!("- Query events by correlation, time, aggregate, or complex filters");
    println!("- Transform streams with filtering, grouping, and windowing");
    println!("- Compose multiple streams with set operations");
    println!("- Save and share important event queries");
    println!("- Visualize event flows as ContextGraphs");

    Ok(())
}
