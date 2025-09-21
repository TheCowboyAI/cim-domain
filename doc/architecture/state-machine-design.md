<!-- Copyright (c) 2025 - Cowboy AI, LLC. -->

# State Machine Design in `cim-domain`

`cim-domain` models lifecycle and orchestration with explicit state machines. Rather than relying on implicit business rules, every allowed transition is encoded in the type system and verified by tests.

## Traits You Implement

```rust
use cim_domain::state_machine::{State, MealyStateTransitions, TransitionInput, TransitionOutput};

pub trait State: Debug + Clone + PartialEq + Eq + Send + Sync {
    fn name(&self) -> &'static str;
    fn is_terminal(&self) -> bool { false }
}

pub trait MealyStateTransitions: State {
    type Input: TransitionInput;
    type Output: TransitionOutput;

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool;
    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self>;
    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output;
}
```

A *Mealy* machine is used for domain flows where transitions may emit events. The `Output` type lets you attach structured data (for example, acknowledgments or domain events) to each transition.

## Production Example: `TransactionState`

`src/transaction_state.rs` contains the production implementation used by the BDD harness.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState { Idle, Started, Applied, Committed, Cancelled, Failed }

#[derive(Debug, Clone)]
pub enum TransactionInput { Start, ValidateOk, ValidateFail, Commit, Cancel }

impl MealyStateTransitions for TransactionState {
    type Input = TransactionInput;
    type Output = TxOutput; // wraps Vec<Box<dyn DomainEvent>>

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool { /* pattern match */ }
    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> { /* same information */ }
    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output {
        // emit deterministic transaction events for the BDD harness
    }
}
```

### Why Events Are Emitted Here

The transaction BDD (`examples/domain_examples/tests/bdd_transaction_state.rs`) asserts the full event stream for every scenario. The output of `transition_output` now contains a boxed `TransactionEvent` with a deterministic `event_type`. This makes it trivial to prove in tests that every happy path and rejection produces the expected domain events.

## Pattern for New State Machines

1. **Model your states and inputs as enums.** Avoid “stringly typed” transitions.
2. **Implement `MealyStateTransitions`.** The compiler forces you to enumerate allowed transitions.
3. **Return structured output.** Wrap domain events, acknowledgments, or audit data in a dedicated output type.
4. **Use the helper traits (`TransitionInput`, `TransitionOutput`)** to keep implementations ergonomic.
5. **Add exhaustive tests.** Unit tests that cover all inputs (and a BDD scenario when behaviour crosses module boundaries).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approver_can_escalate() {
        let next = ApprovalState::Pending.valid_transitions(&ApprovalInput::Escalate);
        assert_eq!(next, vec![ApprovalState::Escalated]);
    }
}
```

## Composition & Sagas

For orchestration across aggregates, use `saga::Saga` and `vector_clock::VectorClock`. A saga coordinates multiple state machines and keeps participant order deterministic without introducing infrastructure dependencies.

## When to Use a Moore Machine

`state_machine::MooreStateTransitions` is still available for the cases where output depends only on the resulting state (not on the transition itself). Most domain flows require Mealy semantics because they need to emit events that depend on the input, which is why the production code favours `MealyStateTransitions`.

## Reference Materials

- `src/state_machine.rs` – trait definitions and helper types
- `src/transaction_state.rs` – production grade example with events
- `examples/domain_examples/tests/bdd_transaction_state.rs` – BDD proof of the transition lattice

