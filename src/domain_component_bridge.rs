//! Bridge between domain events and component events
//!
//! This module ensures that when domain events occur that affect components,
//! corresponding component events are published via NATS for synchronization.

use crate::{
    DomainResult,
    DomainEventEnum, DomainEventEnvelope,
    Component, ComponentEvent, EcsComponentData,
    component_sync::DomainComponentSync,
    GraphId,
};
use std::sync::Arc;
use uuid::Uuid;

/// Bridges domain events to component events
pub struct DomainComponentBridge {
    component_sync: Arc<DomainComponentSync>,
}

impl DomainComponentBridge {
    /// Create a new bridge
    pub fn new(component_sync: Arc<DomainComponentSync>) -> Self {
        Self { component_sync }
    }
    
    /// Process a domain event and emit component events if needed
    pub async fn process_domain_event(
        &self,
        envelope: &DomainEventEnvelope<DomainEventEnum>,
    ) -> DomainResult<()> {
        // Extract components from domain events
        match &envelope.event {
            // For workflow events, we might want to sync workflow state as a component
            DomainEventEnum::WorkflowStarted(event) => {
                self.emit_workflow_component_event(
                    event.workflow_id.into(),
                    WorkflowStateComponent {
                        state: event.initial_state.clone(),
                        definition_id: event.definition_id,
                        started_at: Some(event.started_at),
                        completed_at: None,
                    }
                ).await?;
            }
            
            DomainEventEnum::WorkflowTransitioned(event) => {
                self.emit_workflow_component_event(
                    event.workflow_id.into(),
                    WorkflowStateComponent {
                        state: event.to_state.clone(),
                        definition_id: GraphId::new(), // Default for now
                        started_at: None,
                        completed_at: None,
                    }
                ).await?;
            }
            
            DomainEventEnum::WorkflowCompleted(event) => {
                self.emit_workflow_component_event(
                    event.workflow_id.into(),
                    WorkflowStateComponent {
                        state: "completed".to_string(),
                        definition_id: GraphId::new(), // Default for now
                        started_at: None,
                        completed_at: Some(event.completed_at),
                    }
                ).await?;
            }
            
            // Other events might not have component implications
            _ => {}
        }
        
        Ok(())
    }
    
    /// Emit a component event for a workflow state change
    async fn emit_workflow_component_event<C: Component>(
        &self,
        entity_id: Uuid,
        component: C,
    ) -> DomainResult<()> {
        // Convert to ECS data
        let ecs_data = EcsComponentData {
            component_type: component.type_name().to_string(),
            data: component.to_json(),
        };
        
        // Create component event
        let event = ComponentEvent::Updated {
            entity_id,
            component_data: ecs_data,
        };
        
        // Publish via NATS
        self.component_sync.publish_component_event(event).await
    }
}

/// Component representing workflow state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WorkflowStateComponent {
    state: String,
    definition_id: GraphId,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Component for WorkflowStateComponent {
    fn as_any(&self) -> &dyn std::any::Any { self }
    
    fn clone_box(&self) -> Box<dyn Component> { 
        Box::new(self.clone()) 
    }
    
    fn type_name(&self) -> &'static str { 
        "WorkflowStateComponent" 
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would go here
}