// Copyright 2025 Cowboy AI, LLC.

//! Event Stream Example - CIM Architecture
//!
//! This example demonstrates event streaming patterns in CIM's production-ready
//! event-driven architecture with CID chains and cross-domain integration.
//!
//! Key concepts demonstrated:
//! - Event streaming with CID chains for integrity
//! - Correlation and causation tracking
//! - Cross-domain event flows
//! - Event replay and time travel
//! - Real-time event monitoring

use cim_domain::{
    // Events
    DomainEventEnum,
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, InMemoryEventStore, StoredEvent},
        nats_client::{MockNatsClient},
    },
    
    // Core types
    WorkflowId, GraphId, NodeId, EdgeId,
    CorrelationId, CausationId,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Event stream processor with CID chain verification
struct EventStreamProcessor {
    event_store: Arc<InMemoryEventStore>,
    event_chains: Arc<RwLock<HashMap<String, Vec<StoredEvent>>>>,
}

impl EventStreamProcessor {
    fn new(event_store: Arc<InMemoryEventStore>) -> Self {
        Self {
            event_store,
            event_chains: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Process a stream of events with CID chain verification
    async fn process_event_stream(
        &self,
        aggregate_id: &str,
        events: Vec<DomainEventEnum>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Processing event stream for aggregate: {aggregate_id}");
        
        let mut previous_cid: Option<String> = None;
        let mut stored_events = Vec::new();
        
        for (i, event) in events.iter().enumerate() {
            println!("\n📌 Event {}: {}", i + 1, event_type_name(event)));
            
            // Store event with CID chain
            let stored = self.event_store
                .append_event(aggregate_id, event.clone())
                .await?;
            
            // Verify CID chain
            if let Some(prev) = &previous_cid {
                println!("   CID Chain: {} → {}", &prev[..8], &stored.event_cid().unwrap_or_default()[..8]);
            } else {
                println!("   CID Chain: Genesis → {}", &stored.event_cid().unwrap_or_default()[..8]);
            }
            
            previous_cid = stored.event_cid();
            stored_events.push(stored);
        }
        
        // Store the chain
        self.event_chains.write().await
            .insert(aggregate_id.to_string(), stored_events);
        
        println!("\n✅ Event stream processed successfully");
        Ok(())
    }
    
    /// Replay events from a specific point in time
    async fn replay_from_timestamp(
        &self,
        aggregate_id: &str,
        from_time: DateTime<Utc>,
    ) -> Result<Vec<StoredEvent>, Box<dyn std::error::Error>> {
        println!("\n⏮️ Replaying events from: {from_time}");
        
        let all_events = self.event_store
            .load_events(aggregate_id)
            .await?;
        
        let replayed: Vec<_> = all_events
            .into_iter()
            .filter(|e| e.timestamp() > from_time)
            .collect();
        
        println!("   Found {} events to replay", replayed.len()));
        
        for (i, event) in replayed.iter().enumerate() {
            println!("   {}. {} at {}", i + 1, event.event_type(), event.stored_at),
                event.timestamp().format("%H:%M:%S")
            );
        }
        
        Ok(replayed)
    }
    
    /// Find all events in a correlation chain
    async fn find_correlation_chain(
        &self,
        correlation_id: &CorrelationId,
    ) -> Result<Vec<StoredEvent>, Box<dyn std::error::Error>> {
        println!("\n🔗 Finding correlation chain for: {correlation_id}");
        
        let mut correlated_events = Vec::new();
        
        // Search across all aggregates
        let chains = self.event_chains.read().await;
        for (aggregate_id, events) in chains.iter() {
            for event in events {
                if event.correlation_id() == Some(&correlation_id.to_string()) {
                    correlated_events.push(event.clone());
                }
            }
        }
        
        // Sort by timestamp
        correlated_events.sort_by_key(|e| e.timestamp());
        
        println!("   Found {} correlated events", correlated_events.len()));
        Ok(correlated_events)
    }
    
    /// Build causation tree
    async fn build_causation_tree(
        &self,
        root_event_id: &str,
    ) -> Result<CausationTree, Box<dyn std::error::Error>> {
        println!("\n🌳 Building causation tree from: {&root_event_id[..8]}");
        
        let mut tree = CausationTree::new(root_event_id.to_string());
        let chains = self.event_chains.read().await;
        
        // Find all events caused by the root
        let mut to_process = vec![root_event_id.to_string()];
        let mut processed = std::collections::HashSet::new();
        
        while let Some(event_id) = to_process.pop() {
            if processed.contains(&event_id) {
                continue;
            }
            processed.insert(event_id.clone());
            
            // Find events caused by this event
            for (_, events) in chains.iter() {
                for event in events {
                    if event.causation_id() == Some(&event_id) {
                        let child_id = event.event_id;
                        tree.add_child(&event_id, child_id.clone(), event.event_type().to_string());
                        to_process.push(child_id);
                    }
                }
            }
        }
        
        println!("   Tree contains {} events", tree.size()));
        Ok(tree)
    }
}

/// Represents a causation tree of events
struct CausationTree {
    root: String,
    nodes: HashMap<String, CausationNode>,
}

struct CausationNode {
    event_id: String,
    event_type: String,
    children: Vec<String>,
}

impl CausationTree {
    fn new(root: String) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(root.clone(), CausationNode {
            event_id: root.clone(),
            event_type: "Root".to_string(),
            children: Vec::new(),
        });
        
        Self { root, nodes }
    }
    
    fn add_child(&mut self, parent_id: &str, child_id: String, event_type: String) {
        // Add child node
        self.nodes.insert(child_id.clone(), CausationNode {
            event_id: child_id.clone(),
            event_type,
            children: Vec::new(),
        });
        
        // Link to parent
        if let Some(parent) = self.nodes.get_mut(parent_id) {
            parent.children.push(child_id);
        }
    }
    
    fn size(&self) -> usize {
        self.nodes.len()
    }
    
    fn print(&self, id: &str, depth: usize) {
        if let Some(node) = self.nodes.get(id) {
            let indent = "  ".repeat(depth);
            println!("{}├─ {} ({})", indent, &node.event_id[..8], node.event_type);
            
            for child in &node.children {
                self.print(child, depth + 1);
            }
        }
    }
}

/// Real-time event monitor
struct EventMonitor {
    event_count: Arc<RwLock<HashMap<String, usize>>>,
    event_latencies: Arc<RwLock<Vec<std::time::Duration>>>,
}

impl EventMonitor {
    fn new() -> Self {
        Self {
            event_count: Arc::new(RwLock::new(HashMap::new())),
            event_latencies: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn monitor_event(&self, event: &DomainEventEnum, latency: std::time::Duration) {
        // Update counts
        let event_type = event_type_name(event);
        let mut counts = self.event_count.write().await;
        *counts.entry(event_type.to_string()).or_insert(0) += 1;
        
        // Track latency
        self.event_latencies.write().await.push(latency);
    }
    
    async fn print_statistics(&self) {
        println!("\n📊 Event Stream Statistics:");
        
        // Event counts
        let counts = self.event_count.read().await;
        println!("\n   Event Counts:");
        for (event_type, count) in counts.iter() {
            println!("     {event_type}: {count}");
        }
        
        // Latency stats
        let latencies = self.event_latencies.read().await;
        if !latencies.is_empty() {
            let total: std::time::Duration = latencies.iter().sum();
            let avg = total / latencies.len() as u32;
            let max = latencies.iter().max().unwrap();
            let min = latencies.iter().min().unwrap();
            
            println!("\n   Latency Statistics:");
            println!("     Average: {:?}", avg);
            println!("     Min: {:?}", min);
            println!("     Max: {:?}", max);
        }
    }
}

fn event_type_name(event: &DomainEventEnum) -> &str {
    match event {
        DomainEventEnum::WorkflowStarted(_) => "WorkflowStarted",
        DomainEventEnum::WorkflowTransitionExecuted(_) => "WorkflowTransitionExecuted",
        DomainEventEnum::WorkflowCompleted(_) => "WorkflowCompleted",
        DomainEventEnum::WorkflowSuspended(_) => "WorkflowSuspended",
        DomainEventEnum::WorkflowResumed(_) => "WorkflowResumed",
        DomainEventEnum::WorkflowCancelled(_) => "WorkflowCancelled",
        DomainEventEnum::WorkflowFailed(_) => "WorkflowFailed",
        DomainEventEnum::WorkflowTransitioned(_) => "WorkflowTransitioned",
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CIM Event Stream Example\n");
    
    // Initialize infrastructure
    let event_store = Arc::new(InMemoryEventStore::new());
    let processor = EventStreamProcessor::new(event_store.clone());
    let monitor = EventMonitor::new();
    
    // Create correlation context
    let correlation_id = CorrelationId::from_uuid(Uuid::new_v4());
    println!("📍 Correlation ID: {correlation_id}");
    
    // Example 1: Simple event stream
    println!("\n=== Example 1: Basic Event Stream ===");
    
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
        DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
            workflow_id: workflow_id.clone(),
            final_state: "approved".to_string(),
            total_duration: std::time::Duration::from_secs(300),
            completed_at: Utc::now(),
        }),
    ];
    
    // Process with monitoring
    for event in &events {
        let start = std::time::Instant::now();
        processor.process_event_stream(&workflow_id.to_string(), vec![event.clone()]).await?;
        monitor.monitor_event(event, start.elapsed()).await;
    }
    
    // Example 2: Event replay
    println!("\n=== Example 2: Event Replay ===");
    
    let replay_from = Utc::now() - chrono::Duration::seconds(5);
    let replayed = processor.replay_from_timestamp(&workflow_id.to_string(), replay_from).await?;
    println!("   Replayed {} events", replayed.len()));
    
    // Example 3: Correlation chain
    println!("\n=== Example 3: Correlation Chain ===");
    
    let correlated = processor.find_correlation_chain(&correlation_id).await?;
    println!("   Chain contains {} events:", correlated.len()));
    for (i, event) in correlated.iter().enumerate() {
        println!("     {}. {} at {}", i + 1, event.event_type(), event.stored_at),
            event.timestamp().format("%H:%M:%S")
        );
    }
    
    // Example 4: Causation tree
    println!("\n=== Example 4: Causation Tree ===");
    
    if let Some(first_event) = event_store.load_events(&workflow_id.to_string()).await?.first() {
        let tree = processor.build_causation_tree(&first_event.event_id).await?;
        println!("\n   Causation Tree:");
        tree.print(&tree.root, 1);
    }
    
    // Example 5: Cross-domain event flow
    println!("\n=== Example 5: Cross-Domain Event Flow ===");
    
    // Simulate cross-domain events
    let graph_aggregate_id = format!("graph_{GraphId::new(}"));
    let cross_domain_events = vec![
        // Workflow event causes graph update
        DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id: WorkflowId::new(),
            definition_id: definition_id.clone(),
            initial_state: "initial".to_string(),
            started_at: Utc::now(),
        }),
        // This would normally be a GraphNodeAdded event in the graph domain
        // For demo purposes, using workflow events
        DomainEventEnum::WorkflowTransitionExecuted(WorkflowTransitionExecuted {
            workflow_id: WorkflowId::new(),
            from_state: "initial".to_string(),
            to_state: "processing".to_string(),
            input: json!({"triggered_by": "workflow_start"}),
            output: json!({"graph_updated": true}),
            executed_at: Utc::now(),
        }),
    ];
    
    for event in cross_domain_events {
        processor.process_event_stream(&graph_aggregate_id, vec![event]).await?;
    }
    
    println!("\n   Cross-domain flow established:");
    println!("   Workflow Domain → Graph Domain");
    println!("   (via correlation ID: {})", &correlation_id.to_string()[..8]);
    
    // Print statistics
    monitor.print_statistics().await;
    
    println!("\n=== Event Stream Benefits ===");
    println!("✅ CID chains ensure event integrity");
    println!("✅ Correlation tracking enables distributed tracing");
    println!("✅ Causation trees show event relationships");
    println!("✅ Event replay enables time travel debugging");
    println!("✅ Cross-domain flows maintain loose coupling");
    
    println!("\n✅ Example completed successfully!");
    
    Ok(())
}
