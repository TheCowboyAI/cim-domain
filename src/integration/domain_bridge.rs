// Copyright 2025 Cowboy AI, LLC.

//! Domain bridge for cross-domain communication
//!
//! This module implements the bridge pattern for connecting
//! different domains while maintaining their independence.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::category::functor::ContextMappingFunctor;
use crate::commands::DomainCommand;
use crate::errors::DomainError;
use crate::events::DomainEvent;

// Type aliases for complex types
type BridgeMap = HashMap<(String, String), Arc<DomainBridge>>;
type SubscriberMap = HashMap<String, Vec<String>>;

/// Serializable command wrapper for cross-domain communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedCommand {
    /// Type/name of the command
    pub command_type: String,
    /// ID of the aggregate the command targets
    pub aggregate_id: String,
    /// Serialized command data
    pub payload: serde_json::Value,
}

impl SerializedCommand {
    /// Create a serialized command from a domain command
    pub fn from_command<C: DomainCommand + Serialize>(command: &C) -> Result<Self, DomainError> {
        Ok(Self {
            command_type: command.command_type().to_string(),
            aggregate_id: command.aggregate_id(),
            payload: serde_json::to_value(command)
                .map_err(|e| DomainError::SerializationError(e.to_string()))?,
        })
    }
}

/// A bridge between two domains
pub struct DomainBridge {
    /// Source domain name
    pub source_domain: String,

    /// Target domain name
    pub target_domain: String,

    /// Message translator
    pub translator: Option<Box<dyn MessageTranslator>>,

    /// Bridge adapter
    pub adapter: Option<Box<dyn BridgeAdapter>>,

    /// Context mapping functor
    pub functor: ContextMappingFunctor,

    /// Bridge metadata
    pub metadata: HashMap<String, String>,
}

/// Translates messages between domains
#[async_trait]
pub trait MessageTranslator: Send + Sync {
    /// Translate a command from source to target domain
    async fn translate_command(
        &self,
        command: SerializedCommand,
        context: &TranslationContext,
    ) -> Result<SerializedCommand, DomainError>;

    /// Translate an event from source to target domain
    async fn translate_event(
        &self,
        event: Box<dyn DomainEvent>,
        context: &TranslationContext,
    ) -> Result<Box<dyn DomainEvent>, DomainError>;

    /// Check if a command can be translated
    fn can_translate_command(&self, command_type: &str) -> bool;

    /// Check if an event can be translated
    fn can_translate_event(&self, event_type: &str) -> bool;
}

/// Context for message translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationContext {
    /// Source domain context
    pub source_context: HashMap<String, serde_json::Value>,

    /// Target domain context
    pub target_context: HashMap<String, serde_json::Value>,

    /// Translation hints
    pub hints: HashMap<String, String>,
}

impl Default for TranslationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationContext {
    /// Create a new translation context
    pub fn new() -> Self {
        Self {
            source_context: HashMap::new(),
            target_context: HashMap::new(),
            hints: HashMap::new(),
        }
    }

    /// Add data to the source context
    pub fn with_source_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.source_context.insert(key, value);
        self
    }

    /// Add data to the target context
    pub fn with_target_data(mut self, key: String, value: serde_json::Value) -> Self {
        self.target_context.insert(key, value);
        self
    }

    /// Add a translation hint
    ///
    /// # Arguments
    /// * `key` - Hint key
    /// * `value` - Hint value
    pub fn with_hint(mut self, key: String, value: String) -> Self {
        self.hints.insert(key, value);
        self
    }
}

/// Adapter for bridge communication
#[async_trait]
pub trait BridgeAdapter: Send + Sync {
    /// Send a command through the bridge
    async fn send_command(
        &self,
        command: SerializedCommand,
        target_domain: &str,
    ) -> Result<(), DomainError>;

    /// Send an event through the bridge
    async fn send_event(
        &self,
        event: Box<dyn DomainEvent>,
        target_domain: &str,
    ) -> Result<(), DomainError>;

    /// Subscribe to events from a domain
    async fn subscribe_events(
        &self,
        source_domain: &str,
        event_types: Vec<String>,
    ) -> Result<(), DomainError>;

    /// Health check for the adapter
    async fn health_check(&self) -> Result<BridgeHealth, DomainError>;
}

/// Health status of a bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHealth {
    /// Is the bridge operational
    pub is_healthy: bool,

    /// Last successful communication
    pub last_success: Option<chrono::DateTime<chrono::Utc>>,

    /// Error count in last hour
    pub recent_errors: u32,

    /// Average latency in milliseconds
    pub avg_latency_ms: Option<f64>,
}

impl DomainBridge {
    /// Create a new domain bridge
    pub fn new(source_domain: String, target_domain: String) -> Self {
        let functor = ContextMappingFunctor::new(source_domain.clone(), target_domain.clone());

        Self {
            source_domain,
            target_domain,
            translator: None,
            adapter: None,
            functor,
            metadata: HashMap::new(),
        }
    }

    /// Set the translator
    pub fn set_translator(&mut self, translator: Box<dyn MessageTranslator>) {
        self.translator = Some(translator);
    }

    /// Set the adapter
    pub fn set_adapter(&mut self, adapter: Box<dyn BridgeAdapter>) {
        self.adapter = Some(adapter);
    }

    /// Forward a command through the bridge
    pub async fn forward_command(
        &self,
        command: SerializedCommand,
        context: &TranslationContext,
    ) -> Result<(), DomainError> {
        // Check if translator is available
        let translator = self
            .translator
            .as_ref()
            .ok_or_else(|| DomainError::InvalidOperation {
                reason: "No translator configured for bridge".to_string(),
            })?;

        // Check if translation is possible
        let command_type = &command.command_type;
        if !translator.can_translate_command(command_type) {
            return Err(DomainError::InvalidOperation {
                reason: format!("Cannot translate command type: {command_type}"),
            });
        }

        // Translate the command
        let translated = translator.translate_command(command, context).await?;

        // Send through adapter if available
        if let Some(adapter) = &self.adapter {
            adapter.send_command(translated, &self.target_domain).await
        } else {
            Err(DomainError::InvalidOperation {
                reason: "No adapter configured for bridge".to_string(),
            })
        }
    }

    /// Forward an event through the bridge
    pub async fn forward_event(
        &self,
        event: Box<dyn DomainEvent>,
        context: &TranslationContext,
    ) -> Result<(), DomainError> {
        // Check if translator is available
        let translator = self
            .translator
            .as_ref()
            .ok_or_else(|| DomainError::InvalidOperation {
                reason: "No translator configured for bridge".to_string(),
            })?;

        // Check if translation is possible
        let event_type = event.event_type();
        if !translator.can_translate_event(&event_type) {
            return Err(DomainError::InvalidOperation {
                reason: format!("Cannot translate event type: {event_type}"),
            });
        }

        // Translate the event
        let translated = translator.translate_event(event, context).await?;

        // Send through adapter if available
        if let Some(adapter) = &self.adapter {
            adapter.send_event(translated, &self.target_domain).await
        } else {
            Err(DomainError::InvalidOperation {
                reason: "No adapter configured for bridge".to_string(),
            })
        }
    }

    /// Check bridge health
    pub async fn health_check(&self) -> Result<BridgeHealth, DomainError> {
        if let Some(adapter) = &self.adapter {
            adapter.health_check().await
        } else {
            Ok(BridgeHealth {
                is_healthy: false,
                last_success: None,
                recent_errors: 0,
                avg_latency_ms: None,
            })
        }
    }
}

/// Registry for domain bridges
pub struct BridgeRegistry {
    /// Registered bridges
    bridges: Arc<RwLock<BridgeMap>>,
}

impl Default for BridgeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeRegistry {
    /// Create a new bridge registry
    pub fn new() -> Self {
        Self {
            bridges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a bridge
    pub async fn register(&self, bridge: DomainBridge) -> Result<(), DomainError> {
        let key = (bridge.source_domain.clone(), bridge.target_domain.clone());
        let mut bridges = self.bridges.write().await;

        if bridges.contains_key(&key) {
            return Err(DomainError::AlreadyExists(format!(
                "Bridge from {} to {} already exists",
                key.0, key.1
            )));
        }

        bridges.insert(key, Arc::new(bridge));
        Ok(())
    }

    /// Get a bridge
    pub async fn get_bridge(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Arc<DomainBridge>, DomainError> {
        let bridges = self.bridges.read().await;
        let bridge = bridges
            .get(&(source.to_string(), target.to_string()))
            .ok_or_else(|| {
                DomainError::NotFound(format!("Bridge from {source} to {target} not found"))
            })?;

        Ok(bridge.clone())
    }

    /// Find all bridges from a source domain
    pub async fn find_from_source(&self, source: &str) -> Vec<(String, String)> {
        let bridges = self.bridges.read().await;
        bridges
            .keys()
            .filter(|(s, _)| s == source)
            .cloned()
            .collect()
    }

    /// Find all bridges to a target domain
    pub async fn find_to_target(&self, target: &str) -> Vec<(String, String)> {
        let bridges = self.bridges.read().await;
        bridges
            .keys()
            .filter(|(_, t)| t == target)
            .cloned()
            .collect()
    }

    /// Send a command through a bridge
    pub async fn send_command(
        &self,
        source: &str,
        target: &str,
        command: SerializedCommand,
        context: TranslationContext,
    ) -> Result<SerializedCommand, DomainError> {
        let bridge = self.get_bridge(source, target).await?;

        // Translate the command if translator is available
        let translated_command = if let Some(translator) = &bridge.translator {
            translator.translate_command(command, &context).await?
        } else {
            command
        };

        // Send through adapter if available
        if let Some(adapter) = &bridge.adapter {
            adapter
                .send_command(translated_command.clone(), target)
                .await?;
        }

        Ok(translated_command)
    }
}

/// Mapping configuration for a command or event type
#[derive(Clone)]
struct TypeMapping {
    /// Target type name
    target_type: String,
    /// Property mappings (source -> target)
    property_mappings: Vec<(String, String)>,
}

/// Example: Simple property-based translator
pub struct PropertyBasedTranslator {
    /// Command mappings
    command_mappings: HashMap<String, TypeMapping>,

    /// Event mappings
    event_mappings: HashMap<String, TypeMapping>,
}

impl Default for PropertyBasedTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyBasedTranslator {
    /// Create a new property-based translator
    pub fn new() -> Self {
        Self {
            command_mappings: HashMap::new(),
            event_mappings: HashMap::new(),
        }
    }

    /// Add a command type mapping
    ///
    /// # Arguments
    /// * `source` - Source command type
    /// * `target` - Target command type
    /// * `property_mappings` - Property name mappings
    pub fn add_command_mapping(
        &mut self,
        source: String,
        target: String,
        property_mappings: Vec<(String, String)>,
    ) {
        self.command_mappings.insert(
            source,
            TypeMapping {
                target_type: target,
                property_mappings,
            },
        );
    }

    /// Add an event type mapping
    ///
    /// # Arguments
    /// * `source` - Source event type
    /// * `target` - Target event type
    /// * `property_mappings` - Property name mappings
    pub fn add_event_mapping(
        &mut self,
        source: String,
        target: String,
        property_mappings: Vec<(String, String)>,
    ) {
        self.event_mappings.insert(
            source,
            TypeMapping {
                target_type: target,
                property_mappings,
            },
        );
    }
}

#[async_trait]
impl MessageTranslator for PropertyBasedTranslator {
    async fn translate_command(
        &self,
        command: SerializedCommand,
        context: &TranslationContext,
    ) -> Result<SerializedCommand, DomainError> {
        // Get the mapping for this command type
        let mapping = self
            .command_mappings
            .get(&command.command_type)
            .ok_or_else(|| DomainError::InvalidOperation {
                reason: format!(
                    "No mapping found for command type: {}",
                    command.command_type
                ),
            })?;

        // Create translated payload map
        let mut payload_map = serde_json::Map::new();

        // Apply property mappings
        if let Some(obj) = command.payload.as_object() {
            for (source_prop, target_prop) in &mapping.property_mappings {
                if let Some(value) = obj.get(source_prop) {
                    payload_map.insert(target_prop.clone(), value.clone());
                }
            }
        }

        // Apply any context-specific transformations
        if let Some(target_domain_context) = context.target_context.get("domain_context") {
            payload_map.insert("_context".to_string(), target_domain_context.clone());
        }

        // Create translated command with mapped type
        let translated = SerializedCommand {
            command_type: mapping.target_type.clone(),
            aggregate_id: command.aggregate_id,
            payload: serde_json::Value::Object(payload_map),
        };

        Ok(translated)
    }

    async fn translate_event(
        &self,
        event: Box<dyn DomainEvent>,
        context: &TranslationContext,
    ) -> Result<Box<dyn DomainEvent>, DomainError> {
        // Get the mapping for this event type
        let event_type = event.event_type();
        let mapping =
            self.event_mappings
                .get(event_type)
                .ok_or_else(|| DomainError::InvalidOperation {
                    reason: format!("No mapping found for event type: {event_type}"),
                })?;

        // Since we can't serialize trait objects, create a simple payload
        // In a real implementation, you would need type-specific handling
        let mut translated_payload = serde_json::Map::new();

        // Add basic event information
        translated_payload.insert(
            "source_event_type".to_string(),
            serde_json::Value::String(event_type.to_string()),
        );
        translated_payload.insert(
            "aggregate_id".to_string(),
            serde_json::Value::String(event.aggregate_id().to_string()),
        );
        translated_payload.insert(
            "subject".to_string(),
            serde_json::Value::String(event.subject()),
        );

        // Add context metadata if provided
        if let Some(target_context) = context.target_context.get("domain_context") {
            translated_payload.insert("_context".to_string(), target_context.clone());
        }

        // For now, return a generic translated event wrapper
        // In a real implementation, this would deserialize to the target event type
        use crate::events::{DomainEvent as DomainEventTrait, EventMetadata};
        use uuid::Uuid;

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        struct TranslatedEvent {
            event_type: String,
            aggregate_id: Uuid,
            payload: serde_json::Value,
            metadata: EventMetadata,
        }

        impl DomainEventTrait for TranslatedEvent {
            fn subject(&self) -> String {
                format!("{}.translated.v1", self.event_type)
            }

            fn aggregate_id(&self) -> Uuid {
                self.aggregate_id
            }

            fn event_type(&self) -> &'static str {
                "TranslatedEvent"
            }
        }

        let translated_event = TranslatedEvent {
            event_type: mapping.target_type.clone(),
            aggregate_id: event.aggregate_id(),
            payload: serde_json::Value::Object(translated_payload),
            metadata: EventMetadata {
                source: context
                    .source_context
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("integration")
                    .to_string(),
                version: "1.0".to_string(),
                propagation_scope: crate::events::PropagationScope::Leaf,
                properties: context.source_context.clone(),
            },
        };

        Ok(Box::new(translated_event) as Box<dyn DomainEvent>)
    }

    fn can_translate_command(&self, command_type: &str) -> bool {
        self.command_mappings.contains_key(command_type)
    }

    fn can_translate_event(&self, event_type: &str) -> bool {
        self.event_mappings.contains_key(event_type)
    }
}

/// Example: In-memory bridge adapter
pub struct InMemoryBridgeAdapter {
    /// Event subscribers
    subscribers: Arc<RwLock<SubscriberMap>>,

    /// Health metrics
    health_metrics: Arc<RwLock<BridgeHealth>>,
}

impl InMemoryBridgeAdapter {
    /// Create a new in-memory bridge adapter
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            health_metrics: Arc::new(RwLock::new(BridgeHealth {
                is_healthy: true,
                last_success: None,
                recent_errors: 0,
                avg_latency_ms: None,
            })),
        }
    }
}

#[async_trait]
impl BridgeAdapter for InMemoryBridgeAdapter {
    async fn send_command(
        &self,
        _command: SerializedCommand,
        _target_domain: &str,
    ) -> Result<(), DomainError> {
        // Update health metrics
        let mut health = self.health_metrics.write().await;
        health.last_success = Some(chrono::Utc::now());

        // In real implementation, would route to target domain
        Ok(())
    }

    async fn send_event(
        &self,
        _event: Box<dyn DomainEvent>,
        _target_domain: &str,
    ) -> Result<(), DomainError> {
        // Update health metrics
        let mut health = self.health_metrics.write().await;
        health.last_success = Some(chrono::Utc::now());

        // In real implementation, would publish to subscribers
        Ok(())
    }

    async fn subscribe_events(
        &self,
        source_domain: &str,
        event_types: Vec<String>,
    ) -> Result<(), DomainError> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.insert(source_domain.to_string(), event_types);
        Ok(())
    }

    async fn health_check(&self) -> Result<BridgeHealth, DomainError> {
        let health = self.health_metrics.read().await;
        Ok(health.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_bridge_creation() {
        let bridge = DomainBridge::new("Sales".to_string(), "Billing".to_string());

        assert_eq!(bridge.source_domain, "Sales");
        assert_eq!(bridge.target_domain, "Billing");
    }

    #[tokio::test]
    async fn test_bridge_registry() {
        let registry = BridgeRegistry::new();

        let bridge = DomainBridge::new("Domain1".to_string(), "Domain2".to_string());

        registry.register(bridge).await.unwrap();

        // Test finding bridges
        let from_domain1 = registry.find_from_source("Domain1").await;
        assert_eq!(from_domain1.len(), 1);
        assert_eq!(
            from_domain1[0],
            ("Domain1".to_string(), "Domain2".to_string())
        );

        let to_domain2 = registry.find_to_target("Domain2").await;
        assert_eq!(to_domain2.len(), 1);
    }

    #[tokio::test]
    async fn test_translation_context() {
        let context = TranslationContext::new()
            .with_source_data("order_id".to_string(), serde_json::json!("123"))
            .with_target_data("invoice_id".to_string(), serde_json::json!("INV-123"))
            .with_hint("priority".to_string(), "high".to_string());

        assert_eq!(
            context.source_context.get("order_id").unwrap(),
            &serde_json::json!("123")
        );
        assert_eq!(context.hints.get("priority").unwrap(), "high");
    }

    #[tokio::test]
    async fn test_bridge_health() {
        let adapter = InMemoryBridgeAdapter::new();

        // Send a command to update health
        let command = SerializedCommand {
            command_type: "AcknowledgeCommand".to_string(),
            aggregate_id: "test_aggregate".to_string(),
            payload: serde_json::json!({}),
        };
        adapter.send_command(command, "TestDomain").await.unwrap();

        let health = adapter.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert!(health.last_success.is_some());
    }
}
