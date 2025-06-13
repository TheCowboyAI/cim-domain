//! Basic workflow example demonstrating category theory-based workflows
//!
//! This example shows:
//! - Creating injectable workflow states
//! - Defining transitions with guards
//! - Composing transitions using category operations
//! - Verifying category laws

use cim_domain::workflow::{
    WorkflowState, WorkflowCategory, SimpleState, SimpleInput, SimpleOutput,
    SimpleTransition, WorkflowContext, ContextKeyGuard, ActorGuard,
    WorkflowTransition,
};

fn main() {
    println!("=== Workflow Category Theory Example ===\n");

    // Create workflow states (injectable by users)
    let draft = SimpleState::new("Draft")
        .with_description("Document is being created");
    let review = SimpleState::new("Review")
        .with_description("Document is under review");
    let approved = SimpleState::new("Approved")
        .with_description("Document has been approved");
    let published = SimpleState::terminal("Published")
        .with_description("Document is published and immutable");
    let rejected = SimpleState::terminal("Rejected")
        .with_description("Document was rejected");

    println!("Created workflow states:");
    println!("- {} (terminal: {})", draft.name(), draft.is_terminal());
    println!("- {} (terminal: {})", review.name(), review.is_terminal());
    println!("- {} (terminal: {})", approved.name(), approved.is_terminal());
    println!("- {} (terminal: {})", published.name(), published.is_terminal());
    println!("- {} (terminal: {})", rejected.name(), rejected.is_terminal());

    // Create transitions
    let submit = SimpleTransition::new(
        "Submit for Review",
        draft.clone(),
        review.clone(),
        SimpleInput::new("submit"),
        SimpleOutput::new("submitted"),
    ).with_guard(Box::new(ContextKeyGuard::new("document_id")));

    let approve = SimpleTransition::new(
        "Approve",
        review.clone(),
        approved.clone(),
        SimpleInput::new("approve"),
        SimpleOutput::new("approved"),
    ).with_guard(Box::new(ActorGuard::single("reviewer")));

    let publish = SimpleTransition::new(
        "Publish",
        approved.clone(),
        published.clone(),
        SimpleInput::new("publish"),
        SimpleOutput::new("published"),
    ).with_guard(Box::new(ActorGuard::single("publisher")));

    let reject = SimpleTransition::new(
        "Reject",
        review.clone(),
        rejected.clone(),
        SimpleInput::new("reject"),
        SimpleOutput::new("rejected"),
    ).with_guard(Box::new(ActorGuard::single("reviewer")));

    println!("\nCreated transitions:");
    println!("- {}: {} -> {}", submit.name(), submit.source().name(), submit.target().name());
    println!("- {}: {} -> {}", approve.name(), approve.source().name(), approve.target().name());
    println!("- {}: {} -> {}", publish.name(), publish.source().name(), publish.target().name());
    println!("- {}: {} -> {}", reject.name(), reject.source().name(), reject.target().name());

    // Test guards
    println!("\nTesting transition guards:");

    let mut ctx = WorkflowContext::new();
    println!("Empty context - submit guard: {}", submit.guard(&ctx));

    ctx.set("document_id", "doc123").unwrap();
    println!("With document_id - submit guard: {}", submit.guard(&ctx));

    println!("Without actor - approve guard: {}", approve.guard(&ctx));

    ctx.set_actor("user".to_string());
    println!("With wrong actor - approve guard: {}", approve.guard(&ctx));

    ctx.set_actor("reviewer".to_string());
    println!("With correct actor - approve guard: {}", approve.guard(&ctx));

    // Demonstrate category operations
    println!("\n=== Category Theory Operations ===");

    let category = WorkflowCategory::new();

    // Create identity transition
    let id_review = category.identity_transition(review.clone());
    println!("\nIdentity transition: {} -> {}",
        id_review.source().name(),
        id_review.target().name()
    );

    // Compose transitions
    println!("\nComposing transitions:");
    let submit_box = Box::new(submit);
    let approve_box = Box::new(approve);

    match category.compose_transitions(submit_box, approve_box) {
        Ok(composed) => {
            println!("Successfully composed: {} -> {} -> {}",
                composed.source().name(),
                "Review", // intermediate state
                composed.target().name()
            );
        }
        Err(e) => println!("Composition failed: {}", e),
    }

    // Try invalid composition
    let publish_box = Box::new(publish);
    let reject_box = Box::new(reject);

    match category.compose_transitions(publish_box, reject_box) {
        Ok(_) => println!("Unexpected success!"),
        Err(e) => println!("Expected failure: {}", e),
    }

    // Verify category laws
    println!("\n=== Category Laws ===");
    println!("Associativity holds by construction: {}",
        category.verify_associativity_conceptual()
    );

    println!("\n=== Workflow Execution Context ===");

    // Create a workflow context with metadata
    let mut workflow_ctx = WorkflowContext::with_actor("alice".to_string());
    workflow_ctx.set_correlation_id("workflow-123".to_string());
    workflow_ctx.set("document_id", "doc456").unwrap();
    workflow_ctx.set("version", 2).unwrap();
    workflow_ctx.set("tags", vec!["important", "confidential"]).unwrap();

    println!("Workflow context:");
    println!("- Actor: {:?}", workflow_ctx.actor());
    println!("- Correlation ID: {:?}", workflow_ctx.correlation_id());
    println!("- Document ID: {:?}", workflow_ctx.get::<String>("document_id"));
    println!("- Version: {:?}", workflow_ctx.get::<i32>("version"));
    println!("- Tags: {:?}", workflow_ctx.get::<Vec<String>>("tags"));

    println!("\n=== Summary ===");
    println!("This example demonstrates:");
    println!("1. States are fully injectable (not hardcoded)");
    println!("2. Transitions are morphisms with guards");
    println!("3. Category operations (identity, composition)");
    println!("4. Category laws hold by construction");
    println!("5. Rich workflow context for runtime data");
}
