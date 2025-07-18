// Copyright 2025 Cowboy AI, LLC.

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
    _config: BridgeConfig,
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
    PropertyEquals { 
        /// Property path to check
        property: String, 
        /// Expected value
        value: serde_json::Value 
    },
    
    /// Property matches regex
    PropertyMatches { 
        /// Property path to check
        property: String, 
        /// Regex pattern to match
        pattern: String 
    },
    
    /// Property exists
    PropertyExists { 
        /// Property path to check for existence
        property: String 
    },
    
    /// Custom condition
    Custom { 
        /// Type of custom condition
        condition_type: String, 
        /// Condition data
        data: serde_json::Value 
    },
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
            _config: config,
        }
    }
    
    /// Add a routing rule
    pub async fn add_routing_rule(
        &self,
        name: String,
        source_pattern: String,
        event_type_pattern: String,
        targets: Vec<String>,
        condition: Option<RoutingCondition>,
    ) -> Result<(), DomainError> {
        let rule = RoutingRule {
            name,
            source_pattern,
            event_pattern: event_type_pattern,
            targets,
            priority: 100, // Default priority
            conditions: condition.map(|c| vec![c]).unwrap_or_default(),
        };
        
        let mut rules = self.router.rules.write().await;
        rules.push(rule);
        rules.sort_by_key(|r| std::cmp::Reverse(r.priority));
        Ok(())
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
        envelope: EventEnvelope,
        source: &str,
        target: &str,
    ) -> Result<(), DomainError> {
        // Apply transformations
        let _transform_context = TransformContext {
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
            .or_default()
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
    /// Create a new event router
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            default_routes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
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
    /// Property name to filter on
    property: String,
    /// Expected property value
    value: serde_json::Value,
    /// Whether to include (true) or exclude (false) matching events
    include: bool, // true = include matching, false = exclude matching
}

impl PropertyFilter {
    /// Create a filter that includes events matching the property
    pub fn include_matching(property: String, value: serde_json::Value) -> Self {
        Self { property, value, include: true }
    }
    
    /// Create a filter that excludes events matching the property
    pub fn exclude_matching(property: String, value: serde_json::Value) -> Self {
        Self { property, value, include: false }
    }
}

#[async_trait]
impl EventFilter for PropertyFilter {
    async fn should_filter(&self, _event: &dyn DomainEvent) -> bool {
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
    /// Field name mappings (source -> target)
    mappings: HashMap<String, String>,
}

impl FieldMappingTransformer {
    /// Create a new field mapping transformer
    pub fn new(mappings: HashMap<String, String>) -> Self {
        Self { mappings }
    }
}

#[async_trait]
impl EventTransformer for FieldMappingTransformer {
    async fn transform(
        &self,
        event: Box<dyn DomainEvent>,
        _context: &TransformContext,
    ) -> Result<Box<dyn DomainEvent>, DomainError> {
        // In real implementation, would map fields
        Ok(event)
    }
    
    fn applies_to(&self, _event_type: &str, _source: &str, _target: &str) -> bool {
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
    use futures::StreamExt;
    
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
    
    #[test]
    fn test_bridge_config_default() {
        let config = BridgeConfig::default();
        assert_eq!(config.buffer_size, 10000);
        assert_eq!(config.event_ttl_seconds, 3600);
        assert!(config.enable_dlq);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_backoff_multiplier, 2.0);
    }
    
    #[test]
    fn test_routing_rule_serialization() {
        let rule = RoutingRule {
            name: "test_rule".to_string(),
            source_pattern: "src.*".to_string(),
            event_pattern: "Event*".to_string(),
            targets: vec!["target1".to_string(), "target2".to_string()],
            priority: 50,
            conditions: vec![
                RoutingCondition::PropertyEquals {
                    property: "status".to_string(),
                    value: serde_json::json!("active"),
                },
                RoutingCondition::PropertyExists {
                    property: "user_id".to_string(),
                },
            ],
        };
        
        // Test serialization
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("test_rule"));
        assert!(json.contains("src.*"));
        
        // Test deserialization
        let deserialized: RoutingRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, rule.name);
        assert_eq!(deserialized.priority, rule.priority);
        assert_eq!(deserialized.targets.len(), 2);
        assert_eq!(deserialized.conditions.len(), 2);
    }
    
    #[tokio::test]
    async fn test_pattern_matching() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Test exact match
        assert!(bridge.matches_patterns("OrderPlaced", &["OrderPlaced".to_string()]));
        
        // Test wildcard match
        assert!(bridge.matches_patterns("OrderPlaced", &["*".to_string()]));
        
        // Test prefix wildcard
        assert!(bridge.matches_patterns("OrderPlaced", &["Order*".to_string()]));
        assert!(bridge.matches_patterns("OrderCancelled", &["Order*".to_string()]));
        assert!(!bridge.matches_patterns("PaymentReceived", &["Order*".to_string()]));
        
        // Test multiple patterns
        assert!(bridge.matches_patterns("PaymentReceived", &[
            "Order*".to_string(),
            "Payment*".to_string(),
        ]));
    }
    
    #[tokio::test]
    async fn test_multiple_routing_rules() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Add multiple rules with different priorities
        let rule1 = RoutingRule {
            name: "high_priority".to_string(),
            source_pattern: "orders.*".to_string(),
            event_pattern: "Order*".to_string(),
            targets: vec!["high_priority_handler".to_string()],
            priority: 100,
            conditions: vec![],
        };
        
        let rule2 = RoutingRule {
            name: "low_priority".to_string(),
            source_pattern: "orders.*".to_string(),
            event_pattern: "*".to_string(),
            targets: vec!["low_priority_handler".to_string()],
            priority: 10,
            conditions: vec![],
        };
        
        bridge.add_rule(rule1).await.unwrap();
        bridge.add_rule(rule2).await.unwrap();
        
        // Both rules should match
        let routes = bridge.router.determine_routes("orders.service", "OrderPlaced").await.unwrap();
        assert_eq!(routes.len(), 2);
        assert!(routes.contains(&"high_priority_handler".to_string()));
        assert!(routes.contains(&"low_priority_handler".to_string()));
    }
    
    #[tokio::test]
    async fn test_transformer_registration() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        let transformer = Box::new(FieldMappingTransformer::new(
            HashMap::from([
                ("old_field".to_string(), "new_field".to_string()),
            ])
        ));
        
        // First registration should succeed
        assert!(bridge.add_transformer("mapper1".to_string(), transformer).await.is_ok());
        
        // Duplicate registration should fail
        let transformer2 = Box::new(FieldMappingTransformer::new(HashMap::new()));
        let result = bridge.add_transformer("mapper1".to_string(), transformer2).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::AlreadyExists(msg) => assert!(msg.contains("mapper1")),
            _ => panic!("Expected AlreadyExists error"),
        }
    }
    
    #[tokio::test]
    async fn test_filter_functionality() {
        let bridge = EventBridge::new(BridgeConfig::default());
        
        let filter = Box::new(PropertyFilter::include_matching(
            "status".to_string(),
            serde_json::json!("active"),
        ));
        
        assert!(bridge.add_filter(filter).await.is_ok());
        
        // Test filter description
        let filter2 = PropertyFilter::exclude_matching(
            "deleted".to_string(),
            serde_json::json!(true),
        );
        assert_eq!(filter2.description(), "Exclude events where deleted = true");
    }
    
    #[test]
    fn test_event_envelope_metadata() {
        let metadata = EventMetadata {
            event_id: "test-123".to_string(),
            source: "test_service".to_string(),
            event_type: "TestEvent".to_string(),
            timestamp: chrono::Utc::now(),
            correlation_id: Some("corr-456".to_string()),
            causation_id: Some("cause-789".to_string()),
            retry_count: 2,
            headers: HashMap::from([
                ("custom-header".to_string(), "value".to_string()),
            ]),
        };
        
        assert_eq!(metadata.event_id, "test-123");
        assert_eq!(metadata.retry_count, 2);
        assert_eq!(metadata.headers.get("custom-header").unwrap(), "value");
    }
    
    #[tokio::test]
    async fn test_concurrent_subscription() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
        let subscription_count = Arc::new(AtomicUsize::new(0));
        
        // Spawn multiple tasks to subscribe concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let bridge_clone = bridge.clone();
            let count_clone = subscription_count.clone();
            
            handles.push(tokio::spawn(async move {
                let (tx, _rx) = mpsc::channel(10);
                let subscriber = EventSubscriber {
                    id: format!("sub_{}", i),
                    name: format!("Subscriber {}", i),
                    patterns: vec!["*".to_string()],
                    channel: tx,
                };
                
                bridge_clone.subscribe("test_domain".to_string(), subscriber).await.unwrap();
                count_clone.fetch_add(1, Ordering::SeqCst);
            }));
        }
        
        // Wait for all subscriptions
        for handle in handles {
            handle.await.unwrap();
        }
        
        assert_eq!(subscription_count.load(Ordering::SeqCst), 10);
        
        // Verify all subscribers are registered
        let subscribers = bridge.subscribers.read().await;
        assert_eq!(subscribers.get("test_domain").unwrap().len(), 10);
    }
    
    #[test]
    fn test_routing_condition_variants() {
        use RoutingCondition::*;
        
        let conditions = vec![
            PropertyEquals {
                property: "status".to_string(),
                value: serde_json::json!("active"),
            },
            PropertyMatches {
                property: "email".to_string(),
                pattern: r".*@example\.com".to_string(),
            },
            PropertyExists {
                property: "user_id".to_string(),
            },
            Custom {
                condition_type: "time_range".to_string(),
                data: serde_json::json!({
                    "start": "09:00",
                    "end": "17:00"
                }),
            },
        ];
        
        // Test serialization
        for condition in conditions {
            let json = serde_json::to_string(&condition).unwrap();
            let _deserialized: RoutingCondition = serde_json::from_str(&json).unwrap();
        }
    }
    
    #[test]
    fn test_transform_context() {
        let context = TransformContext {
            source: "domain_a".to_string(),
            target: "domain_b".to_string(),
            routing_metadata: HashMap::from([
                ("route_id".to_string(), "route-123".to_string()),
            ]),
            hints: HashMap::from([
                ("format".to_string(), serde_json::json!("compact")),
            ]),
        };
        
        assert_eq!(context.source, "domain_a");
        assert_eq!(context.target, "domain_b");
        assert_eq!(context.routing_metadata.get("route_id").unwrap(), "route-123");
        assert_eq!(context.hints.get("format").unwrap(), &serde_json::json!("compact"));
    }
    
    #[tokio::test]
    async fn test_route_determination_with_no_matches() {
        let router = EventRouter {
            rules: Arc::new(RwLock::new(Vec::new())),
            default_routes: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Add rule that won't match
        let mut rules = router.rules.write().await;
        rules.push(RoutingRule {
            name: "no_match".to_string(),
            source_pattern: "specific_service".to_string(),
            event_pattern: "SpecificEvent".to_string(),
            targets: vec!["handler".to_string()],
            priority: 100,
            conditions: vec![],
        });
        drop(rules);
        
        // Should return empty routes
        let routes = router.determine_routes("other_service", "OtherEvent").await.unwrap();
        assert_eq!(routes.len(), 0);
    }
    
    #[tokio::test]
    async fn test_event_stream_creation() {
            
        let bridge = EventBridge::new(BridgeConfig::default());
        
        // Create stream with specific patterns
        let mut stream = bridge.event_stream(
            "test_domain".to_string(),
            vec!["Order*".to_string(), "Payment*".to_string()],
            50,
        ).await.unwrap();
        
        // Test that stream is created and can be polled
        // In real scenario, would publish events and verify they appear
        let timeout = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            stream.next()
        ).await;
        
        // Should timeout as no events published
        assert!(timeout.is_err());
    }
    
    #[test]
    fn test_field_mapping_transformer() {
        let mappings = HashMap::from([
            ("old_name".to_string(), "new_name".to_string()),
            ("old_status".to_string(), "new_status".to_string()),
        ]);
        
        let transformer = FieldMappingTransformer::new(mappings);
        
        assert!(transformer.applies_to("AnyEvent", "source", "target"));
        assert_eq!(transformer.description(), "Maps 2 fields");
    }
    
    // ===== CONCURRENT TESTING MODULE =====
    
    /// Tests for concurrent access patterns and race conditions
    mod concurrent_tests {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Duration;
        use tokio::time::timeout;
        
        #[tokio::test]
        async fn test_concurrent_publish_multiple_publishers() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            let publish_count = Arc::new(AtomicU64::new(0));
            
            // Add a routing rule
            bridge.add_rule(RoutingRule {
                name: "concurrent_test".to_string(),
                source_pattern: "*".to_string(),
                event_pattern: "*".to_string(),
                targets: vec!["target".to_string()],
                priority: 100,
                conditions: vec![],
            }).await.unwrap();
            
            // Create a subscriber to count received events
            let (tx, mut rx) = mpsc::channel(1000);
            bridge.subscribe("target".to_string(), EventSubscriber {
                id: "counter".to_string(),
                name: "Counter".to_string(),
                patterns: vec!["*".to_string()],
                channel: tx,
            }).await.unwrap();
            
            // Spawn multiple publishers
            let mut handles = vec![];
            for i in 0..20 {
                let bridge_clone = bridge.clone();
                let count_clone = publish_count.clone();
                
                handles.push(tokio::spawn(async move {
                    for j in 0..50 {
                        // Create a test event
                        #[derive(Debug)]
                        struct TestEvent {
                            id: String,
                        }
                        impl DomainEvent for TestEvent {
                            fn subject(&self) -> String { "test.event".to_string() }
                            fn aggregate_id(&self) -> uuid::Uuid { uuid::Uuid::new_v4() }
                            fn event_type(&self) -> &'static str { "TestEvent" }
                        }
                        
                        let event = Box::new(TestEvent {
                            id: format!("pub-{i}-event-{j}"),
                        });
                        
                        bridge_clone.publish(event, format!("publisher-{}", i)).await.unwrap();
                        count_clone.fetch_add(1, Ordering::SeqCst);
                    }
                }));
            }
            
            // Wait for all publishers
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Verify all events were published
            assert_eq!(publish_count.load(Ordering::SeqCst), 1000); // 20 * 50
            
            // Verify events were received (with timeout to prevent hanging)
            let mut received_count = 0;
            while let Ok(Some(_)) = timeout(Duration::from_millis(100), rx.recv()).await {
                received_count += 1;
                if received_count >= 1000 {
                    break;
                }
            }
            
            // Should have received all events
            assert_eq!(received_count, 1000);
        }
        
        #[tokio::test]
        async fn test_concurrent_rule_modifications() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            let rule_count = Arc::new(AtomicU64::new(0));
            
            // Spawn tasks that add rules concurrently
            let mut handles = vec![];
            for i in 0..10 {
                let bridge_clone = bridge.clone();
                let count_clone = rule_count.clone();
                
                handles.push(tokio::spawn(async move {
                    for j in 0..10 {
                        let rule = RoutingRule {
                            name: format!("rule_{i}_{j}"),
                            source_pattern: format!("source_{}", i),
                            event_pattern: "*".to_string(),
                            targets: vec![format!("target_{}", j)],
                            priority: ((i * 10 + j) % 100) as u32,
                            conditions: vec![],
                        };
                        
                        bridge_clone.add_rule(rule).await.unwrap();
                        count_clone.fetch_add(1, Ordering::SeqCst);
                        
                        // Also test route determination during modifications
                        let routes = bridge_clone.router
                            .determine_routes(&format!("source_{}", i), "TestEvent")
                            .await
                            .unwrap();
                        assert!(!routes.is_empty());
                    }
                }));
            }
            
            // Wait for all rule additions
            for handle in handles {
                handle.await.unwrap();
            }
            
            assert_eq!(rule_count.load(Ordering::SeqCst), 100);
            
            // Verify rules are sorted by priority
            let rules = bridge.router.rules.read().await;
            for i in 1..rules.len() {
                assert!(rules[i-1].priority >= rules[i].priority);
            }
        }
        
        #[tokio::test]
        async fn test_concurrent_transformer_and_filter_modifications() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            
            // Concurrent transformer additions
            let mut handles = vec![];
            for i in 0..5 {
                let bridge_clone = bridge.clone();
                
                handles.push(tokio::spawn(async move {
                    let transformer = Box::new(FieldMappingTransformer::new(
                        HashMap::from([(format!("field_{i}"), format!("mapped_{i}"))]),
                    ));
                    
                    bridge_clone.add_transformer(
                        format!("transformer_{}", i),
                        transformer,
                    ).await.unwrap();
                    
                    // Add filters concurrently
                    let filter = Box::new(PropertyFilter::include_matching(
                        format!("prop_{}", i),
                        serde_json::json!(i),
                    ));
                    
                    bridge_clone.add_filter(filter).await.unwrap();
                }));
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Verify all transformers and filters were added
            let transformers = bridge.transformers.read().await;
            assert_eq!(transformers.len(), 5);
            
            let filters = bridge.filters.read().await;
            assert_eq!(filters.len(), 5);
        }
        
        #[tokio::test]
        async fn test_concurrent_subscribe_unsubscribe_pattern() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            
            // Add routing rule
            bridge.add_rule(RoutingRule {
                name: "sub_test".to_string(),
                source_pattern: "*".to_string(),
                event_pattern: "*".to_string(),
                targets: vec!["domain_a".to_string(), "domain_b".to_string()],
                priority: 100,
                conditions: vec![],
            }).await.unwrap();
            
            // Simulate rapid subscribe/unsubscribe cycles
            let mut handles = vec![];
            for i in 0..10 {
                let bridge_clone = bridge.clone();
                
                handles.push(tokio::spawn(async move {
                    for j in 0..20 {
                        let (tx, _rx) = mpsc::channel(10);
                        let subscriber = EventSubscriber {
                            id: format!("sub_{i}_{j}"),
                            name: format!("Subscriber {i} {j}"),
                            patterns: vec!["*".to_string()],
                            channel: tx,
                        };
                        
                        let domain = if j % 2 == 0 { "domain_a" } else { "domain_b" };
                        bridge_clone.subscribe(domain.to_string(), subscriber).await.unwrap();
                        
                        // Small delay to simulate real usage
                        tokio::time::sleep(Duration::from_micros(10)).await;
                    }
                }));
            }
            
            for handle in handles {
                handle.await.unwrap();
            }
            
            // Verify subscribers
            let subscribers = bridge.subscribers.read().await;
            let domain_a_subs = subscribers.get("domain_a").unwrap().len();
            let domain_b_subs = subscribers.get("domain_b").unwrap().len();
            
            assert_eq!(domain_a_subs + domain_b_subs, 200);
        }
        
        #[tokio::test]
        async fn test_deadlock_prevention_nested_locks() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            
            // This tests potential deadlock scenarios when acquiring multiple locks
            let mut handles = vec![];
            
            // Task 1: Adds transformers while reading filters
            let bridge1 = bridge.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    // Read filters first
                    let _filters = bridge1.filters.read().await;
                    
                    // Then write to transformers
                    let transformer = Box::new(FieldMappingTransformer::new(HashMap::new()));
                    bridge1.add_transformer(format!("t1_{}", i), transformer).await.unwrap();
                }
            }));
            
            // Task 2: Adds filters while reading transformers
            let bridge2 = bridge.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    // Read transformers first
                    let _transformers = bridge2.transformers.read().await;
                    
                    // Then write to filters
                    let filter = Box::new(PropertyFilter::include_matching(
                        "test".to_string(),
                        serde_json::json!(i),
                    ));
                    bridge2.add_filter(filter).await.unwrap();
                }
            }));
            
            // Task 3: Routes events (reads multiple locks)
            let bridge3 = bridge.clone();
            handles.push(tokio::spawn(async move {
                for i in 0..50 {
                    #[derive(Debug)]
                    struct TestEvent;
                    impl DomainEvent for TestEvent {
                        fn subject(&self) -> String { "test".to_string() }
                        fn aggregate_id(&self) -> uuid::Uuid { uuid::Uuid::new_v4() }
                        fn event_type(&self) -> &'static str { "Test" }
                    }
                    
                    bridge3.publish(
                        Box::new(TestEvent),
                        format!("source_{}", i),
                    ).await.unwrap();
                }
            }));
            
            // All tasks should complete without deadlock
            let results = futures::future::join_all(handles).await;
            for result in results {
                assert!(result.is_ok());
            }
        }
        
        #[tokio::test]
        async fn test_high_throughput_event_storm() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig {
                buffer_size: 10000,
                event_ttl_seconds: 60,
                enable_dlq: true,
                max_retries: 1,
                retry_backoff_multiplier: 1.0,
            }));
            
            // Setup routing
            bridge.add_rule(RoutingRule {
                name: "storm_test".to_string(),
                source_pattern: "*".to_string(),
                event_pattern: "*".to_string(),
                targets: vec!["consumer".to_string()],
                priority: 100,
                conditions: vec![],
            }).await.unwrap();
            
            // Create multiple consumers
            let received_count = Arc::new(AtomicU64::new(0));
            let mut consumer_handles = vec![];
            
            for i in 0..5 {
                let (tx, mut rx) = mpsc::channel(2000);
                bridge.subscribe("consumer".to_string(), EventSubscriber {
                    id: format!("consumer_{}", i),
                    name: format!("Consumer {}", i),
                    patterns: vec!["*".to_string()],
                    channel: tx,
                }).await.unwrap();
                
                let count_clone = received_count.clone();
                consumer_handles.push(tokio::spawn(async move {
                    while (rx.recv().await).is_some() {
                        count_clone.fetch_add(1, Ordering::SeqCst);
                    }
                }));
            }
            
            // Generate event storm
            let start = tokio::time::Instant::now();
            let mut publisher_handles = vec![];
            
            for i in 0..10 {
                let bridge_clone = bridge.clone();
                publisher_handles.push(tokio::spawn(async move {
                    for j in 0..100 {
                        #[derive(Debug)]
                        struct StormEvent { id: usize }
                        impl DomainEvent for StormEvent {
                            fn subject(&self) -> String { "storm.event".to_string() }
                            fn aggregate_id(&self) -> uuid::Uuid { uuid::Uuid::new_v4() }
                            fn event_type(&self) -> &'static str { "Storm" }
                        }
                        
                        bridge_clone.publish(
                            Box::new(StormEvent { id: i * 100 + j }),
                            format!("storm_{}", i),
                        ).await.unwrap();
                    }
                }));
            }
            
            // Wait for publishers
            for handle in publisher_handles {
                handle.await.unwrap();
            }
            
            let publish_duration = start.elapsed();
            println!("Published 1000 events in {:?}", publish_duration);
            
            // Give consumers time to process
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Verify throughput
            let received = received_count.load(Ordering::SeqCst);
            println!("Received {} events across 5 consumers", received);
            
            // Each consumer should receive each event, so 5000 total
            assert_eq!(received, 5000);
            assert!(publish_duration.as_millis() < 5000); // Should be fast
        }
        
        #[tokio::test]
        async fn test_channel_backpressure_handling() {
            let bridge = Arc::new(EventBridge::new(BridgeConfig::default()));
            
            // Add routing
            bridge.add_rule(RoutingRule {
                name: "backpressure_test".to_string(),
                source_pattern: "*".to_string(),
                event_pattern: "*".to_string(),
                targets: vec!["slow_consumer".to_string()],
                priority: 100,
                conditions: vec![],
            }).await.unwrap();
            
            // Create a slow consumer with small buffer
            let (tx, mut rx) = mpsc::channel(5); // Very small buffer
            bridge.subscribe("slow_consumer".to_string(), EventSubscriber {
                id: "slow".to_string(),
                name: "Slow Consumer".to_string(),
                patterns: vec!["*".to_string()],
                channel: tx,
            }).await.unwrap();
            
            // Publish many events quickly
            let mut publish_handles = vec![];
            for i in 0..100 {
                let bridge_clone = bridge.clone();
                publish_handles.push(tokio::spawn(async move {
                    #[derive(Debug)]
                    struct BackpressureEvent { id: usize }
                    impl DomainEvent for BackpressureEvent {
                        fn subject(&self) -> String { "test".to_string() }
                        fn aggregate_id(&self) -> uuid::Uuid { uuid::Uuid::new_v4() }
                        fn event_type(&self) -> &'static str { "Backpressure" }
                    }
                    
                    bridge_clone.publish(
                        Box::new(BackpressureEvent { id: i }),
                        "publisher".to_string(),
                    ).await.unwrap();
                }));
            }
            
            // Publishers should complete despite slow consumer
            for handle in publish_handles {
                handle.await.unwrap();
            }
            
            // Consume some events
            let mut consumed = 0;
            while let Ok(Some(_)) = timeout(Duration::from_millis(50), rx.recv()).await {
                consumed += 1;
                if consumed >= 5 {
                    break;
                }
            }
            
            // Should have consumed at least buffer size
            assert!(consumed >= 5);
        }
    }
}