//! Example of using the Bevy bridge with subject-based routing
//!
//! This example demonstrates how NATS domain events can be translated
//! to Bevy ECS commands using our subject-based routing pattern.

use cim_domain::{
    // Events
    PersonRegistered, OrganizationCreated, AgentDeployed,
    EventMetadata, DomainEventEnvelope, PropagationScope,

    // Bevy bridge
    NatsToBevyTranslator, BevyEventRouter, NatsMessage,
    BevyCommand, BevyEvent, MessageTranslator,
};
use uuid::Uuid;
use std::collections::HashMap;

fn main() {
    println!("=== CIM Domain Bevy Integration Example ===\n");

    // Create translator and router
    let translator = NatsToBevyTranslator::new();
    let router = BevyEventRouter::new();

    // Example 1: Person registered event from NATS
    println!("1. Person Registration Event:");
    let person_id = Uuid::new_v4();
    let location_id = Uuid::new_v4();
    let person_event = PersonRegistered {
        person_id,
        identity: cim_domain::IdentityComponent {
            legal_name: "Alice Johnson".to_string(),
            preferred_name: Some("Alice".to_string()),
            date_of_birth: None,
            government_id: None,
        },
        contact: Some(cim_domain::ContactComponent {
            emails: vec![cim_domain::EmailAddress {
                email: "alice@example.com".to_string(),
                email_type: "work".to_string(),
                is_primary: true,
                is_verified: true,
            }],
            phones: vec![],
            addresses: vec![],
        }),
        location_id: Some(location_id),
    };

    // Create event envelope with metadata
    let metadata = EventMetadata::new("example-system".to_string())
        .with_propagation(PropagationScope::LocalOnly);

    let envelope = DomainEventEnvelope {
        metadata,
        event: serde_json::to_value(&person_event).unwrap(),
        subject: "people.person.registered.v1".to_string(),
    };

    // Simulate NATS message
    let nats_msg = NatsMessage {
        subject: envelope.subject.clone(),
        payload: serde_json::to_vec(&envelope).unwrap(),
        headers: HashMap::new(),
    };

    // Translate to Bevy command
    match translator.translate(nats_msg) {
        Ok(BevyCommand::SpawnEntity { entity_id, components, parent }) => {
            println!("  ✓ Translated to SpawnEntity command");
            println!("    Entity ID: {}", entity_id);
            println!("    Components:");
            for comp in &components {
                println!("      - {}: {:?}", comp.component_type, comp.data);
            }
            println!("    Parent: {:?}", parent);
        }
        _ => println!("  ✗ Unexpected command type"),
    }

    // Example 2: Organization created event
    println!("\n2. Organization Creation Event:");
    let org_id = Uuid::new_v4();
    let org_event = OrganizationCreated {
        organization_id: org_id,
        name: "Acme Corp".to_string(),
        org_type: "Corporation".to_string(),
    };

    let envelope = DomainEventEnvelope {
        metadata: EventMetadata::new("example-system".to_string()),
        event: serde_json::to_value(&org_event).unwrap(),
        subject: "organizations.organization.created.v1".to_string(),
    };

    let nats_msg = NatsMessage {
        subject: envelope.subject.clone(),
        payload: serde_json::to_vec(&envelope).unwrap(),
        headers: HashMap::new(),
    };

    match translator.translate(nats_msg) {
        Ok(BevyCommand::SpawnEntity { entity_id, components, .. }) => {
            println!("  ✓ Translated to SpawnEntity command");
            println!("    Entity ID: {}", entity_id);
            println!("    Organization type: Corporation");
            println!("    Transform scale: 1.5x (organizations are larger)");
        }
        _ => println!("  ✗ Unexpected command type"),
    }

    // Example 3: Agent deployed event (with parent relationship)
    println!("\n3. Agent Deployment Event:");
    let agent_id = Uuid::new_v4();
    let agent_event = AgentDeployed {
        agent_id,
        agent_type: "Assistant".to_string(),
        owner_id: person_id, // Owned by the person we created
        capabilities: vec!["chat".to_string(), "search".to_string()],
    };

    let envelope = DomainEventEnvelope {
        metadata: EventMetadata::new("example-system".to_string()),
        event: serde_json::to_value(&agent_event).unwrap(),
        subject: "agents.agent.deployed.v1".to_string(),
    };

    let nats_msg = NatsMessage {
        subject: envelope.subject.clone(),
        payload: serde_json::to_vec(&envelope).unwrap(),
        headers: HashMap::new(),
    };

    match translator.translate(nats_msg) {
        Ok(BevyCommand::SpawnEntity { entity_id, parent, .. }) => {
            println!("  ✓ Translated to SpawnEntity command");
            println!("    Entity ID: {}", entity_id);
            println!("    Parent (owner): {:?}", parent);
            println!("    Capabilities: chat, search");
        }
        _ => println!("  ✗ Unexpected command type"),
    }

    // Example 4: Bevy events back to NATS
    println!("\n4. Bevy UI Events to NATS:");

    let select_event = BevyEvent::EntitySelected {
        entity_id: person_id,
        position: [100.0, 200.0, 0.0],
    };

    let subject = router.route_event(&select_event);
    println!("  Entity selected → Subject: {}", subject);

    let move_event = BevyEvent::EntityMoved {
        entity_id: person_id,
        old_position: [100.0, 200.0, 0.0],
        new_position: [150.0, 250.0, 0.0],
    };

    let subject = router.route_event(&move_event);
    println!("  Entity moved → Subject: {}", subject);

    let create_event = BevyEvent::EntityCreationRequested {
        entity_type: "Agent".to_string(),
        position: [300.0, 400.0, 0.0],
        metadata: serde_json::json!({
            "requested_by": person_id,
            "agent_type": "Helper"
        }),
    };

    let subject = router.route_event(&create_event);
    println!("  Creation requested → Subject: {}", subject);

    // Example 5: Subject pattern matching
    println!("\n5. Subject Pattern Benefits:");
    println!("  - Hierarchical routing: 'people.person.registered.v1'");
    println!("  - Context isolation: 'people.*' vs 'organizations.*'");
    println!("  - Version support: '*.*.*.v1' vs '*.*.*.v2'");
    println!("  - UI events: 'ui.entity.selected.v1'");
    println!("  - Wildcard subscriptions: 'people.>' for all people events");

    println!("\n=== Pattern Summary ===");
    println!("Domain events → NATS subjects → Bevy commands");
    println!("Bevy events → NATS subjects → Domain handlers");
    println!("Subject structure provides natural routing and filtering");
}
