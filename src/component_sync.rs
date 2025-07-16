//! Component synchronization between DDD and ECS via NATS
//!
//! This module integrates the isomorphic component architecture from cim-component
//! with cim-domain's event-driven infrastructure, ensuring all component updates
//! are properly synchronized via NATS.

use crate::{
    DomainError, DomainResult,
    infrastructure::nats_client::NatsClient,
};
use cim_component::{
    Component, ComponentEvent, ComponentEventPublisher, 
    ComponentSyncManager, EcsComponentData,
};
use futures::StreamExt;
use std::sync::Arc;
use uuid::Uuid;

/// Domain-specific component sync service that integrates with NATS
pub struct DomainComponentSync {
    /// The underlying sync manager from cim-component
    sync_manager: Arc<ComponentSyncManager>,
    /// NATS event publisher
    event_publisher: Arc<ComponentEventPublisher>,
    /// NATS client for subscriptions
    _nats_client: Arc<NatsClient>,
}

impl DomainComponentSync {
    /// Create a new domain component sync service
    pub async fn new(nats_client: Arc<NatsClient>) -> DomainResult<Self> {
        // Create NATS adapter for cim-component
        let nats_adapter = Arc::new(NatsAdapter::new(nats_client.clone()));
        
        // Create event publisher with NATS
        let event_publisher = Arc::new(ComponentEventPublisher::new(nats_adapter));
        
        // Create sync manager
        let sync_manager = Arc::new(ComponentSyncManager::new());
        
        Ok(Self {
            sync_manager,
            event_publisher,
            _nats_client: nats_client,
        })
    }
    
    /// Register a DDD component update and sync to ECS
    pub async fn sync_ddd_to_ecs(
        &self,
        entity_id: Uuid,
        component: Box<dyn Component>,
    ) -> DomainResult<()> {
        // Register with sync manager
        self.sync_manager.register_ddd_update(entity_id, component.clone_box()).await
            .map_err(|e| DomainError::ComponentError(e.to_string()))?;
        
        // Convert to ECS data
        let ecs_data = EcsComponentData {
            component_type: component.type_name().to_string(),
            data: component.to_json(),
        };
        
        // Publish component event via NATS
        let event = ComponentEvent::Updated {
            entity_id,
            component_data: ecs_data,
        };
        
        self.event_publisher.publish(event).await
            .map_err(|e| DomainError::NatsError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Register an ECS component update and sync to DDD
    pub async fn sync_ecs_to_ddd(
        &self,
        entity_id: Uuid,
        ecs_data: EcsComponentData,
    ) -> DomainResult<()> {
        // Register with sync manager
        self.sync_manager.register_ecs_update(entity_id, ecs_data).await
            .map_err(|e| DomainError::ComponentError(e.to_string()))?;
        
        // Process syncs
        self.sync_manager.process_pending_syncs().await
            .map_err(|e| DomainError::ComponentError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Subscribe to component events from NATS and process them
    pub async fn subscribe_to_component_events(
        &self,
        pattern: &str,
    ) -> DomainResult<tokio::task::JoinHandle<()>> {
        let subscription = self._nats_client.client()
            .subscribe(pattern.to_string())
            .await
            .map_err(|e| DomainError::NatsError(e.to_string()))?;
        
        let sync_manager = self.sync_manager.clone();
        
        // Spawn a task to handle incoming messages
        let handle = tokio::spawn(async move {
            let mut subscription = subscription;
            
            while let Some(message) = subscription.next().await {
                // Try to deserialize as component event
                if let Ok(event) = serde_json::from_slice::<ComponentEvent>(&message.payload) {
                    match event {
                        ComponentEvent::Added { entity_id, component_data } |
                        ComponentEvent::Updated { entity_id, component_data } => {
                            // Sync ECS update to DDD
                            if let Err(e) = sync_manager.register_ecs_update(entity_id, component_data).await {
                                tracing::error!("Failed to sync ECS update: {}", e);
                            }
                        }
                        ComponentEvent::Removed { .. } => {
                            // Component removal handling could be added here
                            tracing::debug!("Component removal events not yet handled");
                        }
                    }
                }
            }
        });
        
        Ok(handle)
    }
    
    /// Publish a component event directly via NATS
    pub async fn publish_component_event(&self, event: ComponentEvent) -> DomainResult<()> {
        self.event_publisher.publish(event).await
            .map_err(|e| DomainError::ComponentError(e.to_string()))
    }
}

/// Adapter to make cim-domain's NatsClient compatible with cim-component's trait
struct NatsAdapter {
    client: Arc<NatsClient>,
}

impl NatsAdapter {
    fn new(client: Arc<NatsClient>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl cim_component::NatsClient for NatsAdapter {
    async fn publish(&self, subject: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let subject = subject.to_string();
        self.client.client()
            .publish(subject, data.into())
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(())
    }
    
    async fn subscribe(&self, pattern: &str) -> Result<Box<dyn std::any::Any + Send + Sync>, Box<dyn std::error::Error>> {
        let pattern = pattern.to_string();
        let subscription = self.client.client()
            .subscribe(pattern)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        Ok(Box::new(subscription))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would go here
}