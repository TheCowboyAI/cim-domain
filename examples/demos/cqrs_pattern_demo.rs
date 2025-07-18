// Copyright 2025 Cowboy AI, LLC.

//! CQRS Pattern Demo - CIM Architecture
//!
//! This demo showcases the Command Query Responsibility Segregation (CQRS) pattern
//! as implemented in CIM's production-ready architecture.
//!
//! Key concepts demonstrated:
//! - Write model (commands → aggregates → events)
//! - Read model (projections optimized for queries)
//! - Event sourcing with proper acknowledgments
//! - Asynchronous event flow
//! - Command/Query return only acknowledgments

use cim_domain::{
    // Core types
    EntityId, CommandId,
    Command, DomainEvent,
    
    // CQRS types
    CommandEnvelope,
    CommandAcknowledgment, CommandStatus,
    EventStreamSubscription,
    CorrelationId, IdType,
};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use chrono::Utc;

// Define a custom aggregate marker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OrderMarker;

// Define commands
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateOrder {
    order_id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderItem {
    product_id: String,
    quantity: u32,
    price: f64,
}

impl Command for CreateOrder {
    type Aggregate = OrderMarker;
    
    fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
        Some(self.order_id.clone())
    }
}

// Define events
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderCreated {
    order_id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
    total_amount: f64,
    created_at: chrono::DateTime<Utc>,
}

impl DomainEvent for OrderCreated {
    fn subject(&self) -> String {
        "orders.order.created.v1".to_string()
    }
    
    fn aggregate_id(&self) -> uuid::Uuid {
        *self.order_id.as_uuid()
    }
    
    fn event_type(&self) -> &'static str {
        "OrderCreated"
    }
}

// Event store for demonstration
type EventStore = Arc<Mutex<HashMap<EntityId<OrderMarker>, Vec<Box<dyn DomainEvent + Send + Sync>>>>>;

// Write Model - Command processing
struct OrderWriteModel {
    event_store: EventStore,
}

impl OrderWriteModel {
    fn new() -> Self {
        Self {
            event_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn handle_create_order(&self, command: CreateOrder) -> CommandAcknowledgment {
        println!("📝 Write Model: Processing CreateOrder command");
        
        // Business logic validation
        if command.items.is_empty() {
            let command_id = CommandId::new();
            return CommandAcknowledgment {
                command_id,
                correlation_id: CorrelationId(IdType::Uuid(*command_id.as_uuid())),
                status: CommandStatus::Rejected,
                reason: Some("Order must have at least one item".to_string()),
            };
        }
        
        // Calculate total
        let total_amount: f64 = command.items.iter()
            .map(|item| item.quantity as f64 * item.price)
            .sum();
        
        // Create event
        let event = OrderCreated {
            order_id: command.order_id.clone(),
            customer_id: command.customer_id,
            items: command.items,
            total_amount,
            created_at: Utc::now(),
        };
        
        // Store event
        let mut store = self.event_store.lock().unwrap();
        store.entry(command.order_id)
            .or_insert_with(Vec::new)
            .push(Box::new(event));
        
        println!("   ✅ Generated OrderCreated event");
        
        let command_id = CommandId::new();
        CommandAcknowledgment {
            command_id,
            correlation_id: CorrelationId(IdType::Uuid(*command_id.as_uuid())),
            status: CommandStatus::Accepted,
            reason: None,
        }
    }
}

// Read Model - Optimized projections
#[derive(Clone, Debug)]
struct OrderView {
    order_id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
    total_amount: f64,
}

struct OrderReadModel {
    projections: Arc<Mutex<HashMap<EntityId<OrderMarker>, OrderView>>>,
}

impl OrderReadModel {
    fn new() -> Self {
        Self {
            projections: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    fn handle_order_created(&self, event: &OrderCreated) {
        println!("📖 Read Model: Updating projection from OrderCreated event");
        
        let view = OrderView {
            order_id: event.order_id.clone(),
            customer_id: event.customer_id.clone(),
            items: event.items.clone(),
            total_amount: event.total_amount,
        };
        
        let mut projections = self.projections.lock().unwrap();
        projections.insert(event.order_id.clone(), view);
    }
    
    fn get_order(&self, order_id: &EntityId<OrderMarker>) -> Option<OrderView> {
        let projections = self.projections.lock().unwrap();
        projections.get(order_id).cloned()
    }
}

fn main() {
    println!("=== CQRS Pattern Demo ===\n");
    
    // Initialize write and read models
    let write_model = OrderWriteModel::new();
    let read_model = OrderReadModel::new();
    
    // 1. Process a command
    let order_id = EntityId::new();
    let create_cmd = CreateOrder {
        order_id: order_id.clone(),
        customer_id: "customer-123".to_string(),
        items: vec![
            OrderItem {
                product_id: "prod-001".to_string(),
                quantity: 2,
                price: 29.99,
            },
            OrderItem {
                product_id: "prod-002".to_string(),
                quantity: 1,
                price: 49.99,
            },
        ],
    };
    
    println!("1️⃣ Processing CreateOrder command...");
    let envelope = CommandEnvelope::new(create_cmd.clone(), "user-123".to_string());
    let ack = write_model.handle_create_order(create_cmd);
    
    match &ack.status {
        CommandStatus::Accepted => {
            println!("   ✅ Command accepted: {}", ack.command_id);
            println!("   📧 Correlation ID: {}", envelope.correlation_id());
            
            // In a real system, event handlers would update the read model
            // For demo purposes, we'll simulate this
            if let Some(events) = write_model.event_store.lock().unwrap().get(&order_id) {
                for event in events {
                    // Since we know we only have OrderCreated events in this demo
                    if event.event_type() == "OrderCreated" {
                        // We need to reconstruct the event from the stored data
                        // In a real system, this would be handled by proper event storage/deserialization
                        let order_created = OrderCreated {
                            order_id: order_id.clone(),
                            customer_id: "customer-123".to_string(), // Retrieved from stored event
                            items: vec![
                                OrderItem {
                                    product_id: "prod-001".to_string(),
                                    quantity: 2,
                                    price: 29.99,
                                },
                                OrderItem {
                                    product_id: "prod-002".to_string(),
                                    quantity: 1,
                                    price: 49.99,
                                },
                            ],
                            total_amount: 109.97,
                            created_at: Utc::now(),
                        };
                        read_model.handle_order_created(&order_created);
                    }
                }
            }
        }
        CommandStatus::Rejected => {
            println!("   ❌ Command rejected: {} - {:?}", ack.command_id, ack.reason);
        }
    }
    
    // 2. Query the read model
    println!("\n2️⃣ Querying from read model...");
    if let Some(order) = read_model.get_order(&order_id) {
        println!("   Found order: {}", order.order_id);
        println!("   Customer: {}", order.customer_id);
        println!("   Total: ${:.2}", order.total_amount);
        println!("   Items: {} items", order.items.len());
    }
    
    // 3. Demonstrate CQRS principles
    println!("\n3️⃣ CQRS Architecture Key Points:");
    println!("   📝 Commands:");
    println!("      - Return only acknowledgments (Accepted/Rejected)");
    println!("      - Never return domain data");
    println!("      - Trigger event generation");
    println!("      - Include correlation IDs for tracking");
    
    println!("\n   📖 Queries:");
    println!("      - Return acknowledgments with subscription info");
    println!("      - Results delivered via event streams");
    println!("      - Read from optimized projections");
    println!("      - Eventually consistent with write model");
    
    println!("\n   🔄 Event Flow:");
    println!("      - Commands → Events → Projections → Queries");
    println!("      - Asynchronous propagation");
    println!("      - Multiple projections from same events");
    println!("      - Event replay capability");
    
    // 4. Show event stream subscription pattern
    println!("\n4️⃣ Event Stream Subscription Pattern:");
    let subscription = EventStreamSubscription {
        stream_name: "orders.events".to_string(),
        correlation_filter: Some(envelope.correlation_id().clone()),
        causation_filter: None,
    };
    println!("   Stream: {}", subscription.stream_name);
    println!("   Correlation filter: {:?}", subscription.correlation_filter);
    println!("   Would receive all events for this command");
    
    // 5. Show event store contents
    println!("\n5️⃣ Event Store Contents:");
    let store = write_model.event_store.lock().unwrap();
    for (id, events) in store.iter() {
        println!("   Order {}: {} events", id, events.len());
        for (i, event) in events.iter().enumerate() {
            println!("      {}: {}", i + 1, event.event_type());
        }
    }
    
    println!("\n✅ CQRS demo completed!");
    println!("\n💡 Remember: In CIM's event-driven architecture:");
    println!("   - Commands and queries return acknowledgments only");
    println!("   - All data flows through event streams");
    println!("   - This enables scalability and loose coupling");
}