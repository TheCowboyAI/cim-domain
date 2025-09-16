# Formal Domain Structure: The Complete Algebraic System

## Core Domain Traits (Marker Traits for Type Safety)

```rust
// ============================================================================
// PRIMITIVE DDD MARKER TRAITS
// ============================================================================

/// Marker trait for all domain concepts
pub trait DomainConcept: Send + Sync + 'static {}

/// Value Objects are immutable and compared by value
pub trait ValueObject: DomainConcept + Clone + PartialEq + Eq {}

/// Entities have identity beyond their attributes
pub trait DomainEntity: DomainConcept {
    type Id: EntityId;
    fn id(&self) -> Self::Id;
}

/// Aggregates are consistency boundaries with state machines
pub trait Aggregate: DomainEntity + MealyStateMachine {
    type State: AggregateState;
    type Command: DomainCommand;
    type Event: DomainEvent;
    
    /// Current state of the aggregate
    fn state(&self) -> Self::State;
    
    /// Apply command to produce new state and events (Mealy output)
    fn handle(self, cmd: Self::Command) -> Result<(Self, Vec<Self::Event>), DomainError>;
}

/// Policies are pure business rules
pub trait Policy<A: Aggregate>: DomainConcept {
    fn evaluate(&self, aggregate: &A) -> PolicyResult;
}

/// Domain Services coordinate between aggregates
pub trait DomainService: DomainConcept {
    type Input;
    type Output;
    
    fn execute(&self, input: Self::Input) -> Result<Self::Output, DomainError>;
}
```

## Mealy State Machine Foundation

```rust
// ============================================================================
// MEALY STATE MACHINE (Output depends on State AND Input)
// ============================================================================

pub trait MealyStateMachine {
    type State: Clone + PartialEq;
    type Input;
    type Output;
    
    /// Transition function: δ: State × Input → State
    fn transition(&self, state: Self::State, input: Self::Input) -> Self::State;
    
    /// Output function: λ: State × Input → Output
    fn output(&self, state: Self::State, input: Self::Input) -> Self::Output;
    
    /// Combined step (returns new state and output)
    fn step(&self, state: Self::State, input: Self::Input) -> (Self::State, Self::Output) {
        let output = self.output(state.clone(), input.clone());
        let new_state = self.transition(state, input);
        (new_state, output)
    }
}

/// Aggregate states form a finite set
pub trait AggregateState: Clone + PartialEq + Send + Sync {
    /// All possible states (for graph construction)
    fn all_states() -> Vec<Self>;
    
    /// Initial state
    fn initial() -> Self;
    
    /// Terminal states (if any)
    fn is_terminal(&self) -> bool { false }
}
```

## State Transition Graph

```rust
// ============================================================================
// DIRECTED STATE TRANSITION GRAPH
// ============================================================================

pub struct StateTransitionGraph<S: AggregateState, I> {
    /// Adjacency list representation
    transitions: HashMap<(S, I), S>,
    
    /// Transition probabilities (for Markov chain)
    probabilities: HashMap<(S, I), f64>,
}

impl<S: AggregateState, I: Clone> StateTransitionGraph<S, I> {
    /// Build graph from state machine
    pub fn from_state_machine<M: MealyStateMachine<State = S, Input = I>>(
        machine: &M,
        inputs: Vec<I>,
    ) -> Self {
        let mut transitions = HashMap::new();
        let mut probabilities = HashMap::new();
        
        for state in S::all_states() {
            for input in &inputs {
                let new_state = machine.transition(state.clone(), input.clone());
                transitions.insert((state.clone(), input.clone()), new_state);
                
                // Calculate transition probability (domain-specific)
                let prob = calculate_transition_probability(&state, input);
                probabilities.insert((state, input.clone()), prob);
            }
        }
        
        StateTransitionGraph { transitions, probabilities }
    }
    
    /// Check if graph is strongly connected
    pub fn is_strongly_connected(&self) -> bool {
        // Implementation of Tarjan's algorithm
        true // simplified
    }
    
    /// Find all paths between states
    pub fn find_paths(&self, from: S, to: S) -> Vec<Vec<(S, I)>> {
        // DFS/BFS to find paths
        vec![] // simplified
    }
}
```

## Markov Chain Properties

```rust
// ============================================================================
// MARKOV CHAIN (Stochastic Process)
// ============================================================================

pub struct MarkovChain<S: AggregateState> {
    /// Transition probability matrix
    transition_matrix: Matrix<f64>,
    
    /// State space
    states: Vec<S>,
    
    /// Current probability distribution
    distribution: Vec<f64>,
}

impl<S: AggregateState> MarkovChain<S> {
    /// Create from state transition graph
    pub fn from_graph<I>(graph: &StateTransitionGraph<S, I>) -> Self {
        let states = S::all_states();
        let n = states.len();
        let mut matrix = Matrix::zeros(n, n);
        
        // Build transition matrix from graph probabilities
        for (i, state_i) in states.iter().enumerate() {
            for (j, state_j) in states.iter().enumerate() {
                let prob = graph.probability_between(state_i, state_j);
                matrix[(i, j)] = prob;
            }
        }
        
        MarkovChain {
            transition_matrix: matrix,
            states,
            distribution: uniform_distribution(n),
        }
    }
    
    /// Calculate steady-state distribution
    pub fn steady_state(&self) -> Vec<f64> {
        // Power iteration or eigenvalue decomposition
        self.distribution.clone() // simplified
    }
    
    /// Expected time to reach state
    pub fn expected_hitting_time(&self, target: S) -> f64 {
        // Solve system of linear equations
        0.0 // simplified
    }
}
```

## Saga: Composed Aggregate

```rust
// ============================================================================
// SAGA: AGGREGATE OF AGGREGATES
// ============================================================================

/// Saga composes multiple aggregates with coordinated state transitions
pub trait Saga: DomainConcept {
    /// The aggregates this saga coordinates
    type Aggregates: AggregateCollection;
    
    /// Saga's own state (derived from aggregate states)
    type State: SagaState;
    
    /// Commands that span aggregates
    type Command: SagaCommand;
    
    /// Events from saga coordination
    type Event: SagaEvent;
    
    /// Current saga state (computed from aggregate states)
    fn state(&self) -> Self::State;
    
    /// Handle saga command (may affect multiple aggregates)
    fn handle(&mut self, cmd: Self::Command) -> Result<Vec<Self::Event>, SagaError>;
    
    /// Compensate on failure (saga pattern)
    fn compensate(&mut self, failed_step: usize) -> Result<(), SagaError>;
}

/// Collection of aggregates managed by saga
pub trait AggregateCollection {
    type Item: Aggregate;
    
    fn get(&self, id: EntityId<Self::Item>) -> Option<&Self::Item>;
    fn get_mut(&mut self, id: EntityId<Self::Item>) -> Option<&mut Self::Item>;
    fn all(&self) -> Vec<&Self::Item>;
}

/// Saga state is computed from aggregate states
pub trait SagaState {
    type AggregateStates;
    
    /// Derive saga state from aggregate states
    fn from_aggregates(states: Self::AggregateStates) -> Self;
    
    /// Check if saga is in valid state
    fn is_valid(&self) -> bool;
    
    /// Terminal condition for saga
    fn is_complete(&self) -> bool;
}
```

## Concrete Example: Order Fulfillment Saga

```rust
// ============================================================================
// EXAMPLE: ORDER FULFILLMENT SAGA
// ============================================================================

/// Order aggregate with Mealy state machine
#[derive(Clone)]
pub struct Order {
    id: EntityId<Order>,
    state: OrderState,
    items: Vec<OrderItem>,
}

#[derive(Clone, PartialEq)]
pub enum OrderState {
    Created,
    Validated,
    Paid,
    Shipped,
    Delivered,
    Cancelled,
}

impl MealyStateMachine for Order {
    type State = OrderState;
    type Input = OrderCommand;
    type Output = Vec<OrderEvent>;
    
    fn transition(&self, state: OrderState, input: OrderCommand) -> OrderState {
        use OrderState::*;
        use OrderCommand::*;
        
        match (state, input) {
            (Created, Validate) => Validated,
            (Validated, Pay) => Paid,
            (Paid, Ship) => Shipped,
            (Shipped, Deliver) => Delivered,
            (_, Cancel) => Cancelled,
            (s, _) => s, // No transition
        }
    }
    
    fn output(&self, state: OrderState, input: OrderCommand) -> Vec<OrderEvent> {
        use OrderState::*;
        use OrderCommand::*;
        
        match (state, input) {
            (Created, Validate) => vec![OrderEvent::Validated],
            (Validated, Pay) => vec![OrderEvent::PaymentProcessed],
            (Paid, Ship) => vec![OrderEvent::Shipped],
            (Shipped, Deliver) => vec![OrderEvent::Delivered],
            (_, Cancel) => vec![OrderEvent::Cancelled],
            _ => vec![],
        }
    }
}

/// Order fulfillment saga coordinates Order, Payment, and Shipping
pub struct OrderFulfillmentSaga {
    order: Order,
    payment: Payment,
    shipping: Shipping,
    state: OrderFulfillmentState,
}

#[derive(Clone, PartialEq)]
pub enum OrderFulfillmentState {
    Started,
    OrderValidated,
    PaymentProcessed,
    ShippingArranged,
    Completed,
    CompensatingPayment,
    CompensatingShipping,
    Failed,
}

impl Saga for OrderFulfillmentSaga {
    type Aggregates = (Order, Payment, Shipping);
    type State = OrderFulfillmentState;
    type Command = FulfillmentCommand;
    type Event = FulfillmentEvent;
    
    fn handle(&mut self, cmd: FulfillmentCommand) -> Result<Vec<FulfillmentEvent>, SagaError> {
        match (self.state.clone(), cmd) {
            (Started, FulfillmentCommand::Start) => {
                // Validate order
                let (new_order, events) = self.order.handle(OrderCommand::Validate)?;
                self.order = new_order;
                self.state = OrderValidated;
                Ok(events.into_iter().map(Into::into).collect())
            }
            (OrderValidated, FulfillmentCommand::ProcessPayment) => {
                // Process payment
                let (new_payment, events) = self.payment.handle(PaymentCommand::Process)?;
                self.payment = new_payment;
                self.state = PaymentProcessed;
                Ok(events.into_iter().map(Into::into).collect())
            }
            (PaymentProcessed, FulfillmentCommand::ArrangeShipping) => {
                // Arrange shipping
                let (new_shipping, events) = self.shipping.handle(ShippingCommand::Schedule)?;
                self.shipping = new_shipping;
                self.state = ShippingArranged;
                Ok(events.into_iter().map(Into::into).collect())
            }
            _ => Err(SagaError::InvalidTransition),
        }
    }
    
    fn compensate(&mut self, failed_step: usize) -> Result<(), SagaError> {
        match failed_step {
            2 => {
                // Shipping failed, refund payment
                self.state = CompensatingPayment;
                self.payment.handle(PaymentCommand::Refund)?;
                Ok(())
            }
            1 => {
                // Payment failed, cancel order
                self.order.handle(OrderCommand::Cancel)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

## The Mathematical Properties

### 1. Aggregates as Mealy Machines
- **Deterministic**: Same input + state always produces same output
- **Compositional**: Can compose machines via product construction
- **Observable**: Output reveals internal state transitions

### 2. State Transition Graphs
- **Reachability**: Can analyze which states are reachable
- **Cycles**: Detect loops in business processes  
- **Critical paths**: Find shortest/longest paths through states

### 3. Markov Chain Properties
- **Ergodicity**: System explores all states over time
- **Steady state**: Long-term behavior predictable
- **Absorption**: Terminal states for process completion

### 4. Saga Composition
- **Hierarchical**: Sagas can contain other sagas
- **Compensatable**: Each step has compensation
- **Eventually consistent**: Aggregates converge to consistent state

## This Gives Us

1. **Formal verification** of business processes
2. **Probabilistic analysis** of system behavior
3. **Composition** of complex workflows from simple aggregates
4. **Mathematical proofs** about system properties
5. **Predictable behavior** under all conditions