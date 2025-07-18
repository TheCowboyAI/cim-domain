// Copyright 2025 Cowboy AI, LLC.

//! Example: State Machine Aggregates with Isomorphic Components
//!
//! This example demonstrates how to use state machines with isomorphic components
//! that sync between DDD and ECS via NATS.

use cim_domain::{
    Component, ComponentExt,
    DomainComponentSync,
    state_machine::{State, TransitionOutput, MooreStateTransitions},
    infrastructure::nats_client::{NatsClient, NatsConfig},
    DomainEvent,
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use uuid::Uuid;

/// Example door states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum DoorState {
    Closed,
    Open,
    Locked,
}

impl State for DoorState {
    fn name(&self) -> &'static str {
        match self {
            DoorState::Closed => "Closed",
            DoorState::Open => "Open",
            DoorState::Locked => "Locked",
        }
    }
    
    fn is_terminal(&self) -> bool {
        false // No terminal states in this example
    }
}

/// Door state component for isomorphic mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DoorStateComponent {
    current_state: DoorState,
    last_transition: Option<String>,
    locked_by: Option<Uuid>,
}

impl Component for DoorStateComponent {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn type_name(&self) -> &'static str {
        "DoorStateComponent"
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Simple event output (no events for this example)
#[derive(Debug, Clone)]
struct NoEventOutput;

impl TransitionOutput for NoEventOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        vec![]
    }
}

impl Default for NoEventOutput {
    fn default() -> Self {
        NoEventOutput
    }
}

/// Moore machine implementation for door states
impl MooreStateTransitions for DoorState {
    type Output = NoEventOutput;
    
    fn can_transition_to(&self, target: &Self) -> bool {
        use DoorState::*;
        matches!(
            (self, target),
            (Open, Closed) | (Closed, Open) | (Closed, Locked) | (Locked, Closed)
        )
    }
    
    fn valid_transitions(&self) -> Vec<Self> {
        use DoorState::*;
        match self {
            Open => vec![Closed],
            Closed => vec![Open, Locked],
            Locked => vec![Closed],
        }
    }
    
    fn entry_output(&self) -> Self::Output {
        NoEventOutput
    }
}

/// Demonstrate state changes with component sync
async fn demonstrate_state_sync(
    door_entity_id: Uuid,
    component_sync: Arc<DomainComponentSync>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Door State Machine with Component Sync Example\n");
    
    // Helper function to sync state changes
    async fn sync_state(
        state: DoorState, 
        transition: &str,
        door_entity_id: Uuid,
        component_sync: &DomainComponentSync,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let component = DoorStateComponent {
            current_state: state,
            last_transition: Some(transition.to_string()),
            locked_by: if state == DoorState::Locked { 
                Some(Uuid::new_v4()) 
            } else { 
                None 
            },
        };
        
        println!("State: {:?} ({})", state, transition);
        
        // Sync to ECS via NATS
        component_sync.sync_ddd_to_ecs(door_entity_id, Box::new(component.clone())).await?;
        
        // Show ECS representation
        let ecs_data = component.to_ecs_data()?;
        println!("  -> ECS Component Type: {}", ecs_data.component_type);
        println!("  -> ECS Data: {}\n", serde_json::to_string_pretty(&ecs_data.data)?);
        
        Ok(())
    }
    
    // Demonstrate state transitions
    let mut current_state = DoorState::Closed;
    sync_state(current_state, "initial", door_entity_id, &component_sync).await?;
    
    // Open the door
    current_state = DoorState::Open;
    sync_state(current_state, "opened", door_entity_id, &component_sync).await?;
    
    // Close the door
    current_state = DoorState::Closed;
    sync_state(current_state, "closed", door_entity_id, &component_sync).await?;
    
    // Lock the door
    current_state = DoorState::Locked;
    sync_state(current_state, "locked", door_entity_id, &component_sync).await?;
    
    // Unlock the door
    current_state = DoorState::Closed;
    sync_state(current_state, "unlocked", door_entity_id, &component_sync).await?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize NATS client
    let nats_config = NatsConfig::default();
    
    // Check if NATS is available
    match NatsClient::connect(nats_config).await {
        Ok(nats_client) => {
            let nats_client = Arc::new(nats_client);
            
            // Create component sync service
            let component_sync = Arc::new(DomainComponentSync::new(nats_client.clone()).await?);
            
            // Subscribe to component events
            let _subscription_handle = component_sync
                .subscribe_to_component_events("cim.component.>")
                .await?;
            
            // Entity ID for this door
            let door_entity_id = Uuid::new_v4();
            
            // Run the demonstration
            demonstrate_state_sync(door_entity_id, component_sync).await?;
            
            println!("State machine example completed successfully!");
            println!("All state changes were synchronized to ECS via NATS");
        }
        Err(e) => {
            println!("Could not connect to NATS: {}", e);
            println!("To run this example, ensure NATS is running:");
            println!("  docker run -p 4222:4222 nats:latest");
            println!("\nRunning example without NATS synchronization...\n");
            
            // Show the state transitions without sync
            let states = vec![
                (DoorState::Closed, "initial"),
                (DoorState::Open, "opened"),
                (DoorState::Closed, "closed"),
                (DoorState::Locked, "locked"),
                (DoorState::Closed, "unlocked"),
            ];
            
            for (state, transition) in states {
                println!("State: {:?} ({})", state, transition);
                
                let component = DoorStateComponent {
                    current_state: state,
                    last_transition: Some(transition.to_string()),
                    locked_by: if state == DoorState::Locked { 
                        Some(Uuid::new_v4()) 
                    } else { 
                        None 
                    },
                };
                
                let ecs_data = component.to_ecs_data()?;
                println!("  -> Would sync to ECS: {}", ecs_data.component_type);
                println!("  -> Data: {}\n", serde_json::to_string_pretty(&ecs_data.data)?);
            }
        }
    }
    
    Ok(())
}