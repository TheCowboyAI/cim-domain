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

## Generic Example (Domain-Neutral)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum Lifecycle {
    Init,
    Active,
    Closed,
}

impl cim_domain::state_machine::State for Lifecycle {
    fn name(&self) -> &'static str {
        match self { Self::Init => "Init", Self::Active => "Active", Self::Closed => "Closed" }
    }
}

impl cim_domain::state_machine::StateTransitions for Lifecycle {
    fn can_transition_to(&self, target: &Self) -> bool {
        matches!((self, target), (Self::Init, Self::Active) | (Self::Active, Self::Closed))
    }
    fn valid_transitions(&self) -> Vec<Self> { use Lifecycle::*; match self { Init => vec![Active], Active => vec![Closed], Closed => vec![] } }
}

use cim_domain::state_machine::StateMachine;
let mut sm = StateMachine::new(Lifecycle::Init);
assert!(sm.transition_to(Lifecycle::Active).is_ok());
```

This example demonstrates the pattern without relying on any specific domain semantics.

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
