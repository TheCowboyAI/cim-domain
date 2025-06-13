//! Workflow status projection
//!
//! Provides a view of workflow execution status and history.

use super::{EventSequence, Projection};
use crate::{
    domain_events::{DomainEventEnum, WorkflowStarted, WorkflowCompleted, WorkflowFailed, WorkflowTransitioned},
    identifiers::{WorkflowId, GraphId},
    workflow::WorkflowStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Status information about a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatusInfo {
    /// Unique identifier of the workflow instance
    pub workflow_id: WorkflowId,
    /// ID of the workflow definition (graph)
    pub definition_id: GraphId,
    /// Current status of the workflow
    pub status: WorkflowStatus,
    /// Current state in the workflow
    pub current_state: String,
    /// When the workflow was started
    pub started_at: DateTime<Utc>,
    /// When the workflow completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,
    /// When the workflow failed (if applicable)
    pub failed_at: Option<DateTime<Utc>>,
    /// Error message if workflow failed
    pub error: Option<String>,
    /// Number of state transitions executed
    pub transition_count: usize,
    /// Total duration of the workflow execution
    pub total_duration: Option<Duration>,
}

/// Projection that maintains workflow status information
#[derive(Debug, Clone)]
pub struct WorkflowStatusProjection {
    workflows: HashMap<WorkflowId, WorkflowStatusInfo>,
    workflows_by_status: HashMap<WorkflowStatus, Vec<WorkflowId>>,
    workflows_by_definition: HashMap<GraphId, Vec<WorkflowId>>,
    checkpoint: Option<EventSequence>,
}

impl WorkflowStatusProjection {
    /// Create a new workflow status projection
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            workflows_by_status: HashMap::new(),
            workflows_by_definition: HashMap::new(),
            checkpoint: None,
        }
    }

    /// Get workflow status by ID
    pub fn get_workflow_status(&self, workflow_id: &WorkflowId) -> Option<&WorkflowStatusInfo> {
        self.workflows.get(workflow_id)
    }

    /// Get all workflows
    pub fn get_all_workflows(&self) -> Vec<&WorkflowStatusInfo> {
        self.workflows.values().collect()
    }

    /// Get workflows by status
    pub fn get_workflows_by_status(&self, status: WorkflowStatus) -> Vec<&WorkflowStatusInfo> {
        self.workflows_by_status
            .get(&status)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.workflows.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get workflows by definition
    pub fn get_workflows_by_definition(&self, definition_id: &GraphId) -> Vec<&WorkflowStatusInfo> {
        self.workflows_by_definition
            .get(definition_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.workflows.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get active workflows
    pub fn get_active_workflows(&self) -> Vec<&WorkflowStatusInfo> {
        self.get_workflows_by_status(WorkflowStatus::Active)
    }

    /// Get completed workflows
    pub fn get_completed_workflows(&self) -> Vec<&WorkflowStatusInfo> {
        self.get_workflows_by_status(WorkflowStatus::Completed)
    }

    /// Get failed workflows
    pub fn get_failed_workflows(&self) -> Vec<&WorkflowStatusInfo> {
        self.get_workflows_by_status(WorkflowStatus::Failed)
    }

    /// Get workflow count by status
    pub fn count_by_status(&self) -> HashMap<WorkflowStatus, usize> {
        self.workflows_by_status
            .iter()
            .map(|(status, ids)| (*status, ids.len()))
            .collect()
    }

    /// Get average completion time for completed workflows
    pub fn average_completion_time(&self) -> Option<Duration> {
        let completed: Vec<_> = self.get_completed_workflows()
            .into_iter()
            .filter_map(|w| w.total_duration)
            .collect();

        if completed.is_empty() {
            None
        } else {
            let total: Duration = completed.iter().sum();
            Some(total / completed.len() as u32)
        }
    }
}

#[async_trait]
impl Projection for WorkflowStatusProjection {
    async fn handle_event(&mut self, event: DomainEventEnum) -> Result<(), String> {
        match event {
            DomainEventEnum::WorkflowStarted(WorkflowStarted {
                workflow_id,
                definition_id,
                initial_state,
                started_at,
            }) => {
                let status_info = WorkflowStatusInfo {
                    workflow_id,
                    definition_id,
                    status: WorkflowStatus::Active,
                    current_state: initial_state,
                    started_at,
                    completed_at: None,
                    failed_at: None,
                    error: None,
                    transition_count: 0,
                    total_duration: None,
                };

                // Add to main index
                self.workflows.insert(workflow_id, status_info);

                // Add to status index
                self.workflows_by_status
                    .entry(WorkflowStatus::Active)
                    .or_insert_with(Vec::new)
                    .push(workflow_id);

                // Add to definition index
                self.workflows_by_definition
                    .entry(definition_id)
                    .or_insert_with(Vec::new)
                    .push(workflow_id);
            }

            DomainEventEnum::WorkflowTransitioned(WorkflowTransitioned {
                workflow_id,
                to_state,
                ..
            }) => {
                if let Some(status_info) = self.workflows.get_mut(&workflow_id) {
                    status_info.current_state = to_state;
                    status_info.transition_count += 1;
                }
            }

            DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
                workflow_id,
                completed_at,
                total_duration,
                ..
            }) => {
                if let Some(status_info) = self.workflows.get_mut(&workflow_id) {
                    // Update status
                    let old_status = status_info.status;
                    status_info.status = WorkflowStatus::Completed;
                    status_info.completed_at = Some(completed_at);
                    status_info.total_duration = Some(total_duration);

                    // Update status index
                    if let Some(workflows) = self.workflows_by_status.get_mut(&old_status) {
                        workflows.retain(|id| id != &workflow_id);
                    }
                    self.workflows_by_status
                        .entry(WorkflowStatus::Completed)
                        .or_insert_with(Vec::new)
                        .push(workflow_id);
                }
            }

            DomainEventEnum::WorkflowFailed(WorkflowFailed {
                workflow_id,
                error,
                failed_at,
                ..
            }) => {
                if let Some(status_info) = self.workflows.get_mut(&workflow_id) {
                    // Update status
                    let old_status = status_info.status;
                    status_info.status = WorkflowStatus::Failed;
                    status_info.failed_at = Some(failed_at);
                    status_info.error = Some(error);

                    // Calculate duration
                    if let Ok(duration) = failed_at.signed_duration_since(status_info.started_at).to_std() {
                        status_info.total_duration = Some(duration);
                    }

                    // Update status index
                    if let Some(workflows) = self.workflows_by_status.get_mut(&old_status) {
                        workflows.retain(|id| id != &workflow_id);
                    }
                    self.workflows_by_status
                        .entry(WorkflowStatus::Failed)
                        .or_insert_with(Vec::new)
                        .push(workflow_id);
                }
            }

            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }

    async fn get_checkpoint(&self) -> Option<EventSequence> {
        self.checkpoint
    }

    async fn save_checkpoint(&mut self, sequence: EventSequence) -> Result<(), String> {
        self.checkpoint = Some(sequence);
        Ok(())
    }

    async fn clear(&mut self) -> Result<(), String> {
        self.workflows.clear();
        self.workflows_by_status.clear();
        self.workflows_by_definition.clear();
        self.checkpoint = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_status_projection() {
        let mut projection = WorkflowStatusProjection::new();
        let workflow_id = WorkflowId::new();
        let definition_id = GraphId::new();

        // Start workflow
        let start_event = DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id,
            definition_id,
            initial_state: "Start".to_string(),
            started_at: Utc::now(),
        });

        projection.handle_event(start_event).await.unwrap();

        // Verify workflow was started
        let status = projection.get_workflow_status(&workflow_id).unwrap();
        assert_eq!(status.status, WorkflowStatus::Active);
        assert_eq!(status.current_state, "Start");
        assert_eq!(status.transition_count, 0);

        // Transition workflow
        let transition_event = DomainEventEnum::WorkflowTransitioned(WorkflowTransitioned {
            workflow_id,
            from_state: "Start".to_string(),
            to_state: "Processing".to_string(),
            transition_id: "t1".to_string(),
        });

        projection.handle_event(transition_event).await.unwrap();

        // Verify transition
        let status = projection.get_workflow_status(&workflow_id).unwrap();
        assert_eq!(status.current_state, "Processing");
        assert_eq!(status.transition_count, 1);

        // Complete workflow
        let complete_event = DomainEventEnum::WorkflowCompleted(WorkflowCompleted {
            workflow_id,
            final_state: "End".to_string(),
            completed_at: Utc::now(),
            total_duration: Duration::from_secs(60),
        });

        projection.handle_event(complete_event).await.unwrap();

        // Verify completion
        let status = projection.get_workflow_status(&workflow_id).unwrap();
        assert_eq!(status.status, WorkflowStatus::Completed);
        assert!(status.completed_at.is_some());
        assert_eq!(status.total_duration, Some(Duration::from_secs(60)));
    }

    #[tokio::test]
    async fn test_workflow_failure() {
        let mut projection = WorkflowStatusProjection::new();
        let workflow_id = WorkflowId::new();
        let definition_id = GraphId::new();

        // Start workflow
        let start_event = DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id,
            definition_id,
            initial_state: "Start".to_string(),
            started_at: Utc::now(),
        });

        projection.handle_event(start_event).await.unwrap();

        // Fail workflow
        let fail_event = DomainEventEnum::WorkflowFailed(WorkflowFailed {
            workflow_id,
            current_state: "Processing".to_string(),
            error: "Connection timeout".to_string(),
            failed_at: Utc::now(),
        });

        projection.handle_event(fail_event).await.unwrap();

        // Verify failure
        let status = projection.get_workflow_status(&workflow_id).unwrap();
        assert_eq!(status.status, WorkflowStatus::Failed);
        assert_eq!(status.error, Some("Connection timeout".to_string()));
        assert!(status.failed_at.is_some());

        // Check failed workflows list
        let failed = projection.get_failed_workflows();
        assert_eq!(failed.len(), 1);
    }
}
