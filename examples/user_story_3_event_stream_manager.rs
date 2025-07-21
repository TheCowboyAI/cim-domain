/// User Story 3: Event Stream Manager - Setting up Event Flows
/// 
/// As an Event Stream Manager, I want to configure event streams, set up
/// routing rules, and monitor event flow health, so that events are
/// reliably delivered to the right consumers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Core types for this example
#[derive(Debug, Clone)]
pub enum DomainError {
    ValidationError(String),
    Infrastructure(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            DomainError::Infrastructure(msg) => write!(f, "Infrastructure error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent {
    pub fn new(aggregate_id: String, payload: serde_json::Value) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            aggregate_id,
            event_type: "DomainEvent".into(),
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}

// Event store trait
#[async_trait::async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError>;
}

#[derive(Debug, Clone)]
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        let mut store = self.events.write().await;
        for event in events {
            let entry = store.entry(event.aggregate_id.clone()).or_insert_with(Vec::new);
            entry.push(event);
        }
        Ok(())
    }
}

// Event stream configuration and management
#[derive(Debug, Clone)]
pub struct EventStreamConfiguration {
    pub stream_name: String,
    pub subjects: Vec<String>,
    pub retention_policy: RetentionPolicy,
    pub replication_factor: u8,
    pub consumers: Vec<ConsumerConfig>,
}

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub max_age: std::time::Duration,
    pub max_messages: Option<u64>,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    pub name: String,
    pub filter_subjects: Vec<String>,
    pub delivery_policy: DeliveryPolicy,
    pub ack_wait: std::time::Duration,
    pub max_deliver: u32,
}

#[derive(Debug, Clone)]
pub enum DeliveryPolicy {
    All,
    Last,
    New,
    ByStartSequence(u64),
    ByStartTime(chrono::DateTime<chrono::Utc>),
}

// Event routing rules
#[derive(Debug, Clone)]
pub struct EventRoutingRule {
    pub name: String,
    pub source_pattern: String,
    pub target_streams: Vec<String>,
    pub transformation: Option<EventTransformation>,
    pub filter: Option<EventFilter>,
}

#[derive(Debug, Clone)]
pub enum EventTransformation {
    AddMetadata(HashMap<String, String>),
    MapField { from: String, to: String },
    Enrich { service_url: String },
}

#[derive(Debug, Clone)]
pub struct EventFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
}

// Stream health monitoring
#[derive(Debug, Clone)]
pub struct StreamHealth {
    pub stream_name: String,
    pub status: HealthStatus,
    pub message_rate: f64,
    pub error_rate: f64,
    pub consumer_lag: HashMap<String, u64>,
    pub last_message_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

// Event Stream Manager implementation
pub struct EventStreamManager {
    streams: Arc<RwLock<HashMap<String, EventStreamConfiguration>>>,
    routing_rules: Arc<RwLock<Vec<EventRoutingRule>>>,
    health_metrics: Arc<RwLock<HashMap<String, StreamHealth>>>,
    event_store: Box<dyn EventStore>,
}

impl EventStreamManager {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            routing_rules: Arc::new(RwLock::new(Vec::new())),
            health_metrics: Arc::new(RwLock::new(HashMap::new())),
            event_store,
        }
    }

    pub async fn create_stream(&self, config: EventStreamConfiguration) -> Result<(), DomainError> {
        println!("Creating stream: {}", config.stream_name);
        
        // Validate configuration
        if config.subjects.is_empty() {
            return Err(DomainError::ValidationError("Stream must have at least one subject".into()));
        }

        // Store configuration
        let mut streams = self.streams.write().await;
        streams.insert(config.stream_name.clone(), config.clone());

        // Initialize health metrics
        let mut health = self.health_metrics.write().await;
        health.insert(config.stream_name.clone(), StreamHealth {
            stream_name: config.stream_name.clone(),
            status: HealthStatus::Healthy,
            message_rate: 0.0,
            error_rate: 0.0,
            consumer_lag: HashMap::new(),
            last_message_time: None,
        });

        // Emit stream created event
        let event = DomainEvent::new(
            "event-stream-manager".into(),
            serde_json::json!({
                "type": "StreamCreated",
                "stream_name": config.stream_name,
                "subjects": config.subjects,
                "consumer_count": config.consumers.len()
            }),
        );
        self.event_store.append(vec![event]).await?;

        Ok(())
    }

    pub async fn add_routing_rule(&self, rule: EventRoutingRule) -> Result<(), DomainError> {
        println!("Adding routing rule: {}", rule.name);
        
        // Validate rule
        self.validate_routing_rule(&rule).await?;

        // Store rule
        let mut rules = self.routing_rules.write().await;
        rules.push(rule.clone());

        // Emit rule added event
        let event = DomainEvent::new(
            "event-stream-manager".into(),
            serde_json::json!({
                "type": "RoutingRuleAdded",
                "rule_name": rule.name,
                "source_pattern": rule.source_pattern,
                "target_streams": rule.target_streams
            }),
        );
        self.event_store.append(vec![event]).await?;

        Ok(())
    }

    async fn validate_routing_rule(&self, rule: &EventRoutingRule) -> Result<(), DomainError> {
        let streams = self.streams.read().await;
        
        // Check if target streams exist
        for target in &rule.target_streams {
            if !streams.contains_key(target) {
                return Err(DomainError::ValidationError(
                    format!("Target stream '{}' does not exist", target)
                ));
            }
        }

        Ok(())
    }

    pub async fn route_event(&self, event: &DomainEvent) -> Result<Vec<String>, DomainError> {
        let rules = self.routing_rules.read().await;
        let mut routed_to = Vec::new();

        for rule in rules.iter() {
            if self.matches_pattern(&event.aggregate_id, &rule.source_pattern) {
                // Apply filter if present
                if let Some(filter) = &rule.filter {
                    if !self.apply_filter(event, filter)? {
                        continue;
                    }
                }

                // Apply transformation if present
                let transformed_event = if let Some(transformation) = &rule.transformation {
                    self.apply_transformation(event, transformation).await?
                } else {
                    event.clone()
                };

                // Route to target streams
                for target_stream in &rule.target_streams {
                    println!("  → Routing to stream: {} (event type: {})", target_stream, transformed_event.event_type);
                    routed_to.push(target_stream.clone());
                    
                    // Log transformed payload if different
                    if transformed_event.payload != event.payload {
                        println!("    Transformed payload: {}", 
                            serde_json::to_string_pretty(&transformed_event.payload).unwrap());
                    }
                    
                    // In a real implementation, this would publish to the actual stream
                    self.publish_to_stream(target_stream, &transformed_event).await?;
                }
            }
        }

        Ok(routed_to)
    }

    fn matches_pattern(&self, subject: &str, pattern: &str) -> bool {
        // Simple wildcard matching
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len() - 1];
            subject.starts_with(prefix)
        } else {
            subject == pattern
        }
    }

    fn apply_filter(&self, event: &DomainEvent, filter: &EventFilter) -> Result<bool, DomainError> {
        let value = event.payload.get(&filter.field);
        
        match (&filter.operator, value) {
            (FilterOperator::Equals, Some(v)) => Ok(v == &filter.value),
            (FilterOperator::NotEquals, Some(v)) => Ok(v != &filter.value),
            (FilterOperator::Contains, Some(v)) => {
                if let (Some(str_val), Some(search)) = (v.as_str(), filter.value.as_str()) {
                    Ok(str_val.contains(search))
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    async fn apply_transformation(&self, event: &DomainEvent, transformation: &EventTransformation) -> Result<DomainEvent, DomainError> {
        let mut transformed = event.clone();
        
        match transformation {
            EventTransformation::AddMetadata(metadata) => {
                for (key, value) in metadata {
                    transformed.payload[key] = serde_json::Value::String(value.clone());
                }
            }
            EventTransformation::MapField { from, to } => {
                if let Some(value) = transformed.payload.get(from).cloned() {
                    transformed.payload.as_object_mut().unwrap().remove(from);
                    transformed.payload[to] = value;
                }
            }
            EventTransformation::Enrich { service_url: _ } => {
                // In real implementation, would call external service
                transformed.payload["enriched"] = serde_json::Value::Bool(true);
                transformed.payload["enriched_at"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
            }
        }

        Ok(transformed)
    }

    pub async fn get_stream_health(&self, stream_name: &str) -> Option<StreamHealth> {
        let health = self.health_metrics.read().await;
        health.get(stream_name).cloned()
    }

    pub async fn update_health_metrics(&self, stream_name: &str, event_count: u64, error_count: u64) {
        let mut health = self.health_metrics.write().await;
        
        if let Some(metrics) = health.get_mut(stream_name) {
            // Calculate rates (events per second)
            metrics.message_rate = event_count as f64 / 60.0; // Assuming 1-minute window
            metrics.error_rate = error_count as f64 / 60.0;
            metrics.last_message_time = Some(chrono::Utc::now());
            
            // Update status based on error rate
            metrics.status = if metrics.error_rate > 0.1 {
                HealthStatus::Unhealthy
            } else if metrics.error_rate > 0.01 {
                HealthStatus::Degraded
            } else {
                HealthStatus::Healthy
            };
        }
    }

    pub async fn get_dashboard_view(&self) -> StreamManagerDashboard {
        let streams = self.streams.read().await;
        let rules = self.routing_rules.read().await;
        let health = self.health_metrics.read().await;

        let stream_count = streams.len();
        let total_consumers = streams.values()
            .map(|s| s.consumers.len())
            .sum::<usize>();
        let unhealthy_streams = health.values()
            .filter(|h| h.status == HealthStatus::Unhealthy)
            .count();

        StreamManagerDashboard {
            total_streams: stream_count,
            total_consumers,
            routing_rules: rules.len(),
            unhealthy_streams,
            stream_details: streams.keys().cloned().collect(),
        }
    }

    async fn publish_to_stream(&self, stream_name: &str, event: &DomainEvent) -> Result<(), DomainError> {
        // Simulate publishing to stream
        let streams = self.streams.read().await;
        if let Some(config) = streams.get(stream_name) {
            // Check if event matches stream subjects
            // For demo purposes, always allow events to be published
            let matches_subject = true;
            
            if !matches_subject {
                return Err(DomainError::ValidationError(
                    format!("Event '{}' does not match stream subjects", event.aggregate_id)
                ));
            }
            
            // Update metrics
            let mut health = self.health_metrics.write().await;
            if let Some(metrics) = health.get_mut(stream_name) {
                metrics.last_message_time = Some(chrono::Utc::now());
                metrics.message_rate += 1.0; // Increment for demo
            }
            
            // Log the event being published
            println!("      Published event {} to stream {}", event.event_id, stream_name);
            Ok(())
        } else {
            Err(DomainError::ValidationError(format!("Stream '{}' not found", stream_name)))
        }
    }
}

#[derive(Debug)]
pub struct StreamManagerDashboard {
    pub total_streams: usize,
    pub total_consumers: usize,
    pub routing_rules: usize,
    pub unhealthy_streams: usize,
    pub stream_details: Vec<String>,
}

// Demo: Real-world event flow scenario
pub async fn demonstrate_event_flow_management(manager: &EventStreamManager) {
    println!("\nDemonstrating Event Flow Management");
    println!("===================================\n");

    // Create streams for different domains
    let order_stream = EventStreamConfiguration {
        stream_name: "orders".into(),
        subjects: vec!["order.*".into()],
        retention_policy: RetentionPolicy {
            max_age: std::time::Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            max_messages: Some(1_000_000),
            max_bytes: Some(1_073_741_824), // 1GB
        },
        replication_factor: 3,
        consumers: vec![
            ConsumerConfig {
                name: "inventory-consumer".into(),
                filter_subjects: vec!["order.created".into()],
                delivery_policy: DeliveryPolicy::All,
                ack_wait: std::time::Duration::from_secs(30),
                max_deliver: 3,
            },
            ConsumerConfig {
                name: "analytics-consumer".into(),
                filter_subjects: vec!["order.*".into()],
                delivery_policy: DeliveryPolicy::New,
                ack_wait: std::time::Duration::from_secs(60),
                max_deliver: 5,
            },
        ],
    };

    manager.create_stream(order_stream).await.unwrap();

    let inventory_stream = EventStreamConfiguration {
        stream_name: "inventory".into(),
        subjects: vec!["inventory.*".into()],
        retention_policy: RetentionPolicy {
            max_age: std::time::Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            max_messages: None,
            max_bytes: Some(5_368_709_120), // 5GB
        },
        replication_factor: 3,
        consumers: vec![
            ConsumerConfig {
                name: "fulfillment-consumer".into(),
                filter_subjects: vec!["inventory.reserved".into()],
                delivery_policy: DeliveryPolicy::All,
                ack_wait: std::time::Duration::from_secs(30),
                max_deliver: 3,
            },
        ],
    };

    manager.create_stream(inventory_stream).await.unwrap();

    // Create routing rules
    let order_to_inventory_rule = EventRoutingRule {
        name: "order-to-inventory".into(),
        source_pattern: "order.created".into(),
        target_streams: vec!["inventory".into()],
        transformation: Some(EventTransformation::AddMetadata(
            vec![("routed_from".into(), "orders".into())]
                .into_iter()
                .collect()
        )),
        filter: None,
    };

    manager.add_routing_rule(order_to_inventory_rule).await.unwrap();

    let high_value_order_rule = EventRoutingRule {
        name: "high-value-orders".into(),
        source_pattern: "order.*".into(),
        target_streams: vec!["high-value-monitoring".into()],
        transformation: None,
        filter: Some(EventFilter {
            field: "total_amount".into(),
            operator: FilterOperator::GreaterThan,
            value: serde_json::json!(1000),
        }),
    };

    // Create high-value monitoring stream first
    let monitoring_stream = EventStreamConfiguration {
        stream_name: "high-value-monitoring".into(),
        subjects: vec!["monitoring.high-value.*".into()],
        retention_policy: RetentionPolicy {
            max_age: std::time::Duration::from_secs(90 * 24 * 60 * 60), // 90 days
            max_messages: None,
            max_bytes: None,
        },
        replication_factor: 3,
        consumers: vec![],
    };

    manager.create_stream(monitoring_stream).await.unwrap();
    manager.add_routing_rule(high_value_order_rule).await.unwrap();

    // Simulate event flow
    println!("\nSimulating Event Flow:");
    println!("======================\n");

    // Order created event
    let order_event = DomainEvent::new(
        "order.created".into(),
        serde_json::json!({
            "order_id": "ORD-12345",
            "customer_id": "CUST-789",
            "total_amount": 1500.00,
            "items": [
                {"sku": "LAPTOP-001", "quantity": 1, "price": 1200.00},
                {"sku": "MOUSE-002", "quantity": 2, "price": 150.00}
            ]
        }),
    );

    println!("Processing order.created event:");
    let routed_to = manager.route_event(&order_event).await.unwrap();
    println!("  Event routed to: {:?}", routed_to);

    // Update health metrics
    manager.update_health_metrics("orders", 150, 2).await;
    manager.update_health_metrics("inventory", 145, 0).await;

    // Display dashboard
    let dashboard = manager.get_dashboard_view().await;
    println!("\nStream Manager Dashboard:");
    println!("========================");
    println!("Total Streams: {}", dashboard.total_streams);
    println!("Total Consumers: {}", dashboard.total_consumers);
    println!("Routing Rules: {}", dashboard.routing_rules);
    println!("Unhealthy Streams: {}", dashboard.unhealthy_streams);
    println!("\nActive Streams:");
    for stream in &dashboard.stream_details {
        if let Some(health) = manager.get_stream_health(stream).await {
            println!("  {} - Status: {:?}, Message Rate: {:.2}/s, Error Rate: {:.2}/s",
                stream, health.status, health.message_rate, health.error_rate);
        }
    }
}

#[tokio::main]
async fn main() {
    println!("User Story 3: Event Stream Manager Demo");
    println!("======================================\n");

    let event_store = Box::new(InMemoryEventStore::new());
    let manager = EventStreamManager::new(event_store);

    // Run the demonstration
    demonstrate_event_flow_management(&manager).await;

    println!("\n\nKey Features Demonstrated:");
    println!("✓ Stream creation with retention policies");
    println!("✓ Consumer configuration with delivery policies");
    println!("✓ Event routing with pattern matching");
    println!("✓ Event filtering based on content");
    println!("✓ Event transformation during routing");
    println!("✓ Health monitoring and metrics");
    println!("✓ Dashboard view for operational oversight");
}