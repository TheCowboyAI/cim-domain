// Copyright 2025 Cowboy AI, LLC.

//! Simple example demonstrating core cim-domain functionality
//!
//! This example shows:
//! - Creating entities with components
//! - Using state machines for state management
//! - Command and event handling patterns

use cim_domain::{
    markers::AggregateMarker,

    // State machine
    state_machine::{DocumentState, MooreMachine},

    AggregateRoot,
    // CQRS
    Command,
    CommandEnvelope,
    Component,
    DomainResult,
    // Core types
    EntityId,
};
use std::any::Any;

/// Example component for storing metadata
#[derive(Debug, Clone)]
struct MetadataComponent {
    title: String,
    author: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl MetadataComponent {
    fn display(&self) {
        println!("  Title: {}", self.title);
        println!("  Author: {}", self.author);
        println!("  Created: {}", self.created_at);
    }
}

impl Component for MetadataComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "MetadataComponent"
    }
}

/// Example aggregate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocumentAggregate;

impl AggregateRoot for DocumentAggregate {
    type Id = EntityId<AggregateMarker>;

    fn id(&self) -> Self::Id {
        EntityId::new()
    }

    fn version(&self) -> u64 {
        1
    }

    fn increment_version(&mut self) {
        // In a real implementation, this would increment a version field
    }
}

/// Wrapper for the aggregate with state machine
struct DocumentAggregateWrapper {
    aggregate: DocumentAggregate,
    state_machine: MooreMachine<DocumentState, DocumentAggregate>,
    version: u64,
}

impl DocumentAggregateWrapper {
    fn new() -> Self {
        let aggregate = DocumentAggregate;
        let aggregate_id = EntityId::<DocumentAggregate>::new();
        let state_machine = MooreMachine::new(DocumentState::Draft, aggregate_id);

        Self {
            aggregate,
            state_machine,
            version: 0,
        }
    }

    fn submit_for_review(&mut self) -> DomainResult<()> {
        self.state_machine
            .transition_to(DocumentState::UnderReview)?;
        self.version += 1;
        Ok(())
    }

    fn approve(&mut self) -> DomainResult<()> {
        self.state_machine.transition_to(DocumentState::Approved)?;
        self.version += 1;
        Ok(())
    }

    fn current_state(&self) -> &DocumentState {
        self.state_machine.current_state()
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn aggregate_id(&self) -> EntityId<AggregateMarker> {
        self.aggregate.id()
    }
}

/// Example command
#[derive(Debug, Clone)]
enum DocumentCommand {
    Create {
        title: String,
        author: String,
    },
    SubmitForReview {
        document_id: EntityId<AggregateMarker>,
    },
    Approve {
        document_id: EntityId<AggregateMarker>,
    },
}

impl Command for DocumentCommand {
    type Aggregate = AggregateMarker;

    fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
        match self {
            Self::Create { .. } => None,
            Self::SubmitForReview { document_id } => Some(*document_id),
            Self::Approve { document_id } => Some(*document_id),
        }
    }
}

fn main() {
    println!("=== CIM Domain Simple Example ===\n");

    // Create a document aggregate wrapper
    let mut document = DocumentAggregateWrapper::new();

    println!("Created document");
    println!("Initial state: {:?}", document.current_state());
    println!("Version: {}\n", document.version());

    // Create a metadata component
    let metadata = MetadataComponent {
        title: "Technical Specification".to_string(),
        author: "Alice".to_string(),
        created_at: chrono::Utc::now(),
    };

    println!("Created metadata component:");
    metadata.display();
    println!("  Type: {}\n", metadata.type_name());

    // Display aggregate ID
    println!("Document ID: {}", document.aggregate_id());

    // Test command handling
    println!("\n=== Command Handling ===");

    // Create command
    let create_cmd = DocumentCommand::Create {
        title: "Technical Specification".to_string(),
        author: "Alice".to_string(),
    };
    if let DocumentCommand::Create { title, author } = &create_cmd {
        println!("Create command - Title: {}, Author: {}", title, author);
    }

    // Submit for review command
    let submit_cmd = DocumentCommand::SubmitForReview {
        document_id: document.aggregate_id(),
    };
    println!("Submit command aggregate: {:?}", submit_cmd.aggregate_id());

    // State transitions
    println!("\n=== State Transitions ===");

    // Submit for review
    match document.submit_for_review() {
        Ok(()) => {
            println!("✓ Submitted for review");
            println!("  New state: {:?}", document.current_state());
            println!("  Version: {}", document.version());
        }
        Err(e) => println!("✗ Failed to submit: {e}"),
    }

    // Try invalid transition
    println!("\nTrying to archive from UnderReview state...");
    match document
        .state_machine
        .transition_to(DocumentState::Archived)
    {
        Ok(_) => println!("✓ Unexpected success!"),
        Err(e) => println!("✗ Expected failure: {e}"),
    }

    // Approve
    println!("\nApproving document...");

    // Create approve command
    let approve_cmd = DocumentCommand::Approve {
        document_id: document.aggregate_id(),
    };
    println!("Approve command: {:?}", approve_cmd);

    match document.approve() {
        Ok(()) => {
            println!("✓ Approved");
            println!("  New state: {:?}", document.current_state());
            println!("  Version: {}", document.version());
        }
        Err(e) => println!("✗ Failed to approve: {e}"),
    }

    // Create command envelope
    println!("\n=== Command Example ===");

    let command = DocumentCommand::SubmitForReview {
        document_id: EntityId::new(),
    };
    let envelope = CommandEnvelope::new(command, "user123".to_string());

    println!("Created command envelope:");
    println!("  Command ID: {}", envelope.id);
    println!("  Issued by: {}", envelope.issued_by);
    println!("  Correlation ID: {}", envelope.correlation_id());

    println!("\n✅ Example completed successfully!");
}
