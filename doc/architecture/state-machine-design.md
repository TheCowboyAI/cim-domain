<!-- Copyright 2025 Cowboy AI, LLC. -->

# State Machine Design in CIM

## Overview

Following Domain-Driven Design principles, CIM uses enum-based state machines to control and restrict state transitions. This ensures that domain entities can only exist in valid states and can only transition through allowed paths.

## Core Principles

1. **Enums for States**: All states are represented as enum variants, providing compile-time safety
2. **Explicit Transitions**: Valid transitions are explicitly defined, preventing invalid state changes
3. **Transition History**: All state changes are recorded for audit and debugging
4. **Type Safety**: The type system enforces that only valid transitions can be attempted

## Architecture

### State Trait
```rust
pub trait State: Debug + Clone + PartialEq + Eq + Send + Sync {
    fn name(&self) -> &'static str;
}
```

### StateTransitions Trait
```rust
pub trait StateTransitions: State {
    fn can_transition_to(&self, target: &Self) -> bool;
    fn valid_transitions(&self) -> Vec<Self>;
}
```

### StateMachine Generic Type
```rust
pub struct StateMachine<S: StateTransitions> {
    current_state: S,
    transition_history: Vec<StateTransition<S>>,
}
```

## Example: Order State Machine

```rust
pub enum OrderState {
    Draft,
    Submitted,
    PaymentPending,
    PaymentConfirmed,
    Fulfilling,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}
```

### Valid Transitions
- `Draft` → `Submitted`, `Cancelled`
- `Submitted` → `PaymentPending`, `Cancelled`
- `PaymentPending` → `PaymentConfirmed`, `Cancelled`
- `PaymentConfirmed` → `Fulfilling`, `Refunded`
- `Fulfilling` → `Shipped`, `Refunded`
- `Shipped` → `Delivered`, `Refunded`
- `Delivered` → `Refunded`
- `Cancelled` → (terminal state)
- `Refunded` → (terminal state)

## Example: Person State Machine

```rust
pub enum PersonState {
    Registered,
    PendingVerification,
    Verified,
    Active,
    Suspended,
    Deactivated,
    Archived,
}
```

### Valid Transitions
- `Registered` → `PendingVerification`, `Archived`
- `PendingVerification` → `Verified`, `Archived`
- `Verified` → `Active`, `Archived`
- `Active` → `Suspended`, `Deactivated`, `Archived`
- `Suspended` → `Active`, `Deactivated`, `Archived`
- `Deactivated` → `Active`, `Archived`
- `Archived` → (terminal state)

## Usage Pattern

```rust
// Create a new state machine
let mut order = StateMachine::new(OrderState::Draft);

// Attempt a valid transition
match order.transition_to(OrderState::Submitted) {
    Ok(transition) => {
        // Transition successful, emit event
        emit_event(OrderSubmitted {
            order_id,
            transition_id: transition.transition_id,
        });
    }
    Err(DomainError::InvalidStateTransition { from, to }) => {
        // Handle invalid transition
    }
}

// Check current state
if order.is_in_state(&OrderState::PaymentPending) {
    // Process payment
}

// Get valid next states
let next_states = order.valid_next_states();
```

## Integration with Event Sourcing

State transitions naturally map to domain events:

```rust
// When a state transition occurs
let transition = order.transition_to(OrderState::Shipped)?;

// Generate corresponding domain event
let event = OrderShipped {
    order_id: self.id,
    transition_id: transition.transition_id,
    shipped_at: SystemTime::now(),
    tracking_number: generate_tracking_number(),
};

// The event captures both the state change and business data
event_store.append(event).await?;
```

## Benefits

1. **Compile-Time Safety**: Invalid states and transitions are caught at compile time
2. **Self-Documenting**: The enum and transition rules clearly document the domain model
3. **Audit Trail**: Complete history of state changes with transition IDs
4. **Business Rule Enforcement**: State machines enforce business rules about valid workflows
5. **Testing**: Easy to test all valid and invalid transition paths

## Best Practices

1. **Keep States Domain-Focused**: State names should reflect business concepts, not technical details
2. **Minimize State Count**: Only create states that have different business rules or behaviors
3. **Document Terminal States**: Clearly indicate which states are terminal (no outgoing transitions)
4. **Consider Compensating Transitions**: Some states may need "undo" transitions (e.g., Refunded)
5. **Emit Events on Transitions**: Each successful transition should emit a corresponding domain event

## Macro for Concise Definition

For simple state machines, use the provided macro:

```rust
define_state_transitions!(
    MyState,
    StateA => [StateB, StateC],
    StateB => [StateC, StateD],
    StateC => [StateD],
    StateD => [], // Terminal
);
```

This generates the `StateTransitions` implementation automatically.
