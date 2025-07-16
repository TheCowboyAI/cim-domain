//! Event bridge for cross-domain event routing
//!
//! This module provides event routing and transformation capabilities
//! for cross-domain event communication.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use futures::stream::Stream;

use crate::errors::DomainError;
use crate::events::DomainEvent;

/// Event bridge for routing events between domains
pub struct EventBridge {
    /// Event router
    router: Arc<EventRouter>,
    
    /// Event transformers
    transformers: Arc<RwLock<HashMap<String, Box<dyn EventTransformer>>>>,
    
    /// Event filters
    filters: Arc<RwLock<Vec<Box<dyn EventFilter>>>>,
    
    /// Event subscribers
    subscribers: Arc<RwLock<HashMap<String, Vec<EventSubscriber>>>>,
    
    /// Bridge configuration
    config: BridgeConfig,
}

/// Configuration for the event bridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// Maximum events in buffer
    pub buffer_size: usize,
    
    /// Event TTL in seconds
    pub event_ttl_seconds: u64,
    
    /// Enable dead letter queue
    pub enable_dlq: bool,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Retry backoff multiplier
    pub retry_backoff_multiplier: f32,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10000,
            event_ttl_seconds: 3600,
            enable_dlq: true,
            max_retries: 3,
            retry_backoff_multiplier: 2.0,
        }
    }
}

/// Event router for determining event destinations
pub struct EventRouter {
    /// Routing rules
    rules: Arc<RwLock<Vec<RoutingRule>>>,
    
    /// Default routes
    default_routes: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

/// Routing rule for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Rule name
    pub name: String,
    
    /// Source domain pattern (supports wildcards)
    pub source_pattern: String,
    
    /// Event type pattern (supports wildcards)
    pub event_pattern: String,
    
    /// Target domains
    pub targets: Vec<String>,
    
    /// Rule priority (higher = more important)
    pub priority: u32,
    
    /// Additional conditions
    pub conditions: Vec<RoutingCondition>,
}

/// Condition for routing rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingCondition {
    /// Property equals value
    PropertyEquals { property: String, value: serde_json::Value },
    
    /// Property matches regex
    PropertyMatches { property: String, pattern: String },
    
    /// Property exists
    PropertyExists { property: String },
    
    /// Custom condition
    Custom { condition_type: String, data: serde_json::Value },
}

/// Event transformer for modifying events during routing
#[async_trait]
pub trait EventTransformer: Send + Sync {
    /// Transform an event
    async fn transform(
        &self,
        event: Box<dyn DomainEvent>,
        context: &TransformContext,
    ) -> Result<Box<dyn DomainEvent>, DomainError>;
    
    /// Check if this transformer applies to an event
    fn applies_to(&self, event_type: &str, source: &str, target: &str) -> bool;
    
    /// Get transformer description
    fn description(&self) -> String;
}

/// Context for event transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformContext {
    /// Source domain
    pub source: String,
    
    /// Target domain
    pub target: String,
    
    /// Routing metadata
    pub routing_metadata: HashMap<String, String>,
    
    /// Transform hints
    pub hints: HashMap<String, serde_json::Value>,
}

/// Event filter for filtering events
#[async_trait]
pub trait EventFilter: Send + Sync {
    /// Check if an event should be filtered
    async fn should_filter(&self, event: &dyn DomainEvent) -> bool;
    
    /// Get filter description
    fn description(&self) -> String;
}

/// Event subscriber
#[derive(Clone)]
pub struct EventSubscriber {
    /// Subscriber ID
    pub id: String,
    
    /// Subscriber name
    pub name: String,
    
    /// Event patterns to subscribe to
    pub patterns: Vec<String>,
    
    /// Delivery channel
    pub channel: mpsc::Sender<EventEnvelope>,
}

/// Event envelope with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// The event
    pub event: serde_json::Value, // Serialized DomainEvent
    
    /// Event metadata
    pub metadata: EventMetadata,
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Event ID
    pub event_id: String,
    
    /// Source domain
    pub source: String,
    
    /// Event type
    pub event_type: String,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Correlation ID
    pub correlation_id: Option<String>,
    
    /// Causation ID
    pub causation_id: Option<String>,
    
    /// Retry count
    pub retry_count: u32,
    
    /// Custom headers
    pub headers: HashMap<String, String>,
}

impl EventBridge {
    /// Create a new event bridge
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            router: Arc::new(EventRouter {
                rules: Arc::new(RwLock::new(Vec::new())),
                default_routes: Arc::new(RwLock::new(HashMap::new())),
            }),
            transformers: Arc::new(RwLock::new(HashMap::new())),
            filters: Arc::new(RwLock::new(Vec::new())),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Publish an event
    pub async fn publish(
        &self,
        event: Box<dyn DomainEvent>,
        source: String,
    ) -> Result<(), DomainError> {
        let event_type = event.event_type();
        
        // Check filters
        let filters = self.filters.read().await;
        for filter in filters.iter() {
            if filter.should_filter(&*event).await {
                return Ok(()); // Event filtered out
            }
        }
        drop(filters);
        
        // Determine routes
        let routes = self.router.determine_routes(&source, &event_type).await?;
        
        // Create event envelope
        // Note: Cannot serialize trait object directly, need concrete type
        let envelope = EventEnvelope {
            event: serde_json::json!({
                "event_type": event_type,
                "source": source,
                "note": "Serialization of trait objects not supported"
            }),
            metadata: EventMetadata {
                event_id: uuid::Uuid::new_v4().to_string(),
                source: source.clone(),
                event_type: event_type.to_string(),
                timestamp: chrono::Utc::now(),
                correlation_id: None, // Would be set from event
                causation_id: None,   // Would be set from event
                retry_count: 0,
                headers: HashMap::new(),
            },
        };
        
        // Route to each target
        for target in routes {
            self.route_to_target(envelope.clone(), &source, &target).await?;
        }
        
        Ok(())
    }
    
    /// Route event to a specific target
    async fn route_to_target(
        &self,
        mut envelope: EventEnvelope,
        source: &str,
        target: &str,
    ) -> Result<(), DomainError> {
        // Apply transformations
        let transform_context = TransformContext {
            source: source.to_string(),
            target: target.to_string(),
            routing_metadata: HashMap::new(),
            hints: HashMap::new(),
        };
        
        // Get applicable transformers
        let transformers = self.transformers.read().await;
        for (_, transformer) in transformers.iter() {
            if transformer.applies_to(&envelope.metadata.event_type, source, target) {
                // Transform event
                // In real implementation, would deserialize, transform, and re-serialize
            }
        }
        drop(transformers);
        
        // Deliver to subscribers
        let subscribers = self.subscribers.read().await;
        if let Some(target_subscribers) = subscribers.get(target) {
            for subscriber in target_subscribers {
                if self.matches_patterns(&envelope.metadata.event_type, &subscriber.patterns) {
                    // Send to subscriber (ignore if channel is full)
                    let _ = subscriber.channel.try_send(envelope.clone());
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if event type matches patterns
    fn matches_patterns(&self, event_type: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            if pattern == "*" || pattern == event_type {
                return true;
            }
            // Support simple wildcards
            if pattern.ends_with("*") {
                let prefix = &pattern[..pattern.len() - 1];
                if event_type.starts_with(prefix) {
                    return true;
                }
            }
        }
        false
    }
    
    /// Add a routing rule
    pub async fn add_rule(&self, rule: RoutingRule) -> Result<(), DomainError> {
        let mut rules = self.router.rules.write().await;
        rules.push(rule);
        // Sort by priority (highest first)
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        Ok(())
    }
    
    /// Add a transformer
    pub async fn add_transformer(
        &self,
        name: String,
        transformer: Box<dyn EventTransformer>,
    ) -> Result<(), DomainError> {
        let mut transformers = self.transformers.write().await;
        if transformers.contains_key(&name) {
            return Err(DomainError::AlreadyExists(
                format!("Transformer {} already exists", name)
            ));
        }
        transformers.insert(name, transformer);
        Ok(())
    }
    
    /// Add a filter
    pub async fn add_filter(&self, filter: Box<dyn EventFilter>) -> Result<(), DomainError> {
        let mut filters = self.filters.write().await;
        filters.push(filter);
        Ok(())
    }
    
    /// Subscribe to events
    pub async fn subscribe(
        &self,
        target_domain: String,
        subscriber: EventSubscriber,
    ) -> Result<(), DomainError> {
        let mut subscribers = self.subscribers.write().await;
        subscribers.entry(target_domain)
            .or_insert_with(Vec::new)
            .push(subscriber);
        Ok(())
    }
    
    /// Create an event stream for a domain
    pub async fn event_stream(
        &self,
        domain: String,
        patterns: Vec<String>,
        buffer_size: usize,
    ) -> Result<impl Stream<Item = EventEnvelope>, DomainError> {
        let (tx, rx) = mpsc::channel(buffer_size);
        
        let subscriber = EventSubscriber {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("{}_stream", domain),
            patterns,
            channel: tx,
        };
        
        self.subscribe(domain, subscriber).await?;
        
        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }
}

impl EventRouter {
    /// Determine routes for an event
    pub async fn determine_routes(
        &self,
        source: &str,
        event_type: &str,
    ) -> Result<Vec<String>, DomainError> {
        let mut targets = Vec::new();
        
        // Check routing rules
        let rules = self.rules.read().await;
        for rule in rules.iter() {
            if self.matches_pattern(source, &rule.source_pattern) &&
               self.matches_pattern(event_type, &rule.event_pattern) {
                // Add targets
                for target in &rule.targets {
                    if !targets.contains(target) {
                        targets.push(target.clone());
                    }
                }
            }
        }
        drop(rules);
        
        // If no rules matched, check default routes
        if targets.is_empty() {
            let defaults = self.default_routes.read().await;
            if let Some(default_targets) = defaults.get(source) {
                targets.extend(default_targets.clone());
            }
        }
        
        Ok(targets)
    }
    
    /// Check if a value matches a pattern
    fn matches_pattern(&self, value: &str, pattern: &str) -> bool {
        if pattern == "*" || pattern == value {
            return true;
        }
        // Support simple wildcards
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            return value.starts_with(prefix);
        }
        false
    }
    
    /// Set default route
    pub async fn set_default_route(
        &self,
        source: String,
        targets: Vec<String>,
    ) -> Result<(), DomainError> {
        let mut defaults = self.default_routes.write().await;
        defaults.insert(source, targets);
        Ok(())
    }
}

/// Example: Property-based event filter
pub struct PropertyFilter {
    property: String,
    value: serde_json::Value,
    include: bool, // true = include matching, false = exclude matching
}

impl PropertyFilter {
    pub fn include_matching(property: String, value: serde_json::Value) -> Self {
        Self { property, value, include: true }
    }
    
    pub fn exclude_matching(property: String, value: serde_json::Value) -> Self {
        Self { property, value, include: false }
    }
}

#[async_trait]
impl EventFilter for PropertyFilter {
    async fn should_filter(&self, event: &dyn DomainEvent) -> bool {
        // In real implementation, would check event properties
        // For now, don't filter
        false
    }
    
    fn description(&self) -> String {
        format!(
            "{} events where {} = {}",
            if self.include { "Include" } else { "Exclude" },
            self.property,
            self.value
        )
    }
}

/// Example: Field mapping transformer
pub struct FieldMappingTransformer {
    mappings: HashMap<String, String>,
}

impl FieldMappingTransformer {
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }
}

#[async_trait]
impl EventTransformer for FieldMappingTransformer {
    async fn transform(
        &self,
        event: Box<dyn DomainEvent>,
        context: &TransformContext,
    ) -> Result<Box<dyn DomainEvent>, DomainError> {
        // In real implementation, would map fields
        Ok(event)
    }
    
    fn applies_to(&self, event_type: &str, source: &str, target: &str) -> bool {
        // Apply to all events
        true
    }
    
    fn description(&self) -> String {
        format!("Maps {} fields", self.mappings.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_event_routing() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Add routing rule
        let rule = RoutingRule {
            name: "order_to_billing".to_string(),
            source_pattern: "orders.*".to_string(),
            event_pattern: "OrderPlaced".to_string(),
            targets: vec!["billing".to_string()],
            priority: 100,
            conditions: vec![],
        };
        
        bridge.add_rule(rule).await.unwrap();
        
        // Test route determination
        let routes = bridge.router.determine_routes("orders.service", "OrderPlaced").await.unwrap();
        assert_eq!(routes, vec!["billing"]);
    }
    
    #[tokio::test]
    async fn test_event_subscription() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Create subscriber
        let (tx, _rx) = mpsc::channel(10);
        let subscriber = EventSubscriber {
            id: "test_sub".to_string(),
            name: "Test Subscriber".to_string(),
            patterns: vec!["Order*".to_string()],
            channel: tx,
        };
        
        bridge.subscribe("test_domain".to_string(), subscriber).await.unwrap();
        
        // Test pattern matching
        assert!(bridge.matches_patterns("OrderPlaced", &["Order*".to_string()]));
        assert!(bridge.matches_patterns("OrderCancelled", &["Order*".to_string()]));
        assert!(!bridge.matches_patterns("PaymentReceived", &["Order*".to_string()]));
    }
    
    #[tokio::test]
    async fn test_event_stream() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Create event stream
        let _stream = bridge.event_stream(
            "test_domain".to_string(),
            vec!["*".to_string()],
            100,
        ).await.unwrap();
        
        // Stream should be ready to receive events
        // In real test, would publish events and verify they appear in stream
    }
    
    #[tokio::test]
    async fn test_default_routes() {
        let router = EventRouter {
            rules: Arc::new(RwLock::new(Vec::new())),
            default_routes: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Set default route
        router.set_default_route(
            "unknown_service".to_string(),
            vec!["default_handler".to_string()],
        ).await.unwrap();
        
        // Should use default route when no rules match
        let routes = router.determine_routes("unknown_service", "SomeEvent").await.unwrap();
        assert_eq!(routes, vec!["default_handler"]);
    }
}