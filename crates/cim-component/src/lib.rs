//! Component trait for attaching data to domain objects with isomorphic ECS mapping
//!
//! This module provides:
//! - The foundational Component trait for DDD-style components
//! - Isomorphic mapping between DDD components and ECS representations
//! - NATS-based transport for component synchronization between processes
//!
//! # Architecture
//!
//! The system maintains two parallel component representations:
//! 1. DDD Components - Using the Component trait for domain logic
//! 2. ECS Components - Bevy components for visualization and systems
//!
//! These are kept in sync via NATS messaging, ensuring all inter-process
//! communication follows the prescribed architecture.

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};

#[cfg(test)]
mod tests;

/// Trait for components that can be attached to domain objects
///
/// Components are immutable data that can be attached to entities, nodes, or edges.
/// They provide a way to extend domain objects with additional data without modifying
/// their core structure.
///
/// # Example
///
/// ```
/// use cim_component::Component;
/// use std::any::Any;
///
/// #[derive(Debug, Clone)]
/// struct Label(String);
///
/// impl Component for Label {
///     fn as_any(&self) -> &dyn Any { self }
///     fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
///     fn type_name(&self) -> &'static str { "Label" }
///     fn to_json(&self) -> serde_json::Value { serde_json::json!({"value": self.0}) }
/// }
/// ```
pub trait Component: Any + Send + Sync + Debug {
    /// Get the component as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Clone the component into a box
    fn clone_box(&self) -> Box<dyn Component>;

    /// Get the name of this component type
    fn type_name(&self) -> &'static str;
    
    /// Convert to JSON for ECS serialization
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({})
    }
}

/// Error type for component operations
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ComponentError {
    /// Component of this type already exists
    #[error("Component already exists: {0}")]
    AlreadyExists(String),
    
    /// Component not found
    #[error("Component not found: {0}")]
    NotFound(String),
    
    /// Serialization failed
    #[error("Serialization failed: {0}")]
    SerializationError(String),
    
    /// Deserialization failed
    #[error("Deserialization failed: {0}")]
    DeserializationError(String),
    
    /// Component type not registered
    #[error("Component type not registered: {0}")]
    UnregisteredType(String),
    
    /// NATS communication error
    #[error("NATS error: {0}")]
    NatsError(String),
}

/// Result type for component operations
pub type ComponentResult<T> = Result<T, ComponentError>;

/// Get the TypeId of a component type
pub fn component_type_id<T: Component + 'static>() -> TypeId {
    TypeId::of::<T>()
}

// ===== Isomorphic Mapping =====

/// ECS-compatible representation of a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcsComponentData {
    /// Type identifier for the component
    pub component_type: String,
    /// Serialized component data
    pub data: serde_json::Value,
}

/// Extension methods for Component to support ECS mapping
pub trait ComponentExt {
    /// Convert this component to ECS-compatible data
    fn to_ecs_data(&self) -> Result<EcsComponentData, ComponentError>;
}

/// Default implementation for components that can be serialized
impl<T> ComponentExt for T
where
    T: Component + Serialize,
{
    fn to_ecs_data(&self) -> Result<EcsComponentData, ComponentError> {
        // We need to serialize the concrete type, not through the trait
        let data = serde_json::to_value(self)
            .map_err(|e| ComponentError::SerializationError(e.to_string()))?;
        
        Ok(EcsComponentData {
            component_type: self.type_name().to_string(),
            data,
        })
    }
}

// ===== Component Registry =====

/// Registry for component types and their constructors
pub struct ComponentRegistry {
    constructors: dashmap::DashMap<String, Box<dyn Fn(&serde_json::Value) -> Option<Box<dyn Component>> + Send + Sync>>,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            constructors: dashmap::DashMap::new(),
        }
    }
    
    /// Register a component type with its constructor
    pub fn register_type(
        &self,
        type_name: &str,
        constructor: Box<dyn Fn(&serde_json::Value) -> Option<Box<dyn Component>> + Send + Sync>,
    ) {
        self.constructors.insert(type_name.to_string(), constructor);
    }
    
    /// Register a typed component
    pub fn register<T>(&self)
    where
        T: Component + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        // For tests, we'll use a simplified type name
        let type_name = std::any::type_name::<T>()
            .split("::").last().unwrap_or("Unknown").to_string();
        self.register_type(
            &type_name,
            Box::new(|data| {
                let component: T = serde_json::from_value(data.clone()).ok()?;
                Some(Box::new(component) as Box<dyn Component>)
            }),
        );
    }
    
    /// Register a typed component with explicit type name
    pub fn register_with_name<T>(&self, type_name: &str)
    where
        T: Component + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        self.register_type(
            type_name,
            Box::new(|data| {
                let component: T = serde_json::from_value(data.clone()).ok()?;
                Some(Box::new(component) as Box<dyn Component>)
            }),
        );
    }
    
    /// Check if a component type is registered
    pub fn is_registered(&self, type_name: &str) -> bool {
        self.constructors.contains_key(type_name)
    }
    
    /// Reconstruct a component from ECS data
    pub fn reconstruct_component(&self, ecs_data: &EcsComponentData) -> Result<Box<dyn Component>, ComponentError> {
        let constructor = self.constructors
            .get(&ecs_data.component_type)
            .ok_or_else(|| ComponentError::UnregisteredType(ecs_data.component_type.clone()))?;
        
        constructor(&ecs_data.data)
            .ok_or_else(|| ComponentError::DeserializationError("Failed to construct component".to_string()))
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed component registry for compile-time safety
pub struct TypedComponentRegistry {
    registry: ComponentRegistry,
}

impl TypedComponentRegistry {
    /// Create a new typed registry
    pub fn new() -> Self {
        Self {
            registry: ComponentRegistry::new(),
        }
    }
    
    /// Register a component type
    pub fn register<T>(&mut self)
    where
        T: Component + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        self.registry.register::<T>();
    }
    
    /// Create a component from data
    pub fn create<T>(&self, data: &serde_json::Value) -> Option<T>
    where
        T: Component + for<'de> Deserialize<'de> + 'static,
    {
        serde_json::from_value(data.clone()).ok()
    }
}

// ===== NATS Events =====

/// Events for component lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentEvent {
    /// Component added to entity
    Added {
        entity_id: uuid::Uuid,
        component_data: EcsComponentData,
    },
    /// Component updated on entity
    Updated {
        entity_id: uuid::Uuid,
        component_data: EcsComponentData,
    },
    /// Component removed from entity
    Removed {
        entity_id: uuid::Uuid,
        component_type: String,
    },
}

/// Component event publisher for NATS
pub struct ComponentEventPublisher {
    client: Arc<dyn NatsClient>,
    retry_policy: Option<RetryPolicy>,
}

impl ComponentEventPublisher {
    /// Create a new publisher
    pub fn new(client: Arc<dyn NatsClient>) -> Self {
        Self {
            client,
            retry_policy: None,
        }
    }
    
    /// Set retry policy
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = Some(policy);
        self
    }
    
    /// Publish a component event
    pub async fn publish(&self, event: ComponentEvent) -> Result<(), ComponentError> {
        let subject = match &event {
            ComponentEvent::Added { .. } => "cim.component.added",
            ComponentEvent::Updated { .. } => "cim.component.updated",
            ComponentEvent::Removed { .. } => "cim.component.removed",
        };
        
        let data = serde_json::to_vec(&event)
            .map_err(|e| ComponentError::SerializationError(e.to_string()))?;
        
        // Apply retry policy if configured
        if let Some(policy) = &self.retry_policy {
            self.publish_with_retry(subject, data, policy).await
        } else {
            self.client.publish(subject, data).await
                .map_err(|e| ComponentError::NatsError(e.to_string()))
        }
    }
    
    async fn publish_with_retry(
        &self,
        subject: &str,
        data: Vec<u8>,
        policy: &RetryPolicy,
    ) -> Result<(), ComponentError> {
        let mut attempts = 0;
        let mut delay = policy.initial_delay;
        
        loop {
            match self.client.publish(subject, data.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= policy.max_attempts {
                        return Err(ComponentError::NatsError(format!(
                            "Failed after {} attempts: {}",
                            attempts, e
                        )));
                    }
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    delay = (delay as f64 * policy.backoff_factor) as u64;
                }
            }
        }
    }
}

/// Component event subscriber
pub struct ComponentEventSubscriber {
    client: Arc<dyn NatsClient>,
}

impl ComponentEventSubscriber {
    /// Create a new subscriber
    pub fn new(client: Arc<dyn NatsClient>) -> Self {
        Self { client }
    }
    
    /// Subscribe to component events
    pub async fn subscribe(&mut self, pattern: &str) -> Result<ComponentEventStream, ComponentError> {
        let subscription = self.client.subscribe(pattern).await
            .map_err(|e| ComponentError::NatsError(e.to_string()))?;
        
        Ok(ComponentEventStream { subscription })
    }
}

/// Stream of component events
pub struct ComponentEventStream {
    subscription: Box<dyn Any + Send + Sync>,
}

impl ComponentEventStream {
    /// Get next event (mock implementation for tests)
    pub async fn next(&mut self) -> Option<ComponentEvent> {
        // In real implementation, this would read from NATS subscription
        // For tests, we'll check if the subscription contains a mock receiver
        if let Some(receiver) = self.subscription.downcast_mut::<tokio::sync::mpsc::UnboundedReceiver<ComponentEvent>>() {
            receiver.recv().await
        } else {
            None
        }
    }
}

/// Retry policy for NATS operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub initial_delay: u64,
    pub backoff_factor: f64,
}

impl RetryPolicy {
    /// Create exponential backoff retry policy
    pub fn exponential_backoff(max_attempts: usize, initial_delay: u64) -> Self {
        Self {
            max_attempts,
            initial_delay,
            backoff_factor: 2.0,
        }
    }
}

// ===== Bidirectional Sync =====

/// Manages synchronization between DDD and ECS components
pub struct ComponentSyncManager {
    ddd_updates: Arc<Mutex<Vec<ComponentUpdate>>>,
    ecs_updates: Arc<Mutex<Vec<ComponentUpdate>>>,
    processed_updates: Arc<Mutex<Vec<ProcessedUpdate>>>,
}

#[derive(Debug, Clone)]
pub struct ComponentUpdate {
    pub entity_id: uuid::Uuid,
    pub component_type: String,
    pub data: serde_json::Value,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ProcessedUpdate {
    pub entity_id: uuid::Uuid,
    pub component_type: String,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ComponentSyncManager {
    /// Create a new sync manager
    pub fn new() -> Self {
        Self {
            ddd_updates: Arc::new(Mutex::new(Vec::new())),
            ecs_updates: Arc::new(Mutex::new(Vec::new())),
            processed_updates: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Register a DDD component update
    pub async fn register_ddd_update(
        &self,
        entity_id: uuid::Uuid,
        component: Box<dyn Component>,
    ) -> Result<(), ComponentError> {
        // Convert to ECS data using the component's type name and serialization
        let ecs_data = EcsComponentData {
            component_type: component.type_name().to_string(),
            data: component.to_json(),
        };
        
        let update = ComponentUpdate {
            entity_id,
            component_type: ecs_data.component_type.clone(),
            data: ecs_data.data,
            metadata: std::collections::HashMap::from([
                ("sync_source".to_string(), "ddd".to_string()),
            ]),
        };
        
        self.ddd_updates.lock().await.push(update);
        Ok(())
    }
    
    /// Register an ECS component update
    pub async fn register_ecs_update(
        &self,
        entity_id: uuid::Uuid,
        ecs_data: EcsComponentData,
    ) -> Result<(), ComponentError> {
        let update = ComponentUpdate {
            entity_id,
            component_type: ecs_data.component_type,
            data: ecs_data.data,
            metadata: std::collections::HashMap::from([
                ("sync_source".to_string(), "ecs".to_string()),
            ]),
        };
        
        self.ecs_updates.lock().await.push(update);
        Ok(())
    }
    
    /// Register ECS update with metadata (for loop prevention)
    pub async fn register_ecs_update_with_metadata(
        &self,
        entity_id: uuid::Uuid,
        ecs_data: EcsComponentData,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<(), ComponentError> {
        // Check for sync loop
        if metadata.get("sync_source") == Some(&"ddd".to_string()) {
            // This update originated from DDD, don't sync back
            return Ok(());
        }
        
        let update = ComponentUpdate {
            entity_id,
            component_type: ecs_data.component_type,
            data: ecs_data.data,
            metadata,
        };
        
        self.ecs_updates.lock().await.push(update);
        Ok(())
    }
    
    /// Process pending syncs
    pub async fn process_pending_syncs(&self) -> Result<(), ComponentError> {
        // Process DDD to ECS
        let ddd_updates = {
            let mut updates = self.ddd_updates.lock().await;
            std::mem::take(&mut *updates)
        };
        
        for update in ddd_updates {
            let processed = ProcessedUpdate {
                entity_id: update.entity_id,
                component_type: update.component_type.clone(),
                metadata: update.metadata.clone(),
            };
            self.processed_updates.lock().await.push(processed);
        }
        
        Ok(())
    }
    
    /// Get pending ECS updates
    pub async fn get_pending_ecs_updates(&self) -> Vec<ComponentUpdate> {
        // Clone the current DDD updates as they need to sync to ECS
        self.ddd_updates.lock().await.clone()
    }
    
    /// Get pending DDD updates
    pub async fn get_pending_ddd_updates(&self) -> Vec<ComponentUpdate> {
        // Clone the current ECS updates as they need to sync to DDD
        self.ecs_updates.lock().await.clone()
    }
    
    /// Get processed ECS updates (for testing)
    pub async fn get_processed_ecs_updates(&self) -> Vec<ProcessedUpdate> {
        self.processed_updates.lock().await.clone()
    }
}

// ===== NATS Client Trait =====

/// Trait for NATS client operations
#[async_trait::async_trait]
pub trait NatsClient: Send + Sync {
    /// Publish a message
    async fn publish(&self, subject: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Subscribe to a subject pattern
    async fn subscribe(&self, pattern: &str) -> Result<Box<dyn Any + Send + Sync>, Box<dyn std::error::Error>>;
}

