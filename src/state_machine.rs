//! State Machine implementation for domain aggregates
//!
//! This module provides both Mealy and Moore state machines for managing
//! aggregate state transitions following DDD principles.
//!
//! - **Moore Machine**: Output depends only on current state
//! - **Mealy Machine**: Output depends on current state AND input
//!
//! Aggregates use these state machines to enforce valid state transitions
//! and maintain consistency.
//!
//! ## Relationship to Enriched Categories
//!
//! In our domain model, aggregates are **Enriched Categories** where:
//! - **Objects**: The states of the aggregate (DocumentState, etc.)
//! - **Morphisms**: State transitions (provided by the state machines)
//!   - **Enrichment**: The "cost" or "distance" of transitions, captured by:
//!     - Event count (number of events generated)
//!     - Semantic distance between states
//!     - Time cost of transitions
//!     - Business value of states
//!
//! The state machines (Moore/Mealy) provide the morphism structure, while
//! the TransitionOutput (especially EventOutput) provides the enrichment.
//!
//! When multiple aggregates compose, they form a **Topos** with internal
//! logic for cross-aggregate invariants and saga orchestration.

use crate::{
    entity::{AggregateRoot, EntityId},
    errors::{DomainError, DomainResult},
    DomainEvent,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;
use uuid::Uuid;

/// Input to a state machine transition
pub trait TransitionInput: Debug + Clone + Send + Sync {
    /// Get a description of this input for logging
    fn description(&self) -> String;
}

/// Output from a state machine transition
pub trait TransitionOutput: Debug + Clone + Send + Sync {
    /// Convert to domain events
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>>;
}

/// Trait for types that can be used as states in a state machine
pub trait State: Debug + Clone + PartialEq + Eq + Send + Sync {
    /// Get the name of this state for logging/debugging
    fn name(&self) -> &'static str;

    /// Check if this is a terminal state
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Moore Machine: Output depends only on current state
///
/// # Examples
///
/// ```rust
/// use cim_domain::state_machine::{State, MooreStateTransitions, TransitionOutput};
/// use cim_domain::DomainEvent;
/// 
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// enum TrafficLight {
///     Red,
///     Yellow,
///     Green,
/// }
/// 
/// #[derive(Debug, Clone)]
/// struct LightOutput {
///     message: String,
/// }
/// 
/// impl TransitionOutput for LightOutput {
///     fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
///         vec![] // Example: no events
///     }
/// }
/// 
/// impl State for TrafficLight {
///     fn name(&self) -> &'static str {
///         match self {
///             TrafficLight::Red => "Red",
///             TrafficLight::Yellow => "Yellow",
///             TrafficLight::Green => "Green",
///         }
///     }
/// }
/// 
/// impl MooreStateTransitions for TrafficLight {
///     type Output = LightOutput;
///     
///     fn can_transition_to(&self, target: &Self) -> bool {
///         match (self, target) {
///             (TrafficLight::Red, TrafficLight::Green) => true,
///             (TrafficLight::Green, TrafficLight::Yellow) => true,
///             (TrafficLight::Yellow, TrafficLight::Red) => true,
///             _ => false,
///         }
///     }
///     
///     fn valid_transitions(&self) -> Vec<Self> {
///         match self {
///             TrafficLight::Red => vec![TrafficLight::Green],
///             TrafficLight::Green => vec![TrafficLight::Yellow],
///             TrafficLight::Yellow => vec![TrafficLight::Red],
///         }
///     }
///     
///     fn entry_output(&self) -> Self::Output {
///         LightOutput {
///             message: format!("Light is now {}", self.name()),
///         }
///     }
/// }
/// 
/// let light = TrafficLight::Red;
/// assert!(light.can_transition_to(&TrafficLight::Green));
/// assert!(!light.can_transition_to(&TrafficLight::Yellow));
/// ```
pub trait MooreStateTransitions: State {
    /// The output type for this state machine
    type Output: TransitionOutput;

    /// Check if a transition to the target state is valid
    fn can_transition_to(&self, target: &Self) -> bool;

    /// Get all valid target states from this state
    fn valid_transitions(&self) -> Vec<Self>;

    /// Get the output for entering this state
    fn entry_output(&self) -> Self::Output;
}

/// Mealy Machine: Output depends on current state AND input
pub trait MealyStateTransitions: State {
    /// The input type for transitions
    type Input: TransitionInput;
    /// The output type for transitions
    type Output: TransitionOutput;

    /// Check if a transition is valid given the input
    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool;

    /// Get valid transitions for a given input
    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self>;

    /// Get the output for a transition
    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output;
}

/// Record of a state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition<S, I, O> {
    /// The state before the transition
    pub from: S,
    /// The state after the transition
    pub to: S,
    /// The input that triggered the transition (if any)
    pub input: Option<I>,
    /// The output produced by the transition
    pub output: O,
    /// Unique identifier for this transition instance
    pub transition_id: Uuid,
    /// When the transition occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Moore state machine for aggregates
#[derive(Debug, Clone)]
pub struct MooreMachine<S: MooreStateTransitions, A: AggregateRoot> {
    current_state: S,
    aggregate_id: EntityId<A>,
    transition_history: Vec<StateTransition<S, (), S::Output>>,
    _phantom: PhantomData<A>,
}

impl<S: MooreStateTransitions, A: AggregateRoot> MooreMachine<S, A> {
    /// Create a new Moore machine for an aggregate
    pub fn new(initial_state: S, aggregate_id: EntityId<A>) -> Self {
        Self {
            current_state: initial_state,
            aggregate_id,
            transition_history: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> &S {
        &self.current_state
    }

    /// Get the aggregate ID
    pub fn aggregate_id(&self) -> &EntityId<A> {
        &self.aggregate_id
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: S) -> DomainResult<StateTransition<S, (), S::Output>> {
        if self.current_state.is_terminal() {
            return Err(DomainError::InvalidStateTransition {
                from: self.current_state.name().to_string(),
                to: new_state.name().to_string(),
            });
        }

        if !self.current_state.can_transition_to(&new_state) {
            return Err(DomainError::InvalidStateTransition {
                from: self.current_state.name().to_string(),
                to: new_state.name().to_string(),
            });
        }

        let output = new_state.entry_output();

        let transition = StateTransition {
            from: self.current_state.clone(),
            to: new_state.clone(),
            input: None,
            output,
            transition_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
        };

        self.current_state = new_state;
        self.transition_history.push(transition.clone());

        Ok(transition)
    }

    /// Get the transition history
    pub fn history(&self) -> &[StateTransition<S, (), S::Output>] {
        &self.transition_history
    }

    /// Check if in a specific state
    pub fn is_in_state(&self, state: &S) -> bool {
        &self.current_state == state
    }

    /// Get valid next states
    pub fn valid_next_states(&self) -> Vec<S> {
        self.current_state.valid_transitions()
    }
}

/// Mealy state machine for aggregates
#[derive(Debug, Clone)]
pub struct MealyMachine<S: MealyStateTransitions, A: AggregateRoot> {
    current_state: S,
    aggregate_id: EntityId<A>,
    transition_history: Vec<StateTransition<S, S::Input, S::Output>>,
    _phantom: PhantomData<A>,
}

impl<S: MealyStateTransitions, A: AggregateRoot> MealyMachine<S, A> {
    /// Create a new Mealy machine for an aggregate
    pub fn new(initial_state: S, aggregate_id: EntityId<A>) -> Self {
        Self {
            current_state: initial_state,
            aggregate_id,
            transition_history: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> &S {
        &self.current_state
    }

    /// Get the aggregate ID
    pub fn aggregate_id(&self) -> &EntityId<A> {
        &self.aggregate_id
    }

    /// Transition to a new state with input
    pub fn transition_to(&mut self, new_state: S, input: S::Input) -> DomainResult<StateTransition<S, S::Input, S::Output>> {
        if self.current_state.is_terminal() {
            return Err(DomainError::InvalidStateTransition {
                from: self.current_state.name().to_string(),
                to: new_state.name().to_string(),
            });
        }

        if !self.current_state.can_transition_to(&new_state, &input) {
            return Err(DomainError::InvalidStateTransition {
                from: self.current_state.name().to_string(),
                to: new_state.name().to_string(),
            });
        }

        let output = self.current_state.transition_output(&new_state, &input);

        let transition = StateTransition {
            from: self.current_state.clone(),
            to: new_state.clone(),
            input: Some(input),
            output,
            transition_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
        };

        self.current_state = new_state;
        self.transition_history.push(transition.clone());

        Ok(transition)
    }

    /// Get the transition history
    pub fn history(&self) -> &[StateTransition<S, S::Input, S::Output>] {
        &self.transition_history
    }

    /// Check if in a specific state
    pub fn is_in_state(&self, state: &S) -> bool {
        &self.current_state == state
    }

    /// Get valid next states for given input
    pub fn valid_next_states(&self, input: &S::Input) -> Vec<S> {
        self.current_state.valid_transitions(input)
    }
}

// Example implementations for common patterns

/// Simple output that contains events
#[derive(Debug, Default)]
pub struct EventOutput {
    /// The domain events produced by the state transition
    pub events: Vec<Box<dyn DomainEvent>>,
}

impl Clone for EventOutput {
    fn clone(&self) -> Self {
        // Since DomainEvent doesn't implement Clone, we return an empty output
        // In practice, outputs are consumed, not cloned
        EventOutput::default()
    }
}

impl EventOutput {
    /// Create a new EventOutput with the given events
    pub fn new(events: Vec<Box<dyn DomainEvent>>) -> Self {
        Self { events }
    }

    /// Create an empty EventOutput
    pub fn empty() -> Self {
        Self::default()
    }
}

impl TransitionOutput for EventOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        // Since we can't clone events, we return an empty vec
        // In real usage, the EventOutput should be consumed, not borrowed
        Vec::new()
    }
}

/// Empty input for simple transitions
#[derive(Debug, Clone)]
pub struct EmptyInput;

impl TransitionInput for EmptyInput {
    fn description(&self) -> String {
        "Empty".to_string()
    }
}

/// Command input for transitions
#[derive(Debug, Clone)]
pub struct CommandInput<C> {
    /// The command that triggers the state transition
    pub command: C,
}

impl<C: Debug + Clone + Send + Sync> TransitionInput for CommandInput<C> {
    fn description(&self) -> String {
        format!("Command: {:?}", self.command)
    }
}

// Example: Document state machine (Moore)
/// Represents the lifecycle states of a document
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentState {
    /// Initial state - document is being created or edited
    Draft,
    /// Document is submitted for review
    UnderReview,
    /// Document has been approved by reviewers
    Approved,
    /// Document is published and available
    Published,
    /// Terminal state - document is archived and read-only
    Archived,
}

impl State for DocumentState {
    fn name(&self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::UnderReview => "UnderReview",
            Self::Approved => "Approved",
            Self::Published => "Published",
            Self::Archived => "Archived",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self, Self::Archived)
    }
}

impl MooreStateTransitions for DocumentState {
    type Output = EventOutput;

    fn can_transition_to(&self, target: &Self) -> bool {
        use DocumentState::*;

        let valid_transitions = match self {
            Draft => vec![UnderReview],
            UnderReview => vec![Draft, Approved],
            Approved => vec![Published, UnderReview],
            Published => vec![Archived],
            Archived => vec![],
        };

        valid_transitions.contains(target)
    }

    fn valid_transitions(&self) -> Vec<Self> {
        use DocumentState::*;

        match self {
            Draft => vec![UnderReview],
            UnderReview => vec![Draft, Approved],
            Approved => vec![Published, UnderReview],
            Published => vec![Archived],
            Archived => vec![],
        }
    }

    fn entry_output(&self) -> Self::Output {
        // In a real implementation, these would create actual domain events
        EventOutput {
            events: vec![], // Placeholder
        }
    }
}



/// Macro to define Moore state transitions more concisely
#[macro_export]
macro_rules! define_moore_transitions {
    ($state_type:ty, $output_type:ty, $($from:pat => [$($to:expr),*]),* $(,)?) => {
        impl MooreStateTransitions for $state_type {
            type Output = $output_type;

            fn can_transition_to(&self, target: &Self) -> bool {
                match self {
                    $($from => {
                        let valid = vec![$($to),*];
                        valid.contains(target)
                    })*
                }
            }

            fn valid_transitions(&self) -> Vec<Self> {
                match self {
                    $($from => vec![$($to),*],)*
                }
            }

            fn entry_output(&self) -> Self::Output {
                Default::default()
            }
        }
    };
}

/// Macro to define Mealy state transitions
#[macro_export]
macro_rules! define_mealy_transitions {
    ($state_type:ty, $input_type:ty, $output_type:ty,
     $(($from:pat, $to:pat, $input:pat) => $valid:expr),* $(,)?) => {
        impl MealyStateTransitions for $state_type {
            type Input = $input_type;
            type Output = $output_type;

            fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
                match (self, target, input) {
                    $(($from, $to, $input) => $valid,)*
                    _ => false,
                }
            }

            fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
                let mut result = Vec::new();
                $(
                    if let ($from, $input) = (self, input) {
                        if let $to = Default::default() {
                            if self.can_transition_to(&$to, input) {
                                result.push($to);
                            }
                        }
                    }
                )*
                result
            }

            fn transition_output(&self, _target: &Self, _input: &Self::Input) -> Self::Output {
                Default::default()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moore_machine_document_state() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct DocumentAggregate;
        impl AggregateRoot for DocumentAggregate {
            type Id = EntityId<DocumentAggregate>;
            fn id(&self) -> Self::Id { EntityId::<DocumentAggregate>::new() }
            fn version(&self) -> u64 { 0 }
            fn increment_version(&mut self) {}
        }

        let aggregate_id = EntityId::<DocumentAggregate>::new();
        let mut machine = MooreMachine::new(DocumentState::Draft, aggregate_id);

        // Valid transition
        assert!(machine.transition_to(DocumentState::UnderReview).is_ok());
        assert_eq!(machine.current_state(), &DocumentState::UnderReview);

        // Invalid transition
        assert!(machine.transition_to(DocumentState::Published).is_err());

        // Valid transitions to terminal state
        assert!(machine.transition_to(DocumentState::Approved).is_ok());
        assert!(machine.transition_to(DocumentState::Published).is_ok());
        assert!(machine.transition_to(DocumentState::Archived).is_ok());
        assert!(machine.transition_to(DocumentState::Draft).is_err()); // Can't transition from terminal
    }



    #[test]
    fn test_moore_machine() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct DocumentAggregate;
        impl AggregateRoot for DocumentAggregate {
            type Id = EntityId<DocumentAggregate>;
            fn id(&self) -> Self::Id { EntityId::<DocumentAggregate>::new() }
            fn version(&self) -> u64 { 0 }
            fn increment_version(&mut self) {}
        }

        let aggregate_id = EntityId::<DocumentAggregate>::new();
        let mut machine = MooreMachine::new(DocumentState::Draft, aggregate_id);

        // Valid transition
        assert!(machine.transition_to(DocumentState::UnderReview).is_ok());
        assert_eq!(machine.current_state(), &DocumentState::UnderReview);

        // Invalid transition
        assert!(machine.transition_to(DocumentState::Published).is_err());

        // Transition to terminal state
        assert!(machine.transition_to(DocumentState::Approved).is_ok());
        assert!(machine.transition_to(DocumentState::Published).is_ok());
        assert!(machine.transition_to(DocumentState::Archived).is_ok());
        assert!(machine.transition_to(DocumentState::Draft).is_err()); // Can't transition from terminal
    }
}
