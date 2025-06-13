//! Workflow aggregate implementation
//!
//! This will be implemented in Phase 3 of the workflow implementation plan.

// TODO: Implement WorkflowAggregate
// - Stores current state
// - References workflow definition (graph)
// - Maintains execution context
// - Tracks transition history

use crate::{
    AggregateRoot, Component, ComponentStorage,
    identifiers::{WorkflowId, GraphId},
    workflow::{WorkflowState, TransitionInput, TransitionOutput, WorkflowContext},
    errors::DomainError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// A running instance of a workflow
#[derive(Debug, Clone)]
pub struct WorkflowAggregate<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// Unique identifier for this workflow instance
    pub id: WorkflowId,

    /// Reference to the workflow definition (graph)
    pub definition_id: GraphId,

    /// Current state of the workflow
    pub current_state: S,

    /// Execution context with runtime data
    pub context: WorkflowContext,

    /// History of all transitions
    pub history: Vec<TransitionEvent<S, I, O>>,

    /// When the workflow was started
    pub started_at: SystemTime,

    /// When the workflow was last updated
    pub updated_at: SystemTime,

    /// Current status of the workflow
    pub status: WorkflowStatus,

    /// Version for optimistic concurrency
    version: u64,

    /// Component storage for extensibility
    components: ComponentStorage,
}

/// Status of a workflow instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// Workflow is actively running
    Active,

    /// Workflow is temporarily suspended
    Suspended,

    /// Workflow has completed successfully
    Completed,

    /// Workflow was cancelled
    Cancelled,

    /// Workflow encountered an error
    Failed,
}

/// Record of a state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionEvent<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// State before the transition
    pub from_state: S,

    /// State after the transition
    pub to_state: S,

    /// Input that triggered the transition
    pub input: I,

    /// Output produced by the transition
    pub output: O,

    /// When the transition occurred
    pub timestamp: SystemTime,

    /// How long the transition took
    pub duration: Duration,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl<S, I, O> WorkflowAggregate<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// Create a new workflow instance
    pub fn new(
        definition_id: GraphId,
        initial_state: S,
        initial_context: WorkflowContext,
    ) -> Self {
        let now = SystemTime::now();

        Self {
            id: WorkflowId::new(),
            definition_id,
            current_state: initial_state,
            context: initial_context,
            history: Vec::new(),
            started_at: now,
            updated_at: now,
            status: WorkflowStatus::Active,
            version: 0,
            components: ComponentStorage::new(),
        }
    }

    /// Get the current state
    pub fn current_state(&self) -> &S {
        &self.current_state
    }

    /// Get the execution context
    pub fn context(&self) -> &WorkflowContext {
        &self.context
    }

    /// Get the workflow status
    pub fn status(&self) -> WorkflowStatus {
        self.status
    }

    /// Check if the workflow is in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.current_state.is_terminal()
    }

    /// Check if the workflow can accept transitions
    pub fn can_transition(&self) -> bool {
        matches!(self.status, WorkflowStatus::Active) && !self.is_terminal()
    }

    /// Record a successful transition
    pub fn record_transition(
        &mut self,
        from_state: S,
        to_state: S,
        input: I,
        output: O,
        duration: Duration,
    ) {
        let event = TransitionEvent {
            from_state,
            to_state: to_state.clone(),
            input,
            output,
            timestamp: SystemTime::now(),
            duration,
            metadata: HashMap::new(),
        };

        self.history.push(event);
        self.current_state = to_state;
        self.updated_at = SystemTime::now();
        self.increment_version();

        // Check if we've reached a terminal state
        if self.current_state.is_terminal() {
            self.status = WorkflowStatus::Completed;
        }
    }

    /// Suspend the workflow
    pub fn suspend(&mut self, reason: &str) -> Result<(), DomainError> {
        if !matches!(self.status, WorkflowStatus::Active) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Suspended".to_string(),
            });
        }

        self.status = WorkflowStatus::Suspended;
        self.context.set("suspension_reason", reason)?;
        self.updated_at = SystemTime::now();
        self.increment_version();

        Ok(())
    }

    /// Resume a suspended workflow
    pub fn resume(&mut self) -> Result<(), DomainError> {
        if !matches!(self.status, WorkflowStatus::Suspended) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Active".to_string(),
            });
        }

        self.status = WorkflowStatus::Active;
        // Remove suspension reason by setting it to null
        self.context.set("suspension_reason", serde_json::Value::Null)?;
        self.updated_at = SystemTime::now();
        self.increment_version();

        Ok(())
    }

    /// Cancel the workflow
    pub fn cancel(&mut self, reason: &str) -> Result<(), DomainError> {
        if matches!(self.status, WorkflowStatus::Completed | WorkflowStatus::Cancelled) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Cancelled".to_string(),
            });
        }

        self.status = WorkflowStatus::Cancelled;
        self.context.set("cancellation_reason", reason)?;
        self.updated_at = SystemTime::now();
        self.increment_version();

        Ok(())
    }

    /// Mark the workflow as failed
    pub fn fail(&mut self, error: &str) -> Result<(), DomainError> {
        if matches!(self.status, WorkflowStatus::Completed | WorkflowStatus::Cancelled) {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Failed".to_string(),
            });
        }

        self.status = WorkflowStatus::Failed;
        self.context.set("failure_reason", error)?;
        self.updated_at = SystemTime::now();
        self.increment_version();

        Ok(())
    }

    /// Get the total duration of the workflow
    pub fn duration(&self) -> Duration {
        self.updated_at
            .duration_since(self.started_at)
            .unwrap_or_default()
    }

    /// Get the number of transitions executed
    pub fn transition_count(&self) -> usize {
        self.history.len()
    }

    /// Add a component to the workflow
    pub fn add_component<C: Component + 'static>(&mut self, component: C) -> Result<(), DomainError> {
        self.components.add(component)?;
        self.updated_at = SystemTime::now();
        self.increment_version();
        Ok(())
    }

    /// Get a component from the workflow
    pub fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Remove a component from the workflow
    pub fn remove_component<C: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        let result = self.components.remove::<C>();
        if result.is_some() {
            self.updated_at = SystemTime::now();
            self.increment_version();
        }
        result
    }
}

impl<S, I, O> AggregateRoot for WorkflowAggregate<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    type Id = WorkflowId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{SimpleState, SimpleInput, SimpleOutput};

    #[test]
    fn test_workflow_creation() {
        let initial_state = SimpleState::new("Start");
        let context = WorkflowContext::new();
        let workflow = WorkflowAggregate::<SimpleState, SimpleInput, SimpleOutput>::new(
            GraphId::new(),
            initial_state.clone(),
            context,
        );

        assert_eq!(workflow.current_state().name(), "Start");
        assert_eq!(workflow.status(), WorkflowStatus::Active);
        assert_eq!(workflow.transition_count(), 0);
        assert!(workflow.can_transition());
        assert_eq!(workflow.version(), 0);
    }

    #[test]
    fn test_workflow_transitions() {
        let start = SimpleState::new("Start");
        let middle = SimpleState::new("Middle");
        let context = WorkflowContext::new();

        let mut workflow = WorkflowAggregate::new(
            GraphId::new(),
            start.clone(),
            context,
        );

        // Record a transition
        workflow.record_transition(
            start,
            middle.clone(),
            SimpleInput::new("next"),
            SimpleOutput::new("moved"),
            Duration::from_millis(100),
        );

        assert_eq!(workflow.current_state().name(), "Middle");
        assert_eq!(workflow.transition_count(), 1);
        assert_eq!(workflow.history[0].from_state.name(), "Start");
        assert_eq!(workflow.history[0].to_state.name(), "Middle");
        assert_eq!(workflow.version(), 1);
    }

    #[test]
    fn test_workflow_suspension() {
        let state = SimpleState::new("Active");
        let context = WorkflowContext::new();

        let mut workflow = WorkflowAggregate::<SimpleState, SimpleInput, SimpleOutput>::new(
            GraphId::new(),
            state,
            context,
        );

        // Suspend workflow
        workflow.suspend("Maintenance").unwrap();
        assert_eq!(workflow.status(), WorkflowStatus::Suspended);
        assert!(!workflow.can_transition());

        // Resume workflow
        workflow.resume().unwrap();
        assert_eq!(workflow.status(), WorkflowStatus::Active);
        assert!(workflow.can_transition());
    }

    #[test]
    fn test_terminal_state_completion() {
        let start = SimpleState::new("Start");
        let end = SimpleState::terminal("End");
        let context = WorkflowContext::new();

        let mut workflow = WorkflowAggregate::new(
            GraphId::new(),
            start.clone(),
            context,
        );

        // Transition to terminal state
        workflow.record_transition(
            start,
            end,
            SimpleInput::new("finish"),
            SimpleOutput::new("completed"),
            Duration::from_millis(50),
        );

        assert!(workflow.is_terminal());
        assert_eq!(workflow.status(), WorkflowStatus::Completed);
        assert!(!workflow.can_transition());
    }
}
