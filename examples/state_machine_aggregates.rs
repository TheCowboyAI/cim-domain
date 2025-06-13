//! Example: Using State Machines in Aggregates
//!
//! This example demonstrates how to use both Moore and Mealy state machines
//! in domain aggregates for managing state transitions.

use cim_domain::{
    AggregateRoot, Entity, EntityId,
    state_machine::*,
    DomainEvent, DomainError, DomainResult,
    Component, ComponentStorage,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use uuid::Uuid;

// Example 1: Document Aggregate with Moore Machine
// Output depends only on the state we're entering

#[derive(Debug, Clone)]
struct DocumentAggregate {
    entity: Entity<DocumentMarker>,
    state_machine: MooreMachine<DocumentState, Self>,
    title: String,
    content: String,
    author: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DocumentMarker;

// Define custom events for Document state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentSubmittedForReview {
    document_id: Uuid,
    title: String,
    author: Uuid,
}

impl DomainEvent for DocumentSubmittedForReview {
    fn aggregate_id(&self) -> Uuid {
        self.document_id
    }

    fn event_type(&self) -> &'static str {
        "DocumentSubmittedForReview"
    }

    fn subject(&self) -> String {
        "documents.document.submitted_for_review.v1".to_string()
    }
}

// Extend DocumentState with proper event generation
impl MooreStateTransitions for DocumentState {
    type Output = EventOutput;

    fn can_transition_to(&self, target: &Self) -> bool {
        use DocumentState::*;

        match (self, target) {
            (Draft, UnderReview) => true,
            (UnderReview, Draft) => true,
            (UnderReview, Approved) => true,
            (Approved, Published) => true,
            (Approved, UnderReview) => true,
            (Published, Archived) => true,
            _ => false,
        }
    }

    fn valid_transitions(&self) -> Vec<Self> {
        use DocumentState::*;

        match self {
            Draft => vec![UnderReview],
            UnderReview => vec![Draft, Approved],
            Approved => vec![Published, UnderReview],
            Published => vec![Archived],
            Archived => vec![],
        }
    }

    fn entry_output(&self) -> Self::Output {
        // Generate events based on the state we're entering
        let events: Vec<Box<dyn DomainEvent>> = match self {
            DocumentState::UnderReview => {
                vec![Box::new(DocumentSubmittedForReview {
                    document_id: Uuid::new_v4(), // In real code, get from aggregate
                    title: String::new(), // In real code, get from aggregate
                    author: Uuid::new_v4(), // In real code, get from aggregate
                })]
            }
            // Add more events for other states...
            _ => vec![],
        };

        EventOutput { events }
    }
}

impl AggregateRoot for DocumentAggregate {
    type Id = EntityId<Self>;

    fn id(&self) -> Self::Id {
        EntityId::from_uuid(self.entity.id().as_uuid())
    }

    fn version(&self) -> u64 {
        self.entity.version()
    }

    fn increment_version(&mut self) {
        self.entity.increment_version()
    }
}

impl DocumentAggregate {
    pub fn new(id: Uuid, title: String, author: Uuid) -> Self {
        let entity = Entity::<DocumentMarker>::new(EntityId::from_uuid(id));
        let aggregate_id = EntityId::<Self>::from_uuid(id);
        let state_machine = MooreMachine::new(DocumentState::Draft, aggregate_id);

        Self {
            entity,
            state_machine,
            title,
            content: String::new(),
            author,
        }
    }

    pub fn submit_for_review(&mut self) -> DomainResult<Vec<Box<dyn DomainEvent>>> {
        if self.content.is_empty() {
            return Err(DomainError::ValidationError("Cannot submit empty document".to_string()));
        }

        let transition = self.state_machine.transition_to(DocumentState::UnderReview)?;
        self.increment_version();

        Ok(transition.output.to_events())
    }

    pub fn approve(&mut self) -> DomainResult<Vec<Box<dyn DomainEvent>>> {
        let transition = self.state_machine.transition_to(DocumentState::Approved)?;
        self.increment_version();

        Ok(transition.output.to_events())
    }

    pub fn publish(&mut self) -> DomainResult<Vec<Box<dyn DomainEvent>>> {
        let transition = self.state_machine.transition_to(DocumentState::Published)?;
        self.increment_version();

        Ok(transition.output.to_events())
    }
}

// Example 2: Approval Aggregate with Mealy Machine
// Output depends on both state AND input (e.g., approval method)

#[derive(Debug, Clone)]
struct ApprovalAggregate {
    entity: Entity<ApprovalMarker>,
    state_machine: MealyMachine<ApprovalState, Self>,
    policy_id: Uuid,
    required_approvals: Vec<ApprovalRequirement>,
    received_approvals: Vec<ReceivedApproval>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ApprovalMarker;

#[derive(Debug, Clone)]
struct ApprovalRequirement {
    approver_type: ApproverType,
    method: ApprovalMethod,
}

#[derive(Debug, Clone)]
enum ApproverType {
    User(Uuid),
    Role(String),
    External(String),
}

#[derive(Debug, Clone)]
enum ApprovalMethod {
    Click,
    Yubikey,
    Biometric,
    TwoFactor,
}

#[derive(Debug, Clone)]
struct ReceivedApproval {
    approver: Uuid,
    method: ApprovalMethod,
    timestamp: chrono::DateTime<chrono::Utc>,
    verification_data: Option<String>,
}

// Approval states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ApprovalState {
    Pending,
    AwaitingExternalVerification,
    PartiallyApproved,
    Approved,
    Rejected,
    Expired,
}

impl State for ApprovalState {
    fn name(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::AwaitingExternalVerification => "AwaitingExternalVerification",
            Self::PartiallyApproved => "PartiallyApproved",
            Self::Approved => "Approved",
            Self::Rejected => "Rejected",
            Self::Expired => "Expired",
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self, Self::Approved | Self::Rejected | Self::Expired)
    }
}

// Approval inputs
#[derive(Debug, Clone)]
enum ApprovalInput {
    ProvideApproval {
        approver: Uuid,
        method: ApprovalMethod,
    },
    RequestExternalVerification {
        method: ApprovalMethod,
        challenge: String,
    },
    ReceiveExternalVerification {
        verification_data: String,
        success: bool,
    },
    Reject {
        rejector: Uuid,
        reason: String,
    },
    Expire,
}

impl TransitionInput for ApprovalInput {
    fn description(&self) -> String {
        match self {
            Self::ProvideApproval { method, .. } => {
                format!("Approval provided via {:?}", method)
            }
            Self::RequestExternalVerification { method, .. } => {
                format!("External verification requested via {:?}", method)
            }
            Self::ReceiveExternalVerification { success, .. } => {
                format!("External verification received: {}", success)
            }
            Self::Reject { reason, .. } => {
                format!("Rejected: {}", reason)
            }
            Self::Expire => "Expired".to_string(),
        }
    }
}

// Define approval events
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExternalVerificationRequested {
    approval_id: Uuid,
    method: String,
    challenge: String,
}

impl DomainEvent for ExternalVerificationRequested {
    fn aggregate_id(&self) -> Uuid {
        self.approval_id
    }

    fn event_type(&self) -> &'static str {
        "ExternalVerificationRequested"
    }

    fn subject(&self) -> String {
        "approvals.approval.external_verification_requested.v1".to_string()
    }
}

impl MealyStateTransitions for ApprovalState {
    type Input = ApprovalInput;
    type Output = EventOutput;

    fn can_transition_to(&self, target: &Self, input: &Self::Input) -> bool {
        use ApprovalState::*;
        use ApprovalInput::*;

        match (self, target, input) {
            // From Pending
            (Pending, AwaitingExternalVerification, RequestExternalVerification { .. }) => true,
            (Pending, PartiallyApproved, ProvideApproval { .. }) => true,
            (Pending, Approved, ProvideApproval { .. }) => true, // If all approvals met
            (Pending, Rejected, Reject { .. }) => true,
            (Pending, Expired, Expire) => true,

            // From AwaitingExternalVerification
            (AwaitingExternalVerification, PartiallyApproved, ReceiveExternalVerification { success: true, .. }) => true,
            (AwaitingExternalVerification, Approved, ReceiveExternalVerification { success: true, .. }) => true,
            (AwaitingExternalVerification, Rejected, ReceiveExternalVerification { success: false, .. }) => true,
            (AwaitingExternalVerification, Expired, Expire) => true,

            // From PartiallyApproved
            (PartiallyApproved, AwaitingExternalVerification, RequestExternalVerification { .. }) => true,
            (PartiallyApproved, Approved, ProvideApproval { .. }) => true,
            (PartiallyApproved, Rejected, Reject { .. }) => true,
            (PartiallyApproved, Expired, Expire) => true,

            _ => false,
        }
    }

    fn valid_transitions(&self, input: &Self::Input) -> Vec<Self> {
        use ApprovalState::*;
        use ApprovalInput::*;

        match (self, input) {
            (Pending, RequestExternalVerification { .. }) => vec![AwaitingExternalVerification],
            (Pending, ProvideApproval { .. }) => vec![PartiallyApproved, Approved],
            (Pending, Reject { .. }) => vec![Rejected],
            (Pending, Expire) => vec![Expired],

            (AwaitingExternalVerification, ReceiveExternalVerification { success: true, .. }) => {
                vec![PartiallyApproved, Approved]
            }
            (AwaitingExternalVerification, ReceiveExternalVerification { success: false, .. }) => {
                vec![Rejected]
            }
            (AwaitingExternalVerification, Expire) => vec![Expired],

            (PartiallyApproved, RequestExternalVerification { .. }) => vec![AwaitingExternalVerification],
            (PartiallyApproved, ProvideApproval { .. }) => vec![Approved],
            (PartiallyApproved, Reject { .. }) => vec![Rejected],
            (PartiallyApproved, Expire) => vec![Expired],

            _ => vec![],
        }
    }

    fn transition_output(&self, target: &Self, input: &Self::Input) -> Self::Output {
        use ApprovalInput::*;

        let events: Vec<Box<dyn DomainEvent>> = match input {
            RequestExternalVerification { method, challenge } => {
                vec![Box::new(ExternalVerificationRequested {
                    approval_id: Uuid::new_v4(), // In real code, get from aggregate
                    method: format!("{:?}", method),
                    challenge: challenge.clone(),
                })]
            }
            // Add more events for other inputs...
            _ => vec![],
        };

        EventOutput { events }
    }
}

impl AggregateRoot for ApprovalAggregate {
    type Id = EntityId<Self>;

    fn id(&self) -> Self::Id {
        EntityId::from_uuid(self.entity.id().as_uuid())
    }

    fn version(&self) -> u64 {
        self.entity.version()
    }

    fn increment_version(&mut self) {
        self.entity.increment_version()
    }
}

impl ApprovalAggregate {
    pub fn new(id: Uuid, policy_id: Uuid, requirements: Vec<ApprovalRequirement>) -> Self {
        let entity = Entity::<ApprovalMarker>::new(EntityId::from_uuid(id));
        let aggregate_id = EntityId::<Self>::from_uuid(id);
        let state_machine = MealyMachine::new(ApprovalState::Pending, aggregate_id);

        Self {
            entity,
            state_machine,
            policy_id,
            required_approvals: requirements,
            received_approvals: Vec::new(),
        }
    }

    pub fn request_yubikey_verification(&mut self, challenge: String) -> DomainResult<Vec<Box<dyn DomainEvent>>> {
        let input = ApprovalInput::RequestExternalVerification {
            method: ApprovalMethod::Yubikey,
            challenge,
        };

        let transition = self.state_machine.transition_to(
            ApprovalState::AwaitingExternalVerification,
            input
        )?;

        self.increment_version();
        Ok(transition.output.to_events())
    }

    pub fn receive_yubikey_response(&mut self, verification_data: String) -> DomainResult<Vec<Box<dyn DomainEvent>>> {
        // Validate yubikey response
        let is_valid = self.validate_yubikey(&verification_data);

        let input = ApprovalInput::ReceiveExternalVerification {
            verification_data,
            success: is_valid,
        };

        let target_state = if is_valid {
            if self.all_approvals_received() {
                ApprovalState::Approved
            } else {
                ApprovalState::PartiallyApproved
            }
        } else {
            ApprovalState::Rejected
        };

        let transition = self.state_machine.transition_to(target_state, input)?;

        if is_valid {
            self.received_approvals.push(ReceivedApproval {
                approver: Uuid::new_v4(), // In real code, get from yubikey
                method: ApprovalMethod::Yubikey,
                timestamp: chrono::Utc::now(),
                verification_data: Some(verification_data),
            });
        }

        self.increment_version();
        Ok(transition.output.to_events())
    }

    fn validate_yubikey(&self, data: &str) -> bool {
        // In real implementation, validate yubikey OTP
        !data.is_empty()
    }

    fn all_approvals_received(&self) -> bool {
        // Check if all required approvals have been received
        self.received_approvals.len() >= self.required_approvals.len()
    }
}

fn main() {
    println!("=== Moore Machine Example: Document Aggregate ===\n");

    // Create a document
    let mut document = DocumentAggregate::new(Uuid::new_v4(), "Technical Specification".to_string(), Uuid::new_v4());
    document.content = "This is a sample document content".to_string();

    // Submit the document for review (Moore machine - output depends only on entering UnderReview state)
    match document.submit_for_review() {
        Ok(events) => {
            println!("Document submitted for review successfully!");
            println!("Generated {} events", events.len());
            println!("Current state: {:?}", document.state_machine.current_state());
            println!("Valid next states: {:?}\n", document.state_machine.valid_next_states());
        }
        Err(e) => println!("Failed to submit document for review: {}", e),
    }

    println!("=== Mealy Machine Example: Approval Aggregate ===\n");

    // Create an approval requiring yubikey verification
    let requirements = vec![
        ApprovalRequirement {
            approver_type: ApproverType::User(Uuid::new_v4()),
            method: ApprovalMethod::Yubikey,
        },
    ];

    let mut approval = ApprovalAggregate::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        requirements
    );

    // Request yubikey verification (Mealy machine - output depends on state AND input)
    let challenge = "challenge-12345".to_string();
    match approval.request_yubikey_verification(challenge) {
        Ok(events) => {
            println!("Yubikey verification requested!");
            println!("Generated {} events", events.len());
            println!("Current state: {:?}", approval.state_machine.current_state());

            // Simulate receiving yubikey response
            println!("\nSimulating yubikey touch...");
            let yubikey_otp = "cccccccfhcbelgjhijblfbidjbijlbicubidhbfejnbl".to_string();

            match approval.receive_yubikey_response(yubikey_otp) {
                Ok(events) => {
                    println!("Yubikey verification successful!");
                    println!("Generated {} events", events.len());
                    println!("Final state: {:?}", approval.state_machine.current_state());
                }
                Err(e) => println!("Yubikey verification failed: {}", e),
            }
        }
        Err(e) => println!("Failed to request verification: {}", e),
    }

    println!("\n=== State Machine Benefits ===");
    println!("1. Type-safe state transitions");
    println!("2. Clear separation of concerns:");
    println!("   - Moore: Simple state-based outputs (e.g., document lifecycle)");
    println!("   - Mealy: Input-dependent outputs (e.g., approval workflows)");
    println!("3. Audit trail through transition history");
    println!("4. Domain events generated automatically");
}

