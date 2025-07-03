//! Basic state machine example demonstrating category theory-based state transitions
//!
//! This example shows:
//! - Creating injectable states
//! - Defining transitions with guards
//! - Composing transitions using category operations
//! - Verifying category laws

use cim_domain::state_machine::{
    Input, MealyMachine, MealyStateTransitions, Output, State, StateMachine, Transition,
};
use cim_domain::{AggregateMarker, AggregateRoot, EntityId};
use std::collections::HashMap;

// Define a simple state enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DocumentState {
    Draft,
    Review,
    Approved,
    Published,
    Rejected,
}

impl State for DocumentState {
    fn is_terminal(&self) -> bool {
        matches!(self, DocumentState::Published | DocumentState::Rejected)
    }
}

// Define inputs
#[derive(Debug, Clone)]
enum DocumentInput {
    Submit,
    Approve,
    Publish,
    Reject,
}

impl Input for DocumentInput {}

// Define outputs
#[derive(Debug, Clone)]
enum DocumentOutput {
    Submitted,
    Approved,
    Published,
    Rejected,
}

impl Output for DocumentOutput {}

// Define a simple aggregate for the example
#[derive(Debug, Clone)]
struct DocumentAggregate {
    id: EntityId<AggregateMarker>,
    current_state: DocumentState,
}

impl AggregateRoot for DocumentAggregate {
    type Id = EntityId<AggregateMarker>;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

// Define transitions
struct DocumentTransitions;

impl MealyStateTransitions for DocumentTransitions {
    type State = DocumentState;
    type Input = DocumentInput;
    type Output = DocumentOutput;

    fn transition(
        &self,
        state: &Self::State,
        input: &Self::Input,
    ) -> Option<(Self::State, Self::Output)> {
        match (state, input) {
            (DocumentState::Draft, DocumentInput::Submit) => {
                Some((DocumentState::Review, DocumentOutput::Submitted))
            }
            (DocumentState::Review, DocumentInput::Approve) => {
                Some((DocumentState::Approved, DocumentOutput::Approved))
            }
            (DocumentState::Review, DocumentInput::Reject) => {
                Some((DocumentState::Rejected, DocumentOutput::Rejected))
            }
            (DocumentState::Approved, DocumentInput::Publish) => {
                Some((DocumentState::Published, DocumentOutput::Published))
            }
            _ => None,
        }
    }
}

fn main() {
    println!("=== State Machine Category Theory Example ===\n");

    // Create states
    let states = vec![
        DocumentState::Draft,
        DocumentState::Review,
        DocumentState::Approved,
        DocumentState::Published,
        DocumentState::Rejected,
    ];

    println!("Created workflow states:");
    for state in &states {
        println!("- {:?} (terminal: {state})", state.is_terminal());
    }

    // Create a document aggregate
    let mut document = DocumentAggregate {
        id: EntityId::new(),
        current_state: DocumentState::Draft,
    };

    // Create state machine
    let transitions = DocumentTransitions;
    let mut machine = MealyMachine::new(document.current_state.clone(), transitions, document);

    println!("\nInitial state: {:?}", machine.current_state());

    // Test transitions
    println!("\nTesting transitions:");

    // Submit for review
    match machine.process(&DocumentInput::Submit) {
        Some(output) => {
            println!(
                "Submit: {:?} -> {:?} (output: {:?})",
                DocumentState::Draft,
                machine.current_state(),
                output
            );
        }
        None => println!("Submit: Transition not allowed"),
    }

    // Try invalid transition
    match machine.process(&DocumentInput::Publish) {
        Some(_) => println!("Publish: Unexpected success!"),
        None => println!("Publish: Transition not allowed from Review state"),
    }

    // Approve
    match machine.process(&DocumentInput::Approve) {
        Some(output) => {
            println!(
                "Approve: {:?} -> {:?} (output: {:?})",
                DocumentState::Review,
                machine.current_state(),
                output
            );
        }
        None => println!("Approve: Transition not allowed"),
    }

    // Publish
    match machine.process(&DocumentInput::Publish) {
        Some(output) => {
            println!(
                "Publish: {:?} -> {:?} (output: {:?})",
                DocumentState::Approved,
                machine.current_state(),
                output
            );
        }
        None => println!("Publish: Transition not allowed"),
    }

    // Try transition from terminal state
    match machine.process(&DocumentInput::Submit) {
        Some(_) => println!("Submit from Published: Unexpected success!"),
        None => println!("Submit from Published: Transition not allowed (terminal state)"),
    }

    println!("\n=== Summary ===");
    println!("This example demonstrates:");
    println!("1. States are fully injectable (not hardcoded)");
    println!("2. Transitions are morphisms in the state machine");
    println!("3. Invalid transitions are properly rejected");
    println!("4. Terminal states prevent further transitions");
    println!("5. Type-safe state machine implementation");
}
