//! Example of command handlers in CIM domain
//!
//! This example demonstrates how to implement command handlers that:
//! - Process commands
//! - Validate business rules
//! - Emit domain events
//! - Return acknowledgments

use cim_domain::{
    // Commands
    RegisterPerson, CreateOrganization,

    // Command handling
    CommandEnvelope, CommandHandler, CommandStatus,
    EventPublisher, InMemoryRepository,
    PersonCommandHandler, OrganizationCommandHandler,

    // Domain types
    Person, Organization,
    IdentityComponent, ContactComponent, EmailAddress,
    OrganizationType,
    DomainEventEnum,
};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Example event publisher that prints events
#[derive(Clone)]
struct PrintingEventPublisher {
    events: Arc<RwLock<Vec<DomainEventEnum>>>,
}

impl PrintingEventPublisher {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn get_events(&self) -> Vec<DomainEventEnum> {
        self.events.read().unwrap().clone()
    }
}

impl EventPublisher for PrintingEventPublisher {
    fn publish_events(
        &self,
        events: Vec<DomainEventEnum>,
        correlation_id: cim_domain::CorrelationId,
    ) -> Result<(), String> {
        println!("Publishing {} events with correlation {:?}", events.len(), correlation_id);

        for event in &events {
            println!("  Event: {:?}", event);
            self.events.write().unwrap().push(event.clone());
        }

        Ok(())
    }
}

fn main() {
    println!("=== Command Handler Example ===\n");

    // Setup infrastructure
    let person_repo = InMemoryRepository::<Person>::new();
    let org_repo = InMemoryRepository::<Organization>::new();
    let event_publisher = PrintingEventPublisher::new();
    let event_publisher2 = PrintingEventPublisher::new();

    // Create command handlers
    let mut person_handler = PersonCommandHandler::new(
        person_repo,
        Box::new(event_publisher.clone()),
    );

    let mut org_handler = OrganizationCommandHandler::new(
        org_repo,
        Box::new(event_publisher2.clone()),
    );

    // Example 1: Register a person
    println!("1. Registering a person...");
    let person_id = Uuid::new_v4();
    let register_cmd = RegisterPerson {
        person_id,
        identity: IdentityComponent {
            legal_name: "Alice Johnson".to_string(),
            preferred_name: Some("Alice".to_string()),
            date_of_birth: None,
            government_id: None,
        },
        contact: Some(ContactComponent {
            emails: vec![EmailAddress {
                email: "alice@example.com".to_string(),
                email_type: "work".to_string(),
                is_primary: true,
                is_verified: false,
            }],
            phones: vec![],
            addresses: vec![],
        }),
        location_id: None,
    };

    let envelope = CommandEnvelope::new(register_cmd, "admin".to_string());
    let ack = person_handler.handle(envelope);

    match ack.status {
        CommandStatus::Accepted => println!("✓ Person registered successfully"),
        CommandStatus::Rejected => println!("✗ Registration failed: {:?}", ack.reason),
    }

    // Example 2: Create an organization
    println!("\n2. Creating an organization...");
    let org_id = Uuid::new_v4();
    let create_org_cmd = CreateOrganization {
        organization_id: org_id,
        name: "Tech Corp".to_string(),
        org_type: OrganizationType::Company,
        parent_id: None,
        primary_location_id: None,
    };

    let envelope = CommandEnvelope::new(create_org_cmd, "admin".to_string());
    let ack = org_handler.handle(envelope);

    match ack.status {
        CommandStatus::Accepted => println!("✓ Organization created successfully"),
        CommandStatus::Rejected => println!("✗ Creation failed: {:?}", ack.reason),
    }

    // Example 3: Try to register duplicate person (should fail)
    println!("\n3. Attempting duplicate registration...");
    let duplicate_cmd = RegisterPerson {
        person_id, // Same ID as before
        identity: IdentityComponent {
            legal_name: "Alice Duplicate".to_string(),
            preferred_name: None,
            date_of_birth: None,
            government_id: None,
        },
        contact: None,
        location_id: None,
    };

    let envelope = CommandEnvelope::new(duplicate_cmd, "admin".to_string());
    let ack = person_handler.handle(envelope);

    match ack.status {
        CommandStatus::Accepted => println!("✗ Unexpected: Duplicate was accepted!"),
        CommandStatus::Rejected => println!("✓ Duplicate rejected as expected: {:?}", ack.reason),
    }

    // Example 4: Define a location
    println!("\n4. Defining a location...");
    let location_repo = InMemoryRepository::<cim_domain::Location>::new();
    let event_publisher3 = PrintingEventPublisher::new();
    let mut location_handler = cim_domain::LocationCommandHandler::new(
        location_repo,
        Box::new(event_publisher3.clone()),
    );

    let location_cmd = cim_domain::DefineLocation {
        location_id: uuid::Uuid::new_v4(),
        name: "Tech Hub".to_string(),
        location_type: cim_domain::LocationType::Physical,
        address: Some(cim_domain::Address::new(
            "456 Innovation Way".to_string(),
            "San Francisco".to_string(),
            "CA".to_string(),
            "USA".to_string(),
            "94105".to_string(),
        )),
        coordinates: None,
        virtual_location: None,
        parent_id: None,
        metadata: std::collections::HashMap::new(),
    };

    let envelope = CommandEnvelope::new(location_cmd, "admin".to_string());
    let ack = location_handler.handle(envelope);

    match ack.status {
        CommandStatus::Accepted => println!("✓ Location defined successfully"),
        CommandStatus::Rejected => println!("✗ Location definition failed: {:?}", ack.reason),
    }

    // Example 5: Enact a policy
    println!("\n5. Enacting a policy...");
    let policy_repo = InMemoryRepository::<cim_domain::Policy>::new();
    let event_publisher4 = PrintingEventPublisher::new();
    let mut policy_handler = cim_domain::PolicyCommandHandler::new(
        policy_repo,
        Box::new(event_publisher4.clone()),
    );

    let policy_cmd = cim_domain::EnactPolicy {
        policy_id: uuid::Uuid::new_v4(),
        policy_type: cim_domain::PolicyType::AccessControl,
        scope: cim_domain::PolicyScope::Global,
        owner_id: uuid::Uuid::new_v4(),
        metadata: cim_domain::PolicyMetadata {
            name: "Data Access Policy".to_string(),
            description: "Controls access to sensitive data".to_string(),
            tags: std::collections::HashSet::new(),
            effective_date: None,
            expiration_date: None,
            compliance_frameworks: std::collections::HashSet::new(),
        },
    };

    let envelope = CommandEnvelope::new(policy_cmd, "admin".to_string());
    let ack = policy_handler.handle(envelope);

    match ack.status {
        CommandStatus::Accepted => println!("✓ Policy enacted successfully"),
        CommandStatus::Rejected => println!("✗ Policy enactment failed: {:?}", ack.reason),
    }

    // Show all published events
    println!("\n=== Published Events ===");
    let person_events = event_publisher.get_events();
    let org_events = event_publisher2.get_events();
    let location_events = event_publisher3.get_events();
    let policy_events = event_publisher4.get_events();

    println!("Person events: {}", person_events.len());
    println!("Organization events: {}", org_events.len());
    println!("Location events: {}", location_events.len());
    println!("Policy events: {}", policy_events.len());

    println!("\n=== Example Complete ===");
}
