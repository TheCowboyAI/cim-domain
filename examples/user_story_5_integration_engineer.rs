/// User Story 5: Integration Engineer - Cross-domain Communication
/// 
/// As an Integration Engineer, I want to set up reliable communication
/// between different domains and external systems, so that data flows
/// seamlessly while maintaining domain boundaries.

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
pub struct InMemoryEventStore;

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, _events: Vec<DomainEvent>) -> Result<(), DomainError> {
        Ok(())
    }
}

// Integration patterns
#[derive(Debug, Clone)]
pub enum IntegrationPattern {
    EventBridge {
        source_domain: String,
        target_domain: String,
        event_mapping: EventMapping,
    },
    ApiGateway {
        domain: String,
        endpoints: Vec<ApiEndpoint>,
        rate_limit: RateLimit,
    },
    MessageTranslator {
        from_format: MessageFormat,
        to_format: MessageFormat,
        transformation_rules: Vec<TransformationRule>,
    },
    Aggregator {
        sources: Vec<String>,
        correlation_id: String,
        timeout: std::time::Duration,
    },
}

#[derive(Debug, Clone)]
pub struct EventMapping {
    pub source_events: Vec<String>,
    pub target_event: String,
    pub field_mappings: HashMap<String, String>,
    pub enrichments: Vec<DataEnrichment>,
}

#[derive(Debug, Clone)]
pub struct ApiEndpoint {
    pub path: String,
    pub method: HttpMethod,
    pub domain_command: String,
    pub auth_required: bool,
}

#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

#[derive(Debug, Clone)]
pub enum MessageFormat {
    Json,
    Xml,
    Protobuf,
    Avro,
}

#[derive(Debug, Clone)]
pub struct TransformationRule {
    pub rule_type: TransformationType,
    pub source_path: String,
    pub target_path: String,
}

#[derive(Debug, Clone)]
pub enum TransformationType {
    Copy,
    Convert { from_type: String, to_type: String },
    Compute { expression: String },
    Lookup { table: String, key: String },
}

#[derive(Debug, Clone)]
pub struct DataEnrichment {
    pub field: String,
    pub source: EnrichmentSource,
}

#[derive(Debug, Clone)]
pub enum EnrichmentSource {
    StaticValue(serde_json::Value),
    DatabaseLookup { table: String, key: String },
    ServiceCall { url: String, cache_ttl: std::time::Duration },
}

// Integration infrastructure
pub struct IntegrationHub {
    patterns: Arc<RwLock<Vec<IntegrationPattern>>>,
    connections: Arc<RwLock<HashMap<String, ConnectionStatus>>>,
    message_queue: Arc<RwLock<Vec<QueuedMessage>>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    event_store: Box<dyn EventStore>,
}

#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub endpoint: String,
    pub status: ConnectionState,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub error_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Degraded,
}

#[derive(Debug, Clone)]
pub struct QueuedMessage {
    pub id: String,
    pub source: String,
    pub target: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub service: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<chrono::DateTime<chrono::Utc>>,
    pub config: CircuitBreakerConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: std::time::Duration,
}

impl IntegrationHub {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(Vec::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            event_store,
        }
    }

    pub async fn register_integration(&self, pattern: IntegrationPattern) -> Result<(), DomainError> {
        println!("Registering integration pattern: {:?}", pattern);
        
        // Initialize circuit breakers for new connections
        match &pattern {
            IntegrationPattern::EventBridge { target_domain, .. } => {
                self.init_circuit_breaker(target_domain).await?;
            }
            IntegrationPattern::ApiGateway { domain, .. } => {
                self.init_circuit_breaker(domain).await?;
            }
            _ => {}
        }

        let mut patterns = self.patterns.write().await;
        patterns.push(pattern.clone());

        // Emit integration registered event
        let event = DomainEvent::new(
            "integration-hub".into(),
            serde_json::json!({
                "type": "IntegrationRegistered",
                "pattern": format!("{:?}", pattern)
            }),
        );
        self.event_store.append(vec![event]).await?;

        Ok(())
    }

    async fn init_circuit_breaker(&self, service: &str) -> Result<(), DomainError> {
        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(service.to_string(), CircuitBreaker {
            service: service.to_string(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            config: CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 3,
                timeout: std::time::Duration::from_secs(60),
            },
        });
        Ok(())
    }

    pub async fn process_cross_domain_event(&self, event: &DomainEvent) -> Result<(), DomainError> {
        let patterns = self.patterns.read().await;
        
        for pattern in patterns.iter() {
            match pattern {
                IntegrationPattern::EventBridge { source_domain, target_domain, event_mapping } => {
                    if event.aggregate_id.starts_with(source_domain) {
                        self.bridge_event(event, target_domain, event_mapping).await?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn bridge_event(&self, event: &DomainEvent, target_domain: &str, mapping: &EventMapping) -> Result<(), DomainError> {
        // Check circuit breaker
        if !self.is_service_available(target_domain).await? {
            return self.queue_for_retry(event, target_domain).await;
        }

        println!("Bridging event from {} to {}", event.aggregate_id, target_domain);

        // Transform event according to mapping
        let mut transformed_payload = serde_json::json!({});
        
        // Apply field mappings
        for (source_field, target_field) in &mapping.field_mappings {
            if let Some(value) = event.payload.get(source_field) {
                transformed_payload[target_field] = value.clone();
            }
        }

        // Apply enrichments
        for enrichment in &mapping.enrichments {
            let enriched_value = self.enrich_data(&enrichment).await?;
            transformed_payload[&enrichment.field] = enriched_value;
        }

        // Create target event
        let target_event = DomainEvent::new(
            mapping.target_event.clone(),
            transformed_payload,
        );

        // Simulate sending to target domain
        self.send_to_domain(target_domain, &target_event).await?;

        // Update circuit breaker on success
        self.record_success(target_domain).await?;

        Ok(())
    }

    async fn is_service_available(&self, service: &str) -> Result<bool, DomainError> {
        let breakers = self.circuit_breakers.read().await;
        
        if let Some(breaker) = breakers.get(service) {
            match breaker.state {
                CircuitState::Open => {
                    // Check if timeout has passed
                    if let Some(last_failure) = breaker.last_failure {
                        let elapsed = chrono::Utc::now() - last_failure;
                        if elapsed > chrono::Duration::from_std(breaker.config.timeout).unwrap() {
                            // Transition to half-open
                            drop(breakers);
                            self.transition_circuit_breaker(service, CircuitState::HalfOpen).await?;
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                CircuitState::Closed | CircuitState::HalfOpen => Ok(true),
            }
        } else {
            Ok(true)
        }
    }

    async fn transition_circuit_breaker(&self, service: &str, new_state: CircuitState) -> Result<(), DomainError> {
        let mut breakers = self.circuit_breakers.write().await;
        
        if let Some(breaker) = breakers.get_mut(service) {
            println!("Circuit breaker for {} transitioning from {:?} to {:?}", 
                service, breaker.state, new_state);
            breaker.state = new_state;
        }

        Ok(())
    }

    async fn record_success(&self, service: &str) -> Result<(), DomainError> {
        let mut breakers = self.circuit_breakers.write().await;
        
        if let Some(breaker) = breakers.get_mut(service) {
            breaker.success_count += 1;
            
            match breaker.state {
                CircuitState::HalfOpen => {
                    if breaker.success_count >= breaker.config.success_threshold {
                        breaker.state = CircuitState::Closed;
                        breaker.failure_count = 0;
                        breaker.success_count = 0;
                        println!("Circuit breaker for {} is now CLOSED", service);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn record_failure(&self, service: &str) -> Result<(), DomainError> {
        let mut breakers = self.circuit_breakers.write().await;
        
        if let Some(breaker) = breakers.get_mut(service) {
            breaker.failure_count += 1;
            breaker.last_failure = Some(chrono::Utc::now());
            
            match breaker.state {
                CircuitState::Closed => {
                    if breaker.failure_count >= breaker.config.failure_threshold {
                        breaker.state = CircuitState::Open;
                        println!("Circuit breaker for {} is now OPEN", service);
                    }
                }
                CircuitState::HalfOpen => {
                    breaker.state = CircuitState::Open;
                    breaker.success_count = 0;
                    println!("Circuit breaker for {} is now OPEN (from half-open)", service);
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn queue_for_retry(&self, event: &DomainEvent, target: &str) -> Result<(), DomainError> {
        println!("Queueing message for retry to {}", target);
        
        let mut queue = self.message_queue.write().await;
        queue.push(QueuedMessage {
            id: uuid::Uuid::new_v4().to_string(),
            source: event.aggregate_id.clone(),
            target: target.to_string(),
            payload: event.payload.clone(),
            timestamp: chrono::Utc::now(),
            retry_count: 0,
        });

        Ok(())
    }

    async fn enrich_data(&self, enrichment: &DataEnrichment) -> Result<serde_json::Value, DomainError> {
        match &enrichment.source {
            EnrichmentSource::StaticValue(value) => Ok(value.clone()),
            EnrichmentSource::DatabaseLookup { table, key } => {
                // Simulate database lookup
                Ok(serde_json::json!({
                    "enriched": true,
                    "source": "database",
                    "table": table,
                    "key": key
                }))
            }
            EnrichmentSource::ServiceCall { url, cache_ttl } => {
                // Simulate service call
                Ok(serde_json::json!({
                    "enriched": true,
                    "source": "service",
                    "url": url,
                    "cached": cache_ttl.as_secs() > 0
                }))
            }
        }
    }

    async fn send_to_domain(&self, domain: &str, event: &DomainEvent) -> Result<(), DomainError> {
        println!("  → Sending to {}: {}", domain, serde_json::to_string_pretty(&event.payload).unwrap());
        
        // Simulate potential failure
        let should_fail = rand::random::<f32>() < 0.2; // 20% failure rate for demo
        
        if should_fail {
            self.record_failure(domain).await?;
            return Err(DomainError::Infrastructure("Simulated connection failure".into()));
        }

        Ok(())
    }

    pub async fn process_retry_queue(&self) -> Result<(), DomainError> {
        let mut queue = self.message_queue.write().await;
        let mut processed = Vec::new();

        for (index, message) in queue.iter_mut().enumerate() {
            if self.is_service_available(&message.target).await? {
                println!("Retrying message {} to {}", message.id, message.target);
                
                // Attempt to resend
                let event = DomainEvent::new(
                    message.source.clone(),
                    message.payload.clone(),
                );
                
                match self.send_to_domain(&message.target, &event).await {
                    Ok(_) => {
                        processed.push(index);
                        self.record_success(&message.target).await?;
                    }
                    Err(_) => {
                        message.retry_count += 1;
                        if message.retry_count > 3 {
                            println!("Message {} exceeded retry limit, moving to DLQ", message.id);
                            processed.push(index);
                        }
                    }
                }
            }
        }

        // Remove processed messages
        for index in processed.iter().rev() {
            queue.remove(*index);
        }

        Ok(())
    }

    pub async fn get_integration_health(&self) -> IntegrationHealthReport {
        let patterns = self.patterns.read().await;
        let connections = self.connections.read().await;
        let breakers = self.circuit_breakers.read().await;
        let queue = self.message_queue.read().await;

        let healthy_connections = connections.values()
            .filter(|c| c.status == ConnectionState::Connected)
            .count();

        let open_circuits = breakers.values()
            .filter(|b| b.state == CircuitState::Open)
            .count();

        IntegrationHealthReport {
            total_integrations: patterns.len(),
            healthy_connections,
            total_connections: connections.len(),
            open_circuit_breakers: open_circuits,
            messages_in_queue: queue.len(),
            circuit_breaker_details: breakers.values().cloned().collect(),
        }
    }
}

#[derive(Debug)]
pub struct IntegrationHealthReport {
    pub total_integrations: usize,
    pub healthy_connections: usize,
    pub total_connections: usize,
    pub open_circuit_breakers: usize,
    pub messages_in_queue: usize,
    pub circuit_breaker_details: Vec<CircuitBreaker>,
}

// Demo scenario: E-commerce to fulfillment integration
pub async fn demonstrate_integration_scenarios(hub: &IntegrationHub) {
    println!("\nDemonstrating Integration Scenarios");
    println!("===================================\n");

    // Scenario 1: Order to Fulfillment Event Bridge
    let order_to_fulfillment = IntegrationPattern::EventBridge {
        source_domain: "order".into(),
        target_domain: "fulfillment".into(),
        event_mapping: EventMapping {
            source_events: vec!["order.confirmed".into()],
            target_event: "fulfillment.requested".into(),
            field_mappings: vec![
                ("order_id".into(), "reference_id".into()),
                ("customer_id".into(), "recipient_id".into()),
                ("shipping_address".into(), "delivery_address".into()),
            ].into_iter().collect(),
            enrichments: vec![
                DataEnrichment {
                    field: "priority".into(),
                    source: EnrichmentSource::StaticValue(serde_json::json!("standard")),
                },
                DataEnrichment {
                    field: "warehouse".into(),
                    source: EnrichmentSource::DatabaseLookup {
                        table: "warehouses".into(),
                        key: "postal_code".into(),
                    },
                },
            ],
        },
    };

    hub.register_integration(order_to_fulfillment).await.unwrap();

    // Scenario 2: API Gateway for External Partners
    let partner_api = IntegrationPattern::ApiGateway {
        domain: "inventory".into(),
        endpoints: vec![
            ApiEndpoint {
                path: "/api/v1/stock/check".into(),
                method: HttpMethod::Get,
                domain_command: "CheckStock".into(),
                auth_required: true,
            },
            ApiEndpoint {
                path: "/api/v1/stock/reserve".into(),
                method: HttpMethod::Post,
                domain_command: "ReserveStock".into(),
                auth_required: true,
            },
        ],
        rate_limit: RateLimit {
            requests_per_minute: 100,
            burst_size: 20,
        },
    };

    hub.register_integration(partner_api).await.unwrap();

    // Process some events
    println!("Processing cross-domain events:");
    println!("================================\n");

    let events = vec![
        DomainEvent::new(
            "order.confirmed".into(),
            serde_json::json!({
                "order_id": "ORD-12345",
                "customer_id": "CUST-789",
                "shipping_address": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "postal_code": "12345"
                },
                "items": [
                    {"sku": "WIDGET-001", "quantity": 2},
                    {"sku": "GADGET-002", "quantity": 1}
                ]
            }),
        ),
        DomainEvent::new(
            "order.confirmed".into(),
            serde_json::json!({
                "order_id": "ORD-12346",
                "customer_id": "CUST-790",
                "shipping_address": {
                    "street": "456 Elm St",
                    "city": "Riverside",
                    "postal_code": "67890"
                }
            }),
        ),
    ];

    for event in &events {
        match hub.process_cross_domain_event(event).await {
            Ok(_) => println!("✓ Event processed successfully"),
            Err(e) => println!("✗ Event processing failed: {}", e),
        }
    }

    // Process retry queue
    println!("\nProcessing retry queue:");
    hub.process_retry_queue().await.unwrap();

    // Display integration health
    let health = hub.get_integration_health().await;
    println!("\nIntegration Health Report:");
    println!("=========================");
    println!("Total Integrations: {}", health.total_integrations);
    println!("Healthy Connections: {}/{}", health.healthy_connections, health.total_connections);
    println!("Open Circuit Breakers: {}", health.open_circuit_breakers);
    println!("Messages in Queue: {}", health.messages_in_queue);
    
    println!("\nCircuit Breaker Status:");
    for breaker in &health.circuit_breaker_details {
        println!("  {} - State: {:?}, Failures: {}, Successes: {}", 
            breaker.service, breaker.state, breaker.failure_count, breaker.success_count);
    }
}

#[tokio::main]
async fn main() {
    println!("User Story 5: Integration Engineer Demo");
    println!("======================================\n");

    let event_store = Box::new(InMemoryEventStore::new());
    let hub = IntegrationHub::new(event_store);

    // Run the demonstration
    demonstrate_integration_scenarios(&hub).await;

    println!("\n\nKey Features Demonstrated:");
    println!("✓ Event bridge pattern for domain integration");
    println!("✓ API gateway for external access");
    println!("✓ Message transformation and enrichment");
    println!("✓ Circuit breaker pattern for resilience");
    println!("✓ Retry queue for failed messages");
    println!("✓ Health monitoring and reporting");
    println!("✓ Rate limiting for API endpoints");
}