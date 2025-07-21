// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating command handling patterns
//!
//! This example shows:
//! - Implementing commands with the Command trait
//! - Creating command envelopes with metadata
//! - Command validation and acknowledgment
//! - Working with correlation and causation IDs

use chrono::Utc;
use cim_domain::{
    markers::AggregateMarker,
    AggregateRoot,

    CausationId,
    // Commands
    Command,
    CommandAcknowledgment,
    CommandEnvelope,
    CommandId,
    CommandStatus,
    CorrelationId,
    DomainError,
    // Events
    DomainEventEnum,
    DomainResult,
    // Core types
    EntityId,
    GraphId,
    IdType,

    // IDs
    WorkflowId,
    WorkflowStarted,
    WorkflowTransitionExecuted,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Example aggregate: Task
#[derive(Debug, Clone)]
struct Task {
    id: EntityId<AggregateMarker>,
    title: String,
    description: String,
    status: TaskStatus,
    assigned_to: Option<String>,
    version: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum TaskStatus {
    Created,
    Assigned,
    InProgress,
    Completed,
    Cancelled,
}

impl AggregateRoot for Task {
    type Id = EntityId<AggregateMarker>;

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

/// Commands for Task aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
enum TaskCommand {
    CreateTask {
        task_id: EntityId<AggregateMarker>,
        title: String,
        description: String,
    },
    AssignTask {
        task_id: EntityId<AggregateMarker>,
        assignee: String,
    },
    StartTask {
        task_id: EntityId<AggregateMarker>,
    },
    CompleteTask {
        task_id: EntityId<AggregateMarker>,
        completion_notes: String,
    },
    CancelTask {
        task_id: EntityId<AggregateMarker>,
        reason: String,
    },
}

impl Command for TaskCommand {
    type Aggregate = AggregateMarker;

    fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
        match self {
            Self::CreateTask { task_id, .. }
            | Self::AssignTask { task_id, .. }
            | Self::StartTask { task_id }
            | Self::CompleteTask { task_id, .. }
            | Self::CancelTask { task_id, .. } => Some(*task_id),
        }
    }
}

/// Command handler for tasks
struct TaskCommandHandler {
    tasks: std::collections::HashMap<EntityId<AggregateMarker>, Task>,
}

impl TaskCommandHandler {
    fn new() -> Self {
        Self {
            tasks: std::collections::HashMap::new(),
        }
    }

    /// Get task details for display
    fn get_task(&self, task_id: &EntityId<AggregateMarker>) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Handle a command and return events
    fn handle_command(&mut self, command: &TaskCommand) -> DomainResult<Vec<DomainEventEnum>> {
        match command {
            TaskCommand::CreateTask {
                task_id,
                title,
                description,
            } => {
                // Check if task already exists
                if self.tasks.contains_key(task_id) {
                    return Err(DomainError::ValidationError(
                        "Task already exists".to_string(),
                    ));
                }

                // Create new task
                let task = Task {
                    id: *task_id,
                    title: title.clone(),
                    description: description.clone(),
                    status: TaskStatus::Created,
                    assigned_to: None,
                    version: 1,
                };

                self.tasks.insert(*task_id, task);

                // Return event (using workflow events as example)
                Ok(vec![DomainEventEnum::WorkflowStarted(WorkflowStarted {
                    workflow_id: WorkflowId::new(),
                    definition_id: GraphId::new(),
                    initial_state: "task_created".to_string(),
                    started_at: Utc::now(),
                })])
            }

            TaskCommand::AssignTask { task_id, assignee } => {
                let task = self
                    .tasks
                    .get_mut(task_id)
                    .ok_or_else(|| DomainError::NotFound("Task not found".to_string()))?;

                // Validate state
                if task.status != TaskStatus::Created {
                    return Err(DomainError::ValidationError(
                        "Can only assign tasks in Created state".to_string(),
                    ));
                }

                task.assigned_to = Some(assignee.clone());
                task.status = TaskStatus::Assigned;
                task.increment_version();

                Ok(vec![DomainEventEnum::WorkflowTransitionExecuted(
                    WorkflowTransitionExecuted {
                        workflow_id: WorkflowId::new(),
                        from_state: "created".to_string(),
                        to_state: "assigned".to_string(),
                        input: json!({"assignee": assignee}),
                        output: json!({"success": true}),
                        executed_at: Utc::now(),
                    },
                )])
            }

            TaskCommand::StartTask { task_id } => {
                let task = self
                    .tasks
                    .get_mut(task_id)
                    .ok_or_else(|| DomainError::NotFound("Task not found".to_string()))?;

                // Validate state
                if task.status != TaskStatus::Assigned {
                    return Err(DomainError::ValidationError(
                        "Can only start assigned tasks".to_string(),
                    ));
                }

                task.status = TaskStatus::InProgress;
                task.increment_version();

                Ok(vec![DomainEventEnum::WorkflowTransitionExecuted(
                    WorkflowTransitionExecuted {
                        workflow_id: WorkflowId::new(),
                        from_state: "assigned".to_string(),
                        to_state: "in_progress".to_string(),
                        input: json!({}),
                        output: json!({"started_at": Utc::now()}),
                        executed_at: Utc::now(),
                    },
                )])
            }

            TaskCommand::CompleteTask {
                task_id,
                completion_notes,
            } => {
                let task = self
                    .tasks
                    .get_mut(task_id)
                    .ok_or_else(|| DomainError::NotFound("Task not found".to_string()))?;

                // Validate state
                if task.status != TaskStatus::InProgress {
                    return Err(DomainError::ValidationError(
                        "Can only complete tasks in progress".to_string(),
                    ));
                }

                task.status = TaskStatus::Completed;
                task.increment_version();

                Ok(vec![DomainEventEnum::WorkflowTransitionExecuted(
                    WorkflowTransitionExecuted {
                        workflow_id: WorkflowId::new(),
                        from_state: "in_progress".to_string(),
                        to_state: "completed".to_string(),
                        input: json!({"notes": completion_notes}),
                        output: json!({"completed_at": Utc::now()}),
                        executed_at: Utc::now(),
                    },
                )])
            }

            TaskCommand::CancelTask { task_id, reason } => {
                let task = self
                    .tasks
                    .get_mut(task_id)
                    .ok_or_else(|| DomainError::NotFound("Task not found".to_string()))?;

                // Can cancel from any state except completed
                if task.status == TaskStatus::Completed {
                    return Err(DomainError::ValidationError(
                        "Cannot cancel completed tasks".to_string(),
                    ));
                }

                let from_state = format!("{:?}", task.status).to_lowercase();
                task.status = TaskStatus::Cancelled;
                task.increment_version();

                Ok(vec![DomainEventEnum::WorkflowTransitionExecuted(
                    WorkflowTransitionExecuted {
                        workflow_id: WorkflowId::new(),
                        from_state,
                        to_state: "cancelled".to_string(),
                        input: json!({"reason": reason}),
                        output: json!({"cancelled_at": Utc::now()}),
                        executed_at: Utc::now(),
                    },
                )])
            }
        }
    }
}

fn main() {
    println!("Command Handler Example");
    println!("======================\n");

    let mut handler = TaskCommandHandler::new();
    let task_id = EntityId::new();

    // Example 1: Create task command
    println!("1. Creating a new task...");
    let create_command = TaskCommand::CreateTask {
        task_id,
        title: "Implement feature X".to_string(),
        description: "Add new functionality to the system".to_string(),
    };

    let envelope = CommandEnvelope::new(create_command.clone(), "user-123".to_string());

    println!("   Command envelope:");
    println!("     ID: {}", envelope.id);
    println!("     Issued by: {}", envelope.issued_by);
    println!("     Correlation ID: {}", envelope.correlation_id());

    match handler.handle_command(&create_command) {
        Ok(events) => {
            println!("   ✓ Command succeeded, produced {} events", events.len());

            // Display the created task details
            if let Some(task) = handler.get_task(&task_id) {
                println!("   Created Task:");
                println!("     Title: {}", task.title);
                println!("     Description: {}", task.description);
                println!("     Status: {:?}", task.status);
            }

            // Create acknowledgment
            let ack = CommandAcknowledgment {
                command_id: envelope.id,
                status: CommandStatus::Accepted,
                reason: None,
                correlation_id: envelope.correlation_id().clone(),
            };

            println!("   Acknowledgment:");
            println!("     Status: {:?}", ack.status);
            if let Some(reason) = &ack.reason {
                println!("     Reason: {}", reason);
            }
        }
        Err(e) => {
            println!("   ✗ Command failed: {}", e);
        }
    }

    // Example 2: Assign task
    println!("\n2. Assigning the task...");
    let assign_command = TaskCommand::AssignTask {
        task_id,
        assignee: "alice@example.com".to_string(),
    };

    match handler.handle_command(&assign_command) {
        Ok(events) => {
            println!("   ✓ Task assigned, produced {} events", events.len());
        }
        Err(e) => {
            println!("   ✗ Failed to assign: {}", e);
        }
    }

    // Example 3: Start task
    println!("\n3. Starting the task...");
    let start_command = TaskCommand::StartTask { task_id };

    match handler.handle_command(&start_command) {
        Ok(events) => {
            println!("   ✓ Task started, produced {} events", events.len());
        }
        Err(e) => {
            println!("   ✗ Failed to start: {}", e);
        }
    }

    // Example 4: Try invalid command
    println!("\n4. Trying to assign already started task...");
    let invalid_assign = TaskCommand::AssignTask {
        task_id,
        assignee: "bob@example.com".to_string(),
    };

    match handler.handle_command(&invalid_assign) {
        Ok(_) => {
            println!("   ✗ Unexpected success!");
        }
        Err(e) => {
            println!("   ✓ Expected error: {}", e);

            // Create rejection acknowledgment
            let ack = CommandAcknowledgment {
                command_id: CommandId::new(),
                status: CommandStatus::Rejected,
                reason: Some(e.to_string()),
                correlation_id: CorrelationId(IdType::Uuid(Uuid::new_v4())),
            };

            println!("   Rejection acknowledgment:");
            println!("     Status: {:?}", ack.status);
            if let Some(reason) = &ack.reason {
                println!("     Reason: {}", reason);
            }
        }
    }

    // Example 5: Complete task
    println!("\n5. Completing the task...");
    let complete_command = TaskCommand::CompleteTask {
        task_id,
        completion_notes: "Feature implemented and tested".to_string(),
    };

    match handler.handle_command(&complete_command) {
        Ok(events) => {
            println!("   ✓ Task completed, produced {} events", events.len());
        }
        Err(e) => {
            println!("   ✗ Failed to complete: {}", e);
        }
    }

    // Example 6: Command with causation
    println!("\n6. Creating linked command with causation...");
    let task2_id = EntityId::new();
    let create_followup = TaskCommand::CreateTask {
        task_id: task2_id,
        title: "Test feature X".to_string(),
        description: "Write tests for the new feature".to_string(),
    };

    // Create envelope with causation from previous command
    let followup_envelope = CommandEnvelope::new(create_followup, "user-123".to_string());
    // In real usage, you'd track the event ID that caused this command
    let causation_id = CausationId(IdType::Uuid(Uuid::new_v4()));

    println!("   Followup command:");
    println!("     Caused by: {}", causation_id);
    println!("     Correlation: {}", followup_envelope.correlation_id());

    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Implementing commands with the Command trait");
    println!("  • Creating command envelopes with metadata");
    println!("  • Command validation and error handling");
    println!("  • Command acknowledgments and rejections");
    println!("  • Working with correlation and causation");

    // Show command trait usage
    println!("\nCommand trait features:");
    println!("  • aggregate_id() - Get the target aggregate");
    println!("  • CommandEnvelope - Wraps commands with metadata");
    println!("  • CommandAcknowledgment - Confirms command receipt");
    println!("  • CommandStatus - Accepted/Rejected/Processing");
}
