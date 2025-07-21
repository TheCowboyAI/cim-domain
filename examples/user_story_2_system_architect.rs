/// User Story 2: System Architect - Defining Domain Boundaries
/// 
/// As a System Architect, I want to define clear domain boundaries and
/// establish rules for cross-domain communication, so that different teams
/// can work independently while maintaining system integrity.

use std::collections::HashMap;

// Define domain boundaries for an e-commerce system
#[derive(Debug, Clone)]
pub struct ECommerceDomainArchitecture {
    domains: HashMap<String, DomainDefinition>,
    cross_domain_rules: Vec<CrossDomainRule>,
    invariants: Vec<DomainInvariant>,
}

#[derive(Debug, Clone)]
pub struct DomainDefinition {
    name: String,
    bounded_context: BoundedContext,
    capabilities: Vec<DomainCapability>,
    published_events: Vec<EventSchema>,
    consumed_events: Vec<EventSchema>,
}

#[derive(Debug, Clone)]
pub struct BoundedContext {
    name: String,
    description: String,
    team_ownership: String,
    ubiquitous_language: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum DomainCapability {
    Command(String),
    Query(String),
    EventPublication(String),
}

#[derive(Debug, Clone)]
pub struct EventSchema {
    name: String,
    version: String,
    fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    name: String,
    field_type: String,
    required: bool,
}

impl ECommerceDomainArchitecture {
    pub fn new() -> Self {
        let mut arch = Self {
            domains: HashMap::new(),
            cross_domain_rules: Vec::new(),
            invariants: Vec::new(),
        };

        // Define the domains
        arch.define_order_domain();
        arch.define_inventory_domain();
        arch.define_payment_domain();
        arch.define_shipping_domain();
        
        // Establish cross-domain rules
        arch.establish_cross_domain_rules();
        
        // Define system-wide invariants
        arch.define_invariants();

        arch
    }

    fn define_order_domain(&mut self) {
        let mut ubiquitous_language = HashMap::new();
        ubiquitous_language.insert("Order".into(), "A customer's purchase request".into());
        ubiquitous_language.insert("OrderLine".into(), "Individual item within an order".into());
        ubiquitous_language.insert("OrderStatus".into(), "Current state of the order lifecycle".into());

        let domain = DomainDefinition {
            name: "Orders".into(),
            bounded_context: BoundedContext {
                name: "Order Management".into(),
                description: "Handles order lifecycle from creation to fulfillment".into(),
                team_ownership: "Order Team".into(),
                ubiquitous_language,
            },
            capabilities: vec![
                DomainCapability::Command("CreateOrder".into()),
                DomainCapability::Command("CancelOrder".into()),
                DomainCapability::Query("GetOrderStatus".into()),
                DomainCapability::EventPublication("OrderCreated".into()),
                DomainCapability::EventPublication("OrderCancelled".into()),
            ],
            published_events: vec![
                EventSchema {
                    name: "OrderCreated".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "customer_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "items".into(), field_type: "Array<OrderItem>".into(), required: true },
                        FieldDefinition { name: "total_amount".into(), field_type: "Decimal".into(), required: true },
                    ],
                },
            ],
            consumed_events: vec![
                EventSchema {
                    name: "PaymentCompleted".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                    ],
                },
                EventSchema {
                    name: "InventoryReserved".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                    ],
                },
            ],
        };

        self.domains.insert("Orders".into(), domain);
    }

    fn define_inventory_domain(&mut self) {
        let mut ubiquitous_language = HashMap::new();
        ubiquitous_language.insert("Stock".into(), "Available quantity of a product".into());
        ubiquitous_language.insert("Reservation".into(), "Temporary hold on inventory".into());
        ubiquitous_language.insert("SKU".into(), "Stock Keeping Unit identifier".into());

        let domain = DomainDefinition {
            name: "Inventory".into(),
            bounded_context: BoundedContext {
                name: "Inventory Management".into(),
                description: "Manages product availability and reservations".into(),
                team_ownership: "Inventory Team".into(),
                ubiquitous_language,
            },
            capabilities: vec![
                DomainCapability::Command("ReserveInventory".into()),
                DomainCapability::Command("ReleaseReservation".into()),
                DomainCapability::Query("CheckAvailability".into()),
                DomainCapability::EventPublication("InventoryReserved".into()),
                DomainCapability::EventPublication("InsufficientInventory".into()),
            ],
            published_events: vec![
                EventSchema {
                    name: "InventoryReserved".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "reservations".into(), field_type: "Array<Reservation>".into(), required: true },
                    ],
                },
            ],
            consumed_events: vec![
                EventSchema {
                    name: "OrderCreated".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "items".into(), field_type: "Array<OrderItem>".into(), required: true },
                    ],
                },
            ],
        };

        self.domains.insert("Inventory".into(), domain);
    }

    fn define_payment_domain(&mut self) {
        let mut ubiquitous_language = HashMap::new();
        ubiquitous_language.insert("Transaction".into(), "A payment operation".into());
        ubiquitous_language.insert("PaymentMethod".into(), "Means of payment (card, bank, etc)".into());
        ubiquitous_language.insert("Authorization".into(), "Payment approval from provider".into());

        let domain = DomainDefinition {
            name: "Payments".into(),
            bounded_context: BoundedContext {
                name: "Payment Processing".into(),
                description: "Handles payment authorization and capture".into(),
                team_ownership: "Payment Team".into(),
                ubiquitous_language,
            },
            capabilities: vec![
                DomainCapability::Command("ProcessPayment".into()),
                DomainCapability::Command("RefundPayment".into()),
                DomainCapability::Query("GetTransactionStatus".into()),
                DomainCapability::EventPublication("PaymentCompleted".into()),
                DomainCapability::EventPublication("PaymentFailed".into()),
            ],
            published_events: vec![
                EventSchema {
                    name: "PaymentCompleted".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "transaction_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "amount".into(), field_type: "Decimal".into(), required: true },
                    ],
                },
            ],
            consumed_events: vec![
                EventSchema {
                    name: "OrderCreated".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "total_amount".into(), field_type: "Decimal".into(), required: true },
                    ],
                },
            ],
        };

        self.domains.insert("Payments".into(), domain);
    }

    fn define_shipping_domain(&mut self) {
        let mut ubiquitous_language = HashMap::new();
        ubiquitous_language.insert("Shipment".into(), "Package ready for delivery".into());
        ubiquitous_language.insert("Carrier".into(), "Shipping service provider".into());
        ubiquitous_language.insert("TrackingNumber".into(), "Unique shipment identifier".into());

        let domain = DomainDefinition {
            name: "Shipping".into(),
            bounded_context: BoundedContext {
                name: "Shipping & Fulfillment".into(),
                description: "Manages order fulfillment and delivery".into(),
                team_ownership: "Fulfillment Team".into(),
                ubiquitous_language,
            },
            capabilities: vec![
                DomainCapability::Command("CreateShipment".into()),
                DomainCapability::Command("UpdateShipmentStatus".into()),
                DomainCapability::Query("TrackShipment".into()),
                DomainCapability::EventPublication("ShipmentCreated".into()),
                DomainCapability::EventPublication("ShipmentDelivered".into()),
            ],
            published_events: vec![
                EventSchema {
                    name: "ShipmentCreated".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                        FieldDefinition { name: "tracking_number".into(), field_type: "String".into(), required: true },
                    ],
                },
            ],
            consumed_events: vec![
                EventSchema {
                    name: "OrderCreated".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                    ],
                },
                EventSchema {
                    name: "PaymentCompleted".into(),
                    version: "1.0".into(),
                    fields: vec![
                        FieldDefinition { name: "order_id".into(), field_type: "String".into(), required: true },
                    ],
                },
            ],
        };

        self.domains.insert("Shipping".into(), domain);
    }

    fn establish_cross_domain_rules(&mut self) {
        // Rule 1: Asynchronous communication only
        self.cross_domain_rules.push(CrossDomainRule {
            name: "Async Communication".into(),
            description: "All cross-domain communication must be asynchronous via events".into(),
            rule_type: RuleType::Communication,
            enforcement: Enforcement::Strict,
        });

        // Rule 2: No direct database access
        self.cross_domain_rules.push(CrossDomainRule {
            name: "Database Isolation".into(),
            description: "Domains cannot access each other's databases directly".into(),
            rule_type: RuleType::DataAccess,
            enforcement: Enforcement::Strict,
        });

        // Rule 3: Event versioning required
        self.cross_domain_rules.push(CrossDomainRule {
            name: "Event Versioning".into(),
            description: "All published events must include version information".into(),
            rule_type: RuleType::EventDesign,
            enforcement: Enforcement::Strict,
        });

        // Rule 4: Eventual consistency
        self.cross_domain_rules.push(CrossDomainRule {
            name: "Eventual Consistency".into(),
            description: "Cross-domain data consistency is eventual, not immediate".into(),
            rule_type: RuleType::Consistency,
            enforcement: Enforcement::Strict,
        });
    }

    fn define_invariants(&mut self) {
        // Invariant 1: Order must have payment before shipping
        self.invariants.push(DomainInvariant {
            name: "Payment Before Shipping".into(),
            description: "A shipment cannot be created without payment completion".into(),
            domains_involved: vec!["Orders".into(), "Payments".into(), "Shipping".into()],
            validation_rule: "ShipmentCreated requires PaymentCompleted for same order_id".into(),
        });

        // Invariant 2: Inventory must be available
        self.invariants.push(DomainInvariant {
            name: "Inventory Availability".into(),
            description: "Orders can only be confirmed if inventory is reserved".into(),
            domains_involved: vec!["Orders".into(), "Inventory".into()],
            validation_rule: "OrderConfirmed requires InventoryReserved for all items".into(),
        });

        // Invariant 3: Single payment per order
        self.invariants.push(DomainInvariant {
            name: "Single Payment".into(),
            description: "Each order can have only one successful payment".into(),
            domains_involved: vec!["Orders".into(), "Payments".into()],
            validation_rule: "Only one PaymentCompleted event per order_id".into(),
        });
    }

    pub fn validate_domain_interaction(&self, from_domain: &str, to_domain: &str, event: &str) -> Result<(), String> {
        // Check if domains exist
        let from = self.domains.get(from_domain)
            .ok_or_else(|| format!("Domain '{}' not found", from_domain))?;
        let to = self.domains.get(to_domain)
            .ok_or_else(|| format!("Domain '{}' not found", to_domain))?;

        // Check if event is published by source domain
        let publishes_event = from.published_events.iter()
            .any(|e| e.name == event);
        if !publishes_event {
            return Err(format!("Domain '{}' does not publish event '{}'", from_domain, event));
        }

        // Check if event is consumed by target domain
        let consumes_event = to.consumed_events.iter()
            .any(|e| e.name == event);
        if !consumes_event {
            return Err(format!("Domain '{}' does not consume event '{}'", to_domain, event));
        }

        Ok(())
    }

    pub fn generate_architecture_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== E-Commerce Domain Architecture ===\n\n");
        
        // List domains
        report.push_str("DOMAINS:\n");
        for (name, domain) in &self.domains {
            report.push_str(&format!("  Domain: {} ({}\n", domain.name, name));
            report.push_str(&format!("  Context: {} (Team: {})\n", domain.bounded_context.name, domain.bounded_context.team_ownership));
            report.push_str(&format!("    Description: {}\n", domain.bounded_context.description));
            
            // Show ubiquitous language
            if !domain.bounded_context.ubiquitous_language.is_empty() {
                report.push_str("    Key Terms:\n");
                for (term, definition) in &domain.bounded_context.ubiquitous_language {
                    report.push_str(&format!("      - {}: {}\n", term, definition));
                }
            }
            
            report.push_str(&format!("    Capabilities: {} commands, {} queries, {} events\n",
                domain.capabilities.iter().filter(|c| matches!(c, DomainCapability::Command(_))).count(),
                domain.capabilities.iter().filter(|c| matches!(c, DomainCapability::Query(_))).count(),
                domain.capabilities.iter().filter(|c| matches!(c, DomainCapability::EventPublication(_))).count()
            ));
            
            // Show event schemas
            if !domain.published_events.is_empty() {
                report.push_str("    Published Events:\n");
                for event in &domain.published_events {
                    report.push_str(&format!("      - {} v{} ({} fields)\n", event.name, event.version, event.fields.len()));
                    for field in &event.fields {
                        report.push_str(&format!("        • {}: {} {}\n", 
                            field.name, 
                            field.field_type,
                            if field.required { "(required)" } else { "(optional)" }
                        ));
                    }
                }
            }
        }
        
        report.push_str("\nCROSS-DOMAIN RULES:\n");
        for rule in &self.cross_domain_rules {
            report.push_str(&format!("  - {} [{:?}]: {}\n", rule.name, rule.rule_type, rule.description));
            report.push_str(&format!("    Enforcement: {:?}\n", rule.enforcement));
        }
        
        report.push_str("\nSYSTEM INVARIANTS:\n");
        for invariant in &self.invariants {
            report.push_str(&format!("  - {}: {}\n", invariant.name, invariant.description));
            report.push_str(&format!("    Domains: {}\n", invariant.domains_involved.join(", ")));
            report.push_str(&format!("    Rule: {}\n", invariant.validation_rule));
        }
        
        report
    }
}

#[derive(Debug, Clone)]
pub struct CrossDomainRule {
    name: String,
    description: String,
    rule_type: RuleType,
    enforcement: Enforcement,
}

#[derive(Debug, Clone)]
pub enum RuleType {
    Communication,
    DataAccess,
    EventDesign,
    Consistency,
}

#[derive(Debug, Clone)]
pub enum Enforcement {
    Strict,
    Warning,
}

#[derive(Debug, Clone)]
pub struct DomainInvariant {
    name: String,
    description: String,
    domains_involved: Vec<String>,
    validation_rule: String,
}

// Demonstrate saga pattern for cross-domain transactions
pub struct OrderFulfillmentSaga {
    saga_id: String,
    order_id: String,
    state: SagaState,
    compensations: Vec<CompensationAction>,
}

#[derive(Debug)]
pub enum SagaState {
    Started,
    InventoryReserved,
    PaymentProcessed,
    ShipmentCreated,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct CompensationAction {
    domain: String,
    action: String,
    event_to_emit: String,
}

impl OrderFulfillmentSaga {
    pub fn new(order_id: String) -> Self {
        Self {
            saga_id: format!("saga-{}", uuid::Uuid::new_v4()),
            order_id,
            state: SagaState::Started,
            compensations: Vec::new(),
        }
    }

    pub fn get_saga_id(&self) -> &str {
        &self.saga_id
    }

    pub async fn execute(&mut self, _architecture: &ECommerceDomainArchitecture) -> Result<(), String> {
        println!("Starting Order Fulfillment Saga {} for order: {}", self.saga_id, self.order_id);

        // Step 1: Reserve Inventory
        match self.reserve_inventory().await {
            Ok(_) => {
                self.state = SagaState::InventoryReserved;
                self.compensations.push(CompensationAction {
                    domain: "Inventory".into(),
                    action: "ReleaseReservation".into(),
                    event_to_emit: "ReservationReleased".into(),
                });
            }
            Err(e) => {
                self.state = SagaState::Failed(e);
                return self.compensate().await;
            }
        }

        // Step 2: Process Payment
        match self.process_payment().await {
            Ok(_) => {
                self.state = SagaState::PaymentProcessed;
                self.compensations.push(CompensationAction {
                    domain: "Payments".into(),
                    action: "RefundPayment".into(),
                    event_to_emit: "PaymentRefunded".into(),
                });
            }
            Err(e) => {
                self.state = SagaState::Failed(e);
                return self.compensate().await;
            }
        }

        // Step 3: Create Shipment
        match self.create_shipment().await {
            Ok(_) => {
                self.state = SagaState::ShipmentCreated;
                // No compensation needed for shipment creation
            }
            Err(e) => {
                self.state = SagaState::Failed(e);
                return self.compensate().await;
            }
        }

        self.state = SagaState::Completed;
        println!("Saga completed successfully!");
        Ok(())
    }

    async fn reserve_inventory(&self) -> Result<(), String> {
        println!("  → Reserving inventory...");
        // Simulate inventory reservation
        Ok(())
    }

    async fn process_payment(&self) -> Result<(), String> {
        println!("  → Processing payment...");
        // Simulate payment processing
        Ok(())
    }

    async fn create_shipment(&self) -> Result<(), String> {
        println!("  → Creating shipment...");
        // Simulate shipment creation
        Ok(())
    }

    async fn compensate(&self) -> Result<(), String> {
        println!("  ⚠ Saga {} failed, executing compensations...", self.saga_id);
        for compensation in self.compensations.iter().rev() {
            println!("    ← Compensating: {} in {} (will emit: {})", 
                compensation.action, 
                compensation.domain,
                compensation.event_to_emit
            );
        }
        Err("Saga failed and compensated".into())
    }
}

#[tokio::main]
async fn main() {
    println!("User Story 2: System Architect Demo");
    println!("===================================\n");

    // Create the domain architecture
    let architecture = ECommerceDomainArchitecture::new();

    // Generate and display architecture report
    println!("{}", architecture.generate_architecture_report());

    // Validate domain interactions
    println!("\nVALIDATING DOMAIN INTERACTIONS:");
    
    // Valid interaction
    match architecture.validate_domain_interaction("Orders", "Inventory", "OrderCreated") {
        Ok(_) => println!("  ✓ Orders → Inventory [OrderCreated]: Valid"),
        Err(e) => println!("  ✗ Orders → Inventory [OrderCreated]: {}", e),
    }

    // Invalid interaction (wrong event)
    match architecture.validate_domain_interaction("Orders", "Inventory", "PaymentCompleted") {
        Ok(_) => println!("  ✓ Orders → Inventory [PaymentCompleted]: Valid"),
        Err(e) => println!("  ✗ Orders → Inventory [PaymentCompleted]: {}", e),
    }

    // Invalid interaction (domain doesn't consume)
    match architecture.validate_domain_interaction("Payments", "Orders", "ShipmentCreated") {
        Ok(_) => println!("  ✓ Payments → Orders [ShipmentCreated]: Valid"),
        Err(e) => println!("  ✗ Payments → Orders [ShipmentCreated]: {}", e),
    }

    // Demonstrate saga pattern
    println!("\n\nDEMONSTRATING SAGA PATTERN:");
    let mut saga = OrderFulfillmentSaga::new("ORD-12345".into());
    match saga.execute(&architecture).await {
        Ok(_) => println!("✓ Order fulfillment completed successfully"),
        Err(e) => println!("✗ Order fulfillment failed: {}", e),
    }

    println!("\nDomain boundaries established:");
    println!("✓ Clear separation of concerns");
    println!("✓ Well-defined team ownership");
    println!("✓ Event-driven communication");
    println!("✓ Cross-domain invariants defined");
    println!("✓ Saga pattern for distributed transactions");
}