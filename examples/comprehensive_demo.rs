/// Comprehensive Demo: E-Commerce Platform
/// 
/// This demo combines all user stories to build a complete e-commerce
/// platform using domain-driven design and event sourcing.

use std::sync::Arc;
use tokio::sync::RwLock;

// Core types for this example
#[derive(Debug, Clone)]
pub enum DomainError {
    ValidationError(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

pub type DomainContext = String;

// Import all user story implementations
mod user_story_1_component_developer;
mod user_story_2_system_architect;
mod user_story_3_event_stream_manager;
mod user_story_4_data_analyst;
mod user_story_5_integration_engineer;

use user_story_1_component_developer::{
    SearchWidget, UserInput
};
use user_story_2_system_architect::{
    ECommerceDomainArchitecture, OrderFulfillmentSaga
};
use user_story_3_event_stream_manager::{
    EventStreamManager, EventStreamConfiguration, RetentionPolicy, ConsumerConfig,
    DeliveryPolicy, EventRoutingRule
};
use user_story_4_data_analyst::{
    DataAnalystWorkspace, ProjectionDefinition, ProjectionHandler, QueryModel,
    FieldDefinition as DataFieldDefinition, FieldType, AggregationDefinition,
    AggregationType, AnalyticsQuery, IndexDefinition
};
use user_story_5_integration_engineer::{
    IntegrationHub, IntegrationPattern, EventMapping, ApiEndpoint, HttpMethod,
    RateLimit
};

// Main e-commerce platform
pub struct ECommercePlatform {
    // Core infrastructure
    event_store: Arc<RwLock<Box<dyn EventStore>>>,
    
    // User Story 1: UI Components
    search_widget: Arc<RwLock<SearchWidget>>,
    
    // User Story 2: Domain Architecture
    domain_architecture: Arc<ECommerceDomainArchitecture>,
    
    // User Story 3: Event Streams
    event_stream_manager: Arc<EventStreamManager>,
    
    // User Story 4: Analytics
    analytics_workspace: Arc<DataAnalystWorkspace>,
    
    // User Story 5: Integration
    integration_hub: Arc<IntegrationHub>,
}

impl ECommercePlatform {
    pub async fn new() -> Self {
        let event_store = Arc::new(RwLock::new(Box::new(InMemoryEventStore::new()) as Box<dyn EventStore>));
        
        // Initialize components
        let search_widget = Arc::new(RwLock::new(
            SearchWidget::new(
                DomainContext::new("product-search".into()),
                event_store.read().await.clone()
            )
        ));
        
        let domain_architecture = Arc::new(ECommerceDomainArchitecture::new());
        
        let event_stream_manager = Arc::new(EventStreamManager::new(
            event_store.read().await.clone()
        ));
        
        let analytics_workspace = Arc::new(DataAnalystWorkspace::new(
            event_store.read().await.clone()
        ));
        
        let integration_hub = Arc::new(IntegrationHub::new(
            event_store.read().await.clone()
        ));
        
        Self {
            event_store,
            search_widget,
            domain_architecture,
            event_stream_manager,
            analytics_workspace,
            integration_hub,
        }
    }

    pub async fn initialize(&self) -> Result<(), DomainError> {
        println!("Initializing E-Commerce Platform");
        println!("=================================\n");

        // Step 1: Set up event streams (User Story 3)
        self.setup_event_streams().await?;
        
        // Step 2: Configure integrations (User Story 5)
        self.setup_integrations().await?;
        
        // Step 3: Create analytics projections (User Story 4)
        self.setup_analytics().await?;
        
        println!("\n✓ Platform initialized successfully!\n");
        Ok(())
    }

    async fn setup_event_streams(&self) -> Result<(), DomainError> {
        println!("Setting up event streams...");
        
        // Create streams for each domain
        let streams = vec![
            EventStreamConfiguration {
                stream_name: "orders".into(),
                subjects: vec!["order.*".into()],
                retention_policy: RetentionPolicy {
                    max_age: std::time::Duration::from_secs(30 * 24 * 60 * 60),
                    max_messages: Some(10_000_000),
                    max_bytes: Some(10_737_418_240), // 10GB
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
                        name: "payment-consumer".into(),
                        filter_subjects: vec!["order.created".into()],
                        delivery_policy: DeliveryPolicy::All,
                        ack_wait: std::time::Duration::from_secs(30),
                        max_deliver: 3,
                    },
                ],
            },
            EventStreamConfiguration {
                stream_name: "inventory".into(),
                subjects: vec!["inventory.*".into()],
                retention_policy: RetentionPolicy {
                    max_age: std::time::Duration::from_secs(90 * 24 * 60 * 60),
                    max_messages: None,
                    max_bytes: Some(5_368_709_120), // 5GB
                },
                replication_factor: 3,
                consumers: vec![],
            },
            EventStreamConfiguration {
                stream_name: "payments".into(),
                subjects: vec!["payment.*".into()],
                retention_policy: RetentionPolicy {
                    max_age: std::time::Duration::from_secs(7 * 365 * 24 * 60 * 60), // 7 years
                    max_messages: None,
                    max_bytes: None,
                },
                replication_factor: 3,
                consumers: vec![],
            },
        ];

        for stream in streams {
            self.event_stream_manager.create_stream(stream).await?;
        }

        // Set up routing rules
        let routing_rules = vec![
            EventRoutingRule {
                name: "order-to-inventory".into(),
                source_pattern: "order.created".into(),
                target_streams: vec!["inventory".into()],
                transformation: None,
                filter: None,
            },
            EventRoutingRule {
                name: "order-to-payment".into(),
                source_pattern: "order.created".into(),
                target_streams: vec!["payments".into()],
                transformation: None,
                filter: None,
            },
        ];

        for rule in routing_rules {
            self.event_stream_manager.add_routing_rule(rule).await?;
        }

        Ok(())
    }

    async fn setup_integrations(&self) -> Result<(), DomainError> {
        println!("Setting up integrations...");

        // Event bridge between domains
        let integrations = vec![
            IntegrationPattern::EventBridge {
                source_domain: "order".into(),
                target_domain: "fulfillment".into(),
                event_mapping: EventMapping {
                    source_events: vec!["order.shipped".into()],
                    target_event: "shipment.created".into(),
                    field_mappings: vec![
                        ("order_id".into(), "reference_id".into()),
                        ("tracking_number".into(), "tracking_id".into()),
                    ].into_iter().collect(),
                    enrichments: vec![],
                },
            },
            IntegrationPattern::ApiGateway {
                domain: "products".into(),
                endpoints: vec![
                    ApiEndpoint {
                        path: "/api/v1/products/search".into(),
                        method: HttpMethod::Post,
                        domain_command: "SearchProducts".into(),
                        auth_required: false,
                    },
                ],
                rate_limit: RateLimit {
                    requests_per_minute: 1000,
                    burst_size: 100,
                },
            },
        ];

        for pattern in integrations {
            self.integration_hub.register_integration(pattern).await?;
        }

        Ok(())
    }

    async fn setup_analytics(&self) -> Result<(), DomainError> {
        println!("Setting up analytics projections...");

        // Sales analytics projection
        let sales_projection = ProjectionDefinition {
            name: "sales_analytics".into(),
            source_streams: vec!["orders".into(), "payments".into()],
            event_handlers: vec![
                ("order.created".into(), ProjectionHandler::Increment { field: "total_orders".into() }),
                ("order.created".into(), ProjectionHandler::Set { 
                    field: "last_order_amount".into(), 
                    value_path: "total_amount".into() 
                }),
                ("payment.completed".into(), ProjectionHandler::Increment { field: "successful_payments".into() }),
            ].into_iter().collect(),
            query_model: QueryModel {
                fields: vec![
                    DataFieldDefinition {
                        name: "total_orders".into(),
                        field_type: FieldType::Number,
                        default_value: serde_json::json!(0),
                    },
                    DataFieldDefinition {
                        name: "successful_payments".into(),
                        field_type: FieldType::Number,
                        default_value: serde_json::json!(0),
                    },
                ],
                indexes: vec![],
                aggregations: vec![
                    AggregationDefinition {
                        name: "conversion_rate".into(),
                        aggregation_type: AggregationType::Average,
                        field: "successful_payments".into(),
                        group_by: None,
                    },
                ],
            },
        };

        self.analytics_workspace.create_projection(sales_projection).await?;

        // Customer analytics projection
        let customer_projection = ProjectionDefinition {
            name: "customer_analytics".into(),
            source_streams: vec!["orders".into(), "customers".into()],
            event_handlers: vec![
                ("order.created".into(), ProjectionHandler::Append {
                    field: "order_history".into(),
                    value_path: "order_id".into(),
                }),
            ].into_iter().collect(),
            query_model: QueryModel {
                fields: vec![
                    DataFieldDefinition {
                        name: "customer_id".into(),
                        field_type: FieldType::String,
                        default_value: serde_json::json!(""),
                    },
                    DataFieldDefinition {
                        name: "order_history".into(),
                        field_type: FieldType::Array(Box::new(FieldType::String)),
                        default_value: serde_json::json!([]),
                    },
                ],
                indexes: vec![
                    IndexDefinition {
                        name: "customer_id_idx".into(),
                        fields: vec!["customer_id".into()],
                        unique: true,
                    },
                ],
                aggregations: vec![],
            },
        };

        self.analytics_workspace.create_projection(customer_projection).await?;

        Ok(())
    }

    pub async fn simulate_customer_journey(&self) -> Result<(), DomainError> {
        println!("\nSimulating Customer Journey");
        println!("===========================\n");

        // Step 1: Customer searches for products (User Story 1)
        println!("1. Customer searches for 'gaming laptop'");
        let mut search_widget = self.search_widget.write().await;
        search_widget.handle_user_input(UserInput::Search("gaming laptop".into())).await?;
        println!("   Search component state: {}", search_widget.render().await);

        // Step 2: Create order (following domain boundaries - User Story 2)
        println!("\n2. Customer places order");
        let order_event = DomainEvent::new(
            "order.created".into(),
            serde_json::json!({
                "order_id": "ORD-DEMO-001",
                "customer_id": "CUST-DEMO-123",
                "items": [
                    {"sku": "LAPTOP-GAMING-001", "quantity": 1, "price": 1499.99}
                ],
                "total_amount": 1499.99
            }),
        );

        // Process through event streams (User Story 3)
        println!("\n3. Processing order through event streams");
        let routed = self.event_stream_manager.route_event(&order_event).await?;
        println!("   Event routed to: {:?}", routed);

        // Update analytics (User Story 4)
        self.analytics_workspace.process_event(&order_event).await?;

        // Step 3: Payment processing
        println!("\n4. Processing payment");
        let payment_event = DomainEvent::new(
            "payment.initiated".into(),
            serde_json::json!({
                "order_id": "ORD-DEMO-001",
                "amount": 1499.99,
                "payment_method": "credit_card"
            }),
        );
        
        // Process through integration hub (User Story 5)
        self.integration_hub.process_cross_domain_event(&payment_event).await?;

        // Step 4: Inventory check and reservation
        println!("\n5. Checking and reserving inventory");
        let inventory_event = DomainEvent::new(
            "inventory.reserved".into(),
            serde_json::json!({
                "order_id": "ORD-DEMO-001",
                "reservations": [
                    {"sku": "LAPTOP-GAMING-001", "quantity": 1, "warehouse": "WH-001"}
                ]
            }),
        );
        
        self.event_stream_manager.route_event(&inventory_event).await?;

        // Step 5: Payment completion
        println!("\n6. Payment completed");
        let payment_complete_event = DomainEvent::new(
            "payment.completed".into(),
            serde_json::json!({
                "order_id": "ORD-DEMO-001",
                "transaction_id": "TXN-12345",
                "amount": 1499.99
            }),
        );
        
        self.analytics_workspace.process_event(&payment_complete_event).await?;

        // Step 6: Order fulfillment
        println!("\n7. Order shipped");
        let shipment_event = DomainEvent::new(
            "order.shipped".into(),
            serde_json::json!({
                "order_id": "ORD-DEMO-001",
                "tracking_number": "TRACK-98765",
                "carrier": "FastShip",
                "estimated_delivery": "2024-01-15"
            }),
        );
        
        self.integration_hub.process_cross_domain_event(&shipment_event).await?;

        Ok(())
    }

    pub async fn display_platform_status(&self) -> Result<(), DomainError> {
        println!("\n\nPlatform Status Dashboard");
        println!("=========================\n");

        // Domain Architecture Status (User Story 2)
        println!("Domain Architecture:");
        println!("{}", self.domain_architecture.generate_architecture_report());

        // Event Stream Status (User Story 3)
        println!("\nEvent Streams:");
        let stream_dashboard = self.event_stream_manager.get_dashboard_view().await;
        println!("  Total Streams: {}", stream_dashboard.total_streams);
        println!("  Total Consumers: {}", stream_dashboard.total_consumers);
        println!("  Routing Rules: {}", stream_dashboard.routing_rules);

        // Analytics Status (User Story 4)
        println!("\nAnalytics:");
        let sales_query = AnalyticsQuery {
            name: "current_sales".into(),
            projection: "sales_analytics".into(),
            filters: vec![],
            aggregations: vec!["conversion_rate".into()],
            group_by: vec![],
            order_by: vec![],
            limit: Some(5),
        };
        
        self.analytics_workspace.save_query(sales_query).await?;
        let report = self.analytics_workspace.generate_report("current_sales").await?;
        println!("  Total Orders: {}", report.row_count);
        if let Some(conversion) = report.aggregations.get("conversion_rate") {
            println!("  Conversion Rate: {}", conversion);
        }

        // Integration Status (User Story 5)
        println!("\nIntegrations:");
        let integration_health = self.integration_hub.get_integration_health().await;
        println!("  Total Integrations: {}", integration_health.total_integrations);
        println!("  Open Circuit Breakers: {}", integration_health.open_circuit_breakers);
        println!("  Messages in Queue: {}", integration_health.messages_in_queue);

        Ok(())
    }
}

// Demonstrate a complete saga across all domains
pub async fn demonstrate_order_saga(platform: &ECommercePlatform) {
    println!("\n\nDemonstrating Order Fulfillment Saga");
    println!("====================================\n");

    let mut saga = OrderFulfillmentSaga::new("ORD-SAGA-001".into());
    
    // Execute saga with proper error handling
    match saga.execute(&platform.domain_architecture).await {
        Ok(_) => println!("✓ Saga completed successfully"),
        Err(e) => println!("✗ Saga failed: {}", e),
    }
}

#[tokio::main]
async fn main() -> Result<(), DomainError> {
    println!("==============================================");
    println!("  Comprehensive E-Commerce Platform Demo");
    println!("==============================================\n");
    
    println!("This demo showcases all 5 user stories working");
    println!("together to create a complete e-commerce platform.\n");

    // Create and initialize platform
    let platform = ECommercePlatform::new().await;
    platform.initialize().await?;

    // Run customer journey simulation
    platform.simulate_customer_journey().await?;

    // Demonstrate saga pattern
    demonstrate_order_saga(&platform).await;

    // Display platform status
    platform.display_platform_status().await?;

    println!("\n\n==============================================");
    println!("           Demo Completed Successfully!");
    println!("==============================================\n");
    
    println!("Key Achievements:");
    println!("✓ Component Developer: Built reusable search widget");
    println!("✓ System Architect: Defined clear domain boundaries");
    println!("✓ Event Stream Manager: Configured reliable event flows");
    println!("✓ Data Analyst: Created real-time analytics projections");
    println!("✓ Integration Engineer: Established resilient integrations");
    println!("\nAll components work together seamlessly to create");
    println!("a robust, scalable e-commerce platform!");

    Ok(())
}