//! CQRS Pattern Demo
//!
//! This demo shows Command Query Responsibility Segregation patterns,
//! demonstrating write and read model separation.

use cim_domain::{
    // Commands
    RegisterPerson, CreateOrganization, AddOrganizationMember, DeployAgent,
    // Events
    PersonRegistered, OrganizationCreated, OrganizationMemberAdded, AgentDeployed,
    // Aggregates and components
    Person, IdentityComponent, ContactComponent, EmailAddress, PhoneNumber,
    Organization, OrganizationType, OrganizationRole, RoleLevel,
    Agent, AgentType, AgentMetadata,
    // Infrastructure
    infrastructure::{
        event_store::{EventStore, StoredEvent, EventMetadata},
        jetstream_event_store::{JetStreamEventStore, JetStreamConfig},
        nats_client::{NatsClient, NatsConfig},
    },
    // Core types
    EntityId,
    DomainEventEnum,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Write model - handles commands and generates events
struct WriteModel {
    event_store: Arc<JetStreamEventStore>,
}

impl WriteModel {
    async fn handle_register_person(
        &self,
        cmd: RegisterPerson,
    ) -> Result<Vec<DomainEventEnum>, Box<dyn std::error::Error>> {
        // Validate command
        if cmd.identity.legal_name.is_empty() {
            return Err("Legal name cannot be empty".into());
        }

        // Generate event
        let event = PersonRegistered {
            person_id: cmd.person_id,
            identity: cmd.identity,
            contact: cmd.contact,
            location_id: cmd.location_id,
            registered_at: Utc::now(),
        };

        // Store event
        self.event_store
            .append_events(
                &cmd.person_id.to_string(),
                "Person",
                vec![DomainEventEnum::PersonRegistered(event.clone())],
                None,
                EventMetadata::default(),
            )
            .await?;

        Ok(vec![DomainEventEnum::PersonRegistered(event)])
    }

    async fn handle_create_organization(
        &self,
        cmd: CreateOrganization,
    ) -> Result<Vec<DomainEventEnum>, Box<dyn std::error::Error>> {
        // Validate command
        if cmd.name.is_empty() {
            return Err("Organization name cannot be empty".into());
        }

        // Generate event
        let event = OrganizationCreated {
            organization_id: cmd.organization_id,
            name: cmd.name,
            org_type: cmd.org_type,
            parent_id: cmd.parent_id,
            primary_location_id: cmd.primary_location_id,
            created_at: Utc::now(),
        };

        // Store event
        self.event_store
            .append_events(
                &cmd.organization_id.to_string(),
                "Organization",
                vec![DomainEventEnum::OrganizationCreated(event.clone())],
                None,
                EventMetadata::default(),
            )
            .await?;

        Ok(vec![DomainEventEnum::OrganizationCreated(event)])
    }

    async fn handle_add_organization_member(
        &self,
        cmd: AddOrganizationMember,
    ) -> Result<Vec<DomainEventEnum>, Box<dyn std::error::Error>> {
        // In a real system, we'd load the aggregate and validate
        // For demo, we'll just generate the event

        let event = OrganizationMemberAdded {
            organization_id: cmd.organization_id,
            person_id: cmd.person_id,
            role: cmd.role,
            reports_to: cmd.reports_to,
            joined_at: Utc::now(),
        };

        // Store event with expected version (would come from aggregate)
        self.event_store
            .append_events(
                &cmd.organization_id.to_string(),
                "Organization",
                vec![DomainEventEnum::OrganizationMemberAdded(event.clone())],
                Some(1), // Assuming org exists
                EventMetadata::default(),
            )
            .await?;

        Ok(vec![DomainEventEnum::OrganizationMemberAdded(event)])
    }

    async fn handle_deploy_agent(
        &self,
        cmd: DeployAgent,
    ) -> Result<Vec<DomainEventEnum>, Box<dyn std::error::Error>> {
        // Validate command
        if cmd.metadata.name.is_empty() {
            return Err("Agent name cannot be empty".into());
        }

        // Generate event
        let event = AgentDeployed {
            agent_id: cmd.agent_id,
            agent_type: cmd.agent_type,
            owner_id: cmd.owner_id,
            metadata: cmd.metadata,
            deployed_at: Utc::now(),
        };

        // Store event
        self.event_store
            .append_events(
                &cmd.agent_id.to_string(),
                "Agent",
                vec![DomainEventEnum::AgentDeployed(event.clone())],
                None,
                EventMetadata::default(),
            )
            .await?;

        Ok(vec![DomainEventEnum::AgentDeployed(event)])
    }
}

/// Read model - optimized for queries
#[derive(Debug, Clone)]
struct ReadModel {
    // Person views
    people_by_id: HashMap<Uuid, PersonView>,
    people_by_name: HashMap<String, Vec<Uuid>>,
    people_by_email: HashMap<String, Uuid>,

    // Organization views
    organizations_by_id: HashMap<Uuid, OrganizationView>,
    organization_members: HashMap<Uuid, Vec<MemberView>>,
    organization_hierarchy: HashMap<Uuid, Vec<Uuid>>, // parent -> children

    // Agent views
    agents_by_id: HashMap<Uuid, AgentView>,
    agents_by_owner: HashMap<Uuid, Vec<Uuid>>,
    agents_by_type: HashMap<AgentType, Vec<Uuid>>,

    // Cross-aggregate views
    person_organizations: HashMap<Uuid, Vec<Uuid>>, // person -> orgs
}

// View models
#[derive(Debug, Clone)]
struct PersonView {
    id: Uuid,
    name: String,
    email: Option<String>,
    phone: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct OrganizationView {
    id: Uuid,
    name: String,
    org_type: OrganizationType,
    member_count: usize,
    parent_id: Option<Uuid>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct MemberView {
    person_id: Uuid,
    person_name: String,
    role: OrganizationRole,
    joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct AgentView {
    id: Uuid,
    name: String,
    agent_type: AgentType,
    owner_id: Uuid,
    deployed_at: chrono::DateTime<chrono::Utc>,
}

impl ReadModel {
    fn new() -> Self {
        Self {
            people_by_id: HashMap::new(),
            people_by_name: HashMap::new(),
            people_by_email: HashMap::new(),
            organizations_by_id: HashMap::new(),
            organization_members: HashMap::new(),
            organization_hierarchy: HashMap::new(),
            agents_by_id: HashMap::new(),
            agents_by_owner: HashMap::new(),
            agents_by_type: HashMap::new(),
            person_organizations: HashMap::new(),
        }
    }

    /// Update read model from event
    fn apply_event(&mut self, event: &DomainEventEnum) {
        match event {
            DomainEventEnum::PersonRegistered(e) => {
                let view = PersonView {
                    id: e.person_id,
                    name: e.identity.legal_name.clone(),
                    email: e.contact.as_ref()
                        .and_then(|c| c.emails.first())
                        .map(|e| e.email.clone()),
                    phone: e.contact.as_ref()
                        .and_then(|c| c.phones.first())
                        .map(|p| p.number.clone()),
                    created_at: e.registered_at,
                };

                // Update indices
                self.people_by_id.insert(e.person_id, view.clone());
                self.people_by_name
                    .entry(e.identity.legal_name.clone())
                    .or_insert_with(Vec::new)
                    .push(e.person_id);

                if let Some(email) = &view.email {
                    self.people_by_email.insert(email.clone(), e.person_id);
                }
            }

            DomainEventEnum::OrganizationCreated(e) => {
                let view = OrganizationView {
                    id: e.organization_id,
                    name: e.name.clone(),
                    org_type: e.org_type.clone(),
                    member_count: 0,
                    parent_id: e.parent_id,
                    created_at: e.created_at,
                };

                self.organizations_by_id.insert(e.organization_id, view);

                if let Some(parent_id) = e.parent_id {
                    self.organization_hierarchy
                        .entry(parent_id)
                        .or_insert_with(Vec::new)
                        .push(e.organization_id);
                }
            }

            DomainEventEnum::OrganizationMemberAdded(e) => {
                // Update member count
                if let Some(org) = self.organizations_by_id.get_mut(&e.organization_id) {
                    org.member_count += 1;
                }

                // Add member view
                if let Some(person) = self.people_by_id.get(&e.person_id) {
                    let member = MemberView {
                        person_id: e.person_id,
                        person_name: person.name.clone(),
                        role: e.role.clone(),
                        joined_at: e.joined_at,
                    };

                    self.organization_members
                        .entry(e.organization_id)
                        .or_insert_with(Vec::new)
                        .push(member);

                    // Update person's organizations
                    self.person_organizations
                        .entry(e.person_id)
                        .or_insert_with(Vec::new)
                        .push(e.organization_id);
                }
            }

            DomainEventEnum::AgentDeployed(e) => {
                let view = AgentView {
                    id: e.agent_id,
                    name: e.metadata.name.clone(),
                    agent_type: e.agent_type,
                    owner_id: e.owner_id,
                    deployed_at: e.deployed_at,
                };

                self.agents_by_id.insert(e.agent_id, view);
                self.agents_by_owner
                    .entry(e.owner_id)
                    .or_insert_with(Vec::new)
                    .push(e.agent_id);
                self.agents_by_type
                    .entry(e.agent_type)
                    .or_insert_with(Vec::new)
                    .push(e.agent_id);
            }

            _ => {} // Handle other events as needed
        }
    }
}

/// Query handlers - read from optimized views
struct QueryHandlers {
    read_model: Arc<RwLock<ReadModel>>,
}

impl QueryHandlers {
    async fn find_person_by_id(&self, id: Uuid) -> Option<PersonView> {
        let model = self.read_model.read().await;
        model.people_by_id.get(&id).cloned()
    }

    async fn find_people_by_name(&self, name: &str) -> Vec<PersonView> {
        let model = self.read_model.read().await;
        model.people_by_name
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| model.people_by_id.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn find_person_by_email(&self, email: &str) -> Option<PersonView> {
        let model = self.read_model.read().await;
        model.people_by_email
            .get(email)
            .and_then(|id| model.people_by_id.get(id).cloned())
    }

    async fn get_organization_members(&self, org_id: Uuid) -> Vec<MemberView> {
        let model = self.read_model.read().await;
        model.organization_members
            .get(&org_id)
            .cloned()
            .unwrap_or_default()
    }

    async fn get_organization_hierarchy(&self, org_id: Uuid) -> OrganizationHierarchy {
        let model = self.read_model.read().await;

        let org = model.organizations_by_id.get(&org_id).cloned();
        let children = model.organization_hierarchy
            .get(&org_id)
            .cloned()
            .unwrap_or_default();

        let parent = org.as_ref()
            .and_then(|o| o.parent_id)
            .and_then(|pid| model.organizations_by_id.get(&pid).cloned());

        OrganizationHierarchy {
            organization: org,
            parent,
            children: children.into_iter()
                .filter_map(|id| model.organizations_by_id.get(&id).cloned())
                .collect(),
        }
    }

    async fn find_agents_by_type(&self, agent_type: AgentType) -> Vec<AgentView> {
        let model = self.read_model.read().await;
        model.agents_by_type
            .get(&agent_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| model.agents_by_id.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn get_person_organizations(&self, person_id: Uuid) -> Vec<OrganizationView> {
        let model = self.read_model.read().await;
        model.person_organizations
            .get(&person_id)
            .map(|org_ids| {
                org_ids.iter()
                    .filter_map(|id| model.organizations_by_id.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct OrganizationHierarchy {
    organization: Option<OrganizationView>,
    parent: Option<OrganizationView>,
    children: Vec<OrganizationView>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CIM CQRS Pattern Demo ===\n");

    // Initialize infrastructure
    println!("Setting up CQRS infrastructure...");
    let nats_config = NatsConfig {
        url: "nats://localhost:4222".to_string(),
        ..Default::default()
    };
    let nats_client = NatsClient::connect(nats_config).await?;

    let config = JetStreamConfig {
        stream_name: "CQRS_DEMO".to_string(),
        stream_subjects: vec!["events.>".to_string()],
        cache_size: 100,
    };

    let event_store = Arc::new(JetStreamEventStore::new(nats_client.client().clone(), config).await?);

    let write_model = WriteModel {
        event_store: event_store.clone(),
    };

    let read_model = Arc::new(RwLock::new(ReadModel::new()));
    let query_handlers = QueryHandlers {
        read_model: read_model.clone(),
    };

    println!("Infrastructure ready!\n");

    // Demonstrate command handling
    println!("=== Command Side (Write Model) ===\n");

    // 1. Register people
    let alice_id = Uuid::new_v4();
    let bob_id = Uuid::new_v4();

    println!("1. Registering people...");
    let alice_events = write_model.handle_register_person(RegisterPerson {
        person_id: alice_id,
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
                number: "+1-555-0101".to_string(),
                phone_type: "mobile".to_string(),
                is_primary: true,
                sms_capable: true,
            }],
            addresses: vec![],
        }),
        location_id: None,
    }).await?;
    println!("   ✓ Alice registered");

    let bob_events = write_model.handle_register_person(RegisterPerson {
        person_id: bob_id,
        identity: IdentityComponent {
            legal_name: "Bob Smith".to_string(),
            preferred_name: None,
            date_of_birth: None,
            government_id: None,
        },
        contact: Some(ContactComponent {
            emails: vec![EmailAddress {
                email: "bob@example.com".to_string(),
                email_type: "work".to_string(),
                is_primary: true,
                is_verified: false,
            }],
            phones: vec![],
            addresses: vec![],
        }),
        location_id: None,
    }).await?;
    println!("   ✓ Bob registered");

    // 2. Create organizations
    let tech_corp_id = Uuid::new_v4();
    let research_lab_id = Uuid::new_v4();

    println!("\n2. Creating organizations...");
    let tech_corp_events = write_model.handle_create_organization(CreateOrganization {
        organization_id: tech_corp_id,
        name: "Tech Corp".to_string(),
        org_type: OrganizationType::Company,
        parent_id: None,
        primary_location_id: None,
    }).await?;
    println!("   ✓ Tech Corp created");

    let research_events = write_model.handle_create_organization(CreateOrganization {
        organization_id: research_lab_id,
        name: "Research Lab".to_string(),
        org_type: OrganizationType::Department,
        parent_id: Some(tech_corp_id),
        primary_location_id: None,
    }).await?;
    println!("   ✓ Research Lab created (subsidiary of Tech Corp)");

    // 3. Add members
    println!("\n3. Adding organization members...");

    // Create roles
    let executive_role = OrganizationRole {
        role_id: "executive".to_string(),
        title: "Chief Technology Officer".to_string(),
        level: RoleLevel::Executive,
        permissions: ["approve_budget", "hire_staff", "strategic_planning"].iter().map(|s| s.to_string()).collect(),
        attributes: HashMap::new(),
    };

    let employee_role = OrganizationRole {
        role_id: "employee".to_string(),
        title: "Research Scientist".to_string(),
        level: RoleLevel::Senior,
        permissions: ["conduct_research", "publish_papers"].iter().map(|s| s.to_string()).collect(),
        attributes: HashMap::new(),
    };

    let alice_member_events = write_model.handle_add_organization_member(AddOrganizationMember {
        organization_id: tech_corp_id,
        person_id: alice_id,
        role: executive_role,
        reports_to: None,
    }).await?;
    println!("   ✓ Alice added as CTO to Tech Corp");

    let bob_member_events = write_model.handle_add_organization_member(AddOrganizationMember {
        organization_id: research_lab_id,
        person_id: bob_id,
        role: employee_role,
        reports_to: Some(alice_id),
    }).await?;
    println!("   ✓ Bob added as Research Scientist to Research Lab");

    // 4. Deploy agents
    println!("\n4. Deploying agents...");
    let assistant_events = write_model.handle_deploy_agent(DeployAgent {
        agent_id: Uuid::new_v4(),
        agent_type: AgentType::AI,
        owner_id: alice_id,
        metadata: AgentMetadata {
            name: "Alice's Assistant".to_string(),
            description: "Personal AI assistant".to_string(),
            tags: ["assistant", "ai"].iter().map(|s| s.to_string()).collect(),
            created_at: Utc::now(),
            last_active: None,
        },
    }).await?;
    println!("   ✓ AI Assistant deployed for Alice");

    // Update read model with all events
    println!("\n=== Updating Read Model ===");
    {
        let mut model = read_model.write().await;
        for event in alice_events.iter()
            .chain(bob_events.iter())
            .chain(tech_corp_events.iter())
            .chain(research_events.iter())
            .chain(alice_member_events.iter())
            .chain(bob_member_events.iter())
            .chain(assistant_events.iter())
        {
            model.apply_event(event);
        }
    }
    println!("Read model updated with all events\n");

    // Demonstrate queries
    println!("=== Query Side (Read Model) ===\n");

    // 1. Find person by ID
    println!("1. Finding person by ID:");
    if let Some(person) = query_handlers.find_person_by_id(alice_id).await {
        println!("   Found: {} ({})", person.name, person.email.unwrap_or_default());
    }

    // 2. Find people by name
    println!("\n2. Finding people by name 'Bob Smith':");
    let bobs = query_handlers.find_people_by_name("Bob Smith").await;
    for person in bobs {
        println!("   Found: {} (ID: {})", person.name, person.id);
    }

    // 3. Find person by email
    println!("\n3. Finding person by email:");
    if let Some(person) = query_handlers.find_person_by_email("alice@example.com").await {
        println!("   Found: {} ({})", person.name, person.id);
    }

    // 4. Get organization members
    println!("\n4. Getting Tech Corp members:");
    let members = query_handlers.get_organization_members(tech_corp_id).await;
    for member in members {
        println!("   - {} ({})", member.person_name, member.role.title);
    }

    // 5. Get organization hierarchy
    println!("\n5. Getting Research Lab hierarchy:");
    let hierarchy = query_handlers.get_organization_hierarchy(research_lab_id).await;
    if let Some(org) = &hierarchy.organization {
        println!("   Organization: {}", org.name);
    }
    if let Some(parent) = &hierarchy.parent {
        println!("   Parent: {}", parent.name);
    }
    println!("   Children: {}", hierarchy.children.len());

    // 6. Find agents by type
    println!("\n6. Finding AI agents:");
    let ai_agents = query_handlers.find_agents_by_type(AgentType::AI).await;
    for agent in ai_agents {
        println!("   - {} (owner: {})", agent.name, agent.owner_id);
    }

    // 7. Get person's organizations
    println!("\n7. Getting Alice's organizations:");
    let alice_orgs = query_handlers.get_person_organizations(alice_id).await;
    for org in alice_orgs {
        println!("   - {} ({:?})", org.name, org.org_type);
    }

    // Demonstrate eventual consistency
    println!("\n=== Eventual Consistency ===\n");
    println!("In a distributed system:");
    println!("• Commands are processed asynchronously");
    println!("• Events are published to message bus");
    println!("• Read models update independently");
    println!("• Queries may show stale data briefly");
    println!("• System converges to consistent state");

    println!("\n=== Demo Complete ===");
    println!("\nKey CQRS Concepts Demonstrated:");
    println!("• Separate write model (commands) and read model (queries)");
    println!("• Commands validate and generate events");
    println!("• Events are the source of truth");
    println!("• Read models are optimized for specific queries");
    println!("• Multiple read models can exist for different needs");
    println!("• Eventual consistency between write and read sides");

    Ok(())
}
