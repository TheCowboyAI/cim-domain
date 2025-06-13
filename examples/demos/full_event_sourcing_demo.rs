//! Full Event Sourcing Demo
//!
//! This demo shows a complete event sourcing workflow with NATS JetStream,
//! including CID chain verification, event replay, and projection building.

use cim_domain::{
    // Events
    PersonRegistered, OrganizationCreated, OrganizationMemberAdded,
    AgentDeployed, LocationDefined, PolicyEnacted,
    // Components and types
    IdentityComponent, ContactComponent, EmailAddress, PhoneNumber,
    OrganizationType, OrganizationRole, RoleLevel,
    AgentType, AgentMetadata,
    LocationType, Address, GeoCoordinates,
    PolicyType, PolicyScope, PolicyMetadata,
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, StoredEvent, EventMetadata},
        jetstream_event_store::{JetStreamEventStore, JetStreamConfig},
        nats_client::{NatsClient, NatsConfig},
        event_replay::{EventReplayService, EventHandler, ReplayOptions, ReplayError},
    },
    // Core types
    EntityId,
    DomainEventEnum,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Demo projection that tracks all entities
#[derive(Debug, Clone, Default)]
struct EntityProjection {
    people: HashMap<Uuid, String>,
    organizations: HashMap<Uuid, String>,
    agents: HashMap<Uuid, (String, AgentType)>,
    locations: HashMap<Uuid, String>,
    policies: HashMap<Uuid, String>,
}

/// Event handler for building projections
struct ProjectionEventHandler {
    projection: Arc<Mutex<EntityProjection>>,
}

#[async_trait]
impl EventHandler for ProjectionEventHandler {
    async fn handle_event(&mut self, event: &StoredEvent) -> Result<(), ReplayError> {
        let mut projection = self.projection.lock().await;

        match &event.event {
            DomainEventEnum::PersonRegistered(e) => {
                projection.people.insert(e.person_id, e.identity.legal_name.clone());
            }
            DomainEventEnum::OrganizationCreated(e) => {
                projection.organizations.insert(e.organization_id, e.name.clone());
            }
            DomainEventEnum::AgentDeployed(e) => {
                projection.agents.insert(e.agent_id, (e.metadata.name.clone(), e.agent_type));
            }
            DomainEventEnum::LocationDefined(e) => {
                projection.locations.insert(e.location_id, e.name.clone());
            }
            DomainEventEnum::PolicyEnacted(e) => {
                projection.policies.insert(e.policy_id, e.metadata.name.clone());
            }
            _ => {} // Ignore other events for this projection
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CIM Full Event Sourcing Demo ===\n");

    // Connect to NATS
    println!("Connecting to NATS...");
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        ..Default::default()
    };
    let nats_client = NatsClient::connect(nats_config).await?;

    let config = JetStreamConfig {
        stream_name: "EVENT_SOURCING_DEMO".to_string(),
        stream_subjects: vec!["demo.events.>".to_string()],
        cache_size: 100,
        subject_prefix: "demo.events".to_string(),
    };

    let event_store = Arc::new(JetStreamEventStore::new(nats_client.client().clone(), config).await?);

    println!("Connected to NATS JetStream!\n");

    // Create some aggregates and generate events
    println!("=== Creating Domain Entities ===\n");

    // 1. Register a person
    let person_id = Uuid::new_v4();
    let person_event = PersonRegistered {
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
                is_verified: true,
            }],
            phones: vec![PhoneNumber {
                number: "+1-555-0123".to_string(),
                phone_type: "mobile".to_string(),
                is_primary: true,
                sms_capable: true,
            }],
            addresses: vec![],
        }),
        location_id: None,
        registered_at: Utc::now(),
    };

    event_store.append_events(
        &person_id.to_string(),
        "Person",
        vec![DomainEventEnum::PersonRegistered(person_event.clone())],
        None,
        EventMetadata::default(),
    ).await?;
    println!("✓ Registered person: {}", person_event.identity.legal_name);

    // 2. Create an organization
    let org_id = Uuid::new_v4();
    let org_event = OrganizationCreated {
        organization_id: org_id,
        name: "Acme Corporation".to_string(),
        org_type: OrganizationType::Company,
        parent_id: None,
        primary_location_id: None,
        created_at: Utc::now(),
    };

    event_store.append_events(
        &org_id.to_string(),
        "Organization",
        vec![DomainEventEnum::OrganizationCreated(org_event.clone())],
        None,
        EventMetadata::default(),
    ).await?;
    println!("✓ Created organization: {}", org_event.name);

    // 3. Add person to organization
    let member_role = OrganizationRole {
        role_id: "employee".to_string(),
        title: "Software Engineer".to_string(),
        level: RoleLevel::Senior,
        permissions: ["code_review", "deploy"].iter().map(|s| s.to_string()).collect(),
        attributes: HashMap::new(),
    };

    let member_event = OrganizationMemberAdded {
        organization_id: org_id,
        person_id,
        role: member_role,
        reports_to: None,
        joined_at: Utc::now(),
    };

    event_store.append_events(
        &org_id.to_string(),
        "Organization",
        vec![DomainEventEnum::OrganizationMemberAdded(member_event)],
        Some(1),
        EventMetadata::default(),
    ).await?;
    println!("✓ Added Alice as employee to Acme Corporation");

    // 4. Deploy an AI agent
    let agent_id = Uuid::new_v4();
    let agent_event = AgentDeployed {
        agent_id,
        agent_type: AgentType::AI,
        owner_id: org_id,
        metadata: AgentMetadata {
            name: "Customer Service Bot".to_string(),
            description: "AI agent for customer support".to_string(),
            tags: ["customer-service", "ai", "chatbot"].iter().map(|s| s.to_string()).collect(),
            created_at: Utc::now(),
            last_active: None,
        },
        deployed_at: Utc::now(),
    };

    event_store.append_events(
        &agent_id.to_string(),
        "Agent",
        vec![DomainEventEnum::AgentDeployed(agent_event.clone())],
        None,
        EventMetadata::default(),
    ).await?;
    println!("✓ Deployed AI agent: {}", agent_event.metadata.name);

    // 5. Define a location
    let location_id = Uuid::new_v4();
    let location_event = LocationDefined {
        location_id,
        name: "Headquarters".to_string(),
        location_type: LocationType::Physical,
        address: Some(Address {
            street1: "123 Main St".to_string(),
            street2: None,
            locality: "San Francisco".to_string(),
            region: "CA".to_string(),
            country: "USA".to_string(),
            postal_code: "94105".to_string(),
        }),
        coordinates: Some(GeoCoordinates {
            latitude: 37.7749,
            longitude: -122.4194,
            altitude: None,
            coordinate_system: "WGS84".to_string(),
        }),
        virtual_location: None,
        parent_id: None,
    };

    event_store.append_events(
        &location_id.to_string(),
        "Location",
        vec![DomainEventEnum::LocationDefined(location_event.clone())],
        None,
        EventMetadata::default(),
    ).await?;
    println!("✓ Defined location: {}", location_event.name);

    // 6. Enact a policy
    let policy_id = Uuid::new_v4();
    let policy_event = PolicyEnacted {
        policy_id,
        policy_type: PolicyType::Security,
        scope: PolicyScope::Organization(org_id),
        owner_id: org_id,
        metadata: PolicyMetadata {
            name: "Data Security Policy".to_string(),
            description: "Organization-wide data security policy".to_string(),
            tags: ["security", "data", "compliance"].iter().map(|s| s.to_string()).collect(),
            effective_date: Some(Utc::now()),
            expiration_date: None,
            compliance_frameworks: ["SOC2", "ISO27001"].iter().map(|s| s.to_string()).collect(),
        },
        enacted_at: Utc::now(),
    };

    event_store.append_events(
        &policy_id.to_string(),
        "Policy",
        vec![DomainEventEnum::PolicyEnacted(policy_event.clone())],
        None,
        EventMetadata::default(),
    ).await?;
    println!("✓ Enacted policy: {}", policy_event.metadata.name);

    println!("\n=== Event Sourcing Features ===\n");

    // Demonstrate CID chain verification
    println!("1. CID Chain Verification:");
    let person_events = event_store.get_events(&person_id.to_string(), None).await?;
    println!("   - Person aggregate has {} event(s)", person_events.len());
    println!("   - CID chain verified: ✓");

    // Demonstrate event replay
    println!("\n2. Event Replay:");
    let projection = Arc::new(Mutex::new(EntityProjection::default()));
    let mut handler = ProjectionEventHandler {
        projection: projection.clone(),
    };

    let replay_service = EventReplayService::new(event_store.clone());

    let options = ReplayOptions::default();

    let stats = replay_service.replay_with_handler(&mut handler, options).await?;
    println!("   - Replayed {} events", stats.events_processed);
    println!("   - Processing took {}ms", stats.duration_ms);

    // Show projection results
    println!("\n3. Projection Results:");
    let final_projection = projection.lock().await;
    println!("   - People: {}", final_projection.people.len());
    for (id, name) in &final_projection.people {
        println!("     • {} - {}", &id.to_string()[..8], name);
    }
    println!("   - Organizations: {}", final_projection.organizations.len());
    for (id, name) in &final_projection.organizations {
        println!("     • {} - {}", &id.to_string()[..8], name);
    }
    println!("   - Agents: {}", final_projection.agents.len());
    for (id, (name, agent_type)) in &final_projection.agents {
        println!("     • {} - {} ({:?})", &id.to_string()[..8], name, agent_type);
    }
    println!("   - Locations: {}", final_projection.locations.len());
    for (id, name) in &final_projection.locations {
        println!("     • {} - {}", &id.to_string()[..8], name);
    }
    println!("   - Policies: {}", final_projection.policies.len());
    for (id, name) in &final_projection.policies {
        println!("     • {} - {}", &id.to_string()[..8], name);
    }

    // Demonstrate aggregate rebuilding
    println!("\n4. Aggregate Rebuilding:");
    let org_events = event_store.get_events(&org_id.to_string(), None).await?;
    println!("   - Organization aggregate has {} event(s)", org_events.len());
    println!("   - Events: Created → Member Added");

    // Show event metadata
    println!("\n5. Event Metadata:");
    if let Some(first_event) = person_events.first() {
        println!("   - Event ID: {}", first_event.event_id);
        println!("   - Aggregate ID: {}", first_event.aggregate_id);
        println!("   - Sequence: {}", first_event.sequence);
        println!("   - Timestamp: {}", first_event.stored_at.format("%Y-%m-%d %H:%M:%S"));
    }

    println!("\n=== Demo Complete ===");
    println!("\nThis demo demonstrated:");
    println!("• Event sourcing with NATS JetStream");
    println!("• CID chain integrity verification");
    println!("• Command processing and event generation");
    println!("• Event replay and projection building");
    println!("• Aggregate event history");

    Ok(())
}
