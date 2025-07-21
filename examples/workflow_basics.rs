// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating workflow basics with state machines
//!
//! This example shows:
//! - Creating state machines for workflows
//! - State transitions with validation
//! - Moore and Mealy machine patterns
//! - Generating events from transitions
//! - Building complete workflows

use chrono::{DateTime, Utc};
use cim_domain::{
    // Domain types
    DomainEvent,
    MealyStateTransitions,
    MooreStateTransitions,
    // State machine types
    State,
    TransitionInput,
    TransitionOutput,
};
use serde::{Deserialize, Serialize};

/// Workflow states for a document approval process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ApprovalState {
    Draft,
    Submitted,
    UnderReview,
    Approved,
    Rejected,
    Published,
}

impl State for ApprovalState {
    fn name(&self) -> &'static str {
        match self {
            ApprovalState::Draft => "Draft",
            ApprovalState::Submitted => "Submitted",
            ApprovalState::UnderReview => "UnderReview",
            ApprovalState::Approved => "Approved",
            ApprovalState::Rejected => "Rejected",
            ApprovalState::Published => "Published",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self, ApprovalState::Published | ApprovalState::Rejected)
    }
}

/// Output for approval transitions
#[derive(Debug, Clone)]
struct ApprovalOutput {
    message: String,
    timestamp: DateTime<Utc>,
    notify_users: Vec<String>,
}

impl Default for ApprovalOutput {
    fn default() -> Self {
        Self {
            message: String::new(),
            timestamp: Utc::now(),
            notify_users: Vec::new(),
        }
    }
}

impl TransitionOutput for ApprovalOutput {
    fn to_events(&self) -> Vec<Box<dyn DomainEvent>> {
        println!("      Output timestamp: {}", self.timestamp);
        vec![] // In real system, would generate domain events
    }
}

/// Moore machine implementation (output based on state only)
impl MooreStateTransitions for ApprovalState {
    type Output = ApprovalOutput;

    fn can_transition_to(&self, target: &Self) -> bool {
        use ApprovalState::*;
        match (self, target) {
            (Draft, Submitted) => true,
            (Submitted, UnderReview) => true,
            (UnderReview, Approved) => true,
            (UnderReview, Rejected) => true,
            (Approved, Published) => true,
            (Rejected, Draft) => true, // Allow resubmission
            _ => false,
        }
    }

    fn valid_transitions(&self) -> Vec<Self> {
        use ApprovalState::*;
        match self {
            Draft => vec![Submitted],
            Submitted => vec![UnderReview],
            UnderReview => vec![Approved, Rejected],
            Approved => vec![Published],
            Rejected => vec![Draft],
            Published => vec![],
        }
    }

    fn entry_output(&self) -> Self::Output {
        let message = match self {
            ApprovalState::Draft => "Document is in draft state",
            ApprovalState::Submitted => "Document submitted for review",
            ApprovalState::UnderReview => "Document is under review",
            ApprovalState::Approved => "Document has been approved",
            ApprovalState::Rejected => "Document has been rejected",
            ApprovalState::Published => "Document has been published",
        };

        let notify_users = match self {
            ApprovalState::Submitted => vec!["reviewers@example.com".to_string()],
            ApprovalState::Approved => vec!["author@example.com".to_string()],
            ApprovalState::Rejected => vec!["author@example.com".to_string()],
            ApprovalState::Published => vec!["subscribers@example.com".to_string()],
            _ => vec![],
        };

        ApprovalOutput {
            message: message.to_string(),
            timestamp: Utc::now(),
            notify_users,
        }
    }
}

/// Input for Mealy machine transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReviewInput {
    reviewer: String,
    comments: String,
    score: u8, // 0-100
}

impl TransitionInput for ReviewInput {
    fn description(&self) -> String {
        format!("Review by {} with score {}", self.reviewer, self.score)
    }
}

/// Mealy machine implementation (output based on state AND input)
impl MealyStateTransitions for ApprovalState {
    type Input = ReviewInput;
    type Output = ApprovalOutput;

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
        use ApprovalState::*;
        match (self, target) {
            (UnderReview, Approved) => input.score >= 70,
            (UnderReview, Rejected) => input.score < 70,
            _ => false,
        }
    }

    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
        use ApprovalState::*;
        match self {
            UnderReview => {
                if input.score >= 70 {
                    vec![Approved]
                } else {
                    vec![Rejected]
                }
            }
            _ => vec![],
        }
    }

    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output {
        let message = match (self, target) {
            (ApprovalState::UnderReview, ApprovalState::Approved) => {
                format!("Approved by {} with score {}", input.reviewer, input.score)
            }
            (ApprovalState::UnderReview, ApprovalState::Rejected) => {
                format!(
                    "Rejected by {} with score {}: {}",
                    input.reviewer, input.score, input.comments
                )
            }
            _ => "Invalid transition".to_string(),
        };

        ApprovalOutput {
            message,
            timestamp: Utc::now(),
            notify_users: vec!["author@example.com".to_string(), input.reviewer.clone()],
        }
    }
}

/// Simple workflow manager (not using aggregate-specific state machines)
#[derive(Debug)]
struct WorkflowManager {
    current_state: ApprovalState,
    transition_history: Vec<(ApprovalState, ApprovalState, DateTime<Utc>)>,
}

impl WorkflowManager {
    fn new() -> Self {
        Self {
            current_state: ApprovalState::Draft,
            transition_history: Vec::new(),
        }
    }

    fn transition_to(&mut self, target: ApprovalState) -> Result<ApprovalOutput, String> {
        if <ApprovalState as MooreStateTransitions>::can_transition_to(&self.current_state, &target)
        {
            let from = self.current_state;
            self.current_state = target;
            self.transition_history.push((from, target, Utc::now()));
            Ok(target.entry_output())
        } else {
            Err(format!(
                "Cannot transition from {} to {}",
                self.current_state.name(),
                target.name()
            ))
        }
    }

    fn transition_with_input(
        &mut self,
        target: ApprovalState,
        input: ReviewInput,
    ) -> Result<ApprovalOutput, String> {
        if <ApprovalState as MealyStateTransitions>::can_transition_to(
            &self.current_state,
            &target,
            &input,
        ) {
            let output = self.current_state.transition_output(&target, &input);
            let from = self.current_state;
            self.current_state = target;
            self.transition_history.push((from, target, Utc::now()));
            Ok(output)
        } else {
            Err(format!(
                "Cannot transition from {} to {} with input: {}",
                self.current_state.name(),
                target.name(),
                input.description()
            ))
        }
    }
}

fn main() {
    println!("Workflow Basics Example");
    println!("======================\n");

    // Example 1: Document approval workflow with Moore machine patterns
    println!("1. Document Approval Workflow...");

    let mut workflow = WorkflowManager::new();
    println!("   Initial state: {}", workflow.current_state.name());

    // Submit document
    match workflow.transition_to(ApprovalState::Submitted) {
        Ok(output) => {
            println!("\n   ✓ {}", output.message);
            if !output.notify_users.is_empty() {
                println!("     Notifying: {:?}", output.notify_users);
            }
        }
        Err(e) => println!("   ✗ {}", e),
    }

    // Start review
    match workflow.transition_to(ApprovalState::UnderReview) {
        Ok(output) => {
            println!("\n   ✓ {}", output.message);
            if !output.notify_users.is_empty() {
                println!("     Notifying: {:?}", output.notify_users);
            }
        }
        Err(e) => println!("   ✗ {}", e),
    }

    // Approve document
    match workflow.transition_to(ApprovalState::Approved) {
        Ok(output) => {
            println!("\n   ✓ {}", output.message);
            if !output.notify_users.is_empty() {
                println!("     Notifying: {:?}", output.notify_users);
            }
        }
        Err(e) => println!("   ✗ {}", e),
    }

    // Publish document
    match workflow.transition_to(ApprovalState::Published) {
        Ok(output) => {
            println!("\n   ✓ {}", output.message);
            if !output.notify_users.is_empty() {
                println!("     Notifying: {:?}", output.notify_users);
            }
        }
        Err(e) => println!("   ✗ {}", e),
    }

    println!("\n   Final state: {}", workflow.current_state.name());
    println!("   Is terminal: {}", workflow.current_state.is_terminal());

    // Example 2: Review task with Mealy machine patterns
    println!("\n2. Review Task Workflow...");

    let mut review_workflow = WorkflowManager::new();

    // Move to review state
    review_workflow.transition_to(ApprovalState::Submitted).ok();
    review_workflow
        .transition_to(ApprovalState::UnderReview)
        .ok();

    // Perform review with high score
    let good_review = ReviewInput {
        reviewer: "alice@example.com".to_string(),
        comments: "Excellent work, well structured.".to_string(),
        score: 85,
    };

    match review_workflow.transition_with_input(ApprovalState::Approved, good_review) {
        Ok(output) => {
            println!("\n   ✓ Review completed: {}", output.message);
            println!("     Notifying: {:?}", output.notify_users);
        }
        Err(e) => println!("   ✗ {}", e),
    }

    // Example 3: Another review with low score
    println!("\n3. Low Score Review...");

    let mut review_workflow2 = WorkflowManager::new();
    review_workflow2
        .transition_to(ApprovalState::Submitted)
        .ok();
    review_workflow2
        .transition_to(ApprovalState::UnderReview)
        .ok();

    let poor_review = ReviewInput {
        reviewer: "bob@example.com".to_string(),
        comments: "Missing key metrics, needs revision.".to_string(),
        score: 45,
    };

    match review_workflow2.transition_with_input(ApprovalState::Rejected, poor_review) {
        Ok(output) => {
            println!("\n   ✓ Review completed: {}", output.message);
            println!("     Notifying: {:?}", output.notify_users);
        }
        Err(e) => println!("   ✗ {}", e),
    }

    // Example 4: Invalid transitions
    println!("\n4. Testing Invalid Transitions...");

    let mut test_workflow = WorkflowManager::new();

    // Try to approve directly from draft (should fail)
    println!("   Trying to approve from draft state...");
    match test_workflow.transition_to(ApprovalState::Approved) {
        Ok(_) => println!("   ✓ Unexpectedly succeeded"),
        Err(e) => println!("   ✗ Expected error: {}", e),
    }

    // Example 5: State queries
    println!("\n5. State Machine Queries...");

    let states = vec![
        ApprovalState::Draft,
        ApprovalState::Submitted,
        ApprovalState::UnderReview,
        ApprovalState::Approved,
        ApprovalState::Rejected,
        ApprovalState::Published,
    ];

    println!("   Terminal states:");
    for state in &states {
        if state.is_terminal() {
            println!("     - {}", state.name());
        }
    }

    println!("\n   Valid transitions from Draft:");
    let draft = ApprovalState::Draft;
    for target in &states {
        if <ApprovalState as MooreStateTransitions>::can_transition_to(&draft, target) {
            println!("     - Draft → {}", target.name());
        }
    }

    // Example 6: Transition history
    println!("\n6. Transition History...");

    println!("   Workflow 1 transitions:");
    for (from, to, timestamp) in &workflow.transition_history {
        println!(
            "     {} → {} at {}",
            from.name(),
            to.name(),
            timestamp.format("%H:%M:%S")
        );
    }

    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Moore machines (output based on state)");
    println!("  • Mealy machines (output based on state + input)");
    println!("  • State transition validation");
    println!("  • Workflow implementation patterns");
    println!("  • Error handling for invalid transitions");

    println!("\nKey Concepts:");
    println!("  • States define the possible stages");
    println!("  • Transitions are validated before execution");
    println!("  • Outputs can trigger notifications/events");
    println!("  • Terminal states end the workflow");
}
