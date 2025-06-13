//! Workflow commands
//!
//! Commands represent intentions to change workflow state.
//! They are processed by command handlers to produce events.

use crate::{
    identifiers::{WorkflowId, GraphId},
    workflow::{TransitionInput, WorkflowContext},
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Commands that can be sent to a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowCommand<I>
where
    I: TransitionInput,
{
    /// Start a new workflow instance
    StartWorkflow {
        /// The workflow definition to use
        definition_id: GraphId,

        /// Initial execution context
        initial_context: WorkflowContext,

        /// Optional workflow ID (if not provided, one will be generated)
        workflow_id: Option<WorkflowId>,

        /// When to start the workflow
        start_time: Option<SystemTime>,
    },

    /// Execute a transition in the workflow
    ExecuteTransition {
        /// The workflow instance to transition
        workflow_id: WorkflowId,

        /// Input that triggers the transition
        input: I,

        /// Additional context for this transition
        context_updates: Option<WorkflowContext>,
    },

    /// Suspend a running workflow
    SuspendWorkflow {
        /// The workflow to suspend
        workflow_id: WorkflowId,

        /// Reason for suspension
        reason: String,

        /// When the suspension should expire (optional)
        expires_at: Option<SystemTime>,
    },

    /// Resume a suspended workflow
    ResumeWorkflow {
        /// The workflow to resume
        workflow_id: WorkflowId,

        /// Additional context updates on resume
        context_updates: Option<WorkflowContext>,
    },

    /// Cancel a workflow
    CancelWorkflow {
        /// The workflow to cancel
        workflow_id: WorkflowId,

        /// Reason for cancellation
        reason: String,

        /// Whether to allow cancellation of completed workflows
        force: bool,
    },

    /// Retry a failed workflow from a specific state
    RetryWorkflow {
        /// The workflow to retry
        workflow_id: WorkflowId,

        /// State to retry from (if None, retry from current state)
        from_state: Option<String>,

        /// New context for the retry
        context_updates: Option<WorkflowContext>,
    },

    /// Update workflow context without transitioning
    UpdateContext {
        /// The workflow to update
        workflow_id: WorkflowId,

        /// Context updates to apply
        updates: WorkflowContext,
    },

    /// Add a component to the workflow
    AddComponent {
        /// The workflow to update
        workflow_id: WorkflowId,

        /// Serialized component data
        component_type: String,
        /// JSON representation of the component to add
        component_data: serde_json::Value,
    },

    /// Remove a component from the workflow
    RemoveComponent {
        /// The workflow to update
        workflow_id: WorkflowId,

        /// Type of component to remove
        component_type: String,
    },
}

/// Batch command for executing multiple workflow commands atomically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandBatch<I>
where
    I: TransitionInput,
{
    /// Commands to execute in order
    pub commands: Vec<WorkflowCommand<I>>,

    /// Whether to stop on first error
    pub stop_on_error: bool,

    /// Transaction ID for the batch
    pub transaction_id: Option<String>,
}

/// Command metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandMetadata {
    /// Who issued the command
    pub issued_by: String,

    /// When the command was issued
    pub issued_at: SystemTime,

    /// Correlation ID for tracking
    pub correlation_id: Option<String>,

    /// Causation ID (what caused this command)
    pub causation_id: Option<String>,

    /// Additional metadata
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Wrapper for commands with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandEnvelope<I>
where
    I: TransitionInput,
{
    /// The actual command
    pub command: WorkflowCommand<I>,

    /// Command metadata
    pub metadata: WorkflowCommandMetadata,
}

impl<I> WorkflowCommand<I>
where
    I: TransitionInput,
{
    /// Get the workflow ID this command targets
    pub fn workflow_id(&self) -> Option<&WorkflowId> {
        match self {
            WorkflowCommand::StartWorkflow { workflow_id, .. } => workflow_id.as_ref(),
            WorkflowCommand::ExecuteTransition { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::SuspendWorkflow { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::ResumeWorkflow { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::CancelWorkflow { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::RetryWorkflow { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::UpdateContext { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::AddComponent { workflow_id, .. } => Some(workflow_id),
            WorkflowCommand::RemoveComponent { workflow_id, .. } => Some(workflow_id),
        }
    }

    /// Check if this is a workflow creation command
    pub fn is_creation(&self) -> bool {
        matches!(self, WorkflowCommand::StartWorkflow { .. })
    }

    /// Check if this is a state-changing command
    pub fn is_state_changing(&self) -> bool {
        matches!(
            self,
            WorkflowCommand::ExecuteTransition { .. }
                | WorkflowCommand::SuspendWorkflow { .. }
                | WorkflowCommand::ResumeWorkflow { .. }
                | WorkflowCommand::CancelWorkflow { .. }
                | WorkflowCommand::RetryWorkflow { .. }
        )
    }
}

// Implement Command trait for WorkflowCommand
impl<I: TransitionInput> crate::cqrs::Command for WorkflowCommand<I> {
    type Aggregate = crate::workflow::WorkflowAggregate<
        crate::workflow::SimpleState,
        I,
        crate::workflow::SimpleOutput
    >;

    fn aggregate_id(&self) -> Option<crate::entity::EntityId<Self::Aggregate>> {
        // For workflow commands, we don't use EntityId since WorkflowId is already defined
        // This is a limitation of the current design - workflows use their own ID type
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::SimpleInput;

    #[test]
    fn test_command_workflow_id() {
        let workflow_id = WorkflowId::new();

        let cmd: WorkflowCommand<SimpleInput> = WorkflowCommand::ExecuteTransition {
            workflow_id: workflow_id.clone(),
            input: SimpleInput::new("test"),
            context_updates: None,
        };

        assert_eq!(cmd.workflow_id(), Some(&workflow_id));
        assert!(!cmd.is_creation());
        assert!(cmd.is_state_changing());
    }

    #[test]
    fn test_start_workflow_command() {
        let cmd: WorkflowCommand<SimpleInput> = WorkflowCommand::StartWorkflow {
            definition_id: GraphId::new(),
            initial_context: WorkflowContext::new(),
            workflow_id: None,
            start_time: None,
        };

        assert!(cmd.workflow_id().is_none());
        assert!(cmd.is_creation());
        assert!(!cmd.is_state_changing());
    }
}
