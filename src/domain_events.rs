// Copyright 2025 Cowboy AI, LLC.

//! Domain events enum wrapper
//!
//! Provides an enum that wraps all domain event types for easier handling

use crate::events::*;
use crate::identifiers::WorkflowId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        "workflows.workflow.started.v1".to_string()
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
        "workflows.workflow.transitioned.v1".to_string()
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
        "workflows.workflow.transition_executed.v1".to_string()
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
        "workflows.workflow.completed.v1".to_string()
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
        "workflows.workflow.suspended.v1".to_string()
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
        "workflows.workflow.resumed.v1".to_string()
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
        "workflows.workflow.cancelled.v1".to_string()
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
        "workflows.workflow.failed.v1".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    #[test]
    fn test_workflow_started_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowStarted {
            workflow_id,
            definition_id: crate::GraphId::new(),
            initial_state: "initial".to_string(),
            started_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowStarted");
        assert_eq!(event.subject(), "workflows.workflow.started.v1");
        let expected_id: Uuid = workflow_id.into();
        assert_eq!(event.aggregate_id(), expected_id);
        assert_eq!(event.initial_state, "initial");
    }

    #[test]
    fn test_workflow_transition_executed_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowTransitionExecuted {
            workflow_id,
            from_state: "state1".to_string(),
            to_state: "state2".to_string(),
            input: serde_json::json!({"key": "value"}),
            output: serde_json::json!({"result": true}),
            executed_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowTransitionExecuted");
        assert_eq!(event.subject(), "workflows.workflow.transition_executed.v1");
        assert_eq!(event.from_state, "state1");
        assert_eq!(event.to_state, "state2");
    }

    #[test]
    fn test_workflow_completed_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowCompleted {
            workflow_id,
            final_state: "completed".to_string(),
            total_duration: Duration::from_secs(3600),
            completed_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowCompleted");
        assert_eq!(event.subject(), "workflows.workflow.completed.v1");
        assert_eq!(event.final_state, "completed");
        assert_eq!(event.total_duration.as_secs(), 3600);
    }

    #[test]
    fn test_workflow_suspended_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowSuspended {
            workflow_id,
            current_state: "processing".to_string(),
            reason: "User requested pause".to_string(),
            suspended_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowSuspended");
        assert_eq!(event.subject(), "workflows.workflow.suspended.v1");
        assert_eq!(event.reason, "User requested pause");
    }

    #[test]
    fn test_workflow_resumed_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowResumed {
            workflow_id,
            current_state: "processing".to_string(),
            resumed_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowResumed");
        assert_eq!(event.subject(), "workflows.workflow.resumed.v1");
        assert_eq!(event.current_state, "processing");
    }

    #[test]
    fn test_workflow_cancelled_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowCancelled {
            workflow_id,
            current_state: "running".to_string(),
            reason: "Timeout exceeded".to_string(),
            cancelled_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowCancelled");
        assert_eq!(event.subject(), "workflows.workflow.cancelled.v1");
        assert_eq!(event.reason, "Timeout exceeded");
    }

    #[test]
    fn test_workflow_failed_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowFailed {
            workflow_id,
            current_state: "processing".to_string(),
            error: "Database connection failed".to_string(),
            failed_at: Utc::now(),
        };

        assert_eq!(event.event_type(), "WorkflowFailed");
        assert_eq!(event.subject(), "workflows.workflow.failed.v1");
        assert_eq!(event.error, "Database connection failed");
    }

    #[test]
    fn test_workflow_transitioned_event() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowTransitioned {
            workflow_id,
            from_state: "a".to_string(),
            to_state: "b".to_string(),
            transition_id: "transition-123".to_string(),
        };

        assert_eq!(event.event_type(), "WorkflowTransitioned");
        assert_eq!(event.subject(), "workflows.workflow.transitioned.v1");
        assert_eq!(event.transition_id, "transition-123");
    }

    #[test]
    fn test_domain_event_enum() {
        let workflow_id = WorkflowId::new();
        let started = WorkflowStarted {
            workflow_id,
            definition_id: crate::GraphId::new(),
            initial_state: "init".to_string(),
            started_at: Utc::now(),
        };

        let event_enum = DomainEventEnum::WorkflowStarted(started.clone());

        assert_eq!(event_enum.event_type(), "WorkflowStarted");
        assert_eq!(event_enum.subject(), "workflows.workflow.started.v1");
        let expected_id: Uuid = workflow_id.into();
        assert_eq!(event_enum.aggregate_id(), expected_id);
    }

    #[test]
    fn test_event_serialization() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowStarted {
            workflow_id,
            definition_id: crate::GraphId::new(),
            initial_state: "start".to_string(),
            started_at: Utc::now(),
        };

        // Test serialization
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("workflow_id"));
        assert!(json.contains("initial_state"));
        assert!(json.contains("start"));

        // Test deserialization
        let deserialized: WorkflowStarted = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.workflow_id, workflow_id);
        assert_eq!(deserialized.initial_state, "start");
    }

    #[test]
    fn test_enum_serialization() {
        let workflow_id = WorkflowId::new();
        let event = WorkflowCompleted {
            workflow_id,
            final_state: "done".to_string(),
            total_duration: Duration::from_secs(120),
            completed_at: Utc::now(),
        };

        let event_enum = DomainEventEnum::WorkflowCompleted(event);

        // Test serialization
        let json = serde_json::to_string(&event_enum).unwrap();
        assert!(json.contains("WorkflowCompleted"));
        assert!(json.contains("final_state"));
        assert!(json.contains("done"));

        // Test deserialization
        let deserialized: DomainEventEnum = serde_json::from_str(&json).unwrap();
        match deserialized {
            DomainEventEnum::WorkflowCompleted(e) => {
                assert_eq!(e.workflow_id, workflow_id);
                assert_eq!(e.final_state, "done");
                assert_eq!(e.total_duration.as_secs(), 120);
            }
            _ => panic!("Wrong event type deserialized"),
        }
    }
}
