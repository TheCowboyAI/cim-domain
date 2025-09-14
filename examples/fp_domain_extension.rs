//! Example: Extending CIM Domain with Functional Programming Patterns
//! 
//! This example shows how to add Entity as MONAD and formal domain traits
//! to the existing cim-domain library.

use cim_domain::{Entity as CimEntity, EntityId as CimEntityId};
use std::marker::PhantomData;
use std::sync::Arc;

// ============================================================================
// FORMAL DOMAIN TRAITS (Add these to your domain)
// ============================================================================

/// Marker trait for all domain concepts
pub trait DomainConcept: Send + Sync + 'static {}

/// Value Objects are immutable and compared by value
pub trait ValueObject: DomainConcept + Clone + PartialEq + Eq {}

/// Domain Entities have identity beyond their attributes
pub trait DomainEntity: DomainConcept {
    type Id: EntityId;
    fn id(&self) -> Self::Id;
}

/// Entity IDs are type-safe identifiers
pub trait EntityId: Clone + PartialEq + Eq + std::hash::Hash + Send + Sync {}

/// Aggregates are consistency boundaries with state machines
pub trait Aggregate: DomainEntity + MealyStateMachine {
    type State: AggregateState;
    type Command: DomainCommand;
    type Event: DomainEvent;
    
    fn state(&self) -> Self::State;
    fn handle(self, cmd: Self::Command) -> Result<(Self, Vec<Self::Event>), DomainError>
    where Self: Sized;
}

pub trait DomainCommand: Send + Sync {}
pub trait DomainEvent: Send + Sync {}

pub trait AggregateState: Clone + PartialEq + Send + Sync {
    fn all_states() -> Vec<Self>;
    fn initial() -> Self;
    fn is_terminal(&self) -> bool { false }
}

// ============================================================================
// MEALY STATE MACHINE (Output depends on State AND Input)
// ============================================================================

pub trait MealyStateMachine {
    type State: Clone + PartialEq;
    type Input;
    type Output;
    
    fn transition(&self, state: Self::State, input: Self::Input) -> Self::State;
    fn output(&self, state: Self::State, input: Self::Input) -> Self::Output;
    
    fn step(&self, state: Self::State, input: Self::Input) -> (Self::State, Self::Output)
    where Self::Input: Clone
    {
        let output = self.output(state.clone(), input.clone());
        let new_state = self.transition(state, input);
        (new_state, output)
    }
}

// ============================================================================
// ENTITY MONAD (The bridge between DDD and ECS)
// ============================================================================

/// Entity is the MONAD M where M(A) wraps type A with identity and components
#[derive(Clone, Debug)]
pub struct EntityMonad<A> {
    pub id: TypedEntityId<A>,
    pub components: Components<A>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypedEntityId<A> {
    value: uuid::Uuid,
    _phantom: PhantomData<A>,
}

impl<A> TypedEntityId<A> {
    pub fn new() -> Self {
        Self {
            value: uuid::Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }
}

impl<A: Send + Sync + Clone + PartialEq + Eq + std::hash::Hash + 'static> EntityId for TypedEntityId<A> {}

#[derive(Clone, Debug)]
pub struct Components<A> {
    pub(crate) data: Arc<dyn std::any::Any + Send + Sync>,
    _phantom: PhantomData<A>,
}

impl<A: 'static + Send + Sync> EntityMonad<A> {
    /// return/pure: Lift a value into the monad
    pub fn pure(value: A) -> EntityMonad<A> {
        EntityMonad {
            id: TypedEntityId::new(),
            components: Components {
                data: Arc::new(value),
                _phantom: PhantomData,
            },
        }
    }
    
    /// bind/flatMap: M(A) -> (A -> M(B)) -> M(B)
    pub fn bind<B, F>(self, f: F) -> EntityMonad<B>
    where
        F: FnOnce(A) -> EntityMonad<B>,
        A: Clone,
        B: Send + Sync + 'static,
    {
        let value = self.components.data
            .downcast_ref::<A>()
            .expect("Type mismatch in Entity monad")
            .clone();
        f(value)
    }
    
    /// map: Functor operation
    pub fn map<B, F>(self, f: F) -> EntityMonad<B>
    where
        F: FnOnce(A) -> B,
        A: Clone,
        B: Send + Sync + 'static,
    {
        self.bind(|a| EntityMonad::pure(f(a)))
    }
}

// Helper to extract value from Entity at module boundaries
// BREAKING FP: Entity extraction at module boundaries
// REASON: Need to bridge monadic and non-monadic code at system boundaries
pub fn run_entity<A: Clone + Send + Sync + 'static>(entity: EntityMonad<A>) -> A {
    entity.components.data
        .downcast_ref::<A>()
        .expect("Type mismatch in Entity extraction")
        .clone()
}

// ============================================================================
// DOMAIN ERRORS
// ============================================================================

#[derive(Debug, Clone)]
pub enum DomainError {
    InvalidTransition(String),
    ValidationFailed(String),
    NotFound(String),
}

// ============================================================================
// EXAMPLE: Creating a Domain with these patterns
// ============================================================================

// 1. Define your value objects
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderId(String);
impl DomainConcept for OrderId {}
impl ValueObject for OrderId {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount(u64);
impl DomainConcept for Amount {}
impl ValueObject for Amount {}

// 2. Define your aggregate state
#[derive(Debug, Clone, PartialEq)]
pub enum OrderState {
    Draft,
    Submitted,
    Approved,
    Shipped,
    Delivered,
}

impl AggregateState for OrderState {
    fn all_states() -> Vec<Self> {
        vec![
            OrderState::Draft,
            OrderState::Submitted,
            OrderState::Approved,
            OrderState::Shipped,
            OrderState::Delivered,
        ]
    }
    
    fn initial() -> Self {
        OrderState::Draft
    }
    
    fn is_terminal(&self) -> bool {
        matches!(self, OrderState::Delivered)
    }
}

// 3. Define commands and events
#[derive(Debug, Clone)]
pub enum OrderCommand {
    Submit,
    Approve,
    Ship,
    Deliver,
}
impl DomainCommand for OrderCommand {}

#[derive(Debug, Clone)]
pub enum OrderEvent {
    Submitted,
    Approved,
    Shipped,
    Delivered,
}
impl DomainEvent for OrderEvent {}

// 4. Define your aggregate
pub struct OrderAggregate {
    id: TypedEntityId<OrderAggregate>,
    state: OrderState,
    amount: Amount,
}

impl DomainConcept for OrderAggregate {}
impl DomainEntity for OrderAggregate {
    type Id = TypedEntityId<OrderAggregate>;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl MealyStateMachine for OrderAggregate {
    type State = OrderState;
    type Input = OrderCommand;
    type Output = Vec<OrderEvent>;
    
    fn transition(&self, state: Self::State, input: Self::Input) -> Self::State {
        match (state, input) {
            (OrderState::Draft, OrderCommand::Submit) => OrderState::Submitted,
            (OrderState::Submitted, OrderCommand::Approve) => OrderState::Approved,
            (OrderState::Approved, OrderCommand::Ship) => OrderState::Shipped,
            (OrderState::Shipped, OrderCommand::Deliver) => OrderState::Delivered,
            (state, _) => state, // Invalid transitions stay in same state
        }
    }
    
    fn output(&self, state: Self::State, input: Self::Input) -> Self::Output {
        match (state, input) {
            (OrderState::Draft, OrderCommand::Submit) => vec![OrderEvent::Submitted],
            (OrderState::Submitted, OrderCommand::Approve) => vec![OrderEvent::Approved],
            (OrderState::Approved, OrderCommand::Ship) => vec![OrderEvent::Shipped],
            (OrderState::Shipped, OrderCommand::Deliver) => vec![OrderEvent::Delivered],
            _ => vec![], // Invalid transitions produce no events
        }
    }
}

impl Aggregate for OrderAggregate {
    type State = OrderState;
    type Command = OrderCommand;
    type Event = OrderEvent;
    
    fn state(&self) -> Self::State {
        self.state.clone()
    }
    
    fn handle(mut self, cmd: Self::Command) -> Result<(Self, Vec<Self::Event>), DomainError> {
        let (new_state, events) = self.step(self.state.clone(), cmd);
        
        if events.is_empty() {
            return Err(DomainError::InvalidTransition(
                format!("Cannot transition from {:?}", self.state)
            ));
        }
        
        // BREAKING FP: Mutating self for performance
        // REASON: Avoiding expensive clone of entire aggregate
        self.state = new_state;
        
        Ok((self, events))
    }
}

// ============================================================================
// EXAMPLE: Using the Entity Monad
// ============================================================================

fn example_entity_monad() {
    // Create an order in the monad
    let order = OrderAggregate {
        id: TypedEntityId::new(),
        state: OrderState::Draft,
        amount: Amount(100),
    };
    
    // Lift into Entity monad
    let entity = EntityMonad::pure(order);
    
    // Chain operations monadically
    let result = entity
        .map(|order| {
            println!("Processing order with amount: {:?}", order.amount);
            order
        })
        .bind(|order| {
            // Process and return new entity
            let mut updated = order;
            updated.state = OrderState::Submitted;
            EntityMonad::pure(updated)
        });
    
    // Extract at module boundary
    let final_order = run_entity(result);
    assert_eq!(final_order.state, OrderState::Submitted);
}

// ============================================================================
// INTEGRATION: Bridge with existing cim-domain
// ============================================================================

/// Bridge between our FP patterns and cim-domain
pub struct DomainBridge<T> {
    /// The cim-domain entity
    pub cim_entity: CimEntity<T>,
    /// Our monadic wrapper
    pub fp_entity: EntityMonad<T>,
}

impl<T: Clone + Send + Sync + 'static> DomainBridge<T> {
    /// Create from cim-domain entity
    pub fn from_cim(entity: CimEntity<T>, data: T) -> Self {
        Self {
            cim_entity: entity,
            fp_entity: EntityMonad::pure(data),
        }
    }
    
    /// Apply monadic transformation
    pub fn transform<U, F>(self, f: F) -> DomainBridge<U>
    where
        F: FnOnce(T) -> U,
        U: Clone + Send + Sync + 'static,
    {
        DomainBridge {
            cim_entity: CimEntity::new(),
            fp_entity: self.fp_entity.map(f),
        }
    }
}

fn main() {
    println!("CIM Domain with FP Extensions");
    println!("==============================");
    
    // Example 1: Using the Entity Monad
    println!("\n1. Entity Monad Example:");
    example_entity_monad();
    
    // Example 2: Using Mealy State Machine
    println!("\n2. Mealy State Machine Example:");
    let order = OrderAggregate {
        id: TypedEntityId::new(),
        state: OrderState::Draft,
        amount: Amount(250),
    };
    
    match order.handle(OrderCommand::Submit) {
        Ok((updated, events)) => {
            println!("   State: {:?} -> {:?}", OrderState::Draft, updated.state);
            println!("   Events: {:?}", events);
        }
        Err(e) => println!("   Error: {:?}", e),
    }
    
    // Example 3: Bridging with cim-domain
    println!("\n3. Bridging with cim-domain:");
    let cim_entity = CimEntity::<String>::new();
    let bridge = DomainBridge::from_cim(cim_entity, "Hello".to_string());
    let transformed = bridge.transform(|s| s.to_uppercase());
    let result = run_entity(transformed.fp_entity);
    println!("   Transformed: {}", result);
    
    println!("\nAll examples completed successfully!");
}