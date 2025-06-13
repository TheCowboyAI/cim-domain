//! Comprehensive workflow execution example
//!
//! This example demonstrates:
//! - Creating a workflow definition (graph)
//! - Starting a workflow instance (aggregate)
//! - Executing transitions
//! - Handling workflow lifecycle (suspend, resume, cancel)
//! - Using components for extensibility

use cim_domain::{
    GraphId, Component, AggregateRoot,
    workflow::{
        WorkflowAggregate, WorkflowCommand, WorkflowStatus,
        SimpleState, SimpleInput, SimpleOutput, SimpleTransition,
        WorkflowContext, WorkflowState, WorkflowTransition,
        WorkflowCategory,
    },
};
use std::time::Duration;

/// Custom component for tracking approvals
#[derive(Debug, Clone)]
struct ApprovalComponent {
    approvers: Vec<String>,
    required_approvals: usize,
    approved_by: Vec<String>,
}

impl Component for ApprovalComponent {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
    fn type_name(&self) -> &'static str { "ApprovalComponent" }
}

fn main() {
    println!("=== Workflow Execution Example ===\n");

    // Define workflow states
    let draft = SimpleState::new("Draft")
        .with_description("Document is being created");
    let review = SimpleState::new("UnderReview")
        .with_description("Document is under review");
    let approved = SimpleState::new("Approved")
        .with_description("Document has been approved");
    let published = SimpleState::terminal("Published")
        .with_description("Document has been published");
    let _rejected = SimpleState::terminal("Rejected")
        .with_description("Document was rejected");

    // Create workflow context with initial data
    let mut context = WorkflowContext::new();
    context.set("document_id", "DOC-123").unwrap();
    context.set("author", "Alice").unwrap();
    context.set("department", "Engineering").unwrap();

    // Create workflow instance
    let definition_id = GraphId::new();
    let mut workflow = WorkflowAggregate::new(
        definition_id,
        draft.clone(),
        context,
    );

    println!("Workflow created: {:?}", workflow.id());
    println!("Initial state: {}", workflow.current_state().name());
    println!("Status: {:?}", workflow.status());
    println!();

    // Add approval component
    let approval_component = ApprovalComponent {
        approvers: vec!["Bob".to_string(), "Charlie".to_string()],
        required_approvals: 2,
        approved_by: vec![],
    };
    workflow.add_component(approval_component).unwrap();

    // Simulate workflow execution
    println!("=== Executing Workflow ===\n");

    // Transition 1: Draft -> UnderReview
    println!("1. Submitting for review...");
    workflow.record_transition(
        draft.clone(),
        review.clone(),
        SimpleInput::new("submit"),
        SimpleOutput::new("submitted"),
        Duration::from_millis(50),
    );
    println!("   Current state: {}", workflow.current_state().name());
    println!("   Transitions executed: {}", workflow.transition_count());

    // Suspend workflow
    println!("\n2. Suspending workflow for external approval...");
    workflow.suspend("Waiting for external approval").unwrap();
    println!("   Status: {:?}", workflow.status());
    println!("   Can transition: {}", workflow.can_transition());

    // Resume workflow
    println!("\n3. Resuming workflow after approval received...");
    workflow.resume().unwrap();
    println!("   Status: {:?}", workflow.status());

    // Update approval component
    if let Some(approval) = workflow.get_component::<ApprovalComponent>() {
        let mut updated_approval = approval.clone();
        updated_approval.approved_by.push("Bob".to_string());
        updated_approval.approved_by.push("Charlie".to_string());
        workflow.remove_component::<ApprovalComponent>();
        workflow.add_component(updated_approval).unwrap();
    }

    // Transition 2: UnderReview -> Approved
    println!("\n4. Approving document...");
    workflow.record_transition(
        review.clone(),
        approved.clone(),
        SimpleInput::new("approve"),
        SimpleOutput::new("approved"),
        Duration::from_millis(100),
    );
    println!("   Current state: {}", workflow.current_state().name());

    // Transition 3: Approved -> Published
    println!("\n5. Publishing document...");
    workflow.record_transition(
        approved.clone(),
        published.clone(),
        SimpleInput::new("publish"),
        SimpleOutput::new("published"),
        Duration::from_millis(200),
    );
    println!("   Current state: {}", workflow.current_state().name());
    println!("   Is terminal: {}", workflow.is_terminal());
    println!("   Status: {:?}", workflow.status());

    // Display workflow summary
    println!("\n=== Workflow Summary ===");
    println!("Workflow ID: {:?}", workflow.id());
    println!("Definition ID: {:?}", workflow.definition_id);
    println!("Total duration: {:?}", workflow.duration());
    println!("Total transitions: {}", workflow.transition_count());
    println!("Final state: {}", workflow.current_state().name());
    println!("Final status: {:?}", workflow.status());
    println!("Version: {}", workflow.version());

    // Display transition history
    println!("\n=== Transition History ===");
    for (i, transition) in workflow.history.iter().enumerate() {
        println!("{}. {} -> {} (took {:?})",
            i + 1,
            transition.from_state.name(),
            transition.to_state.name(),
            transition.duration
        );
    }

    // Demonstrate category operations
    println!("\n=== Category Operations ===");
    demonstrate_category_operations();

    // Demonstrate command creation
    println!("\n=== Workflow Commands ===");
    demonstrate_commands();
}

fn demonstrate_category_operations() {
    // Create states
    let a = SimpleState::new("A");
    let b = SimpleState::new("B");
    let c = SimpleState::new("C");

    // Create transitions
    let t1 = Box::new(SimpleTransition::new(
        "A_to_B".to_string(),
        a.clone(),
        b.clone(),
        SimpleInput::new("go_to_b"),
        SimpleOutput::new("at_b"),
    ));

    let t2 = Box::new(SimpleTransition::new(
        "B_to_C".to_string(),
        b.clone(),
        c.clone(),
        SimpleInput::new("go_to_c"),
        SimpleOutput::new("at_c"),
    ));

    // Create category
    let category = WorkflowCategory::<SimpleState, SimpleInput, SimpleOutput>::new();

    // Test composition
    match category.compose_transitions(t1, t2) {
        Ok(composed) => {
            println!("✓ Composed transition: {} -> {}",
                composed.source().name(),
                composed.target().name()
            );
        }
        Err(e) => println!("✗ Composition failed: {}", e),
    }

    // Test identity
    let identity = category.identity_transition(a.clone());
    println!("✓ Identity transition for state '{}': {} -> {}",
        a.name(),
        identity.source().name(),
        identity.target().name()
    );
}

fn demonstrate_commands() {
    // Start workflow command
    let start_cmd = WorkflowCommand::<SimpleInput>::StartWorkflow {
        definition_id: GraphId::new(),
        initial_context: WorkflowContext::new(),
        workflow_id: None,
        start_time: None,
    };
    println!("✓ StartWorkflow command created");
    println!("  Is creation: {}", start_cmd.is_creation());
    println!("  Is state changing: {}", start_cmd.is_state_changing());

    // Execute transition command
    let workflow_id = cim_domain::WorkflowId::new();
    let transition_cmd = WorkflowCommand::ExecuteTransition {
        workflow_id,
        input: SimpleInput::new("approve"),
        context_updates: None,
    };
    println!("\n✓ ExecuteTransition command created");
    println!("  Target workflow: {:?}", transition_cmd.workflow_id());
    println!("  Is state changing: {}", transition_cmd.is_state_changing());

    // Suspend workflow command
    let _suspend_cmd = WorkflowCommand::<SimpleInput>::SuspendWorkflow {
        workflow_id,
        reason: "Waiting for external system".to_string(),
        expires_at: None,
    };
    println!("\n✓ SuspendWorkflow command created");
    println!("  Reason: Waiting for external system");
}
