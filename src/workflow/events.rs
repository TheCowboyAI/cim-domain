//! Workflow events
//!
//! This will be implemented in Phase 3 of the workflow implementation plan.

// TODO: Implement workflow events
// - WorkflowStarted
// - TransitionExecuted
// - WorkflowCompleted
// - WorkflowSuspended
// - WorkflowCancelled

use crate::{
    identifiers::{WorkflowId, GraphId},
    workflow::{WorkflowState, TransitionInput, TransitionOutput, WorkflowContext, WorkflowStatus},
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, Duration};

/// Events that can occur in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// A new workflow instance was started
    WorkflowStarted {
        /// The workflow instance ID
        workflow_id: WorkflowId,

        /// The workflow definition being used
        definition_id: GraphId,

        /// Initial state of the workflow
        initial_state: S,

        /// Initial execution context
        initial_context: WorkflowContext,

        /// When the workflow was started
        started_at: SystemTime,
    },

    /// A transition was executed in the workflow
    TransitionExecuted {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// State before the transition
        from_state: S,

        /// State after the transition
        to_state: S,

        /// Input that triggered the transition
        input: I,

        /// Output produced by the transition
        output: O,

        /// When the transition occurred
        timestamp: SystemTime,

        /// How long the transition took
        duration: Duration,

        /// Updated context after transition
        context_snapshot: WorkflowContext,
    },

    /// The workflow reached a terminal state and completed
    WorkflowCompleted {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// Final state of the workflow
        final_state: S,

        /// Total duration of the workflow
        total_duration: Duration,

        /// Number of transitions executed
        transition_count: usize,

        /// When the workflow completed
        completed_at: SystemTime,
    },

    /// The workflow was suspended
    WorkflowSuspended {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// Current state when suspended
        current_state: S,

        /// Reason for suspension
        reason: String,

        /// When the suspension expires (if applicable)
        expires_at: Option<SystemTime>,

        /// When the workflow was suspended
        suspended_at: SystemTime,
    },

    /// A suspended workflow was resumed
    WorkflowResumed {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// State when resuming
        current_state: S,

        /// Updated context on resume
        context_updates: Option<WorkflowContext>,

        /// When the workflow was resumed
        resumed_at: SystemTime,
    },

    /// The workflow was cancelled
    WorkflowCancelled {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// State when cancelled
        current_state: S,

        /// Reason for cancellation
        reason: String,

        /// Whether cancellation was forced
        forced: bool,

        /// When the workflow was cancelled
        cancelled_at: SystemTime,
    },

    /// The workflow encountered an error
    WorkflowFailed {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// State when the error occurred
        current_state: S,

        /// Error description
        error: String,

        /// Whether the workflow can be retried
        retryable: bool,

        /// When the failure occurred
        failed_at: SystemTime,
    },

    /// The workflow was retried from a previous state
    WorkflowRetried {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// State being retried from
        retry_from_state: S,

        /// New context for the retry
        new_context: WorkflowContext,

        /// Attempt number
        attempt: u32,

        /// When the retry occurred
        retried_at: SystemTime,
    },

    /// Workflow context was updated
    ContextUpdated {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// Current state (unchanged)
        current_state: S,

        /// Context before update
        old_context: WorkflowContext,

        /// Context after update
        new_context: WorkflowContext,

        /// When the update occurred
        updated_at: SystemTime,
    },

    /// A component was added to the workflow
    ComponentAdded {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// Type of component added
        component_type: String,

        /// Serialized component data
        component_data: serde_json::Value,

        /// When the component was added
        added_at: SystemTime,
    },

    /// A component was removed from the workflow
    ComponentRemoved {
        /// The workflow instance
        workflow_id: WorkflowId,

        /// Type of component removed
        component_type: String,

        /// When the component was removed
        removed_at: SystemTime,
    },
}

/// Event metadata for tracking and auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventMetadata {
    /// Unique event ID
    pub event_id: String,

    /// When the event occurred
    pub occurred_at: SystemTime,

    /// Correlation ID for tracking
    pub correlation_id: Option<String>,

    /// Causation ID (what caused this event)
    pub causation_id: Option<String>,

    /// Who or what triggered the event
    pub triggered_by: Option<String>,

    /// Additional metadata
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// Wrapper for events with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEventEnvelope<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// The actual event
    pub event: WorkflowEvent<S, I, O>,

    /// Event metadata
    pub metadata: WorkflowEventMetadata,
}

impl<S, I, O> WorkflowEvent<S, I, O>
where
    S: WorkflowState,
    I: TransitionInput,
    O: TransitionOutput,
{
    /// Get the workflow ID this event relates to
    pub fn workflow_id(&self) -> &WorkflowId {
        match self {
            WorkflowEvent::WorkflowStarted { workflow_id, .. } => workflow_id,
            WorkflowEvent::TransitionExecuted { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowCompleted { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowSuspended { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowResumed { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowCancelled { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowFailed { workflow_id, .. } => workflow_id,
            WorkflowEvent::WorkflowRetried { workflow_id, .. } => workflow_id,
            WorkflowEvent::ContextUpdated { workflow_id, .. } => workflow_id,
            WorkflowEvent::ComponentAdded { workflow_id, .. } => workflow_id,
            WorkflowEvent::ComponentRemoved { workflow_id, .. } => workflow_id,
        }
    }

    /// Get the timestamp of when this event occurred
    pub fn timestamp(&self) -> SystemTime {
        match self {
            WorkflowEvent::WorkflowStarted { started_at, .. } => *started_at,
            WorkflowEvent::TransitionExecuted { timestamp, .. } => *timestamp,
            WorkflowEvent::WorkflowCompleted { completed_at, .. } => *completed_at,
            WorkflowEvent::WorkflowSuspended { suspended_at, .. } => *suspended_at,
            WorkflowEvent::WorkflowResumed { resumed_at, .. } => *resumed_at,
            WorkflowEvent::WorkflowCancelled { cancelled_at, .. } => *cancelled_at,
            WorkflowEvent::WorkflowFailed { failed_at, .. } => *failed_at,
            WorkflowEvent::WorkflowRetried { retried_at, .. } => *retried_at,
            WorkflowEvent::ContextUpdated { updated_at, .. } => *updated_at,
            WorkflowEvent::ComponentAdded { added_at, .. } => *added_at,
            WorkflowEvent::ComponentRemoved { removed_at, .. } => *removed_at,
        }
    }

    /// Check if this event represents a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowEvent::WorkflowCompleted { .. }
                | WorkflowEvent::WorkflowCancelled { .. }
                | WorkflowEvent::WorkflowFailed { retryable: false, .. }
        )
    }
}

/// Summary of workflow execution for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionSummary {
    /// The workflow instance
    pub workflow_id: WorkflowId,

    /// The workflow definition used
    pub definition_id: GraphId,

    /// When execution started
    pub started_at: SystemTime,

    /// When execution ended (if applicable)
    pub ended_at: Option<SystemTime>,

    /// Final status
    pub final_status: WorkflowStatus,

    /// Total number of transitions
    pub transition_count: usize,

    /// Total execution time
    pub total_duration: Option<Duration>,

    /// Number of retries
    pub retry_count: u32,

    /// Number of suspensions
    pub suspension_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{SimpleState, SimpleInput, SimpleOutput};

    #[test]
    fn test_workflow_started_event() {
        let event: WorkflowEvent<SimpleState, SimpleInput, SimpleOutput> = WorkflowEvent::WorkflowStarted {
            workflow_id: WorkflowId::new(),
            definition_id: GraphId::new(),
            initial_state: SimpleState::new("Start"),
            initial_context: WorkflowContext::new(),
            started_at: SystemTime::now(),
        };

        assert!(!event.is_terminal());
    }

    #[test]
    fn test_workflow_completed_event() {
        let event: WorkflowEvent<SimpleState, SimpleInput, SimpleOutput> = WorkflowEvent::WorkflowCompleted {
            workflow_id: WorkflowId::new(),
            final_state: SimpleState::terminal("End"),
            total_duration: Duration::from_secs(60),
            transition_count: 5,
            completed_at: SystemTime::now(),
        };

        assert!(event.is_terminal());
    }
}
