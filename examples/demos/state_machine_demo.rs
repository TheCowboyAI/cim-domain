//! State Machine Demo - CIM Architecture
//!
//! This demo showcases the state machine capabilities in CIM's domain model,
//! including Moore and Mealy machines, state transitions, and practical usage.
//!
//! Key concepts demonstrated:
//! - Moore machines (output based on state)
//! - Mealy machines (output based on state and input)
//! - State transitions with validation
//! - Domain events from state changes
//! - Practical workflow example

use cim_domain::{
    // State machine types
    State, MooreMachine, MealyMachine,
    MooreStateTransitions, MealyStateTransitions,
    TransitionInput, TransitionOutput,
    DocumentState,
    
    // Domain types
    EntityId, DomainEvent, AggregateRoot,
};
use serde::{Serialize, Deserialize};

// Define custom states for an order processing workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum OrderState {
    Draft,
    Submitted,
    Validated,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

impl State for OrderState {
    fn name(&self) -> &'static str {
        match self {
            OrderState::Draft => "Draft",
            OrderState::Submitted => "Submitted",
            OrderState::Validated => "Validated",
            OrderState::Processing => "Processing",
            OrderState::Shipped => "Shipped",
            OrderState::Delivered => "Delivered",
            OrderState::Cancelled => "Cancelled",
        }
    }
    
    fn is_terminal(&self) -> bool {
        matches!(self, OrderState::Delivered | OrderState::Cancelled)
    }
}

// Define a simple text output type
#[derive(Debug, Clone, Default)]
struct TextOutput {
    message: String,
}

impl TransitionOutput for TextOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        vec![] // For demo, no events
    }
}

// Moore machine transitions (output depends only on state)
impl MooreStateTransitions for OrderState {
    type Output = TextOutput;
    
    fn can_transition_to(&self, target: &Self) -> bool {
        use OrderState::*;
        let valid_transitions = match self {
            Draft => vec![Submitted, Cancelled],
            Submitted => vec![Validated, Draft, Cancelled],
            Validated => vec![Processing, Cancelled],
            Processing => vec![Shipped, Cancelled],
            Shipped => vec![Delivered],
            Delivered => vec![],
            Cancelled => vec![],
        };
        valid_transitions.contains(target)
    }
    
    fn valid_transitions(&self) -> Vec<Self> {
        use OrderState::*;
        match self {
            Draft => vec![Submitted, Cancelled],
            Submitted => vec![Validated, Draft, Cancelled],
            Validated => vec![Processing, Cancelled],
            Processing => vec![Shipped, Cancelled],
            Shipped => vec![Delivered],
            Delivered => vec![],
            Cancelled => vec![],
        }
    }
    
    fn entry_output(&self) -> Self::Output {
        TextOutput {
            message: match self {
                OrderState::Draft => "Order is being prepared".to_string(),
                OrderState::Submitted => "Order submitted for processing".to_string(),
                OrderState::Validated => "Order validated successfully".to_string(),
                OrderState::Processing => "Order processing started".to_string(),
                OrderState::Shipped => "Order shipped to customer".to_string(),
                OrderState::Delivered => "Order delivered to customer".to_string(),
                OrderState::Cancelled => "Order has been cancelled".to_string(),
            }
        }
    }
}

// Define inputs for Mealy machine
#[derive(Debug, Clone, Serialize, Deserialize)]
enum OrderInput {
    Submit { customer_email: String },
    Validate { payment_confirmed: bool },
    StartProcessing { warehouse_id: String },
    Ship { tracking_number: String },
    Deliver { signature: String },
    Cancel { reason: String },
}

impl TransitionInput for OrderInput {
    fn description(&self) -> String {
        match self {
            OrderInput::Submit { customer_email } => format!("Submit order for {}", customer_email),
            OrderInput::Validate { payment_confirmed } => format!("Validate payment: {}", payment_confirmed),
            OrderInput::StartProcessing { warehouse_id } => format!("Start processing at {}", warehouse_id),
            OrderInput::Ship { tracking_number } => format!("Ship with tracking {}", tracking_number),
            OrderInput::Deliver { signature } => format!("Deliver to {}", signature),
            OrderInput::Cancel { reason } => format!("Cancel: {}", reason),
        }
    }
}

// Define outputs for Mealy machine
#[derive(Debug, Clone)]
enum OrderOutput {
    Event(OrderEvent),
    Error(String),
}

impl TransitionOutput for OrderOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        match self {
            OrderOutput::Event(event) => vec![Box::new(event.clone()) as Box<dyn DomainEvent>],
            OrderOutput::Error(_) => vec![],
        }
    }
}

// Define domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
enum OrderEvent {
    OrderSubmitted { order_id: String, customer_email: String },
    OrderValidated { order_id: String },
    ProcessingStarted { order_id: String, warehouse_id: String },
    OrderShipped { order_id: String, tracking_number: String },
    OrderDelivered { order_id: String, signature: String },
    OrderCancelled { order_id: String, reason: String },
}

impl DomainEvent for OrderEvent {
    fn subject(&self) -> String {
        match self {
            OrderEvent::OrderSubmitted { .. } => "orders.order.submitted.v1",
            OrderEvent::OrderValidated { .. } => "orders.order.validated.v1",
            OrderEvent::ProcessingStarted { .. } => "orders.order.processing_started.v1",
            OrderEvent::OrderShipped { .. } => "orders.order.shipped.v1",
            OrderEvent::OrderDelivered { .. } => "orders.order.delivered.v1",
            OrderEvent::OrderCancelled { .. } => "orders.order.cancelled.v1",
        }.to_string()
    }
    
    fn aggregate_id(&self) -> uuid::Uuid {
        // In a real implementation, we'd parse the order_id
        uuid::Uuid::new_v4()
    }
    
    fn event_type(&self) -> &'static str {
        match self {
            OrderEvent::OrderSubmitted { .. } => "OrderSubmitted",
            OrderEvent::OrderValidated { .. } => "OrderValidated",
            OrderEvent::ProcessingStarted { .. } => "ProcessingStarted",
            OrderEvent::OrderShipped { .. } => "OrderShipped",
            OrderEvent::OrderDelivered { .. } => "OrderDelivered",
            OrderEvent::OrderCancelled { .. } => "OrderCancelled",
        }
    }
}

// Mealy machine transitions (output depends on state and input)
impl MealyStateTransitions for OrderState {
    type Input = OrderInput;
    type Output = OrderOutput;
    
    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
        use OrderState::*;
        match (self, target, input) {
            (Draft, Submitted, OrderInput::Submit { .. }) => true,
            (Draft, Cancelled, OrderInput::Cancel { .. }) => true,
            (Submitted, Validated, OrderInput::Validate { payment_confirmed }) => *payment_confirmed,
            (Submitted, Draft, OrderInput::Validate { payment_confirmed }) => !*payment_confirmed,
            (Submitted, Cancelled, OrderInput::Cancel { .. }) => true,
            (Validated, Processing, OrderInput::StartProcessing { .. }) => true,
            (Validated, Cancelled, OrderInput::Cancel { .. }) => true,
            (Processing, Shipped, OrderInput::Ship { .. }) => true,
            (Processing, Cancelled, OrderInput::Cancel { .. }) => true,
            (Shipped, Delivered, OrderInput::Deliver { .. }) => true,
            _ => false,
        }
    }
    
    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
        use OrderState::*;
        match (self, input) {
            (Draft, OrderInput::Submit { .. }) => vec![Submitted],
            (Draft, OrderInput::Cancel { .. }) => vec![Cancelled],
            (Submitted, OrderInput::Validate { payment_confirmed }) => {
                if *payment_confirmed { vec![Validated] } else { vec![Draft] }
            }
            (Submitted, OrderInput::Cancel { .. }) => vec![Cancelled],
            (Validated, OrderInput::StartProcessing { .. }) => vec![Processing],
            (Validated, OrderInput::Cancel { .. }) => vec![Cancelled],
            (Processing, OrderInput::Ship { .. }) => vec![Shipped],
            (Processing, OrderInput::Cancel { .. }) => vec![Cancelled],
            (Shipped, OrderInput::Deliver { .. }) => vec![Delivered],
            _ => vec![],
        }
    }
    
    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output {
        let order_id = "ORDER-123".to_string(); // Simplified for demo
        
        use OrderState::*;
        match (self, target, input) {
            (Draft, Submitted, OrderInput::Submit { customer_email }) => {
                OrderOutput::Event(OrderEvent::OrderSubmitted {
                    order_id,
                    customer_email: customer_email.clone(),
                })
            }
            (Submitted, Validated, OrderInput::Validate { .. }) => {
                OrderOutput::Event(OrderEvent::OrderValidated { order_id })
            }
            (Submitted, Draft, OrderInput::Validate { .. }) => {
                OrderOutput::Error("Payment not confirmed".to_string())
            }
            (Validated, Processing, OrderInput::StartProcessing { warehouse_id }) => {
                OrderOutput::Event(OrderEvent::ProcessingStarted {
                    order_id,
                    warehouse_id: warehouse_id.clone(),
                })
            }
            (Processing, Shipped, OrderInput::Ship { tracking_number }) => {
                OrderOutput::Event(OrderEvent::OrderShipped {
                    order_id,
                    tracking_number: tracking_number.clone(),
                })
            }
            (Shipped, Delivered, OrderInput::Deliver { signature }) => {
                OrderOutput::Event(OrderEvent::OrderDelivered {
                    order_id,
                    signature: signature.clone(),
                })
            }
            (_, Cancelled, OrderInput::Cancel { reason }) => {
                OrderOutput::Event(OrderEvent::OrderCancelled {
                    order_id,
                    reason: reason.clone(),
                })
            }
            _ => OrderOutput::Error("Invalid transition".to_string()),
        }
    }
}

// Demo Moore machine with simple toggle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ToggleState {
    On,
    Off,
}

impl State for ToggleState {
    fn name(&self) -> &'static str {
        match self {
            ToggleState::On => "On",
            ToggleState::Off => "Off",
        }
    }
    
    fn is_terminal(&self) -> bool {
        false // Toggle can always change state
    }
}

impl MooreStateTransitions for ToggleState {
    type Output = TextOutput;
    
    fn can_transition_to(&self, target: &Self) -> bool {
        match (self, target) {
            (ToggleState::On, ToggleState::Off) => true,
            (ToggleState::Off, ToggleState::On) => true,
            _ => false,
        }
    }
    
    fn valid_transitions(&self) -> Vec<Self> {
        match self {
            ToggleState::On => vec![ToggleState::Off],
            ToggleState::Off => vec![ToggleState::On],
        }
    }
    
    fn entry_output(&self) -> Self::Output {
        TextOutput {
            message: match self {
                ToggleState::On => "System is ON".to_string(),
                ToggleState::Off => "System is OFF".to_string(),
            }
        }
    }
}

// Define aggregates for the state machines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderAggregate;

impl AggregateRoot for OrderAggregate {
    type Id = EntityId<OrderAggregate>;
    
    fn id(&self) -> Self::Id {
        EntityId::new()
    }
    
    fn version(&self) -> u64 {
        1
    }
    
    fn increment_version(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SystemAggregate;

impl AggregateRoot for SystemAggregate {
    type Id = EntityId<SystemAggregate>;
    
    fn id(&self) -> Self::Id {
        EntityId::new()
    }
    
    fn version(&self) -> u64 {
        1
    }
    
    fn increment_version(&mut self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocumentAggregate;

impl AggregateRoot for DocumentAggregate {
    type Id = EntityId<DocumentAggregate>;
    
    fn id(&self) -> Self::Id {
        EntityId::new()
    }
    
    fn version(&self) -> u64 {
        1
    }
    
    fn increment_version(&mut self) {}
}

fn main() {
    println!("=== CIM State Machine Demo ===\n");
    
    // 1. Moore Machine Demo - Simple Toggle
    println!("1️⃣ Moore Machine Demo - Toggle Switch");
    println!("   (Output depends only on current state)\n");
    
    let toggle_id = EntityId::<SystemAggregate>::new();
    let mut toggle = MooreMachine::new(ToggleState::Off, toggle_id);
    println!("   Initial state: {:?}", toggle.current_state());
    println!("   Output: {}", toggle.current_state().entry_output().message);
    
    // Transition the toggle
    if let Ok(transition) = toggle.transition_to(ToggleState::On) {
        println!("   Transitioned to: {:?}", transition.to);
        println!("   Message: {}", transition.output.message);
        println!("   Current state: {:?}", toggle.current_state());
    }
    
    if let Ok(transition) = toggle.transition_to(ToggleState::Off) {
        println!("   Transitioned to: {:?}", transition.to);
        println!("   Message: {}", transition.output.message);
        println!("   Current state: {:?}", toggle.current_state());
    }
    
    // 2. Moore Machine Demo - Order Workflow
    println!("\n2️⃣ Moore Machine Demo - Order Workflow");
    println!("   (Using order states with Moore semantics)\n");
    
    let order_id = EntityId::<OrderAggregate>::new();
    let mut order_moore = MooreMachine::new(OrderState::Draft, order_id);
    println!("   Initial state: {:?}", order_moore.current_state());
    println!("   Status: {}", order_moore.current_state().entry_output().message);
    
    // Move through workflow
    let transitions = vec![
        (OrderState::Submitted, "submitting"),
        (OrderState::Validated, "validating"),
        (OrderState::Processing, "processing"),
        (OrderState::Shipped, "shipping"),
        (OrderState::Delivered, "delivering"),
    ];
    
    for (next_state, action) in transitions {
        println!("\n   {} order...", action);
        match order_moore.transition_to(next_state) {
            Ok(transition) => {
                println!("   ✅ {}", transition.output.message);
                println!("   Current state: {:?}", order_moore.current_state());
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }
    }
    
    // 3. Mealy Machine Demo - Order Workflow with Inputs
    println!("\n3️⃣ Mealy Machine Demo - Order Workflow");
    println!("   (Output depends on state AND input)\n");
    
    let mealy_order_id = EntityId::<OrderAggregate>::new();
    let mut order_mealy = MealyMachine::new(OrderState::Draft, mealy_order_id);
    println!("   Initial state: {:?}", order_mealy.current_state());
    
    // Submit order
    let submit_input = OrderInput::Submit {
        customer_email: "customer@example.com".to_string(),
    };
    
    println!("\n   Submitting order...");
    // For Mealy machines, we need to find the next state manually
    let next_states = order_mealy.valid_next_states(&submit_input);
    if let Some(next_state) = next_states.first() {
        match order_mealy.transition_to(*next_state, submit_input) {
            Ok(transition) => {
                match &transition.output {
                    OrderOutput::Event(event) => {
                        println!("   ✅ Event generated: {:?}", event.event_type());
                        println!("   Subject: {}", event.subject());
                    }
                    OrderOutput::Error(err) => println!("   ❌ Error: {}", err),
                }
                println!("   Current state: {:?}", order_mealy.current_state());
            }
            Err(e) => println!("   ❌ Transition error: {}", e),
        }
    }
    
    // Validate order
    let validate_input = OrderInput::Validate {
        payment_confirmed: true,
    };
    
    println!("\n   Validating order...");
    let next_states = order_mealy.valid_next_states(&validate_input);
    if let Some(next_state) = next_states.first() {
        match order_mealy.transition_to(*next_state, validate_input) {
            Ok(transition) => {
                if let OrderOutput::Event(event) = &transition.output {
                    println!("   ✅ Event: {:?}", event.event_type());
                    println!("   Current state: {:?}", order_mealy.current_state());
                }
            }
            _ => {}
        }
    }
    
    // Start processing
    let process_input = OrderInput::StartProcessing {
        warehouse_id: "WH-001".to_string(),
    };
    
    println!("\n   Starting processing...");
    let next_states = order_mealy.valid_next_states(&process_input);
    if let Some(next_state) = next_states.first() {
        match order_mealy.transition_to(*next_state, process_input) {
            Ok(transition) => {
                if let OrderOutput::Event(event) = &transition.output {
                    println!("   ✅ Event: {:?}", event.event_type());
                    println!("   Current state: {:?}", order_mealy.current_state());
                }
            }
            _ => {}
        }
    }
    
    // Try invalid transition
    println!("\n   Attempting to deliver directly from processing (invalid)...");
    let deliver_input = OrderInput::Deliver {
        signature: "John Doe".to_string(),
    };
    
    let next_states = order_mealy.valid_next_states(&deliver_input);
    if next_states.is_empty() {
        println!("   ❌ Expected: No valid transitions (correct!)");
    } else {
        println!("   Unexpected: Found valid transitions!");
    }
    
    // 4. Using built-in DocumentState
    println!("\n4️⃣ Built-in State Machine - DocumentState");
    println!("   (Predefined document lifecycle states)\n");
    
    let doc_id = EntityId::<DocumentAggregate>::new();
    let mut doc_state = MooreMachine::new(DocumentState::Draft, doc_id);
    println!("   Initial: {:?}", doc_state.current_state());
    
    // Document workflow
    let doc_transitions = vec![
        (DocumentState::UnderReview, "submitting for review"),
        (DocumentState::Published, "publishing"),
        (DocumentState::Archived, "archiving"),
    ];
    
    for (next_state, action) in doc_transitions {
        println!("\n   {} document...", action);
        match doc_state.transition_to(next_state) {
            Ok(transition) => {
                println!("   ✅ Transitioned to: {:?}", transition.to);
                println!("   Current state: {:?}", doc_state.current_state());
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }
    }
    
    // 5. State Machine Summary
    println!("\n5️⃣ State Machine Summary\n");
    
    println!("   Moore Machines:");
    println!("   - Output depends only on current state");
    println!("   - Simple state transitions");
    println!("   - Good for status indicators");
    
    println!("\n   Mealy Machines:");
    println!("   - Output depends on state AND input");
    println!("   - Rich transitions with context");
    println!("   - Good for workflows and processes");
    
    println!("\n   Key Benefits:");
    println!("   - Type-safe state transitions");
    println!("   - Invalid transitions prevented at compile time");
    println!("   - Clear business logic encoding");
    println!("   - Event generation from state changes");
    println!("   - Terminal states enforce finality");
    
    println!("\n✅ State Machine demo completed!");
    println!("\n💡 In CIM's architecture:");
    println!("   - State machines encode business rules");
    println!("   - Transitions generate domain events");
    println!("   - Invalid transitions are impossible");
    println!("   - States and transitions are first-class types");
}