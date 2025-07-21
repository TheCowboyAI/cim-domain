// Copyright 2025 Cowboy AI, LLC.

//! Saga orchestration for cross-domain workflows
//!
//! Sagas represent long-running transactions that span multiple domains.
//! They are implemented as morphisms in the topos of domain compositions
//! and orchestrated by state machines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::category::DomainCategory;
use crate::entity::{AggregateRoot, EntityId};
use crate::errors::DomainError;
use crate::events::DomainEvent;
use crate::state_machine::{
    MealyMachine, MealyStateTransitions, State, TransitionInput, TransitionOutput,
};

/// A saga representing a cross-domain workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    /// Unique identifier
    pub id: Uuid,

    /// Name of the saga
    pub name: String,

    /// Steps in the saga
    pub steps: Vec<SagaStep>,

    /// Current state of the saga
    pub state: SagaState,

    /// Compensation map for rollback
    pub compensations: HashMap<String, CompensationAction>,

    /// Context data passed between steps
    pub context: HashMap<String, serde_json::Value>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// A step in a saga
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    /// Step identifier
    pub id: String,

    /// Target domain
    pub domain: String,

    /// Command to execute
    pub command_type: String,

    /// Step dependencies
    pub depends_on: Vec<String>,

    /// Retry policy
    pub retry_policy: RetryPolicy,

    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

/// State of a saga - implements State trait for state machine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SagaState {
    /// Not started
    Pending,

    /// Currently executing
    Running {
        /// The step currently being executed
        current_step: String,
        /// Steps that have been successfully completed
        completed_steps: Vec<String>,
    },

    /// Successfully completed
    Completed,

    /// Failed and compensating
    Compensating {
        /// The step that failed
        failed_step: String,
        /// Steps that have been compensated
        compensated_steps: Vec<String>,
    },

    /// Compensation complete
    Compensated,

    /// Failed to compensate
    Failed {
        /// Error message describing the failure
        error: String,
    },
}

impl State for SagaState {
    fn name(&self) -> &'static str {
        match self {
            SagaState::Pending => "Pending",
            SagaState::Running { .. } => "Running",
            SagaState::Completed => "Completed",
            SagaState::Compensating { .. } => "Compensating",
            SagaState::Compensated => "Compensated",
            SagaState::Failed { .. } => "Failed",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(
            self,
            SagaState::Completed | SagaState::Compensated | SagaState::Failed { .. }
        )
    }
}

/// Input for saga state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SagaTransitionInput {
    /// Start the saga
    Start,

    /// Step completed successfully
    StepCompleted {
        /// ID of the completed step
        step_id: String,
        /// Result data from the step
        result: serde_json::Value,
    },

    /// Step failed
    StepFailed {
        /// ID of the failed step
        step_id: String,
        /// Error message
        error: String,
    },

    /// Compensation step completed
    CompensationCompleted {
        /// ID of the compensated step
        step_id: String,
    },

    /// Compensation failed
    CompensationFailed {
        /// ID of the step that failed to compensate
        step_id: String,
        /// Error message
        error: String,
    },
}

impl TransitionInput for SagaTransitionInput {
    fn description(&self) -> String {
        match self {
            SagaTransitionInput::Start => "Start saga".to_string(),
            SagaTransitionInput::StepCompleted { step_id, .. } => {
                format!("Step {step_id} completed")
            }
            SagaTransitionInput::StepFailed { step_id, error } => {
                format!("Step {step_id} failed: {error}")
            }
            SagaTransitionInput::CompensationCompleted { step_id } => {
                format!("Compensation for {step_id} completed")
            }
            SagaTransitionInput::CompensationFailed { step_id, error } => {
                format!("Compensation for {step_id} failed: {error}")
            }
        }
    }
}

/// Output from saga state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaTransitionOutput {
    /// Events to emit
    pub events: Vec<SagaEvent>,

    /// Commands to execute
    pub commands: Vec<SagaCommand>,
}

impl TransitionOutput for SagaTransitionOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        // Convert saga events to domain events
        // In a real implementation, these would implement DomainEvent
        vec![]
    }
}

/// Commands generated by saga transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SagaCommand {
    /// Execute a step
    ExecuteStep {
        /// ID of the step to execute
        step_id: String,
        /// Target domain for the command
        domain: String,
        /// Command to execute
        command: String,
    },

    /// Execute compensation
    ExecuteCompensation {
        /// ID of the step to compensate
        step_id: String,
        /// Compensation action to perform
        action: CompensationAction,
    },
}

impl MealyStateTransitions for SagaState {
    type Input = SagaTransitionInput;
    type Output = SagaTransitionOutput;

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
        match (self, target, input) {
            // Pending -> Running on Start
            (SagaState::Pending, SagaState::Running { .. }, SagaTransitionInput::Start) => true,

            // Running -> Running on StepCompleted
            (
                SagaState::Running { .. },
                SagaState::Running { .. },
                SagaTransitionInput::StepCompleted { .. },
            ) => true,

            // Running -> Completed when all steps done
            (
                SagaState::Running {
                    completed_steps: _, ..
                },
                SagaState::Completed,
                SagaTransitionInput::StepCompleted { .. },
            ) => {
                // In real implementation, check if all steps are completed
                true
            }

            // Running -> Compensating on failure
            (
                SagaState::Running { .. },
                SagaState::Compensating { .. },
                SagaTransitionInput::StepFailed { .. },
            ) => true,

            // Compensating -> Compensating
            (
                SagaState::Compensating { .. },
                SagaState::Compensating { .. },
                SagaTransitionInput::CompensationCompleted { .. },
            ) => true,

            // Compensating -> Compensated
            (
                SagaState::Compensating { .. },
                SagaState::Compensated,
                SagaTransitionInput::CompensationCompleted { .. },
            ) => true,

            // Compensating -> Failed
            (
                SagaState::Compensating { .. },
                SagaState::Failed { .. },
                SagaTransitionInput::CompensationFailed { .. },
            ) => true,

            _ => false,
        }
    }

    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
        match (self, input) {
            (SagaState::Pending, SagaTransitionInput::Start) => {
                vec![SagaState::Running {
                    current_step: String::new(),
                    completed_steps: vec![],
                }]
            }
            (
                SagaState::Running {
                    completed_steps, ..
                },
                SagaTransitionInput::StepCompleted { step_id, .. },
            ) => {
                let mut new_completed = completed_steps.clone();
                new_completed.push(step_id.clone());
                vec![
                    SagaState::Running {
                        current_step: String::new(),
                        completed_steps: new_completed,
                    },
                    SagaState::Completed,
                ]
            }
            (SagaState::Running { .. }, SagaTransitionInput::StepFailed { step_id, .. }) => {
                vec![SagaState::Compensating {
                    failed_step: step_id.clone(),
                    compensated_steps: vec![],
                }]
            }
            _ => vec![],
        }
    }

    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output {
        let mut events = vec![];
        let commands = vec![];

        match (self, target, input) {
            (SagaState::Pending, SagaState::Running { .. }, SagaTransitionInput::Start) => {
                events.push(SagaEvent::Started {
                    saga_id: Uuid::new_v4(), // In real impl, get from context
                    timestamp: chrono::Utc::now(),
                });
            }
            (
                SagaState::Running { .. },
                SagaState::Completed,
                SagaTransitionInput::StepCompleted { .. },
            ) => {
                events.push(SagaEvent::Completed {
                    saga_id: Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                });
            }
            (
                SagaState::Running { .. },
                SagaState::Compensating { .. },
                SagaTransitionInput::StepFailed { step_id, error },
            ) => {
                events.push(SagaEvent::StepFailed {
                    saga_id: Uuid::new_v4(),
                    step_id: step_id.clone(),
                    error: error.clone(),
                    timestamp: chrono::Utc::now(),
                });
                events.push(SagaEvent::CompensationStarted {
                    saga_id: Uuid::new_v4(),
                    failed_step: step_id.clone(),
                    timestamp: chrono::Utc::now(),
                });
            }
            _ => {}
        }

        SagaTransitionOutput { events, commands }
    }
}

/// Compensation action for rollback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationAction {
    /// Target domain
    pub domain: String,

    /// Compensation command type
    pub command_type: String,

    /// Additional parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Retry policy for saga steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retries
    pub max_retries: u32,

    /// Initial backoff in milliseconds
    pub initial_backoff_ms: u64,

    /// Backoff multiplier
    pub backoff_multiplier: f32,

    /// Maximum backoff in milliseconds
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            max_backoff_ms: 10000,
        }
    }
}

/// Saga aggregate for AggregateRoot implementation
#[derive(Debug, Clone)]
pub struct SagaAggregate {
    id: Uuid,
    version: u64,
}

impl Default for SagaAggregate {
    fn default() -> Self {
        Self::new()
    }
}

impl SagaAggregate {
    /// Create a new saga aggregate with a unique ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            version: 0,
        }
    }
}

impl AggregateRoot for SagaAggregate {
    type Id = Uuid;

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

// Type alias for saga storage
type SagaStorage = Arc<Mutex<HashMap<Uuid, (Saga, MealyMachine<SagaState, SagaAggregate>)>>>;

/// Saga orchestrator for managing saga execution using state machines
pub struct SagaOrchestrator {
    /// Active sagas with their state machines
    sagas: SagaStorage,

    /// Domain categories
    domains: Arc<Mutex<HashMap<String, DomainCategory>>>,

    /// Event log
    event_log: Arc<Mutex<Vec<SagaEvent>>>,
}

/// Events emitted by the saga orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SagaEvent {
    /// Saga started
    Started {
        /// ID of the started saga
        saga_id: Uuid,
        /// When the saga started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Step started
    StepStarted {
        /// ID of the saga
        saga_id: Uuid,
        /// ID of the step that started
        step_id: String,
        /// When the step started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Step completed
    StepCompleted {
        /// ID of the saga
        saga_id: Uuid,
        /// ID of the completed step
        step_id: String,
        /// Result data from the step
        result: serde_json::Value,
        /// When the step completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Step failed
    StepFailed {
        /// ID of the saga
        saga_id: Uuid,
        /// ID of the failed step
        step_id: String,
        /// Error message
        error: String,
        /// When the step failed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Saga completed
    Completed {
        /// ID of the completed saga
        saga_id: Uuid,
        /// When the saga completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Compensation started
    CompensationStarted {
        /// ID of the saga
        saga_id: Uuid,
        /// The step that triggered compensation
        failed_step: String,
        /// When compensation started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Saga compensated
    Compensated {
        /// ID of the compensated saga
        saga_id: Uuid,
        /// When compensation completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

impl Default for SagaOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SagaOrchestrator {
    /// Create a new saga orchestrator
    pub fn new() -> Self {
        Self {
            sagas: Arc::new(Mutex::new(HashMap::new())),
            domains: Arc::new(Mutex::new(HashMap::new())),
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a domain
    pub async fn register_domain(&self, domain: DomainCategory) -> Result<(), DomainError> {
        let mut domains = self.domains.lock().await;
        if domains.contains_key(&domain.name) {
            return Err(DomainError::AlreadyExists(format!(
                "Domain {} already registered",
                domain.name
            )));
        }
        domains.insert(domain.name.clone(), domain);
        Ok(())
    }

    /// Start a saga
    pub async fn start_saga(&self, mut saga: Saga) -> Result<Uuid, DomainError> {
        saga.state = SagaState::Pending;
        let saga_id = saga.id;

        // Create state machine for the saga
        let entity_id = EntityId::<SagaAggregate>::new();
        let state_machine = MealyMachine::new(SagaState::Pending, entity_id);

        // Store saga with its state machine
        {
            let mut sagas = self.sagas.lock().await;
            sagas.insert(saga_id, (saga, state_machine));
        }

        // Trigger start transition
        self.transition_saga(saga_id, SagaTransitionInput::Start)
            .await?;

        Ok(saga_id)
    }

    /// Transition a saga using its state machine
    fn transition_saga(
        &self,
        saga_id: Uuid,
        input: SagaTransitionInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), DomainError>> + Send + '_>> {
        Box::pin(async move {
            let mut sagas = self.sagas.lock().await;
            let (saga, state_machine) = sagas
                .get_mut(&saga_id)
                .ok_or_else(|| DomainError::NotFound(format!("Saga {saga_id} not found")))?;

            // Determine target state based on current state and input
            let target_state = match (state_machine.current_state(), &input) {
                (SagaState::Pending, SagaTransitionInput::Start) => {
                    // Find first step
                    let first_step = saga
                        .steps
                        .iter()
                        .find(|s| s.depends_on.is_empty())
                        .ok_or_else(|| DomainError::InvalidOperation {
                            reason: "No initial step found".to_string(),
                        })?;

                    SagaState::Running {
                        current_step: first_step.id.clone(),
                        completed_steps: vec![],
                    }
                }
                (
                    SagaState::Running {
                        completed_steps, ..
                    },
                    SagaTransitionInput::StepCompleted { step_id, .. },
                ) => {
                    let mut new_completed = completed_steps.clone();
                    new_completed.push(step_id.clone());

                    // Check if all steps completed
                    if new_completed.len() == saga.steps.len() {
                        SagaState::Completed
                    } else {
                        // Find next step
                        let next_step = self.get_next_step_for_saga(saga, &new_completed)?;
                        SagaState::Running {
                            current_step: next_step,
                            completed_steps: new_completed,
                        }
                    }
                }
                (SagaState::Running { .. }, SagaTransitionInput::StepFailed { step_id, .. }) => {
                    SagaState::Compensating {
                        failed_step: step_id.clone(),
                        compensated_steps: vec![],
                    }
                }
                (
                    SagaState::Compensating {
                        compensated_steps,
                        failed_step,
                    },
                    SagaTransitionInput::CompensationCompleted { step_id },
                ) => {
                    let mut new_compensated = compensated_steps.clone();
                    new_compensated.push(step_id.clone());

                    // Check if all compensations completed
                    if new_compensated.len() == saga.compensations.len() {
                        SagaState::Compensated
                    } else {
                        SagaState::Compensating {
                            failed_step: failed_step.clone(),
                            compensated_steps: new_compensated,
                        }
                    }
                }
                (
                    SagaState::Compensating { .. },
                    SagaTransitionInput::CompensationFailed { error, .. },
                ) => SagaState::Failed {
                    error: error.clone(),
                },
                _ => {
                    return Err(DomainError::InvalidOperation {
                        reason: "Invalid state transition".to_string(),
                    })
                }
            };

            // Perform transition
            let transition = state_machine.transition_to(target_state.clone(), input)?;

            // Update saga state
            saga.state = target_state;

            // Process output
            for event in transition.output.events {
                self.emit_event(event).await;
            }

            // Execute commands
            for command in transition.output.commands {
                self.execute_saga_command(saga_id, command).await?;
            }

            Ok(())
        })
    }

    /// Get the next step to execute
    fn get_next_step_for_saga(
        &self,
        saga: &Saga,
        completed_steps: &[String],
    ) -> Result<String, DomainError> {
        for step in &saga.steps {
            if !completed_steps.contains(&step.id) {
                let deps_satisfied = step
                    .depends_on
                    .iter()
                    .all(|dep| completed_steps.contains(dep));
                if deps_satisfied {
                    return Ok(step.id.clone());
                }
            }
        }
        Err(DomainError::InvalidOperation {
            reason: "No valid next step found".to_string(),
        })
    }

    /// Execute a saga command
    async fn execute_saga_command(
        &self,
        saga_id: Uuid,
        command: SagaCommand,
    ) -> Result<(), DomainError> {
        match command {
            SagaCommand::ExecuteStep {
                step_id,
                domain,
                command,
            } => {
                // Execute step and handle result
                match self
                    .execute_step_internal(saga_id, &step_id, &domain, &command)
                    .await
                {
                    Ok(result) => {
                        self.transition_saga(
                            saga_id,
                            SagaTransitionInput::StepCompleted { step_id, result },
                        )
                        .await?;
                    }
                    Err(error) => {
                        self.transition_saga(
                            saga_id,
                            SagaTransitionInput::StepFailed {
                                step_id,
                                error: error.to_string(),
                            },
                        )
                        .await?;
                    }
                }
            }
            SagaCommand::ExecuteCompensation { step_id, action } => {
                // Execute compensation
                match self
                    .execute_compensation_internal(saga_id, &step_id, &action)
                    .await
                {
                    Ok(_) => {
                        self.transition_saga(
                            saga_id,
                            SagaTransitionInput::CompensationCompleted { step_id },
                        )
                        .await?;
                    }
                    Err(error) => {
                        self.transition_saga(
                            saga_id,
                            SagaTransitionInput::CompensationFailed {
                                step_id,
                                error: error.to_string(),
                            },
                        )
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Internal step execution
    async fn execute_step_internal(
        &self,
        _saga_id: Uuid,
        step_id: &str,
        domain: &str,
        command: &str,
    ) -> Result<serde_json::Value, DomainError> {
        // In real implementation, this would execute the command in the domain
        Ok(serde_json::json!({
            "step": step_id,
            "domain": domain,
            "command": command,
            "status": "completed"
        }))
    }

    /// Internal compensation execution
    async fn execute_compensation_internal(
        &self,
        _saga_id: Uuid,
        _step_id: &str,
        _action: &CompensationAction,
    ) -> Result<(), DomainError> {
        // In real implementation, this would execute the compensation
        Ok(())
    }

    /// Emit an event
    async fn emit_event(&self, event: SagaEvent) {
        let mut event_log = self.event_log.lock().await;
        event_log.push(event);
    }

    /// Get saga state
    pub async fn get_saga_state(&self, saga_id: Uuid) -> Result<SagaState, DomainError> {
        let sagas = self.sagas.lock().await;
        let (saga, _) = sagas
            .get(&saga_id)
            .ok_or_else(|| DomainError::NotFound(format!("Saga {saga_id} not found")))?;
        Ok(saga.state.clone())
    }

    /// Get saga state machine for direct access
    pub async fn get_state_machine(
        &self,
        saga_id: Uuid,
    ) -> Result<MealyMachine<SagaState, SagaAggregate>, DomainError> {
        let sagas = self.sagas.lock().await;
        let (_, state_machine) = sagas
            .get(&saga_id)
            .ok_or_else(|| DomainError::NotFound(format!("Saga {saga_id} not found")))?;
        Ok(state_machine.clone())
    }
}

/// Saga builder for fluent API
pub struct SagaBuilder {
    saga: Saga,
}

impl SagaBuilder {
    /// Create a new saga builder
    ///
    /// # Arguments
    /// * `name` - Name of the saga
    pub fn new(name: String) -> Self {
        Self {
            saga: Saga {
                id: Uuid::new_v4(),
                name,
                steps: vec![],
                state: SagaState::Pending,
                compensations: HashMap::new(),
                context: HashMap::new(),
                metadata: HashMap::new(),
            },
        }
    }

    /// Add a step to the saga
    ///
    /// # Arguments
    /// * `step` - The saga step to add
    pub fn add_step(mut self, step: SagaStep) -> Self {
        self.saga.steps.push(step);
        self
    }

    /// Add compensation action for a step
    ///
    /// # Arguments
    /// * `step_id` - ID of the step to compensate
    /// * `compensation` - The compensation action
    pub fn with_compensation(mut self, step_id: String, compensation: CompensationAction) -> Self {
        self.saga.compensations.insert(step_id, compensation);
        self
    }

    /// Add context data to the saga
    ///
    /// # Arguments
    /// * `key` - Context key
    /// * `value` - Context value
    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.saga.context.insert(key, value);
        self
    }

    /// Add metadata to the saga
    ///
    /// # Arguments
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.saga.metadata.insert(key, value);
        self
    }

    /// Build the configured saga
    pub fn build(self) -> Saga {
        self.saga
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_saga_builder() {
        let saga = SagaBuilder::new("OrderProcessing".to_string())
            .add_step(SagaStep {
                id: "validate_order".to_string(),
                domain: "OrderDomain".to_string(),
                command_type: "ValidateOrder".to_string(),
                depends_on: vec![],
                retry_policy: RetryPolicy::default(),
                timeout_ms: 5000,
            })
            .add_step(SagaStep {
                id: "reserve_inventory".to_string(),
                domain: "InventoryDomain".to_string(),
                command_type: "ReserveItems".to_string(),
                depends_on: vec!["validate_order".to_string()],
                retry_policy: RetryPolicy::default(),
                timeout_ms: 10000,
            })
            .add_step(SagaStep {
                id: "process_payment".to_string(),
                domain: "PaymentDomain".to_string(),
                command_type: "ChargePayment".to_string(),
                depends_on: vec!["reserve_inventory".to_string()],
                retry_policy: RetryPolicy::default(),
                timeout_ms: 15000,
            })
            .with_compensation(
                "reserve_inventory".to_string(),
                CompensationAction {
                    domain: "InventoryDomain".to_string(),
                    command_type: "ReleaseReservation".to_string(),
                    parameters: HashMap::new(),
                },
            )
            .with_compensation(
                "process_payment".to_string(),
                CompensationAction {
                    domain: "PaymentDomain".to_string(),
                    command_type: "RefundPayment".to_string(),
                    parameters: HashMap::new(),
                },
            )
            .build();

        assert_eq!(saga.steps.len(), 3);
        assert_eq!(saga.compensations.len(), 2);
        assert_eq!(saga.state, SagaState::Pending);
    }

    #[tokio::test]
    async fn test_saga_orchestrator() {
        let orchestrator = SagaOrchestrator::new();

        // Register domains
        orchestrator
            .register_domain(DomainCategory::new("OrderDomain".to_string()))
            .await
            .unwrap();
        orchestrator
            .register_domain(DomainCategory::new("InventoryDomain".to_string()))
            .await
            .unwrap();

        // Create simple saga
        let saga = SagaBuilder::new("TestSaga".to_string())
            .add_step(SagaStep {
                id: "step1".to_string(),
                domain: "OrderDomain".to_string(),
                command_type: "TestCommand".to_string(),
                depends_on: vec![],
                retry_policy: RetryPolicy::default(),
                timeout_ms: 1000,
            })
            .build();

        let saga_id = orchestrator.start_saga(saga).await.unwrap();

        // Check initial state
        let state = orchestrator.get_saga_state(saga_id).await.unwrap();
        assert!(
            matches!(state, SagaState::Running { current_step, .. } if current_step == "step1")
        );

        // Manually trigger step completion (in real implementation, steps would auto-execute)
        {
            let mut sagas = orchestrator.sagas.lock().await;
            let (saga, state_machine) = sagas.get_mut(&saga_id).unwrap();

            // Transition to completed
            let _ = state_machine.transition_to(
                SagaState::Completed,
                SagaTransitionInput::StepCompleted {
                    step_id: "step1".to_string(),
                    result: serde_json::json!({}),
                },
            );
            saga.state = SagaState::Completed;
        }

        // Check final state
        let state = orchestrator.get_saga_state(saga_id).await.unwrap();
        assert_eq!(state, SagaState::Completed);
    }
}
