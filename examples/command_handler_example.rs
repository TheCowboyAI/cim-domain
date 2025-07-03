//! Command Handler Example - CIM Domain
//! 
//! This example demonstrates the command handling patterns in CIM's event-driven architecture.
//! It showcases how commands flow through handlers to aggregates, generating domain events.
//!
//! Key concepts demonstrated:
//! - Command validation and processing
//! - Event generation from aggregates
//! - Cross-domain integration via events
//! - Async/sync bridge patterns

use cim_domain::{
    // Core command handling
    Command, CommandEnvelope, CommandHandler, CommandStatus, CommandId,
    EventPublisher, AggregateRepository, InMemoryRepository,
    AggregateRoot, AggregateId, EntityId, AggregateMarker,
    DomainEvent, DomainEventEnum, DomainError, DomainResult,
    CorrelationId, CausationId,
    
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, InMemoryEventStore},
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Example domain aggregate
#[derive(Debug, Clone)]
struct ExampleAggregate {
    id: EntityId<AggregateMarker>,
    state: String,
    version: u64,
}

impl ExampleAggregate {
    fn new(id: EntityId<AggregateMarker>) -> Self {
        Self {
            id,
            state: "initial".to_string(),
            version: 0,
        }
    }

    fn handle_command(&mut self, command: ExampleCommand) -> DomainResult<Vec<ExampleEvent>> {
        match command {
            ExampleCommand::Initialize { name } => {
                if self.version > 0 {
                    return Err(DomainError::generic("Already initialized"));
                }
                Ok(vec![ExampleEvent::Initialized { 
                    aggregate_id: self.id.clone(),
                    name 
                }])
            }
            ExampleCommand::UpdateState { new_state } => {
                if self.version == 0 {
                    return Err(DomainError::generic("Not initialized"));
                }
                Ok(vec![ExampleEvent::StateUpdated { 
                    aggregate_id: self.id.clone(),
                    old_state: self.state.clone(),
                    new_state 
                }])
            }
        }
    }

    fn apply_event(&mut self, event: &ExampleEvent) -> DomainResult<()> {
        match event {
            ExampleEvent::Initialized { .. } => {
                self.version = 1;
            }
            ExampleEvent::StateUpdated { new_state, .. } => {
                self.state = new_state.clone();
                self.version += 1;
            }
        }
        Ok(())
    }
}

impl AggregateRoot for ExampleAggregate {
    type Id = EntityId<AggregateMarker>;
    
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

/// Example commands
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ExampleCommand {
    Initialize { name: String },
    UpdateState { new_state: String },
}

impl Command for ExampleCommand {
    fn command_type(&self) -> &'static str {
        match self {
            Self::Initialize { .. } => "Initialize",
            Self::UpdateState { .. } => "UpdateState",
        }
    }
}

/// Example events
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ExampleEvent {
    Initialized { 
        aggregate_id: EntityId<AggregateMarker>,
        name: String 
    },
    StateUpdated { 
        aggregate_id: EntityId<AggregateMarker>,
        old_state: String,
        new_state: String 
    },
}

impl DomainEvent for ExampleEvent {
    fn aggregate_id(&self) -> Uuid {
        match self {
            Self::Initialized { aggregate_id, .. } => aggregate_id.as_uuid(),
            Self::StateUpdated { aggregate_id, .. } => aggregate_id.as_uuid(),
        }
    }

    fn event_type(&self) -> &'static str {
        match self {
            Self::Initialized { .. } => "Initialized",
            Self::StateUpdated { .. } => "StateUpdated",
        }
    }

    fn subject(&self) -> String {
        format!("example.aggregate.{self.event_type(}.v1").to_lowercase())
    }
}

/// Example command handler
struct ExampleCommandHandler<R: AggregateRepository<ExampleAggregate>> {
    repository: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: AggregateRepository<ExampleAggregate>> ExampleCommandHandler<R> {
    async fn handle_command(
        &self,
        envelope: CommandEnvelope<ExampleCommand>,
    ) -> DomainResult<CommandStatus> {
        println!("📋 Processing {envelope.payload.command_type(} command"));
        
        let aggregate_id = EntityId::<AggregateMarker>::new();
        
        // Load or create aggregate
        let mut aggregate = self.repository
            .load(&aggregate_id)
            .await
            .unwrap_or_else(|_| ExampleAggregate::new(aggregate_id.clone()));
        
        // Process command
        let events = aggregate.handle_command(envelope.payload)?;
        
        // Apply events to aggregate
        for event in &events {
            aggregate.apply_event(event)?;
        }
        
        // Save aggregate
        self.repository.save(&aggregate).await?;
        
        // Publish events
        let domain_events: Vec<DomainEventEnum> = events.into_iter()
            .map(|e| DomainEventEnum::WorkflowStarted(cim_domain::WorkflowStarted {
                workflow_id: cim_domain::WorkflowId::new(),
                definition_id: cim_domain::GraphId::new(),
                initial_state: "example".to_string(),
                started_at: chrono::Utc::now(),
            }))
            .collect();
        
        self.event_publisher.publish_events(
            domain_events,
            envelope.correlation_id.clone()
        )?;
        
        println!("✅ Command processed successfully");
        
        Ok(CommandStatus::Accepted)
    }
}

/// Mock event publisher
struct MockEventPublisher {
    events: Arc<RwLock<Vec<(DomainEventEnum, CorrelationId)>>>,
}

impl MockEventPublisher {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn get_published_events(&self) -> Vec<(DomainEventEnum, CorrelationId)> {
        self.events.read().await.clone()
    }
}

impl EventPublisher for MockEventPublisher {
    fn publish_events(
        &self, 
        events: Vec<DomainEventEnum>, 
        correlation_id: CorrelationId
    ) -> Result<(), String> {
        let events_clone = self.events.clone();
        tokio::spawn(async move {
            let mut guard = events_clone.write().await;
            for event in events {
                guard.push((event, correlation_id.clone()));
            }
        });
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CIM Command Handler Example\n");
    
    // Setup infrastructure
    let repository = InMemoryRepository::<ExampleAggregate>::new();
    let event_publisher = Arc::new(MockEventPublisher::new());
    
    let handler = ExampleCommandHandler {
        repository,
        event_publisher: event_publisher.clone(),
    };
    
    // Example 1: Initialize aggregate
    println!("=== Example 1: Initialize Aggregate ===");
    
    let command = ExampleCommand::Initialize {
        name: "Example Aggregate".to_string(),
    };
    
    let envelope = CommandEnvelope {
        command_id: CommandId::new(),
        correlation_id: CorrelationId::from_uuid(Uuid::new_v4()),
        causation_id: CausationId::from_uuid(Uuid::new_v4()),
        payload: command,
    };
    
    let status = handler.handle_command(envelope).await?;
    println!("Command status: {:?}\n", status);
    
    // Example 2: Update state
    println!("=== Example 2: Update State ===");
    
    let command = ExampleCommand::UpdateState {
        new_state: "active".to_string(),
    };
    
    let envelope = CommandEnvelope {
        command_id: CommandId::new(),
        correlation_id: CorrelationId::from_uuid(Uuid::new_v4()),
        causation_id: CausationId::from_uuid(Uuid::new_v4()),
        payload: command,
    };
    
    let status = handler.handle_command(envelope).await?;
    println!("Command status: {:?}\n", status);
    
    // Example 3: Show published events
    println!("=== Example 3: Published Events ===");
    
    let published = event_publisher.get_published_events().await;
    println!("Total events published: {published.len(}"));
    
    for (i, (event, correlation_id)) in published.iter().enumerate() {
        println!("{i + 1}. Event: {event.event_type(} (Correlation: {})"),
            &correlation_id.to_string()[..8]
        );
    }
    
    println!("\n✅ Example completed successfully!");
    
    Ok(())
}
