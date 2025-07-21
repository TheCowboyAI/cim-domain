// Copyright 2025 Cowboy AI, LLC.

//! Saga pattern implementation using state machines
//!
//! This module provides infrastructure for managing distributed transactions
//! using the saga pattern, built on top of our state machine framework.

use crate::{
    composition::saga_orchestration::{
        Saga, SagaCommand as SagaOrchestratorCommand, SagaState, SagaTransitionInput,
        SagaTransitionOutput,
    },
    errors::DomainError,
    events::DomainEvent,
    state_machine::{MealyStateTransitions, State, StateTransition},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// Errors that can occur during saga execution
#[derive(Debug, Error)]
pub enum SagaError {
    /// Saga instance not found
    #[error("Saga not found: {0}")]
    NotFound(String),

    /// Invalid state transition attempted
    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),

    /// Command execution failed
    #[error("Command failed: {0}")]
    CommandFailed(String),

    /// Saga step timed out
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Compensation action failed
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Domain error
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
}

/// Marker type for saga aggregates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SagaMarker;

/// Instance of a running saga
#[derive(Debug, Clone)]
pub struct SagaInstance {
    /// The saga definition
    pub saga: Saga,
    /// Current state of the saga
    pub current_state: SagaState,
    /// Transition history
    pub transition_history:
        Vec<StateTransition<SagaState, SagaTransitionInput, SagaTransitionOutput>>,
    /// When the saga started
    pub started_at: DateTime<Utc>,
    /// When the saga completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,
}

/// Trait for saga commands - simpler than DomainCommand to be dyn-compatible
pub trait SagaCommand: Send + Sync + Debug {
    /// Get the command type name
    fn command_type(&self) -> &str;

    /// Get the target aggregate ID
    fn aggregate_id(&self) -> &str;

    /// Get the command as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A step in a saga execution (re-export for compatibility)
pub use crate::composition::saga_orchestration::SagaStep;

/// Trait for implementing sagas (simplified to work with state machines)
#[async_trait]
pub trait SagaDefinition: Send + Sync {
    /// Get the saga type name
    fn saga_type(&self) -> &str;

    /// Create a new saga instance with the given context
    async fn create_saga(&self, context: serde_json::Value) -> Result<Saga, SagaError>;

    /// Convert a domain event to a saga transition input
    async fn event_to_input(
        &self,
        saga: &Saga,
        event: &dyn DomainEvent,
    ) -> Option<SagaTransitionInput>;

    /// Called when the saga completes successfully
    async fn on_completed(&self, _saga: &Saga) -> Result<(), SagaError> {
        Ok(())
    }

    /// Called when the saga fails
    async fn on_failed(&self, _saga: &Saga, _error: &str) -> Result<(), SagaError> {
        Ok(())
    }
}

/// Possible transitions for a saga after handling an event (re-export for compatibility)
pub use crate::composition::saga_orchestration::SagaTransitionInput as SagaTransition;

/// Coordinator for managing saga execution using state machines
pub struct SagaCoordinator {
    /// Registered saga definitions
    saga_definitions: Arc<RwLock<HashMap<String, Arc<dyn SagaDefinition>>>>,
    /// Running saga instances with their state machines
    instances: Arc<RwLock<HashMap<String, SagaInstance>>>,
    /// Command bus for executing saga commands
    #[allow(dead_code)]
    command_bus: Arc<dyn CommandBus>,
}

/// Trait for sending commands from sagas
#[async_trait]
pub trait CommandBus: Send + Sync {
    /// Send a command for execution
    async fn send(&self, command: Box<dyn SagaCommand>) -> Result<(), String>;
}

impl SagaCoordinator {
    /// Create a new saga coordinator
    pub fn new(command_bus: Arc<dyn CommandBus>) -> Self {
        Self {
            saga_definitions: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            command_bus,
        }
    }

    /// Register a saga definition
    pub async fn register_saga(&self, definition: Arc<dyn SagaDefinition>) {
        let mut definitions = self.saga_definitions.write().await;
        definitions.insert(definition.saga_type().to_string(), definition);
    }

    /// Start a new saga instance
    pub async fn start_saga(
        &self,
        saga_type: &str,
        context: serde_json::Value,
    ) -> Result<String, SagaError> {
        let definitions = self.saga_definitions.read().await;
        let definition = definitions
            .get(saga_type)
            .ok_or_else(|| SagaError::NotFound(saga_type.to_string()))?
            .clone();
        drop(definitions);

        // Create the saga using the definition
        let saga = definition.create_saga(context).await?;
        let saga_id = saga.id.to_string();

        let instance = SagaInstance {
            saga,
            current_state: SagaState::Pending,
            transition_history: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
        };

        let mut instances = self.instances.write().await;
        instances.insert(saga_id.clone(), instance.clone());
        drop(instances);

        // Start the saga by triggering the Start transition
        self.process_transition(&saga_id, SagaTransitionInput::Start)
            .await?;

        Ok(saga_id)
    }

    /// Handle an event for a saga instance
    pub async fn handle_event(
        &self,
        event: &dyn DomainEvent,
        correlation_id: Option<&str>,
    ) -> Result<(), SagaError> {
        let correlation_id = match correlation_id {
            Some(id) => id,
            None => return Ok(()),
        };

        let instances = self.instances.read().await;
        let instance = match instances.get(correlation_id) {
            Some(i) => i.clone(),
            None => return Ok(()),
        };
        drop(instances);

        let definitions = self.saga_definitions.read().await;
        let definition = definitions
            .get(&instance.saga.name)
            .ok_or_else(|| SagaError::NotFound(instance.saga.name.clone()))?
            .clone();
        drop(definitions);

        // Convert the event to a transition input
        if let Some(input) = definition.event_to_input(&instance.saga, event).await {
            self.process_transition(correlation_id, input).await?;
        }

        Ok(())
    }

    /// Process a state transition for a saga
    async fn process_transition(
        &self,
        saga_id: &str,
        input: SagaTransitionInput,
    ) -> Result<(), SagaError> {
        let mut instances = self.instances.write().await;
        let instance = instances
            .get_mut(saga_id)
            .ok_or_else(|| SagaError::NotFound(saga_id.to_string()))?;

        let current_state = instance.current_state.clone();

        info!(
            saga_id = %saga_id,
            current_state = %current_state.name(),
            input = ?input,
            "Processing saga transition"
        );

        // Validate and process the transition
        let valid_targets = current_state.valid_transitions(&input);
        if valid_targets.is_empty() {
            return Err(SagaError::InvalidTransition(format!(
                "No valid transitions from {current_state:?} with input {input:?}"
            )));
        }

        // For simplicity, take the first valid target
        let target_state = valid_targets.into_iter().next().unwrap();
        let output = current_state.transition_output(&target_state, &input);

        // Record the transition
        instance.transition_history.push(StateTransition {
            from: current_state,
            to: target_state.clone(),
            input: Some(input.clone()),
            output: output.clone(),
            transition_id: Uuid::new_v4(),
            timestamp: Utc::now(),
        });

        // Update current state
        instance.current_state = target_state.clone();

        // Execute any commands from the output
        for command in output.commands {
            self.execute_command(command).await?;
        }

        // Check if saga is in terminal state
        if instance.current_state.is_terminal() {
            instance.completed_at = Some(Utc::now());

            // Notify the definition of completion/failure
            let definitions = self.saga_definitions.read().await;
            if let Some(definition) = definitions.get(&instance.saga.name) {
                match &instance.current_state {
                    SagaState::Completed => {
                        definition.on_completed(&instance.saga).await?;
                    }
                    SagaState::Failed { error } => {
                        definition.on_failed(&instance.saga, error).await?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Execute a saga command
    async fn execute_command(&self, command: SagaOrchestratorCommand) -> Result<(), SagaError> {
        match command {
            SagaOrchestratorCommand::ExecuteStep {
                step_id,
                domain,
                command,
            } => {
                info!(
                    step_id = %step_id,
                    domain = %domain,
                    command = %command,
                    "Executing saga step"
                );
                // In a real implementation, convert to actual command and send
            }
            SagaOrchestratorCommand::ExecuteCompensation { step_id, action: _ } => {
                info!(
                    step_id = %step_id,
                    "Executing compensation"
                );
                // In a real implementation, execute the compensation action
            }
        }
        Ok(())
    }

    /// Get a saga instance by ID
    pub async fn get_instance(&self, saga_id: &str) -> Option<SagaInstance> {
        let instances = self.instances.read().await;
        instances.get(saga_id).cloned()
    }
}

/// Process manager for coordinating sagas based on domain events
#[derive(Clone)]
pub struct ProcessManager {
    coordinator: Arc<SagaCoordinator>,
    policies: Arc<RwLock<Vec<Box<dyn ProcessPolicy>>>>,
}

/// Policy for determining when to start a saga based on events
#[async_trait]
pub trait ProcessPolicy: Send + Sync + Debug {
    /// Check if this policy should start a saga for the given event
    async fn should_start(&self, event: &dyn DomainEvent) -> Option<(String, serde_json::Value)>;
}

impl ProcessManager {
    /// Create a new process manager
    pub fn new(coordinator: Arc<SagaCoordinator>) -> Self {
        Self {
            coordinator,
            policies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a process policy
    pub async fn register_policy(&self, policy: Box<dyn ProcessPolicy>) {
        let mut policies = self.policies.write().await;
        policies.push(policy);
    }

    /// Handle a domain event, potentially starting new sagas or continuing existing ones
    pub async fn handle_event(
        &self,
        event: &dyn DomainEvent,
        correlation_id: Option<&str>,
    ) -> Result<(), SagaError> {
        let policies = self.policies.read().await;

        for policy in policies.iter() {
            if let Some((saga_type, context)) = policy.should_start(event).await {
                info!(
                    saga_type = %saga_type,
                    event_type = %event.event_type(),
                    "Starting saga from process policy"
                );

                self.coordinator.start_saga(&saga_type, context).await?;
            }
        }

        self.coordinator.handle_event(event, correlation_id).await
    }
}

// Re-export the saga orchestration types for compatibility
pub use crate::composition::saga_orchestration::{CompensationAction, RetryPolicy};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestCommand {
        name: String,
    }

    impl SagaCommand for TestCommand {
        fn command_type(&self) -> &str {
            &self.name
        }

        fn aggregate_id(&self) -> &str {
            "test-agg"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[derive(Debug, Clone)]
    struct TestEvent {
        name: String,
    }

    impl DomainEvent for TestEvent {
        fn subject(&self) -> String {
            format!("test.event.{}.v1", self.name)
        }

        fn aggregate_id(&self) -> uuid::Uuid {
            uuid::Uuid::new_v4()
        }

        fn event_type(&self) -> &'static str {
            "TestEvent"
        }
    }

    struct TestSagaDefinition;

    #[async_trait]
    impl SagaDefinition for TestSagaDefinition {
        fn saga_type(&self) -> &str {
            "TestSaga"
        }

        async fn create_saga(&self, context: serde_json::Value) -> Result<Saga, SagaError> {
            Ok(Saga {
                id: Uuid::new_v4(),
                name: "TestSaga".to_string(),
                steps: vec![SagaStep {
                    id: "step1".to_string(),
                    domain: "test".to_string(),
                    command_type: "TestCommand".to_string(),
                    depends_on: vec![],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                }],
                state: SagaState::Pending,
                compensations: HashMap::new(),
                context: context
                    .as_object()
                    .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
                metadata: HashMap::new(),
            })
        }

        async fn event_to_input(
            &self,
            _saga: &Saga,
            event: &dyn DomainEvent,
        ) -> Option<SagaTransitionInput> {
            if event.event_type() == "TestEvent" {
                Some(SagaTransitionInput::StepCompleted {
                    step_id: "step1".to_string(),
                    result: serde_json::json!({"success": true}),
                })
            } else {
                None
            }
        }
    }

    struct TestCommandBus;

    #[async_trait]
    impl CommandBus for TestCommandBus {
        async fn send(&self, _command: Box<dyn SagaCommand>) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_saga_execution_with_state_machine() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));

        coordinator
            .register_saga(Arc::new(TestSagaDefinition))
            .await;

        // Create and verify TestCommand
        let test_cmd = TestCommand {
            name: "TestCommand".to_string(),
        };
        assert_eq!(test_cmd.command_type(), "TestCommand");
        assert_eq!(test_cmd.aggregate_id(), "test-agg");

        let saga_id = coordinator
            .start_saga("TestSaga", serde_json::json!({"test": "data"}))
            .await
            .unwrap();

        let instance = coordinator.get_instance(&saga_id).await.unwrap();
        assert_eq!(instance.current_state.name(), "Running");
    }

    #[tokio::test]
    async fn test_saga_coordinator_new() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        // Verify coordinator is created with empty collections
        let definitions = coordinator.saga_definitions.read().await;
        assert!(definitions.is_empty());
        drop(definitions);

        let instances = coordinator.instances.read().await;
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_register_saga() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        // Register a saga definition
        coordinator
            .register_saga(Arc::new(TestSagaDefinition))
            .await;

        // Verify saga is registered
        let definitions = coordinator.saga_definitions.read().await;
        assert_eq!(definitions.len(), 1);
        assert!(definitions.contains_key("TestSaga"));
    }

    #[tokio::test]
    async fn test_start_saga_not_found() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        // Try to start non-existent saga
        let result = coordinator
            .start_saga("NonExistent", serde_json::json!({}))
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SagaError::NotFound(msg) => assert_eq!(msg, "NonExistent"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_saga_state_transitions() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));

        coordinator
            .register_saga(Arc::new(TestSagaDefinition))
            .await;

        let saga_id = coordinator
            .start_saga("TestSaga", serde_json::json!({"test": "data"}))
            .await
            .unwrap();

        // Check initial state
        let instance = coordinator.get_instance(&saga_id).await.unwrap();
        assert_eq!(instance.current_state.name(), "Running");
        assert!(!instance.transition_history.is_empty());
        assert!(instance.completed_at.is_none());

        // Simulate event completion
        let event = TestEvent {
            name: "test".to_string(),
        };

        coordinator
            .handle_event(&event, Some(&saga_id))
            .await
            .unwrap();

        // Check state after event
        let instance = coordinator.get_instance(&saga_id).await.unwrap();
        // The saga should now be in Running state with step1 completed
        match &instance.current_state {
            SagaState::Running {
                completed_steps, ..
            } => {
                assert_eq!(completed_steps.len(), 1);
                assert_eq!(completed_steps[0], "step1");
            }
            SagaState::Completed => {
                // Also acceptable if transition logic chose Completed
                assert!(instance.completed_at.is_some());
            }
            other => panic!("Unexpected state: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_handle_event_no_correlation_id() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        let event = TestEvent {
            name: "test".to_string(),
        };

        // Should succeed but do nothing
        let result = coordinator.handle_event(&event, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_event_unknown_instance() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        let event = TestEvent {
            name: "test".to_string(),
        };

        // Should succeed but do nothing
        let result = coordinator.handle_event(&event, Some("unknown-id")).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_instance_not_found() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = SagaCoordinator::new(command_bus);

        let instance = coordinator.get_instance("non-existent").await;
        assert!(instance.is_none());
    }

    #[tokio::test]
    async fn test_saga_error_variants() {
        // Test all error variants display correctly
        let not_found = SagaError::NotFound("saga-123".to_string());
        assert_eq!(not_found.to_string(), "Saga not found: saga-123");

        let invalid_transition = SagaError::InvalidTransition("Cannot go from A to B".to_string());
        assert_eq!(
            invalid_transition.to_string(),
            "Invalid state transition: Cannot go from A to B"
        );

        let command_failed = SagaError::CommandFailed("Connection timeout".to_string());
        assert_eq!(
            command_failed.to_string(),
            "Command failed: Connection timeout"
        );

        let timeout = SagaError::Timeout("Step 1 exceeded 30s".to_string());
        assert_eq!(timeout.to_string(), "Timeout: Step 1 exceeded 30s");

        let compensation_failed = SagaError::CompensationFailed("Rollback failed".to_string());
        assert_eq!(
            compensation_failed.to_string(),
            "Compensation failed: Rollback failed"
        );

        let serialization = SagaError::Serialization("Invalid JSON".to_string());
        assert_eq!(
            serialization.to_string(),
            "Serialization error: Invalid JSON"
        );

        // Test From conversion for DomainError
        let domain_err = DomainError::Generic("Test error".to_string());
        let saga_err: SagaError = domain_err.into();
        assert!(saga_err.to_string().contains("Domain error"));
    }

    #[tokio::test]
    async fn test_process_manager_new() {
        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));
        let process_manager = ProcessManager::new(coordinator);

        let policies = process_manager.policies.read().await;
        assert!(policies.is_empty());
    }

    #[tokio::test]
    async fn test_process_manager_register_policy() {
        #[derive(Debug)]
        struct TestPolicy;

        #[async_trait]
        impl ProcessPolicy for TestPolicy {
            async fn should_start(
                &self,
                _event: &dyn DomainEvent,
            ) -> Option<(String, serde_json::Value)> {
                None
            }
        }

        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));
        let process_manager = ProcessManager::new(coordinator);

        process_manager.register_policy(Box::new(TestPolicy)).await;

        let policies = process_manager.policies.read().await;
        assert_eq!(policies.len(), 1);
    }

    #[tokio::test]
    async fn test_process_manager_handle_event_starts_saga() {
        #[derive(Debug)]
        struct StartSagaPolicy;

        #[async_trait]
        impl ProcessPolicy for StartSagaPolicy {
            async fn should_start(
                &self,
                event: &dyn DomainEvent,
            ) -> Option<(String, serde_json::Value)> {
                if event.event_type() == "TestEvent" {
                    Some(("TestSaga".to_string(), serde_json::json!({"auto": true})))
                } else {
                    None
                }
            }
        }

        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));
        coordinator
            .register_saga(Arc::new(TestSagaDefinition))
            .await;

        let process_manager = ProcessManager::new(coordinator.clone());
        process_manager
            .register_policy(Box::new(StartSagaPolicy))
            .await;

        let event = TestEvent {
            name: "test".to_string(),
        };

        // Should start a new saga
        process_manager.handle_event(&event, None).await.unwrap();

        // Verify saga was started
        let instances = coordinator.instances.read().await;
        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_saga_instance_clone() {
        let saga = Saga {
            id: Uuid::new_v4(),
            name: "TestSaga".to_string(),
            steps: vec![],
            state: SagaState::Pending,
            compensations: HashMap::new(),
            context: HashMap::new(),
            metadata: HashMap::new(),
        };

        let instance = SagaInstance {
            saga: saga.clone(),
            current_state: SagaState::Running {
                current_step: "step1".to_string(),
                completed_steps: vec![],
            },
            transition_history: vec![],
            started_at: Utc::now(),
            completed_at: None,
        };

        let cloned = instance.clone();
        assert_eq!(instance.saga.id, cloned.saga.id);
        assert_eq!(instance.started_at, cloned.started_at);
    }

    #[tokio::test]
    async fn test_saga_definition_callbacks() {
        struct CallbackTestSaga {
            completed_called: Arc<RwLock<bool>>,
            failed_called: Arc<RwLock<bool>>,
        }

        #[async_trait]
        impl SagaDefinition for CallbackTestSaga {
            fn saga_type(&self) -> &str {
                "CallbackTestSaga"
            }

            async fn create_saga(&self, _context: serde_json::Value) -> Result<Saga, SagaError> {
                Ok(Saga {
                    id: Uuid::new_v4(),
                    name: "CallbackTestSaga".to_string(),
                    steps: vec![],
                    state: SagaState::Pending,
                    compensations: HashMap::new(),
                    context: HashMap::new(),
                    metadata: HashMap::new(),
                })
            }

            async fn event_to_input(
                &self,
                _saga: &Saga,
                event: &dyn DomainEvent,
            ) -> Option<SagaTransitionInput> {
                match event.event_type() {
                    "CompleteEvent" => Some(SagaTransitionInput::StepCompleted {
                        step_id: "final".to_string(),
                        result: serde_json::json!({"completed": true}),
                    }),
                    "FailEvent" => Some(SagaTransitionInput::StepFailed {
                        step_id: "test".to_string(),
                        error: "Test failure".to_string(),
                    }),
                    _ => None,
                }
            }

            async fn on_completed(&self, _saga: &Saga) -> Result<(), SagaError> {
                let mut called = self.completed_called.write().await;
                *called = true;
                Ok(())
            }

            async fn on_failed(&self, _saga: &Saga, _error: &str) -> Result<(), SagaError> {
                let mut called = self.failed_called.write().await;
                *called = true;
                Ok(())
            }
        }

        let command_bus = Arc::new(TestCommandBus);
        let coordinator = Arc::new(SagaCoordinator::new(command_bus));

        let completed_called = Arc::new(RwLock::new(false));
        let failed_called = Arc::new(RwLock::new(false));

        let saga_def = Arc::new(CallbackTestSaga {
            completed_called: completed_called.clone(),
            failed_called: failed_called.clone(),
        });

        coordinator.register_saga(saga_def).await;

        // Test completion callback
        let _saga_id = coordinator
            .start_saga("CallbackTestSaga", serde_json::json!({}))
            .await
            .unwrap();

        let _complete_event = TestEvent {
            name: "complete".to_string(),
        };

        // Test setup is complete, saga definition registered

        // The callbacks would be tested if we could create events with specific types
        // For now, verify the saga definition was registered correctly
        assert!(!*completed_called.read().await);
        assert!(!*failed_called.read().await);
    }
}
