// Copyright 2025 Cowboy AI, LLC.

//! Advanced Event Sourcing Demo - CIM Architecture
//!
//! This demo showcases the complete event sourcing implementation including:
//! - NATS JetStream event store with caching
//! - Automatic snapshot policies
//! - Event versioning and upcasting
//! - Projection checkpointing
//! - Saga pattern for distributed transactions
//! - Event replay with filtering and windowing

use cim_domain::{
    // Aggregates
    aggregates::person::{Person, PersonCreated, PersonEmailUpdated, PersonPhoneUpdated},
    entity::AggregateRoot,
    
    // Events
    events::{DomainEvent, DomainEventEnum, DomainEventMetadata, PropagationScope},
    
    // Commands
    commands::{CreatePerson, UpdatePersonEmail},
    
    // Infrastructure
    infrastructure::{
        // Event Store
        EventStore, JetStreamEventStore, StoredEvent,
        
        // Snapshots
        SnapshotStore, JetStreamSnapshotStore, InMemorySnapshotStore,
        SnapshotPolicy, SnapshotPolicyEngine, AutoSnapshotService,
        
        // Event Versioning
        EventVersioningService, SimpleUpcaster, VersionedEvent,
        
        // Projections
        CheckpointStore, InMemoryCheckpointStore, CheckpointManager,
        ProjectionCheckpoint, EventPosition,
        
        // Replay
        EventReplayService, ReplayOptions, ProjectionBuilder, ProjectionHandler,
        
        // Saga
        Saga, SagaCoordinator, SagaStep, SagaInstance, SagaTransition,
        CommandBus, ProcessManager, ProcessPolicy, SagaCommand,
        
        // NATS
        NatsClient, NatsConfig,
    },
};

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use uuid::Uuid;

// Custom projection for person statistics
#[derive(Default, Debug)]
struct PersonStatisticsProjection {
    total_persons: u64,
    email_domains: HashMap<String, u32>,
    phone_area_codes: HashMap<String, u32>,
    updates_per_person: HashMap<String, u32>,
}

impl ProjectionHandler for PersonStatisticsProjection {
    fn handle_event(&mut self, event: &StoredEvent) -> Result<(), cim_domain::infrastructure::ReplayError> {
        match serde_json::from_value::<DomainEventEnum>(event.data.clone()) {
            Ok(DomainEventEnum::PersonCreated(e)) => {
                self.total_persons += 1;
                
                if let Some(domain) = e.email.split('@').nth(1) {
                    *self.email_domains.entry(domain.to_string()).or_insert(0) += 1;
                }
                
                *self.updates_per_person.entry(e.aggregate_id.clone()).or_insert(0) += 1;
            }
            Ok(DomainEventEnum::PersonEmailUpdated(e)) => {
                if let Some(new_domain) = e.new_email.split('@').nth(1) {
                    *self.email_domains.entry(new_domain.to_string()).or_insert(0) += 1;
                }
                if let Some(old_domain) = e.old_email.split('@').nth(1) {
                    if let Some(count) = self.email_domains.get_mut(old_domain) {
                        *count = count.saturating_sub(1);
                    }
                }
                
                *self.updates_per_person.entry(e.aggregate_id.clone()).or_insert(0) += 1;
            }
            Ok(DomainEventEnum::PersonPhoneUpdated(e)) => {
                if e.new_phone.len() >= 3 {
                    let area_code = &e.new_phone[..3];
                    *self.phone_area_codes.entry(area_code.to_string()).or_insert(0) += 1;
                }
                
                *self.updates_per_person.entry(e.aggregate_id.clone()).or_insert(0) += 1;
            }
            _ => {}
        }
        
        Ok(())
    }
}

// Onboarding saga for coordinating user creation across services
struct UserOnboardingSaga;

#[async_trait]
impl Saga for UserOnboardingSaga {
    fn saga_type(&self) -> &str {
        "UserOnboarding"
    }
    
    async fn start(&self, context: serde_json::Value) -> Result<Vec<SagaStep>, cim_domain::infrastructure::SagaError> {
        let email = context["email"].as_str().unwrap_or("").to_string();
        let first_name = context["first_name"].as_str().unwrap_or("").to_string();
        let last_name = context["last_name"].as_str().unwrap_or("").to_string();
        
        Ok(vec![
            SagaStep {
                name: "CreatePerson".to_string(),
                command: Box::new(MockSagaCommand {
                    name: "CreatePerson".to_string(),
                    aggregate_id: Uuid::new_v4().to_string(),
                }),
                compensation: None,
                timeout: std::time::Duration::from_secs(30),
            },
            // In a real system, these would be actual commands
            SagaStep {
                name: "CreateEmailAccount".to_string(),
                command: Box::new(MockSagaCommand {
                    name: "CreateEmailAccount".to_string(),
                    aggregate_id: Uuid::new_v4().to_string(),
                }),
                compensation: Some(Box::new(MockSagaCommand {
                    name: "DeleteEmailAccount".to_string(),
                    aggregate_id: Uuid::new_v4().to_string(),
                })),
                timeout: std::time::Duration::from_secs(30),
            },
            SagaStep {
                name: "SendWelcomeEmail".to_string(),
                command: Box::new(MockSagaCommand {
                    name: "SendWelcomeEmail".to_string(),
                    aggregate_id: Uuid::new_v4().to_string(),
                }),
                compensation: None,
                timeout: std::time::Duration::from_secs(60),
            },
        ])
    }
    
    async fn handle_event(
        &self,
        instance: &SagaInstance,
        event: &dyn DomainEvent,
    ) -> Result<SagaTransition, cim_domain::infrastructure::SagaError> {
        match (instance.current_step, event.event_type()) {
            (0, "PersonCreated") => Ok(SagaTransition::Continue),
            (1, "EmailAccountCreated") => Ok(SagaTransition::Continue),
            (2, "WelcomeEmailSent") => Ok(SagaTransition::Complete),
            (_, "PersonCreationFailed") => Ok(SagaTransition::Compensate),
            _ => Ok(SagaTransition::Continue),
        }
    }
}

// Mock saga command for demo
#[derive(Debug)]
struct MockSagaCommand {
    name: String,
    aggregate_id: String,
}

impl SagaCommand for MockSagaCommand {
    fn command_type(&self) -> &str {
        &self.name
    }
    
    fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Mock command bus for demo
struct MockCommandBus;

#[async_trait]
impl CommandBus for MockCommandBus {
    async fn send(&self, command: Box<dyn SagaCommand>) -> Result<(), String> {
        info!("Executing command: {}", command.command_type());
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("=== Advanced Event Sourcing Demo ===\n");
    
    // Use in-memory stores for demo (would use NATS in production)
    let event_store = Arc::new(cim_domain::infrastructure::InMemoryEventStore::new());
    let snapshot_store = Arc::new(InMemorySnapshotStore::new());
    let checkpoint_store = Arc::new(InMemoryCheckpointStore::new());
    
    // 1. Event Versioning
    println!("1️⃣ Event Versioning Demo\n");
    
    let mut versioning_service = EventVersioningService::new();
    versioning_service.register_event_type("PersonCreated".to_string(), 2);
    
    // Register upcaster from v1 to v2 (splits full_name into first/last)
    let upcaster = SimpleUpcaster::new(1, 2, |data| {
        let mut new_data = data.clone();
        if let Some(obj) = new_data.as_object_mut() {
            if let Some(full_name) = obj.get("full_name").and_then(|v| v.as_str()) {
                let parts: Vec<&str> = full_name.split_whitespace().collect();
                obj.insert("first_name".to_string(), json!(parts.get(0).unwrap_or(&"")));
                obj.insert("last_name".to_string(), json!(parts.get(1).unwrap_or(&"")));
                obj.remove("full_name");
            }
        }
        Ok(new_data)
    });
    
    versioning_service.register_upcaster("PersonCreated".to_string(), Box::new(upcaster));
    
    // Simulate old event
    let old_event = json!({
        "aggregate_id": "person-old",
        "full_name": "John Doe",
        "email": "john@example.com"
    });
    
    let upcast_result = versioning_service
        .upcast_event("PersonCreated", old_event, 1)?;
    
    println!("   ✅ Upcast v1 → v2:");
    println!("   Old: full_name = 'John Doe'");
    println!("   New: first_name = '{}', last_name = '{}'", 
        upcast_result["first_name"], upcast_result["last_name"]);
    
    // 2. Automatic Snapshot Policies
    println!("\n2️⃣ Automatic Snapshot Policies\n");
    
    let policy_engine = Arc::new(SnapshotPolicyEngine::new(snapshot_store.clone()));
    
    // Configure snapshot policy
    let policy = SnapshotPolicy {
        event_count_threshold: Some(5),
        time_interval: Some(chrono::Duration::minutes(10)),
        after_events: vec!["PersonEmailUpdated".to_string()],
        retention_count: 3,
        enabled: true,
    };
    
    policy_engine.register_policy("Person".to_string(), policy).await;
    
    let auto_snapshot = AutoSnapshotService::<Person>::new(
        policy_engine.clone(),
        snapshot_store.clone(),
    );
    
    println!("   📸 Snapshot Policy:");
    println!("   - Snapshot every 5 events");
    println!("   - Snapshot after email updates");
    println!("   - Keep last 3 snapshots");
    
    // 3. Event Sourcing with Projections
    println!("\n3️⃣ Event Sourcing & Projections\n");
    
    // Create some persons
    let person_ids = vec!["person-1", "person-2", "person-3"];
    let domains = vec!["gmail.com", "company.com", "gmail.com"];
    
    for (i, (id, domain)) in person_ids.iter().zip(domains.iter()).enumerate() {
        let person = Person::new(
            id.to_string(),
            format!("User{}", i + 1),
            "Smith".to_string(),
            format!("user{}@{}", i + 1, domain),
        );
        
        let event = DomainEventEnum::PersonCreated(PersonCreated {
            aggregate_id: id.to_string(),
            first_name: person.first_name.clone(),
            last_name: person.last_name.clone(),
            email: person.email.clone(),
            occurred_at: Utc::now(),
        });
        
        event_store.append_events(id, &[event], 0).await?;
        
        // Check if snapshot needed
        auto_snapshot.maybe_snapshot(&person, "PersonCreated", 1).await?;
    }
    
    // Update some emails
    for (i, id) in person_ids.iter().enumerate().take(2) {
        let event = DomainEventEnum::PersonEmailUpdated(PersonEmailUpdated {
            aggregate_id: id.to_string(),
            old_email: format!("user{}@{}", i + 1, domains[i]),
            new_email: format!("updated{}@newdomain.com", i + 1),
            occurred_at: Utc::now(),
        });
        
        event_store.append_events(id, &[event], 1).await?;
    }
    
    // Build projection with checkpointing
    let projection = Arc::new(RwLock::new(PersonStatisticsProjection::default()));
    let checkpoint_manager = Arc::new(CheckpointManager::new(checkpoint_store.clone()));
    
    let mut projection_builder = ProjectionBuilder::new(
        projection.clone(),
        Some(checkpoint_manager.clone()),
        "person-statistics".to_string(),
    );
    
    // Replay all events
    let replay_service = EventReplayService::new(event_store.clone());
    let options = ReplayOptions {
        batch_size: 10,
        ..Default::default()
    };
    
    let stats = replay_service
        .replay_all_events(&mut projection_builder, options)
        .await?;
    
    println!("   📊 Projection Results:");
    let proj = projection.read().await;
    println!("   - Total persons: {}", proj.total_persons);
    println!("   - Email domains: {:?}", proj.email_domains);
    println!("   - Events processed: {}", stats.events_processed);
    
    // Check checkpoint
    let position = checkpoint_manager
        .get_position("person-statistics")
        .await?;
    println!("   - Checkpoint saved at: {:?}", position);
    
    // 4. Saga Pattern Demo
    println!("\n4️⃣ Saga Pattern Demo\n");
    
    let command_bus = Arc::new(MockCommandBus);
    let saga_coordinator = Arc::new(SagaCoordinator::new(command_bus));
    
    // Register saga
    saga_coordinator.register_saga(Arc::new(UserOnboardingSaga)).await;
    
    // Start onboarding saga
    let saga_context = json!({
        "first_name": "Alice",
        "last_name": "Johnson",
        "email": "alice@example.com"
    });
    
    let saga_id = saga_coordinator
        .start_saga("UserOnboarding", saga_context)
        .await?;
    
    println!("   🔄 Started UserOnboarding saga: {}", saga_id);
    
    let saga_instance = saga_coordinator.get_instance(&saga_id).await.unwrap();
    println!("   - State: {:?}", saga_instance.state);
    println!("   - Current step: {}", saga_instance.current_step);
    
    // 5. Advanced Replay Options
    println!("\n5️⃣ Advanced Event Replay\n");
    
    // Replay with filtering
    let filtered_options = ReplayOptions {
        from_sequence: Some(2),
        to_sequence: Some(4),
        event_types: Some(vec!["PersonEmailUpdated".to_string()]),
        batch_size: 5,
    };
    
    let filtered_projection = Arc::new(RwLock::new(PersonStatisticsProjection::default()));
    let mut filtered_builder = ProjectionBuilder::new(
        filtered_projection.clone(),
        None,
        "filtered-projection".to_string(),
    );
    
    let filtered_stats = replay_service
        .replay_all_events(&mut filtered_builder, filtered_options)
        .await?;
    
    println!("   🔍 Filtered Replay:");
    println!("   - Events processed: {}", filtered_stats.events_processed);
    println!("   - Processing time: {:?}", filtered_stats.duration);
    
    // 6. Metrics and Monitoring
    println!("\n6️⃣ Event Sourcing Metrics\n");
    
    if let Some(metrics) = policy_engine.get_metrics("person-1").await {
        println!("   📈 Snapshot Metrics for person-1:");
        println!("   - Events since snapshot: {}", metrics.events_since_snapshot);
        println!("   - Total snapshots: {}", metrics.total_snapshots);
        println!("   - Last snapshot: {:?}", metrics.last_snapshot_at);
    }
    
    // Summary
    println!("\n✅ Advanced Event Sourcing Demo Complete!\n");
    
    println!("💡 Advanced Features Demonstrated:");
    println!("   - Event versioning with automatic upcasting");
    println!("   - Automatic snapshot policies with retention");
    println!("   - Projection checkpointing for fault tolerance");
    println!("   - Saga pattern for distributed transactions");
    println!("   - Advanced replay with filtering and windowing");
    println!("   - Production-ready metrics and monitoring");
    
    Ok(())
}