// Copyright 2025 Cowboy AI, LLC.

//! Formal Domain Structure with Mathematical Foundations
//!
//! This module defines the formal domain structure required for all CIM domains.
//! Every domain concept MUST have one of these marker traits, establishing a
//! complete algebraic structure for domain modeling.
//!
//! # Domain Algebra
//!
//! The domain forms an algebra with:
//! - **Objects**: ValueObject, DomainEntity, Aggregate
//! - **Morphisms**: Policy, Command, Event, Query
//! - **Composition**: Saga (aggregate of aggregates)
//! - **Identity**: Entity monad provides identity functor

use std::fmt::Debug;
use std::hash::Hash;

use crate::errors::DomainError;
use crate::fp_monad::Entity;

// ============================================================================
// MARKER TRAITS - Every domain concept MUST have one of these
// ============================================================================

/// Root trait for all domain concepts
pub trait DomainConcept: Send + Sync + 'static {}

/// Value Objects are immutable and compared by value
///
/// # Properties
/// - Immutable after creation
/// - No identity beyond their attributes
/// - Compared by structural equality
/// - Can be freely copied and shared
pub trait ValueObject: DomainConcept + Clone + PartialEq + Eq + Debug {}

/// Domain Entities have identity beyond their attributes
///
/// # Properties
/// - Unique identity that persists over time
/// - Mutable state (through controlled operations)
/// - Compared by identity, not attributes
/// - Lifecycle with creation and modification timestamps
pub trait FormalDomainEntity: DomainConcept {
    /// The type of ID for this entity
    type Id: FormalEntityId;

    /// Get the entity's unique identifier
    fn id(&self) -> Self::Id;
}

/// Type-safe entity identifiers
pub trait FormalEntityId: Clone + PartialEq + Eq + Hash + Send + Sync + Debug {}

/// Aggregates are consistency boundaries with state machines
///
/// # Properties
/// - Consistency boundary for a cluster of entities
/// - Root entity controls all access
/// - Implements Mealy State Machine (output depends on state AND input)
/// - Produces events as output
pub trait Aggregate: FormalDomainEntity + MealyStateMachine {
    /// The aggregate's state type
    type State: AggregateState;
    /// Commands this aggregate can handle
    type Command: DomainCommand;
    /// Events this aggregate produces
    type Event: DomainEvent;

    /// Get current state
    fn state(&self) -> <Self as Aggregate>::State;

    /// Handle a command, returning updated aggregate and events
    fn handle(self, cmd: Self::Command) -> Result<(Self, Vec<Self::Event>), DomainError>
    where
        Self: Sized;
}

/// Policy represents pure business rules
///
/// # Properties
/// - Pure functions (no side effects)
/// - Deterministic (same input always produces same output)
/// - Composable through function composition
/// - Can be tested in isolation
pub trait Policy: DomainConcept {
    /// Input type for the policy
    type Input;
    /// Output type for the policy
    type Output;

    /// Apply the policy to input
    fn apply(&self, input: Self::Input) -> Self::Output;

    /// Compose with another policy
    fn compose<P2>(self, other: P2) -> ComposedPolicy<Self, P2>
    where
        Self: Sized,
        P2: Policy<Input = Self::Output>,
    {
        ComposedPolicy {
            first: self,
            second: other,
        }
    }
}

/// Saga is a composed aggregate (aggregate of aggregates)
///
/// # Properties
/// - Manages long-running transactions
/// - Coordinates multiple aggregates
/// - Implements compensation/rollback logic
/// - Maintains saga state across steps
pub trait Saga: DomainConcept {
    /// The type of aggregates this saga coordinates
    type Aggregate: Aggregate;
    /// The saga's own state
    type State: SagaState;

    /// Get current saga state
    fn state(&self) -> Self::State;

    /// Execute next step in the saga
    fn step(&mut self) -> Result<SagaStepResult, DomainError>;

    /// Compensate (rollback) the saga
    fn compensate(&mut self) -> Result<(), DomainError>;
}

// ============================================================================
// MEALY STATE MACHINE - Output depends on State AND Input
// ============================================================================

/// Mealy State Machine where output depends on both state and input
///
/// This is the fundamental model for aggregates in CIM. Unlike Moore machines
/// where output depends only on state, Mealy machines model the reality that
/// the same command in the same state can produce different events based on
/// the command's parameters.
pub trait MealyStateMachine {
    /// The state type
    type State: Clone + PartialEq;
    /// The input type (typically commands)
    type Input;
    /// The output type (typically events)
    type Output;

    /// Compute next state from current state and input
    fn transition(&self, state: Self::State, input: Self::Input) -> Self::State;

    /// Compute output from current state and input
    fn output(&self, state: Self::State, input: Self::Input) -> Self::Output;

    /// Execute one step: return new state and output
    fn step(&self, state: Self::State, input: Self::Input) -> (Self::State, Self::Output)
    where
        Self::Input: Clone,
    {
        let output = self.output(state.clone(), input.clone());
        let new_state = self.transition(state, input);
        (new_state, output)
    }
}

// ============================================================================
// SUPPORTING TYPES
// ============================================================================

/// Commands are requests to change state
pub trait DomainCommand: Send + Sync + Debug {
    /// Get command name for logging/debugging
    fn name(&self) -> &str;
}

/// Events represent things that have happened
pub trait DomainEvent: Send + Sync + Debug + Clone {
    /// Get event name for logging/debugging
    fn name(&self) -> &str;
}

/// Queries are requests for information
pub trait DomainQuery: Send + Sync + Debug {
    /// Get query name for logging/debugging
    fn name(&self) -> &str;
}

/// Aggregate state with lifecycle
pub trait AggregateState: Clone + PartialEq + Send + Sync + Debug {
    /// Get all valid states
    fn all_states() -> Vec<Self>
    where
        Self: Sized;

    /// Get the initial state
    fn initial() -> Self
    where
        Self: Sized;

    /// Check if this is a terminal state
    fn is_terminal(&self) -> bool {
        false
    }

    /// Check if transition to another state is valid
    fn can_transition_to(&self, _other: &Self) -> bool {
        true
    }
}

/// Saga state
pub trait SagaState: Clone + PartialEq + Send + Sync + Debug {
    /// Check if saga is completed
    fn is_completed(&self) -> bool;

    /// Check if saga has failed
    fn is_failed(&self) -> bool;

    /// Check if saga needs compensation
    fn needs_compensation(&self) -> bool;
}

/// Result of a saga step
#[derive(Debug, Clone)]
pub enum SagaStepResult {
    /// Step completed successfully, continue to next
    Continue,
    /// Saga completed successfully
    Completed,
    /// Step failed, need to compensate
    Failed(String),
    /// Waiting for external event
    Waiting,
}

/// Composed policy
pub struct ComposedPolicy<P1, P2> {
    first: P1,
    second: P2,
}

impl<P1, P2> DomainConcept for ComposedPolicy<P1, P2>
where
    P1: Policy,
    P2: Policy<Input = P1::Output>,
{
}

impl<P1, P2> Policy for ComposedPolicy<P1, P2>
where
    P1: Policy,
    P2: Policy<Input = P1::Output>,
{
    type Input = P1::Input;
    type Output = P2::Output;

    fn apply(&self, input: Self::Input) -> Self::Output {
        let intermediate = self.first.apply(input);
        self.second.apply(intermediate)
    }
}

// ============================================================================
// ENTITY-COMPONENT-SYSTEM BRIDGE
// ============================================================================

/// System in ECS - operates on entities with specific components
///
/// Systems are Kleisli arrows: A → Entity<B>
pub trait System<A, B>: Fn(A) -> Entity<B>
where
    B: Send + Sync + 'static,
{
    /// System name for debugging
    fn name(&self) -> &str {
        "unnamed_system"
    }
}

impl<A, B, F> System<A, B> for F
where
    F: Fn(A) -> Entity<B>,
    B: Send + Sync + 'static,
{
}

// ============================================================================
// VALIDATION AND INVARIANTS
// ============================================================================

/// Domain invariant that must always hold
pub trait Invariant: DomainConcept {
    /// The type this invariant validates
    type Target;

    /// Check if the invariant holds
    fn check(&self, target: &Self::Target) -> Result<(), DomainError>;

    /// Get a description of this invariant
    fn description(&self) -> &str;
}

/// Specification pattern for complex validation
pub trait Specification<T>: DomainConcept {
    /// Check if the specification is satisfied
    fn is_satisfied_by(&self, candidate: &T) -> bool;

    /// Combine with another specification using AND
    fn and<S: Specification<T>>(self, other: S) -> AndSpecification<T, Self, S>
    where
        Self: Sized,
    {
        AndSpecification {
            left: self,
            right: other,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Combine with another specification using OR
    fn or<S: Specification<T>>(self, other: S) -> OrSpecification<T, Self, S>
    where
        Self: Sized,
    {
        OrSpecification {
            left: self,
            right: other,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Negate this specification
    fn not(self) -> NotSpecification<T, Self>
    where
        Self: Sized,
    {
        NotSpecification {
            spec: self,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// AND combination of specifications
pub struct AndSpecification<T, L, R> {
    left: L,
    right: R,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, L, R> DomainConcept for AndSpecification<T, L, R>
where
    T: Send + Sync + 'static,
    L: Specification<T>,
    R: Specification<T>,
{
}

impl<T, L, R> Specification<T> for AndSpecification<T, L, R>
where
    T: Send + Sync + 'static,
    L: Specification<T>,
    R: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) && self.right.is_satisfied_by(candidate)
    }
}

/// OR combination of specifications
pub struct OrSpecification<T, L, R> {
    left: L,
    right: R,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, L, R> DomainConcept for OrSpecification<T, L, R>
where
    T: Send + Sync + 'static,
    L: Specification<T>,
    R: Specification<T>,
{
}

impl<T, L, R> Specification<T> for OrSpecification<T, L, R>
where
    T: Send + Sync + 'static,
    L: Specification<T>,
    R: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        self.left.is_satisfied_by(candidate) || self.right.is_satisfied_by(candidate)
    }
}

/// NOT specification
pub struct NotSpecification<T, S> {
    spec: S,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, S> DomainConcept for NotSpecification<T, S>
where
    T: Send + Sync + 'static,
    S: Specification<T>,
{
}

impl<T, S> Specification<T> for NotSpecification<T, S>
where
    T: Send + Sync + 'static,
    S: Specification<T>,
{
    fn is_satisfied_by(&self, candidate: &T) -> bool {
        !self.spec.is_satisfied_by(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example value object
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Money {
        amount: i64,
        currency: String,
    }
    impl DomainConcept for Money {}
    impl ValueObject for Money {}

    // Example specification
    struct PositiveAmount;
    impl DomainConcept for PositiveAmount {}
    impl Specification<Money> for PositiveAmount {
        fn is_satisfied_by(&self, money: &Money) -> bool {
            money.amount > 0
        }
    }

    #[test]
    fn test_value_object() {
        let money1 = Money {
            amount: 100,
            currency: "USD".to_string(),
        };
        let money2 = Money {
            amount: 100,
            currency: "USD".to_string(),
        };
        let money3 = Money {
            amount: 200,
            currency: "USD".to_string(),
        };

        assert_eq!(money1, money2);
        assert_ne!(money1, money3);
    }

    #[test]
    fn test_specification() {
        let spec = PositiveAmount;
        let positive = Money {
            amount: 100,
            currency: "USD".to_string(),
        };
        let negative = Money {
            amount: -50,
            currency: "USD".to_string(),
        };

        assert!(spec.is_satisfied_by(&positive));
        assert!(!spec.is_satisfied_by(&negative));
    }

    #[test]
    fn test_specification_combinators() {
        struct LargAmount;
        impl DomainConcept for LargAmount {}
        impl Specification<Money> for LargAmount {
            fn is_satisfied_by(&self, money: &Money) -> bool {
                money.amount > 1000
            }
        }

        let positive = PositiveAmount;
        let large = LargAmount;
        let combined = positive.and(large);

        let small_positive = Money {
            amount: 100,
            currency: "USD".to_string(),
        };
        let large_positive = Money {
            amount: 2000,
            currency: "USD".to_string(),
        };

        assert!(!combined.is_satisfied_by(&small_positive));
        assert!(combined.is_satisfied_by(&large_positive));
    }
}
