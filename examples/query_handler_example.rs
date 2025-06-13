//! Example of query handlers in CIM domain
//!
//! This example demonstrates how to implement query handlers that:
//! - Process queries to retrieve data
//! - Use read models for optimized data access
//! - Return view models tailored for specific use cases

use cim_domain::{
    // Queries
    GetPersonById, FindPeopleByOrganization,
    FindLocationsByType, FindActivePolicies,
    FindAgentsByCapability, FindWorkflowsByStatus,

    // Query handling
    QueryHandler, QueryCriteria,
    ReadModelStorage, InMemoryReadModel,
    PersonQueryHandler, LocationQueryHandler,
    PolicyQueryHandler, AgentQueryHandler,
    WorkflowQueryHandler,

    // View models
    PersonView, LocationView, PolicyView,
    AgentView, WorkflowView,
};
use uuid::Uuid;

fn main() {
    println!("=== Query Handler Example ===\n");

    // Setup read models
    let person_read_model = InMemoryReadModel::<PersonView>::new();
    let location_read_model = InMemoryReadModel::<LocationView>::new();
    let policy_read_model = InMemoryReadModel::<PolicyView>::new();
    let agent_read_model = InMemoryReadModel::<AgentView>::new();
    let workflow_read_model = InMemoryReadModel::<WorkflowView>::new();

    // Populate read models with sample data
    populate_read_models(
        &person_read_model,
        &location_read_model,
        &policy_read_model,
        &agent_read_model,
        &workflow_read_model,
    );

    // Create query handlers
    let person_handler = PersonQueryHandler::new(person_read_model);
    let location_handler = LocationQueryHandler::new(location_read_model);
    let policy_handler = PolicyQueryHandler::new(policy_read_model);
    let agent_handler = AgentQueryHandler::new(agent_read_model);
    let workflow_handler = WorkflowQueryHandler::new(workflow_read_model);

    // Example 1: Query person by ID
    println!("1. Querying person by ID...");
    let person_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap();
    let query = GetPersonById { person_id };

    match person_handler.handle(query) {
        Ok(Some(person)) => {
            println!("✓ Found person: {} ({})",
                person.legal_name,
                person.preferred_name.as_deref().unwrap_or("no preferred name")
            );
            if let Some(org) = &person.organization_name {
                println!("  Organization: {}", org);
            }
            if !person.roles.is_empty() {
                println!("  Roles: {}", person.roles.join(", "));
            }
        }
        Ok(None) => println!("✗ Person not found"),
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 2: Find people by organization
    println!("\n2. Finding people by organization...");
    let org_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap();
    let query = FindPeopleByOrganization {
        organization_id: org_id,
        limit: Some(5),
    };

    match person_handler.handle(query) {
        Ok(people) => {
            println!("✓ Found {} people in organization", people.len());
            for person in people {
                println!("  - {} ({})",
                    person.legal_name,
                    person.email.as_deref().unwrap_or("no email")
                );
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 3: Find locations by type
    println!("\n3. Finding physical locations...");
    let query = FindLocationsByType {
        location_type: "Physical".to_string(),
        limit: Some(10),
    };

    match location_handler.handle(query) {
        Ok(locations) => {
            println!("✓ Found {} physical locations", locations.len());
            for location in locations {
                println!("  - {} ({})",
                    location.name,
                    location.address.as_deref().unwrap_or("no address")
                );
                if let Some((lat, lon)) = location.coordinates {
                    println!("    Coordinates: {:.4}, {:.4}", lat, lon);
                }
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 4: Find active policies
    println!("\n4. Finding active policies...");
    let query = FindActivePolicies {
        scope: Some("Global".to_string()),
        policy_type: None,
    };

    match policy_handler.handle(query) {
        Ok(policies) => {
            println!("✓ Found {} active global policies", policies.len());
            for policy in policies {
                println!("  - {} (Type: {}, Status: {})",
                    policy.name,
                    policy.policy_type,
                    policy.status
                );
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 5: Find agents by capability
    println!("\n5. Finding agents with text-generation capability...");
    let query = FindAgentsByCapability {
        capability: "text-generation".to_string(),
        status: Some("Active".to_string()),
    };

    match agent_handler.handle(query) {
        Ok(agents) => {
            println!("✓ Found {} agents with text-generation capability", agents.len());
            for agent in agents {
                println!("  - {} (Type: {}, Status: {})",
                    agent.name,
                    agent.agent_type,
                    agent.status
                );
                println!("    Capabilities: {}", agent.capabilities.join(", "));
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    // Example 6: Find running workflows
    println!("\n6. Finding running workflows...");
    let query = FindWorkflowsByStatus {
        status: "Running".to_string(),
        limit: Some(5),
    };

    match workflow_handler.handle(query) {
        Ok(workflows) => {
            println!("✓ Found {} running workflows", workflows.len());
            for workflow in workflows {
                println!("  - {} (State: {}, Transitions: {})",
                    workflow.definition_name,
                    workflow.current_state,
                    workflow.transition_count
                );
                println!("    Started: {}", workflow.started_at);
            }
        }
        Err(e) => println!("✗ Query failed: {}", e),
    }

    println!("\n=== Query Handler Example Complete ===");
}

fn populate_read_models(
    person_model: &InMemoryReadModel<PersonView>,
    location_model: &InMemoryReadModel<LocationView>,
    policy_model: &InMemoryReadModel<PolicyView>,
    agent_model: &InMemoryReadModel<AgentView>,
    workflow_model: &InMemoryReadModel<WorkflowView>,
) {
    // Add sample people
    let person1 = PersonView {
        person_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
        legal_name: "Alice Johnson".to_string(),
        preferred_name: Some("Alice".to_string()),
        email: Some("alice@example.com".to_string()),
        location_name: Some("New York Office".to_string()),
        organization_name: Some("Tech Corp".to_string()),
        roles: vec!["Developer".to_string(), "Team Lead".to_string()],
    };
    person_model.insert(person1.person_id.to_string(), person1);

    let person2 = PersonView {
        person_id: Uuid::new_v4(),
        legal_name: "Bob Smith".to_string(),
        preferred_name: Some("Bob".to_string()),
        email: Some("bob@example.com".to_string()),
        location_name: Some("San Francisco Office".to_string()),
        organization_name: Some("Tech Corp".to_string()),
        roles: vec!["Developer".to_string()],
    };
    person_model.insert(person2.person_id.to_string(), person2);

    let person3 = PersonView {
        person_id: Uuid::new_v4(),
        legal_name: "Carol Davis".to_string(),
        preferred_name: None,
        email: Some("carol@example.com".to_string()),
        location_name: Some("Remote".to_string()),
        organization_name: Some("Tech Corp".to_string()),
        roles: vec!["Designer".to_string()],
    };
    person_model.insert(person3.person_id.to_string(), person3);

    // Add sample locations
    let location1 = LocationView {
        location_id: Uuid::new_v4(),
        name: "New York Office".to_string(),
        location_type: "Physical".to_string(),
        address: Some("123 Broadway, New York, NY 10001".to_string()),
        coordinates: Some((40.7128, -74.0060)),
        parent_location: None,
    };
    location_model.insert(location1.location_id.to_string(), location1);

    let location2 = LocationView {
        location_id: Uuid::new_v4(),
        name: "San Francisco Office".to_string(),
        location_type: "Physical".to_string(),
        address: Some("456 Market St, San Francisco, CA 94105".to_string()),
        coordinates: Some((37.7749, -122.4194)),
        parent_location: None,
    };
    location_model.insert(location2.location_id.to_string(), location2);

    let location3 = LocationView {
        location_id: Uuid::new_v4(),
        name: "Virtual Meeting Room".to_string(),
        location_type: "Virtual".to_string(),
        address: None,
        coordinates: None,
        parent_location: None,
    };
    location_model.insert(location3.location_id.to_string(), location3);

    // Add sample policies
    let policy1 = PolicyView {
        policy_id: Uuid::new_v4(),
        name: "Data Access Policy".to_string(),
        policy_type: "AccessControl".to_string(),
        status: "Active".to_string(),
        scope: "Global".to_string(),
        owner_name: Some("Security Team".to_string()),
        effective_date: Some("2025-01-01".to_string()),
        approval_status: Some("Approved".to_string()),
    };
    policy_model.insert(policy1.policy_id.to_string(), policy1);

    let policy2 = PolicyView {
        policy_id: Uuid::new_v4(),
        name: "Remote Work Policy".to_string(),
        policy_type: "Operational".to_string(),
        status: "Active".to_string(),
        scope: "Global".to_string(),
        owner_name: Some("HR Team".to_string()),
        effective_date: Some("2024-06-01".to_string()),
        approval_status: Some("Approved".to_string()),
    };
    policy_model.insert(policy2.policy_id.to_string(), policy2);

    // Add sample agents
    let agent1 = AgentView {
        agent_id: Uuid::new_v4(),
        name: "AI Assistant".to_string(),
        agent_type: "AI".to_string(),
        status: "Active".to_string(),
        capabilities: vec!["text-generation".to_string(), "code-analysis".to_string()],
        permissions: vec!["read".to_string(), "write".to_string()],
        owner_name: Some("System".to_string()),
    };
    agent_model.insert(agent1.agent_id.to_string(), agent1);

    let agent2 = AgentView {
        agent_id: Uuid::new_v4(),
        name: "Code Reviewer".to_string(),
        agent_type: "AI".to_string(),
        status: "Active".to_string(),
        capabilities: vec!["code-analysis".to_string(), "security-scanning".to_string()],
        permissions: vec!["read".to_string()],
        owner_name: Some("DevOps Team".to_string()),
    };
    agent_model.insert(agent2.agent_id.to_string(), agent2);

    // Add sample workflows
    let workflow1 = WorkflowView {
        workflow_id: Uuid::new_v4(),
        definition_name: "Document Approval".to_string(),
        current_state: "Review".to_string(),
        status: "Running".to_string(),
        started_at: "2025-01-10T10:00:00Z".to_string(),
        transition_count: 2,
        context_data: serde_json::json!({
            "document_id": "doc123",
            "reviewer": "alice@example.com"
        }),
    };
    workflow_model.insert(workflow1.workflow_id.to_string(), workflow1);

    let workflow2 = WorkflowView {
        workflow_id: Uuid::new_v4(),
        definition_name: "Employee Onboarding".to_string(),
        current_state: "IT Setup".to_string(),
        status: "Running".to_string(),
        started_at: "2025-01-09T14:30:00Z".to_string(),
        transition_count: 3,
        context_data: serde_json::json!({
            "employee_id": "emp456",
            "department": "Engineering"
        }),
    };
    workflow_model.insert(workflow2.workflow_id.to_string(), workflow2);
}
