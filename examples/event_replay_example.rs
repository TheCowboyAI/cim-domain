//! Example demonstrating event replay functionality
//!
//! This example shows how to:
//! - Replay events to rebuild aggregates
//! - Build projections from event streams
//! - Handle replay errors and track progress

use cim_domain::{
    infrastructure::{
        EventHandler, ProjectionHandler, ReplayOptions,
        ReplayError, ReplayStats, StoredEvent,
    },
    DomainEventEnum, DomainEvent,
    PersonRegistered, OrganizationCreated, OrganizationMemberAdded,
    IdentityComponent, OrganizationType, OrganizationRole, RoleLevel,
};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Example projection that tracks organization membership
struct OrganizationMembershipProjection {
    /// Map of organization ID to member IDs
    pub organizations: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,

    /// Map of person ID to organization IDs
    pub people: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,

    /// Total events processed
    pub events_processed: u64,
}

impl OrganizationMembershipProjection {
    fn new() -> Self {
        Self {
            organizations: Arc::new(RwLock::new(HashMap::new())),
            people: Arc::new(RwLock::new(HashMap::new())),
            events_processed: 0,
        }
    }
}

#[async_trait]
impl ProjectionHandler for OrganizationMembershipProjection {
    async fn handle_event(&mut self, event: &DomainEventEnum, _sequence: u64) -> Result<(), ReplayError> {
        match event {
            DomainEventEnum::OrganizationCreated(e) => {
                let mut orgs = self.organizations.write().await;
                orgs.insert(e.organization_id, Vec::new());
                println!("Created organization: {} - {}", e.organization_id, e.name);
            }
            DomainEventEnum::OrganizationMemberAdded(e) => {
                // Update organization members
                let mut orgs = self.organizations.write().await;
                let members = orgs.entry(e.organization_id).or_insert_with(Vec::new);
                members.push(e.person_id);
                drop(orgs);

                // Update person organizations
                let mut people = self.people.write().await;
                let person_orgs = people.entry(e.person_id).or_insert_with(Vec::new);
                person_orgs.push(e.organization_id);

                println!(
                    "  Added {} to organization {} as {}",
                    e.person_id, e.organization_id, e.role.title
                );
            }
            _ => {} // Ignore other events
        }

        self.events_processed += 1;
        Ok(())
    }

    fn name(&self) -> &str {
        "OrganizationMembership"
    }

    async fn reset(&mut self) -> Result<(), ReplayError> {
        self.organizations.write().await.clear();
        self.people.write().await.clear();
        self.events_processed = 0;
        println!("Reset OrganizationMembership projection");
        Ok(())
    }
}

/// Example event handler that logs all events
struct LoggingEventHandler {
    pub events_logged: u64,
}

#[async_trait]
impl EventHandler for LoggingEventHandler {
    async fn handle_event(&mut self, event: &StoredEvent) -> Result<(), ReplayError> {
        println!(
            "[{}] Event #{}: {} for aggregate {}",
            event.stored_at,
            event.sequence,
            event.event.event_type(),
            event.aggregate_id,
        );

        self.events_logged += 1;
        Ok(())
    }

    async fn on_replay_start(&mut self) -> Result<(), ReplayError> {
        println!("=== Starting event replay ===");
        Ok(())
    }

    async fn on_replay_complete(&mut self, stats: &ReplayStats) -> Result<(), ReplayError> {
        println!("\n=== Replay completed ===");
        println!("Events processed: {}", stats.events_processed);
        println!("Errors: {}", stats.errors);
        println!("Duration: {}ms", stats.duration_ms);
        println!("Events/second: {:.2}", stats.events_per_second);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Event Replay Example\n");

    // In a real application, you would use the actual event store
    // For this example, we'll simulate it
    println!("Note: This example uses simulated events.");
    println!("In production, you would connect to NATS JetStream.\n");

    // Example 1: Replay with logging handler
    println!("Example 1: Logging all events");
    println!("{}", "-".repeat(50));

    let mut logging_handler = LoggingEventHandler { events_logged: 0 };

    // Simulate some events
    let events = vec![
        StoredEvent {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: Uuid::new_v4().to_string(),
            aggregate_type: "Person".to_string(),
            sequence: 1,
            event: DomainEventEnum::PersonRegistered(PersonRegistered {
                person_id: Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
                identity: IdentityComponent {
                    legal_name: "Alice Smith".to_string(),
                    preferred_name: None,
                    date_of_birth: None,
                    government_id: None,
                },
                contact: None,
                location_id: None,
                registered_at: chrono::Utc::now(),
            }),
            metadata: cim_domain::infrastructure::EventMetadata::default(),
            stored_at: chrono::Utc::now(),
        },
        StoredEvent {
            event_id: Uuid::new_v4().to_string(),
            aggregate_id: Uuid::new_v4().to_string(),
            aggregate_type: "Organization".to_string(),
            sequence: 2,
            event: DomainEventEnum::OrganizationCreated(OrganizationCreated {
                organization_id: Uuid::parse_str("456e7890-e89b-12d3-a456-426614174000").unwrap(),
                name: "Acme Corp".to_string(),
                org_type: OrganizationType::Company,
                parent_id: None,
                primary_location_id: None,
                created_at: chrono::Utc::now(),
            }),
            metadata: cim_domain::infrastructure::EventMetadata::default(),
            stored_at: chrono::Utc::now(),
        },
    ];

    // Simulate replay
    for event in &events {
        logging_handler.handle_event(event).await?;
    }

    let stats = ReplayStats {
        events_processed: events.len() as u64,
        aggregates_rebuilt: 0,
        errors: 0,
        duration_ms: 100,
        events_per_second: (events.len() as f64) * 10.0,
    };

    logging_handler.on_replay_complete(&stats).await?;

    // Example 2: Build projection
    println!("\n\nExample 2: Building organization membership projection");
    println!("{}", "-".repeat(50));

    let mut projection = OrganizationMembershipProjection::new();

    // Create some test events
    let org_id = Uuid::new_v4();
    let person1_id = Uuid::new_v4();
    let person2_id = Uuid::new_v4();

    let projection_events = vec![
        DomainEventEnum::OrganizationCreated(OrganizationCreated {
            organization_id: org_id,
            name: "Tech Startup".to_string(),
            org_type: OrganizationType::Company,
            parent_id: None,
            primary_location_id: None,
            created_at: chrono::Utc::now(),
        }),
        DomainEventEnum::PersonRegistered(PersonRegistered {
            person_id: person1_id,
            identity: IdentityComponent {
                legal_name: "Bob Johnson".to_string(),
                preferred_name: None,
                date_of_birth: None,
                government_id: None,
            },
            contact: None,
            location_id: None,
            registered_at: chrono::Utc::now(),
        }),
        DomainEventEnum::PersonRegistered(PersonRegistered {
            person_id: person2_id,
            identity: IdentityComponent {
                legal_name: "Carol Davis".to_string(),
                preferred_name: None,
                date_of_birth: None,
                government_id: None,
            },
            contact: None,
            location_id: None,
            registered_at: chrono::Utc::now(),
        }),
        DomainEventEnum::OrganizationMemberAdded(OrganizationMemberAdded {
            organization_id: org_id,
            person_id: person1_id,
            role: OrganizationRole {
                role_id: "member".to_string(),
                title: "Member".to_string(),
                level: RoleLevel::Mid,
                permissions: HashSet::new(),
                attributes: HashMap::new(),
            },
            reports_to: None,
            joined_at: chrono::Utc::now(),
        }),
        DomainEventEnum::OrganizationMemberAdded(OrganizationMemberAdded {
            organization_id: org_id,
            person_id: person2_id,
            role: OrganizationRole {
                role_id: "admin".to_string(),
                title: "Admin".to_string(),
                level: RoleLevel::Senior,
                permissions: HashSet::new(),
                attributes: HashMap::new(),
            },
            reports_to: Some(person1_id),
            joined_at: chrono::Utc::now(),
        }),
    ];

    // Process events
    for (seq, event) in projection_events.iter().enumerate() {
        projection.handle_event(event, seq as u64).await?;
    }

    // Display projection state
    println!("\nProjection state after replay:");
    let orgs = projection.organizations.read().await;
    for (org_id, members) in orgs.iter() {
        println!("Organization {}: {} members", org_id, members.len());
        for member_id in members {
            println!("  - Member: {}", member_id);
        }
    }

    let people = projection.people.read().await;
    println!("\nPeople memberships:");
    for (person_id, org_ids) in people.iter() {
        println!("Person {}: member of {} organizations", person_id, org_ids.len());
    }

    println!("\nTotal events processed by projection: {}", projection.events_processed);

    // Example 3: Replay with filters
    println!("\n\nExample 3: Replay with filters");
    println!("{}", "-".repeat(50));

    let options = ReplayOptions {
        max_events: Some(10),
        batch_size: 5,
        continue_on_error: true,
        aggregate_types: Some(vec!["Organization".to_string()]),
        event_types: Some(vec!["OrganizationCreated".to_string(), "OrganizationMemberAdded".to_string()]),
        from_sequence: None,
    };

    println!("Replay options:");
    println!("  Max events: {:?}", options.max_events);
    println!("  Batch size: {}", options.batch_size);
    println!("  Continue on error: {}", options.continue_on_error);
    println!("  Aggregate types: {:?}", options.aggregate_types);
    println!("  Event types: {:?}", options.event_types);

    println!("\nIn production, these filters would be applied during event stream processing.");

    Ok(())
}
