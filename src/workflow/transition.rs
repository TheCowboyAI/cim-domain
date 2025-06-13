//! Workflow transition definitions and traits
//!
//! Transitions are the morphisms in our workflow category. They represent
//! allowed state changes with associated inputs, outputs, and guards.

use crate::identifiers::TransitionId;
use crate::workflow::state::{WorkflowState, WorkflowContext};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Input to a workflow transition
///
/// Inputs trigger state transitions and can carry data
pub trait TransitionInput: Clone + Debug + Send + Sync + 'static {
    /// Type name for serialization
    fn type_name(&self) -> &'static str;
}

/// Output from a workflow transition
///
/// Outputs are produced by transitions and can trigger side effects
pub trait TransitionOutput: Clone + Debug + Send + Sync + 'static {
    /// Type name for serialization
    fn type_name(&self) -> &'static str;
}

/// Guard function for transitions
///
/// Guards determine if a transition is allowed based on context
pub trait TransitionGuard: Send + Sync {
    /// Evaluate the guard condition against the workflow context
    fn evaluate(&self, context: &WorkflowContext) -> bool;
}

/// A workflow transition (morphism in the category)
pub trait WorkflowTransition<S, I, O>: Send + Sync
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// Unique identifier for this transition
    fn id(&self) -> TransitionId;

    /// Source state (domain of the morphism)
    fn source(&self) -> &S;

    /// Target state (codomain of the morphism)
    fn target(&self) -> &S;

    /// Input that triggers this transition
    fn input(&self) -> &I;

    /// Output produced by this transition
    fn output(&self) -> &O;

    /// Guard condition that must be satisfied
    fn guard(&self, context: &WorkflowContext) -> bool;

    /// Human-readable name for this transition
    fn name(&self) -> &str;

    /// Optional description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Check if this transition accepts the given input
    fn accepts_input(&self, _input: &I) -> bool {
        // Default implementation: accept any input of the correct type
        // Override for more specific matching
        true
    }

    /// Execute the transition (for side effects)
    fn execute(&self, _context: &mut WorkflowContext) -> Result<(), Box<dyn std::error::Error>> {
        // Default: no side effects
        Ok(())
    }
}

/// Simple implementation of TransitionInput
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleInput {
    /// Name of the input event or command
    pub name: String,
    /// Additional data payload for the input
    pub data: serde_json::Value,
}

impl SimpleInput {
    /// Create a new simple input with just a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: serde_json::Value::Null,
        }
    }

    /// Create a new simple input with name and data payload
    pub fn with_data(name: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }
}

impl TransitionInput for SimpleInput {
    fn type_name(&self) -> &'static str {
        "SimpleInput"
    }
}

impl Default for SimpleInput {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            data: serde_json::Value::Null,
        }
    }
}

/// Simple implementation of TransitionOutput
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimpleOutput {
    /// Name of the output event or result
    pub name: String,
    /// Additional data payload for the output
    pub data: serde_json::Value,
}

impl SimpleOutput {
    /// Create a new simple output with just a name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: serde_json::Value::Null,
        }
    }

    /// Create a new simple output with name and data payload
    pub fn with_data(name: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }
}

impl TransitionOutput for SimpleOutput {
    fn type_name(&self) -> &'static str {
        "SimpleOutput"
    }
}

impl Default for SimpleOutput {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            data: serde_json::Value::Null,
        }
    }
}

/// Always-true guard for transitions without conditions
pub struct AlwaysGuard;

impl TransitionGuard for AlwaysGuard {
    fn evaluate(&self, _context: &WorkflowContext) -> bool {
        true
    }
}

/// Guard that checks for a specific key in context
pub struct ContextKeyGuard {
    key: String,
}

impl ContextKeyGuard {
    /// Create a new guard that checks for a specific key in the context
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl TransitionGuard for ContextKeyGuard {
    fn evaluate(&self, context: &WorkflowContext) -> bool {
        context.contains(&self.key)
    }
}

/// Guard that checks for a specific actor
pub struct ActorGuard {
    allowed_actors: Vec<String>,
}

impl ActorGuard {
    /// Create a new guard that allows multiple actors
    pub fn new(actors: Vec<String>) -> Self {
        Self { allowed_actors: actors }
    }

    /// Create a new guard that allows only a single actor
    pub fn single(actor: impl Into<String>) -> Self {
        Self {
            allowed_actors: vec![actor.into()],
        }
    }
}

impl TransitionGuard for ActorGuard {
    fn evaluate(&self, context: &WorkflowContext) -> bool {
        context.actor()
            .map(|actor| self.allowed_actors.iter().any(|a| a == actor))
            .unwrap_or(false)
    }
}

/// Concrete implementation of a workflow transition
pub struct SimpleTransition<S: WorkflowState> {
    id: TransitionId,
    name: String,
    source: S,
    target: S,
    input: SimpleInput,
    output: SimpleOutput,
    guard: Box<dyn TransitionGuard>,
    description: Option<String>,
}

impl<S: WorkflowState> SimpleTransition<S> {
    /// Create a new simple transition between states
    pub fn new(
        name: impl Into<String>,
        source: S,
        target: S,
        input: SimpleInput,
        output: SimpleOutput,
    ) -> Self {
        let name = name.into();
        Self {
            id: TransitionId::from(format!("{}->{}", source.id(), target.id())),
            name,
            source,
            target,
            input,
            output,
            guard: Box::new(AlwaysGuard),
            description: None,
        }
    }

    /// Add a guard condition to this transition
    pub fn with_guard(mut self, guard: Box<dyn TransitionGuard>) -> Self {
        self.guard = guard;
        self
    }

    /// Add a description to this transition
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl<S: WorkflowState> WorkflowTransition<S, SimpleInput, SimpleOutput> for SimpleTransition<S> {
    fn id(&self) -> TransitionId {
        self.id.clone()
    }

    fn source(&self) -> &S {
        &self.source
    }

    fn target(&self) -> &S {
        &self.target
    }

    fn input(&self) -> &SimpleInput {
        &self.input
    }

    fn output(&self) -> &SimpleOutput {
        &self.output
    }

    fn guard(&self, context: &WorkflowContext) -> bool {
        self.guard.evaluate(context)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::state::SimpleState;

    #[test]
    fn test_simple_transition() {
        let draft = SimpleState::new("Draft");
        let review = SimpleState::new("Review");

        let transition = SimpleTransition::new(
            "Submit for Review",
            draft.clone(),
            review.clone(),
            SimpleInput::new("submit"),
            SimpleOutput::new("submitted"),
        );

        assert_eq!(transition.name(), "Submit for Review");
        assert_eq!(transition.source().name(), "Draft");
        assert_eq!(transition.target().name(), "Review");

        // Guard should pass by default
        let ctx = WorkflowContext::new();
        assert!(transition.guard(&ctx));
    }

    #[test]
    fn test_context_guard() {
        let draft = SimpleState::new("Draft");
        let review = SimpleState::new("Review");

        let transition = SimpleTransition::new(
            "Submit",
            draft,
            review,
            SimpleInput::new("submit"),
            SimpleOutput::new("submitted"),
        ).with_guard(Box::new(ContextKeyGuard::new("document_id")));

        // Should fail without key
        let ctx = WorkflowContext::new();
        assert!(!transition.guard(&ctx));

        // Should pass with key
        let mut ctx = WorkflowContext::new();
        ctx.set("document_id", "doc123").unwrap();
        assert!(transition.guard(&ctx));
    }

    #[test]
    fn test_actor_guard() {
        let draft = SimpleState::new("Draft");
        let review = SimpleState::new("Review");

        let transition = SimpleTransition::new(
            "Submit",
            draft,
            review,
            SimpleInput::new("submit"),
            SimpleOutput::new("submitted"),
        ).with_guard(Box::new(ActorGuard::single("admin")));

        // Should fail without actor
        let ctx = WorkflowContext::new();
        assert!(!transition.guard(&ctx));

        // Should fail with wrong actor
        let ctx = WorkflowContext::with_actor("user".to_string());
        assert!(!transition.guard(&ctx));

        // Should pass with correct actor
        let ctx = WorkflowContext::with_actor("admin".to_string());
        assert!(transition.guard(&ctx));
    }
}
