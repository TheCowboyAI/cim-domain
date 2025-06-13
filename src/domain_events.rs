//! Domain events enum wrapper
//!
//! Provides an enum that wraps all domain event types for easier handling

use crate::events::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::identifiers::{WorkflowId};
use std::collections::HashMap;

/// Enum wrapper for all domain events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEventEnum {
    // Graph events have been extracted to cim-domain-graph

    // Location events have been extracted to cim-domain-location

    // Workflow events
    /// A workflow was started
    WorkflowStarted(WorkflowStarted),
    /// A workflow transition was executed
    WorkflowTransitionExecuted(WorkflowTransitionExecuted),
    /// A workflow transitioned between states
    WorkflowTransitioned(WorkflowTransitioned),
    /// A workflow was completed
    WorkflowCompleted(WorkflowCompleted),
    /// A workflow was suspended
    WorkflowSuspended(WorkflowSuspended),
    /// A workflow was resumed
    WorkflowResumed(WorkflowResumed),
    /// A workflow was cancelled
    WorkflowCancelled(WorkflowCancelled),
    /// A workflow failed
    WorkflowFailed(WorkflowFailed),
}

// Workflow event structs

/// Workflow started event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStarted {
    /// The unique identifier of the workflow instance
    pub workflow_id: WorkflowId,
    /// The ID of the graph definition this workflow is based on
    pub definition_id: crate::GraphId,
    /// The initial state of the workflow
    pub initial_state: String,
    /// When the workflow was started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow transition executed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransitionExecuted {
    /// The workflow that executed the transition
    pub workflow_id: WorkflowId,
    /// The state before the transition
    pub from_state: String,
    /// The state after the transition
    pub to_state: String,
    /// The input that triggered the transition
    pub input: serde_json::Value,
    /// The output produced by the transition
    pub output: serde_json::Value,
    /// When the transition was executed
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow completed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCompleted {
    /// The workflow that completed
    pub workflow_id: WorkflowId,
    /// The final state of the workflow
    pub final_state: String,
    /// The total duration of the workflow execution
    pub total_duration: std::time::Duration,
    /// When the workflow completed
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow suspended event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSuspended {
    /// The workflow that was suspended
    pub workflow_id: WorkflowId,
    /// The state at which the workflow was suspended
    pub current_state: String,
    /// The reason for suspension
    pub reason: String,
    /// When the workflow was suspended
    pub suspended_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow resumed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResumed {
    /// The workflow that was resumed
    pub workflow_id: WorkflowId,
    /// The state from which the workflow resumed
    pub current_state: String,
    /// When the workflow was resumed
    pub resumed_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow cancelled event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCancelled {
    /// The workflow that was cancelled
    pub workflow_id: WorkflowId,
    /// The state at which the workflow was cancelled
    pub current_state: String,
    /// The reason for cancellation
    pub reason: String,
    /// When the workflow was cancelled
    pub cancelled_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow failed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFailed {
    /// The workflow that failed
    pub workflow_id: WorkflowId,
    /// The state at which the workflow failed
    pub current_state: String,
    /// The error that caused the failure
    pub error: String,
    /// When the workflow failed
    pub failed_at: chrono::DateTime<chrono::Utc>,
}

/// Workflow transition executed event (alias)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransitioned {
    /// The workflow that transitioned
    pub workflow_id: WorkflowId,
    /// The state before the transition
    pub from_state: String,
    /// The state after the transition
    pub to_state: String,
    /// The unique identifier of the transition
    pub transition_id: String,
}

// Implement DomainEvent trait for workflow events
impl DomainEvent for WorkflowStarted {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowStarted"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.started.v1")
    }
}

impl DomainEvent for WorkflowTransitioned {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowTransitioned"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.transitioned.v1")
    }
}

impl DomainEvent for WorkflowTransitionExecuted {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowTransitionExecuted"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.transition_executed.v1")
    }
}

impl DomainEvent for WorkflowCompleted {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowCompleted"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.completed.v1")
    }
}

impl DomainEvent for WorkflowSuspended {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowSuspended"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.suspended.v1")
    }
}

impl DomainEvent for WorkflowResumed {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowResumed"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.resumed.v1")
    }
}

impl DomainEvent for WorkflowCancelled {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowCancelled"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.cancelled.v1")
    }
}

impl DomainEvent for WorkflowFailed {
    fn aggregate_id(&self) -> Uuid {
        self.workflow_id.into()
    }

    fn event_type(&self) -> &'static str {
        "WorkflowFailed"
    }

    fn subject(&self) -> String {
        format!("workflows.workflow.failed.v1")
    }
}

impl DomainEvent for DomainEventEnum {
    fn subject(&self) -> String {
        match self {
            Self::WorkflowStarted(e) => e.subject(),
            Self::WorkflowTransitionExecuted(e) => e.subject(),
            Self::WorkflowTransitioned(e) => e.subject(),
            Self::WorkflowCompleted(e) => e.subject(),
            Self::WorkflowSuspended(e) => e.subject(),
            Self::WorkflowResumed(e) => e.subject(),
            Self::WorkflowCancelled(e) => e.subject(),
            Self::WorkflowFailed(e) => e.subject(),
        }
    }

    fn aggregate_id(&self) -> uuid::Uuid {
        match self {
            Self::WorkflowStarted(e) => e.aggregate_id(),
            Self::WorkflowTransitionExecuted(e) => e.aggregate_id(),
            Self::WorkflowTransitioned(e) => e.aggregate_id(),
            Self::WorkflowCompleted(e) => e.aggregate_id(),
            Self::WorkflowSuspended(e) => e.aggregate_id(),
            Self::WorkflowResumed(e) => e.aggregate_id(),
            Self::WorkflowCancelled(e) => e.aggregate_id(),
            Self::WorkflowFailed(e) => e.aggregate_id(),
        }
    }

    fn event_type(&self) -> &'static str {
        match self {
            Self::WorkflowStarted(e) => e.event_type(),
            Self::WorkflowTransitionExecuted(e) => e.event_type(),
            Self::WorkflowTransitioned(e) => e.event_type(),
            Self::WorkflowCompleted(e) => e.event_type(),
            Self::WorkflowSuspended(e) => e.event_type(),
            Self::WorkflowResumed(e) => e.event_type(),
            Self::WorkflowCancelled(e) => e.event_type(),
            Self::WorkflowFailed(e) => e.event_type(),
        }
    }
}
