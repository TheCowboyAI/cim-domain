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
    CommandEnvelope, CommandHandler, CommandStatus,
    EventPublisher, InMemoryRepository,
    
    // Workflow domain
    WorkflowAggregate, WorkflowId, WorkflowCommand,
    WorkflowStarted, WorkflowTransitionExecuted,
    
    // Graph integration
    GraphId, NodeId, EdgeId,
    
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, InMemoryEventStore},
        nats_client::{MockNatsClient},
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Example workflow command handler showing CIM patterns
struct WorkflowCommandHandler {
    event_store: Arc<InMemoryEventStore>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl WorkflowCommandHandler {
    async fn handle_start_workflow(
        &self,
        workflow_id: WorkflowId,
        graph_id: GraphId,
        initial_state: String,
    ) -> Result<CommandStatus, Box<dyn std::error::Error>> {
        println!("📋 Processing StartWorkflow command");
        
        // Load or create aggregate
        let mut aggregate = match self.event_store.load_events(&workflow_id.to_string()).await {
            Ok(events) if !events.is_empty() => {
                let mut agg = WorkflowAggregate::new(workflow_id.clone());
                for event in events {
                    agg.apply_event(event)?;
                }
                agg
            }
            _ => WorkflowAggregate::new(workflow_id.clone()),
        };
        
        // Process command through aggregate
        let command = WorkflowCommand::StartWorkflow {
            graph_id,
            initial_state: initial_state.clone(),
        };
        
        let events = aggregate.handle_command(command)?;
        
        // Persist events
        for event in &events {
            self.event_store.append_event(
                &workflow_id.to_string(),
                event.clone()
            ).await?;
            
            // Publish for cross-domain integration
            self.event_publisher.publish(event.clone()).await?;
        }
        
        println!("✅ Workflow started successfully");
        println!("   Generated {} events", events.len());
        
        Ok(CommandStatus::Completed {
            aggregate_id: workflow_id.to_string(),
            events_generated: events.len(),
        })
    }
    
    async fn handle_execute_transition(
        &self,
        workflow_id: WorkflowId,
        from_node: NodeId,
        to_node: NodeId,
        transition_data: serde_json::Value,
    ) -> Result<CommandStatus, Box<dyn std::error::Error>> {
        println!("🔄 Processing ExecuteTransition command");
        
        // Load aggregate with all events
        let events = self.event_store.load_events(&workflow_id.to_string()).await?;
        let mut aggregate = WorkflowAggregate::new(workflow_id.clone());
        
        for event in events {
            aggregate.apply_event(event)?;
        }
        
        // Validate workflow is started
        if aggregate.current_state().is_none() {
            return Ok(CommandStatus::Rejected {
                reason: "Workflow not started".to_string(),
            });
        }
        
        // Process transition command
        let command = WorkflowCommand::ExecuteTransition {
            from_node,
            to_node,
            transition_data: transition_data.clone(),
        };
        
        let events = aggregate.handle_command(command)?;
        
        // Persist and publish
        for event in &events {
            self.event_store.append_event(
                &workflow_id.to_string(),
                event.clone()
            ).await?;
            
            self.event_publisher.publish(event.clone()).await?;
        }
        
        println!("✅ Transition executed successfully");
        
        Ok(CommandStatus::Completed {
            aggregate_id: workflow_id.to_string(),
            events_generated: events.len(),
        })
    }
}

/// Example of cross-domain event handling
struct CrossDomainEventHandler {
    graph_commands: Arc<RwLock<Vec<GraphCommand>>>,
}

impl CrossDomainEventHandler {
    async fn handle_workflow_event(
        &self,
        event: &DomainEventEnum,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            DomainEventEnum::WorkflowStarted(e) => {
                println!("🔗 Cross-domain: Creating graph visualization for workflow");
                
                // Generate graph commands from workflow event
                let graph_command = GraphCommand::CreateGraph {
                    graph_id: GraphId::new(),
                    name: format!("Workflow {}", e.workflow_id),
                    graph_type: GraphType::WorkflowGraph,
                };
                
                self.graph_commands.write().await.push(graph_command);
            }
            DomainEventEnum::WorkflowTransitionExecuted(e) => {
                println!("🔗 Cross-domain: Updating graph for transition");
                
                // Update graph visualization
                let edge_command = GraphCommand::HighlightEdge {
                    from_node: e.from_node.clone(),
                    to_node: e.to_node.clone(),
                    highlight_type: "active_transition".to_string(),
                };
                
                self.graph_commands.write().await.push(edge_command);
            }
            _ => {}
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CIM Command Handler Example\n");
    
    // Setup infrastructure
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_publisher = Arc::new(MockEventPublisher::new());
    
    let handler = WorkflowCommandHandler {
        event_store: event_store.clone(),
        event_publisher: event_publisher.clone(),
    };
    
    // Setup cross-domain handler
    let cross_domain = CrossDomainEventHandler {
        graph_commands: Arc::new(RwLock::new(Vec::new())),
    };
    
    // Example 1: Start a workflow
    println!("=== Example 1: Starting a Workflow ===");
    let workflow_id = WorkflowId::new();
    let graph_id = GraphId::new();
    
    let status = handler.handle_start_workflow(
        workflow_id.clone(),
        graph_id,
        "initial".to_string(),
    ).await?;
    
    match status {
        CommandStatus::Completed { events_generated, .. } => {
            println!("✓ Command completed, {} events generated\n", events_generated);
        }
        CommandStatus::Rejected { reason } => {
            println!("✗ Command rejected: {}\n", reason);
        }
    }
    
    // Example 2: Execute a transition
    println!("=== Example 2: Executing a Transition ===");
    let from_node = NodeId::new();
    let to_node = NodeId::new();
    
    let transition_data = serde_json::json!({
        "action": "approve",
        "user": "alice@example.com",
        "timestamp": chrono::Utc::now(),
    });
    
    let status = handler.handle_execute_transition(
        workflow_id.clone(),
        from_node,
        to_node,
        transition_data,
    ).await?;
    
    match status {
        CommandStatus::Completed { .. } => {
            println!("✓ Transition executed successfully\n");
        }
        CommandStatus::Rejected { reason } => {
            println!("✗ Transition rejected: {}\n", reason);
        }
    }
    
    // Example 3: Show cross-domain integration
    println!("=== Example 3: Cross-Domain Event Processing ===");
    
    // Process published events
    let published_events = event_publisher.get_published_events().await;
    for event in published_events {
        cross_domain.handle_workflow_event(&event).await?;
    }
    
    // Show generated graph commands
    let graph_commands = cross_domain.graph_commands.read().await;
    println!("Generated {} graph commands:", graph_commands.len());
    for cmd in graph_commands.iter() {
        println!("  - {:?}", cmd);
    }
    
    println!("\n✅ Example completed successfully!");
    
    Ok(())
}

/// Mock event publisher for the example
struct MockEventPublisher {
    events: Arc<RwLock<Vec<DomainEventEnum>>>,
}

impl MockEventPublisher {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn get_published_events(&self) -> Vec<DomainEventEnum> {
        self.events.read().await.clone()
    }
}

#[async_trait::async_trait]
impl EventPublisher for MockEventPublisher {
    async fn publish(&self, event: DomainEventEnum) -> Result<(), Box<dyn std::error::Error>> {
        self.events.write().await.push(event);
        Ok(())
    }
}

// Additional types for the example
use cim_domain::DomainEventEnum;

#[derive(Debug, Clone)]
enum GraphCommand {
    CreateGraph {
        graph_id: GraphId,
        name: String,
        graph_type: GraphType,
    },
    HighlightEdge {
        from_node: NodeId,
        to_node: NodeId,
        highlight_type: String,
    },
}

#[derive(Debug, Clone)]
enum GraphType {
    WorkflowGraph,
    ConceptualGraph,
    EventFlowGraph,
}
