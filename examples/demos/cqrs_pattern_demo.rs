//! CQRS Pattern Demo - CIM Architecture
//!
//! This demo showcases the Command Query Responsibility Segregation (CQRS) pattern
//! as implemented in CIM's production-ready architecture.
//!
//! Key concepts demonstrated:
//! - Write model (commands → aggregates → events)
//! - Read model (projections optimized for queries)
//! - ContextGraph projection for workflow visualization
//! - Event sourcing with CID chains
//! - Cross-domain integration patterns

use cim_domain::{
    // Workflow domain
    WorkflowAggregate, WorkflowId, WorkflowCommand,
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    WorkflowSuspended, WorkflowResumed, WorkflowCancelled, WorkflowFailed,
    
    // Graph domain integration
    GraphId, NodeId, EdgeId,
    
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, InMemoryEventStore, StoredEvent},
        nats_client::{MockNatsClient},
    },
    
    // Core types
    DomainEventEnum, DomainError,
};
use cim_contextgraph::{
    ContextGraph, ContextNode, ContextEdge,
    JsonExporter, DotExporter,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;
use serde_json::json;

/// Write Model - Command processing through aggregates
struct WorkflowWriteModel {
    event_store: Arc<InMemoryEventStore>,
}

impl WorkflowWriteModel {
    async fn process_command(
        &self,
        workflow_id: &WorkflowId,
        command: WorkflowCommand,
    ) -> Result<Vec<DomainEventEnum>, DomainError> {
        println!("📝 Write Model: Processing command");
        
        // Load aggregate from event store
        let events = self.event_store.load_events(&workflow_id.to_string()).await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        
        let mut aggregate = WorkflowAggregate::new(workflow_id.clone());
        
        // Replay events to rebuild state
        for event in events {
            aggregate.apply_event(event)?;
        }
        
        // Process command
        let new_events = aggregate.handle_command(command)?;
        
        // Persist new events
        for event in &new_events {
            self.event_store.append_event(
                &workflow_id.to_string(),
                event.clone()
            ).await
            .map_err(|e| DomainError::Infrastructure(e.to_string()))?;
        }
        
        println!("   Generated {new_events.len(} events"));
        Ok(new_events)
    }
}

/// Read Model - Optimized projections for queries
struct WorkflowReadModel {
    // Denormalized view of workflow states
    workflow_states: Arc<RwLock<HashMap<WorkflowId, WorkflowStateView>>>,
    
    // ContextGraph projection for visualization
    context_graph: Arc<RwLock<ContextGraph>>,
    
    // Performance metrics
    transition_times: Arc<RwLock<HashMap<(NodeId, NodeId), Vec<std::time::Duration>>>>,
}

#[derive(Debug, Clone)]
struct WorkflowStateView {
    workflow_id: WorkflowId,
    graph_id: GraphId,
    current_state: Option<String>,
    current_node: Option<NodeId>,
    status: WorkflowStatus,
    transitions: Vec<TransitionRecord>,
    started_at: Option<chrono::DateTime<Utc>>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone)]
enum WorkflowStatus {
    NotStarted,
    Running,
    Suspended,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
struct TransitionRecord {
    from_node: NodeId,
    to_node: NodeId,
    executed_at: chrono::DateTime<Utc>,
    duration: std::time::Duration,
    metadata: serde_json::Value,
}

impl WorkflowReadModel {
    fn new() -> Self {
        Self {
            workflow_states: Arc::new(RwLock::new(HashMap::new())),
            context_graph: Arc::new(RwLock::new(ContextGraph::new())),
            transition_times: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Update projections based on events
    async fn handle_event(&self, event: &DomainEventEnum) -> Result<(), DomainError> {
        match event {
            DomainEventEnum::WorkflowStarted(e) => {
                println!("📊 Read Model: Projecting WorkflowStarted");
                
                // Update state view
                let mut states = self.workflow_states.write().await;
                states.insert(e.workflow_id.clone(), WorkflowStateView {
                    workflow_id: e.workflow_id.clone(),
                    graph_id: e.graph_id.clone(),
                    current_state: Some(e.initial_state.clone()),
                    current_node: None,
                    status: WorkflowStatus::Running,
                    transitions: Vec::new(),
                    started_at: Some(Utc::now()),
                    completed_at: None,
                });
                
                // Update context graph
                let mut graph = self.context_graph.write().await;
                graph.add_node(ContextNode {
                    id: format!("workflow_{e.workflow_id}"),
                    node_type: "Workflow".to_string(),
                    properties: json!({
                        "workflow_id": e.workflow_id.to_string(),
                        "graph_id": e.graph_id.to_string(),
                        "initial_state": e.initial_state,
                        "status": "running"
                    }),
                });
            }
            
            DomainEventEnum::WorkflowTransitionExecuted(e) => {
                println!("📊 Read Model: Projecting WorkflowTransitionExecuted");
                
                // Update state view
                let mut states = self.workflow_states.write().await;
                if let Some(state) = states.get_mut(&e.workflow_id) {
                    let duration = std::time::Duration::from_secs(1); // Example duration
                    
                    state.current_node = Some(e.to_node.clone());
                    state.transitions.push(TransitionRecord {
                        from_node: e.from_node.clone(),
                        to_node: e.to_node.clone(),
                        executed_at: Utc::now(),
                        duration,
                        metadata: e.transition_data.clone(),
                    });
                    
                    // Track performance metrics
                    let mut times = self.transition_times.write().await;
                    times.entry((e.from_node.clone(), e.to_node.clone()))
                        .or_insert_with(Vec::new)
                        .push(duration);
                }
                
                // Update context graph
                let mut graph = self.context_graph.write().await;
                
                // Add nodes if not exists
                let from_id = format!("node_{e.from_node}");
                let to_id = format!("node_{e.to_node}");
                
                graph.add_node(ContextNode {
                    id: from_id.clone(),
                    node_type: "WorkflowNode".to_string(),
                    properties: json!({
                        "node_id": e.from_node.to_string(),
                    }),
                });
                
                graph.add_node(ContextNode {
                    id: to_id.clone(),
                    node_type: "WorkflowNode".to_string(),
                    properties: json!({
                        "node_id": e.to_node.to_string(),
                    }),
                });
                
                // Add transition edge
                graph.add_edge(ContextEdge {
                    from: from_id,
                    to: to_id,
                    edge_type: "Transition".to_string(),
                    properties: json!({
                        "executed_at": Utc::now().to_rfc3339(),
                        "data": e.transition_data,
                    }),
                });
            }
            
            DomainEventEnum::WorkflowCompleted(e) => {
                println!("📊 Read Model: Projecting WorkflowCompleted");
                
                let mut states = self.workflow_states.write().await;
                if let Some(state) = states.get_mut(&e.workflow_id) {
                    state.status = WorkflowStatus::Completed;
                    state.completed_at = Some(Utc::now());
                }
                
                // Update context graph
                let mut graph = self.context_graph.write().await;
                if let Some(node) = graph.get_node_mut(&format!("workflow_{e.workflow_id}")) {
                    node.properties["status"] = json!("completed");
                    node.properties["completed_at"] = json!(Utc::now().to_rfc3339());
                    node.properties["final_state"] = json!(e.final_state);
                }
            }
            
            _ => {} // Handle other events similarly
        }
        
        Ok(())
    }
    
    /// Query: Get workflow state
    async fn get_workflow_state(&self, workflow_id: &WorkflowId) -> Option<WorkflowStateView> {
        self.workflow_states.read().await.get(workflow_id).cloned()
    }
    
    /// Query: Get active workflows
    async fn get_active_workflows(&self) -> Vec<WorkflowStateView> {
        self.workflow_states.read().await
            .values()
            .filter(|w| matches!(w.status, WorkflowStatus::Running))
            .cloned()
            .collect()
    }
    
    /// Query: Get average transition time
    async fn get_average_transition_time(
        &self,
        from: &NodeId,
        to: &NodeId,
    ) -> Option<std::time::Duration> {
        let times = self.transition_times.read().await;
        if let Some(durations) = times.get(&(from.clone(), to.clone())) {
            if !durations.is_empty() {
                let total: std::time::Duration = durations.iter().sum();
                Some(total / durations.len() as u32)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Export context graph to JSON
    async fn export_graph_json(&self) -> String {
        let graph = self.context_graph.read().await;
        let exporter = JsonExporter::new();
        exporter.export(&graph).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// Export context graph to DOT format
    async fn export_graph_dot(&self) -> String {
        let graph = self.context_graph.read().await;
        let exporter = DotExporter::new();
        exporter.export(&graph).unwrap_or_else(|_| "digraph {}".to_string())
    }
}

/// Event Processor - Connects write and read models
struct EventProcessor {
    read_model: Arc<WorkflowReadModel>,
}

impl EventProcessor {
    async fn process_events(&self, events: Vec<DomainEventEnum>) -> Result<(), DomainError> {
        for event in events {
            self.read_model.handle_event(&event).await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CIM CQRS Pattern Demo\n");
    
    // Initialize infrastructure
    let event_store = Arc::new(InMemoryEventStore::new());
    
    // Create write model
    let write_model = WorkflowWriteModel {
        event_store: event_store.clone(),
    };
    
    // Create read model
    let read_model = Arc::new(WorkflowReadModel::new());
    
    // Create event processor
    let event_processor = EventProcessor {
        read_model: read_model.clone(),
    };
    
    // Example workflow
    let workflow_id = WorkflowId::new();
    let graph_id = GraphId::new();
    
    println!("=== Phase 1: Command Processing (Write Model) ===\n");
    
    // Start workflow
    println!("1. Starting workflow...");
    let start_events = write_model.process_command(
        &workflow_id,
        WorkflowCommand::StartWorkflow {
            graph_id: graph_id.clone(),
            initial_state: "draft".to_string(),
        }
    ).await?;
    
    // Process events in read model
    event_processor.process_events(start_events).await?;
    
    // Execute transitions
    println!("\n2. Executing workflow transitions...");
    
    let transitions = vec![
        (NodeId::new(), NodeId::new(), json!({"action": "submit", "user": "alice"})),
        (NodeId::new(), NodeId::new(), json!({"action": "review", "user": "bob"})),
        (NodeId::new(), NodeId::new(), json!({"action": "approve", "user": "carol"})),
    ];
    
    for (i, (from, to, data)) in transitions.iter().enumerate() {
        println!("   Transition {i + 1}: {from} → {to}");
        
        let events = write_model.process_command(
            &workflow_id,
            WorkflowCommand::ExecuteTransition {
                from_node: from.clone(),
                to_node: to.clone(),
                transition_data: data.clone(),
            }
        ).await?;
        
        event_processor.process_events(events).await?;
    }
    
    // Complete workflow
    println!("\n3. Completing workflow...");
    let complete_events = write_model.process_command(
        &workflow_id,
        WorkflowCommand::CompleteWorkflow {
            final_state: "approved".to_string(),
        }
    ).await?;
    
    event_processor.process_events(complete_events).await?;
    
    println!("\n=== Phase 2: Query Processing (Read Model) ===\n");
    
    // Query workflow state
    println!("1. Current workflow state:");
    if let Some(state) = read_model.get_workflow_state(&workflow_id).await {
        println!("   Status: {:?}", state.status);
        println!("   Transitions: {state.transitions.len(}"));
        println!("   Started: {:?}", state.started_at);
        println!("   Completed: {:?}", state.completed_at);
    }
    
    // Query active workflows
    println!("\n2. Active workflows:");
    let active = read_model.get_active_workflows().await;
    println!("   Count: {active.len(}"));
    
    // Query performance metrics
    println!("\n3. Transition performance:");
    for ((from, to), _) in transitions.iter().take(2) {
        if let Some(avg_time) = read_model.get_average_transition_time(from, to).await {
            println!("   {from} → {to}: {:?} average", avg_time);
        }
    }
    
    println!("\n=== Phase 3: ContextGraph Export ===\n");
    
    // Export as JSON
    println!("1. JSON Export:");
    let json_export = read_model.export_graph_json().await;
    println!("{serde_json::to_string_pretty(&json_export}")?);
    
    // Export as DOT
    println!("\n2. DOT Export (for Graphviz):");
    let dot_export = read_model.export_graph_dot().await;
    println!("{dot_export}");
    
    println!("\n=== CQRS Benefits Demonstrated ===");
    println!("✅ Write model optimized for business logic and consistency");
    println!("✅ Read model optimized for queries and performance");
    println!("✅ Event sourcing provides complete audit trail");
    println!("✅ ContextGraph enables universal visualization");
    println!("✅ Multiple projections from same event stream");
    
    println!("\n✅ Demo completed successfully!");
    
    Ok(())
}
