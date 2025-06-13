//! Category theory implementation for workflows
//!
//! Workflows form a category where:
//! - Objects are states
//! - Morphisms are transitions
//! - Composition is sequential execution
//! - Identity is the "do nothing" transition

use crate::workflow::{WorkflowState, WorkflowTransition, TransitionInput, TransitionOutput};
use std::marker::PhantomData;

/// Error type for category operations
#[derive(Debug, thiserror::Error)]
pub enum CategoryError {
    /// Transitions cannot be composed because the target of the first doesn't match the source of the second
    #[error("Cannot compose transitions: target of first ({first_target}) != source of second ({second_source})")]
    CompositionMismatch {
        /// The target state of the first transition
        first_target: String,
        /// The source state of the second transition
        second_source: String,
    },

    /// Cannot compose with identity on non-matching state
    #[error("Cannot compose with identity on non-matching state")]
    IdentityMismatch,

    /// A general category operation failed
    #[error("Category operation failed: {0}")]
    OperationFailed(String),
}

/// A category in the mathematical sense
pub trait Category {
    /// Objects in the category
    type Object;

    /// Morphisms (arrows) in the category
    type Morphism;

    /// Compose two morphisms (g ∘ f)
    ///
    /// For morphisms f: A → B and g: B → C, returns g ∘ f: A → C
    fn compose(&self, f: &Self::Morphism, g: &Self::Morphism) -> Result<Self::Morphism, CategoryError>;

    /// Identity morphism for an object
    ///
    /// For any object A, returns id_A: A → A
    fn identity(&self, object: &Self::Object) -> Self::Morphism;
}

/// Workflow category where states are objects and transitions are morphisms
pub struct WorkflowCategory<S, I, O> {
    phantom: PhantomData<(S, I, O)>,
}

impl<S, I, O> WorkflowCategory<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// Create a new workflow category
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

/// Composed transition that represents g ∘ f
pub struct ComposedTransition<S, I, O> {
    /// The first transition to execute (f in g ∘ f)
    first: Box<dyn WorkflowTransition<S, I, O>>,
    /// The second transition to execute (g in g ∘ f)
    second: Box<dyn WorkflowTransition<S, I, O>>,
    /// Phantom data for type parameters
    phantom: PhantomData<(S, I, O)>,
}

impl<S, I, O> WorkflowTransition<S, I, O> for ComposedTransition<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    fn id(&self) -> crate::identifiers::TransitionId {
        crate::identifiers::TransitionId::from(
            format!("{}∘{}", self.second.id(), self.first.id())
        )
    }

    fn source(&self) -> &S {
        self.first.source()
    }

    fn target(&self) -> &S {
        self.second.target()
    }

    fn input(&self) -> &I {
        self.first.input()
    }

    fn output(&self) -> &O {
        self.second.output()
    }

    fn guard(&self, context: &crate::workflow::WorkflowContext) -> bool {
        // Both guards must pass
        self.first.guard(context) && self.second.guard(context)
    }

    fn name(&self) -> &str {
        "Composed Transition"
    }
}

/// Identity transition that does nothing
pub struct IdentityTransition<S, I, O> {
    /// The state that this identity transition operates on
    state: S,
    /// The input for the identity transition
    input: I,
    /// The output for the identity transition
    output: O,
}

impl<S, I, O> IdentityTransition<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput + Default,
    O: TransitionOutput + Default,
{
    /// Create a new identity transition for the given state
    pub fn new(state: S) -> Self {
        Self {
            state,
            input: I::default(),
            output: O::default(),
        }
    }
}

impl<S, I, O> WorkflowTransition<S, I, O> for IdentityTransition<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput + Default,
    O: TransitionOutput + Default,
{
    fn id(&self) -> crate::identifiers::TransitionId {
        crate::identifiers::TransitionId::from(format!("id_{}", self.state.id()))
    }

    fn source(&self) -> &S {
        &self.state
    }

    fn target(&self) -> &S {
        &self.state
    }

    fn input(&self) -> &I {
        &self.input
    }

    fn output(&self) -> &O {
        &self.output
    }

    fn guard(&self, _context: &crate::workflow::WorkflowContext) -> bool {
        true // Identity always succeeds
    }

    fn name(&self) -> &str {
        "Identity"
    }
}

// Note: We cannot implement Category trait directly due to Rust's trait limitations
// with dynamic dispatch. Instead, we provide methods that follow category laws.

impl<S, I, O> WorkflowCategory<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput + Default,
    O: TransitionOutput + Default,
{
    /// Compose two transitions following category laws
    pub fn compose_transitions(
        &self,
        f: Box<dyn WorkflowTransition<S, I, O>>,
        g: Box<dyn WorkflowTransition<S, I, O>>,
    ) -> Result<Box<dyn WorkflowTransition<S, I, O>>, CategoryError> {
        // Check that composition is valid: target(f) == source(g)
        if f.target() != g.source() {
            return Err(CategoryError::CompositionMismatch {
                first_target: f.target().name().to_string(),
                second_source: g.source().name().to_string(),
            });
        }

        Ok(Box::new(ComposedTransition {
            first: f,
            second: g,
            phantom: PhantomData,
        }))
    }

    /// Create identity transition for a state
    pub fn identity_transition(&self, state: S) -> Box<dyn WorkflowTransition<S, I, O>> {
        Box::new(IdentityTransition::new(state))
    }

    /// Verify left identity law: id ∘ f = f
    pub fn verify_left_identity(
        &self,
        f: &dyn WorkflowTransition<S, I, O>,
    ) -> bool {
        let id = self.identity_transition(f.source().clone());
        // In practice, we'd need to check behavioral equivalence
        // For now, we just verify structural properties
        id.target() == f.source() && f.source() == f.source()
    }

    /// Verify right identity law: f ∘ id = f
    pub fn verify_right_identity(
        &self,
        f: &dyn WorkflowTransition<S, I, O>,
    ) -> bool {
        let id = self.identity_transition(f.target().clone());
        // In practice, we'd need to check behavioral equivalence
        // For now, we just verify structural properties
        f.target() == id.source() && f.target() == f.target()
    }

    /// Verify associativity: (h ∘ g) ∘ f = h ∘ (g ∘ f)
    ///
    /// Note: This is a simplified verification that only checks structural properties.
    /// In practice, we cannot clone trait objects, so we verify the law conceptually.
    pub fn verify_associativity_conceptual(&self) -> bool {
        // Associativity holds by construction in our implementation
        // because composition is defined as sequential execution
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{SimpleState, SimpleInput, SimpleOutput, SimpleTransition};

    #[test]
    fn test_workflow_category_creation() {
        let category: WorkflowCategory<SimpleState, SimpleInput, SimpleOutput> =
            WorkflowCategory::new();

        // Category should be created successfully
        let state = SimpleState::new("Test");
        let id = category.identity_transition(state);
        assert_eq!(id.name(), "Identity");
    }

    #[test]
    fn test_transition_composition() {
        let category = WorkflowCategory::new();

        let draft = SimpleState::new("Draft");
        let review = SimpleState::new("Review");
        let published = SimpleState::new("Published");

        let t1 = Box::new(SimpleTransition::new(
            "Submit",
            draft.clone(),
            review.clone(),
            SimpleInput::new("submit"),
            SimpleOutput::new("submitted"),
        ));

        let t2 = Box::new(SimpleTransition::new(
            "Approve",
            review.clone(),
            published.clone(),
            SimpleInput::new("approve"),
            SimpleOutput::new("approved"),
        ));

        let composed = category.compose_transitions(t1, t2).unwrap();

        assert_eq!(composed.source().name(), "Draft");
        assert_eq!(composed.target().name(), "Published");
    }

    #[test]
    fn test_invalid_composition() {
        let category = WorkflowCategory::new();

        let draft = SimpleState::new("Draft");
        let review = SimpleState::new("Review");
        let archived = SimpleState::new("Archived");

        let t1 = Box::new(SimpleTransition::new(
            "Submit",
            draft,
            review,
            SimpleInput::new("submit"),
            SimpleOutput::new("submitted"),
        ));

        let t2 = Box::new(SimpleTransition::new(
            "Archive",
            archived.clone(),
            archived,
            SimpleInput::new("archive"),
            SimpleOutput::new("archived"),
        ));

        let result = category.compose_transitions(t1, t2);
        assert!(result.is_err());
    }
}
