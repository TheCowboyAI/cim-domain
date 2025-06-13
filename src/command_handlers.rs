//! Command handlers for CIM domain aggregates
//!
//! Command handlers process commands, validate business rules, and emit events.
//! They return only acknowledgments, not data - use queries for data retrieval.

use crate::{
    commands::*,
    cqrs::{CommandAcknowledgment, CommandEnvelope, CommandHandler, CommandStatus, CorrelationId},
    entity::EntityId,
    domain_events::DomainEventEnum,
    person::Person,
    organization::Organization,
    agent::Agent,
    location::Location,
    policy::Policy,
    document::Document,
    workflow::{WorkflowAggregate, SimpleState, SimpleInput, SimpleOutput, WorkflowCommand},
    AggregateRoot,
    GraphId,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Event publisher trait for handlers to emit events
pub trait EventPublisher: Send + Sync {
    /// Publish domain events
    fn publish_events(&self, events: Vec<DomainEventEnum>, correlation_id: CorrelationId) -> Result<(), String>;
}

/// Mock event publisher for testing
#[derive(Clone)]
pub struct MockEventPublisher {
    published_events: Arc<RwLock<Vec<(DomainEventEnum, CorrelationId)>>>,
}

impl MockEventPublisher {
    /// Create a new mock event publisher for testing
    pub fn new() -> Self {
        Self {
            published_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all published events for verification in tests
    pub fn get_published_events(&self) -> Vec<(DomainEventEnum, CorrelationId)> {
        self.published_events.read().unwrap().clone()
    }

    /// Get a reference to self as Any for downcasting
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl EventPublisher for MockEventPublisher {
    fn publish_events(&self, events: Vec<DomainEventEnum>, correlation_id: CorrelationId) -> Result<(), String> {
        let mut published = self.published_events.write().unwrap();
        for event in events {
            published.push((event, correlation_id.clone()));
        }
        Ok(())
    }
}

/// Repository trait for loading and saving aggregates
pub trait AggregateRepository<A: AggregateRoot>: Send + Sync {
    /// Load aggregate by ID
    fn load(&self, id: A::Id) -> Result<Option<A>, String>;

    /// Save aggregate
    fn save(&self, aggregate: &A) -> Result<(), String>;
}

/// In-memory repository for testing
pub struct InMemoryRepository<A: AggregateRoot + Clone + Send + Sync> {
    storage: Arc<RwLock<HashMap<A::Id, A>>>,
}

impl<A: AggregateRoot + Clone + Send + Sync> InMemoryRepository<A>
where
    A::Id: std::hash::Hash + Eq + Clone,
{
    /// Create a new in-memory repository for testing
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<A: AggregateRoot + Clone + Send + Sync> AggregateRepository<A> for InMemoryRepository<A>
where
    A::Id: std::hash::Hash + Eq + Clone,
{
    fn load(&self, id: A::Id) -> Result<Option<A>, String> {
        Ok(self.storage.read().unwrap().get(&id).cloned())
    }

    fn save(&self, aggregate: &A) -> Result<(), String> {
        self.storage.write().unwrap().insert(aggregate.id(), aggregate.clone());
        Ok(())
    }
}

// Person Command Handlers

/// Handler for person-related commands
pub struct PersonCommandHandler<R: AggregateRepository<Person>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Person>> PersonCommandHandler<R> {
    /// Create a new person command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Person>> CommandHandler<RegisterPerson> for PersonCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<RegisterPerson>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let person_id = EntityId::from_uuid(cmd.person_id);

        // Check if person already exists
        match self.repository.load(person_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Person already exists".to_string()),
            },
            Ok(None) => {
                // Create new person
                let person = Person::new(person_id, cmd.identity.clone());

                // Save person
                if let Err(e) = self.repository.save(&person) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save person: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::PersonRegistered(crate::PersonRegistered {
                    person_id: cmd.person_id,
                    identity: cmd.identity.clone(),
                    contact: cmd.contact.clone(),
                    location_id: cmd.location_id,
                    registered_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Organization Command Handlers

/// Handler for organization-related commands
pub struct OrganizationCommandHandler<R: AggregateRepository<Organization>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Organization>> OrganizationCommandHandler<R> {
    /// Create a new organization command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Organization>> CommandHandler<CreateOrganization> for OrganizationCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<CreateOrganization>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let org_id = EntityId::from_uuid(cmd.organization_id);

        // Check if organization already exists
        match self.repository.load(org_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Organization already exists".to_string()),
            },
            Ok(None) => {
                // Create new organization
                let organization = Organization::new(
                    cmd.name.clone(),
                    cmd.org_type.clone(),
                );

                // Save organization
                if let Err(e) = self.repository.save(&organization) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save organization: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::OrganizationCreated(crate::OrganizationCreated {
                    organization_id: cmd.organization_id,
                    name: cmd.name.clone(),
                    org_type: cmd.org_type.clone(),
                    parent_id: cmd.parent_id,
                    primary_location_id: cmd.primary_location_id,
                    created_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Agent Command Handlers

/// Handler for agent-related commands
pub struct AgentCommandHandler<R: AggregateRepository<Agent>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Agent>> AgentCommandHandler<R> {
    /// Create a new agent command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Agent>> CommandHandler<DeployAgent> for AgentCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<DeployAgent>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let agent_id = EntityId::from_uuid(cmd.agent_id);

        // Check if agent already exists
        match self.repository.load(agent_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Agent already exists".to_string()),
            },
            Ok(None) => {
                // Create new agent
                let mut agent = Agent::new(
                    cmd.agent_id,
                    cmd.agent_type,
                    cmd.owner_id,
                );

                // Add metadata component
                if let Err(e) = agent.add_component(cmd.metadata.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to add metadata: {}", e)),
                    };
                }

                // Save agent
                if let Err(e) = self.repository.save(&agent) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save agent: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::AgentDeployed(crate::AgentDeployed {
                    agent_id: cmd.agent_id,
                    agent_type: cmd.agent_type,
                    owner_id: cmd.owner_id,
                    metadata: cmd.metadata.clone(),
                    deployed_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Location Command Handlers

/// Handler for location-related commands
pub struct LocationCommandHandler<R: AggregateRepository<Location>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Location>> LocationCommandHandler<R> {
    /// Create a new location command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Location>> CommandHandler<DefineLocation> for LocationCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<DefineLocation>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let location_id = EntityId::from_uuid(cmd.location_id);

        // Check if location already exists
        match self.repository.load(location_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Location already exists".to_string()),
            },
            Ok(None) => {
                // Create new location based on type
                let location = match &cmd.location_type {
                    crate::LocationType::Physical => {
                        if let Some(address) = &cmd.address {
                            match Location::new_physical(location_id, cmd.name.clone(), address.clone()) {
                                Ok(mut loc) => {
                                    // Add coordinates if provided
                                    if let Some(coords) = &cmd.coordinates {
                                        if let Err(e) = loc.set_coordinates(coords.clone()) {
                                            return CommandAcknowledgment {
                                                command_id: envelope.id,
                                                correlation_id: envelope.correlation_id,
                                                status: CommandStatus::Rejected,
                                                reason: Some(format!("Invalid coordinates: {}", e)),
                                            };
                                        }
                                    }
                                    loc
                                }
                                Err(e) => {
                                    return CommandAcknowledgment {
                                        command_id: envelope.id,
                                        correlation_id: envelope.correlation_id,
                                        status: CommandStatus::Rejected,
                                        reason: Some(format!("Failed to create location: {}", e)),
                                    };
                                }
                            }
                        } else if let Some(coords) = &cmd.coordinates {
                            match Location::new_from_coordinates(location_id, cmd.name.clone(), coords.clone()) {
                                Ok(loc) => loc,
                                Err(e) => {
                                    return CommandAcknowledgment {
                                        command_id: envelope.id,
                                        correlation_id: envelope.correlation_id,
                                        status: CommandStatus::Rejected,
                                        reason: Some(format!("Failed to create location: {}", e)),
                                    };
                                }
                            }
                        } else {
                            return CommandAcknowledgment {
                                command_id: envelope.id,
                                correlation_id: envelope.correlation_id,
                                status: CommandStatus::Rejected,
                                reason: Some("Physical location requires either address or coordinates".to_string()),
                            };
                        }
                    }
                    crate::LocationType::Virtual => {
                        if let Some(virtual_loc) = &cmd.virtual_location {
                            match Location::new_virtual(location_id, cmd.name.clone(), virtual_loc.clone()) {
                                Ok(loc) => loc,
                                Err(e) => {
                                    return CommandAcknowledgment {
                                        command_id: envelope.id,
                                        correlation_id: envelope.correlation_id,
                                        status: CommandStatus::Rejected,
                                        reason: Some(format!("Failed to create virtual location: {}", e)),
                                    };
                                }
                            }
                        } else {
                            return CommandAcknowledgment {
                                command_id: envelope.id,
                                correlation_id: envelope.correlation_id,
                                status: CommandStatus::Rejected,
                                reason: Some("Virtual location requires virtual location details".to_string()),
                            };
                        }
                    }
                    _ => {
                        // For Logical and Hybrid types, create a basic location
                        let mut loc = Location::new_from_coordinates(
                            location_id,
                            cmd.name.clone(),
                            crate::GeoCoordinates::new(0.0, 0.0), // Default coordinates
                        ).unwrap();
                        loc.location_type = cmd.location_type.clone();
                        loc
                    }
                };

                // Save location
                if let Err(e) = self.repository.save(&location) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save location: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::LocationDefined(crate::LocationDefined {
                    location_id: cmd.location_id,
                    name: cmd.name.clone(),
                    location_type: cmd.location_type.clone(),
                    address: cmd.address.clone(),
                    coordinates: cmd.coordinates.clone(),
                    virtual_location: cmd.virtual_location.clone(),
                    parent_id: cmd.parent_id,
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Policy Command Handlers

/// Handler for policy-related commands
pub struct PolicyCommandHandler<R: AggregateRepository<Policy>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Policy>> PolicyCommandHandler<R> {
    /// Create a new policy command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Policy>> CommandHandler<EnactPolicy> for PolicyCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<EnactPolicy>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let policy_id = EntityId::from_uuid(cmd.policy_id);

        // Check if policy already exists
        match self.repository.load(policy_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Policy already exists".to_string()),
            },
            Ok(None) => {
                // Create new policy
                let mut policy = Policy::new(
                    cmd.policy_id,
                    cmd.policy_type,
                    cmd.scope.clone(),
                    cmd.owner_id,
                );

                // Add metadata component
                if let Err(e) = policy.add_component(cmd.metadata.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to add metadata: {}", e)),
                    };
                }

                // Save policy
                if let Err(e) = self.repository.save(&policy) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save policy: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::PolicyEnacted(crate::PolicyEnacted {
                    policy_id: cmd.policy_id,
                    policy_type: cmd.policy_type,
                    scope: cmd.scope.clone(),
                    owner_id: cmd.owner_id,
                    metadata: cmd.metadata.clone(),
                    enacted_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Document Command Handlers

/// Handler for document-related commands
pub struct DocumentCommandHandler<R: AggregateRepository<Document>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<Document>> DocumentCommandHandler<R> {
    /// Create a new document command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<Document>> CommandHandler<UploadDocument> for DocumentCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<UploadDocument>) -> CommandAcknowledgment {
        let cmd = &envelope.command;
        let document_id = EntityId::from_uuid(cmd.document_id);

        // Check if document already exists
        match self.repository.load(document_id) {
            Ok(Some(_)) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some("Document already exists".to_string()),
            },
            Ok(None) => {
                // Create new document
                let document = if cmd.is_chunked {
                    Document::new_chunked(
                        document_id,
                        cmd.info.clone(),
                        cmd.chunk_cids.clone(),
                        cmd.content_cid,
                    )
                } else {
                    Document::new(
                        document_id,
                        cmd.info.clone(),
                        cmd.content_cid,
                    )
                };

                // Save document
                if let Err(e) = self.repository.save(&document) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save document: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::DocumentUploaded(crate::DocumentUploaded {
                    document_id: cmd.document_id,
                    info: cmd.info.clone(),
                    content_cid: cmd.content_cid,
                    is_chunked: cmd.is_chunked,
                    chunk_cids: cmd.chunk_cids.clone(),
                    uploaded_by: cmd.uploaded_by,
                    uploaded_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => CommandAcknowledgment {
                command_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: CommandStatus::Rejected,
                reason: Some(format!("Repository error: {}", e)),
            },
        }
    }
}

// Workflow Command Handlers

/// Handler for workflow-related commands
/// Note: This is a simplified handler using SimpleState/SimpleInput/SimpleOutput
/// In production, you would use your domain-specific types
pub struct WorkflowCommandHandler<R: AggregateRepository<WorkflowAggregate<SimpleState, SimpleInput, SimpleOutput>>> {
    repository: R,
    event_publisher: Box<dyn EventPublisher>,
}

impl<R: AggregateRepository<WorkflowAggregate<SimpleState, SimpleInput, SimpleOutput>>> WorkflowCommandHandler<R> {
    /// Create a new workflow command handler
    pub fn new(repository: R, event_publisher: Box<dyn EventPublisher>) -> Self {
        Self {
            repository,
            event_publisher,
        }
    }
}

impl<R: AggregateRepository<WorkflowAggregate<SimpleState, SimpleInput, SimpleOutput>>> CommandHandler<WorkflowCommand<SimpleInput>> for WorkflowCommandHandler<R> {
    fn handle(&mut self, envelope: CommandEnvelope<WorkflowCommand<SimpleInput>>) -> CommandAcknowledgment {
        match &envelope.command {
            WorkflowCommand::StartWorkflow { definition_id, initial_context, workflow_id, .. } => {
                // Check if workflow already exists (if ID provided)
                if let Some(wf_id) = workflow_id {
                    match self.repository.load(*wf_id) {
                        Ok(Some(_)) => {
                            return CommandAcknowledgment {
                                command_id: envelope.id,
                                correlation_id: envelope.correlation_id,
                                status: CommandStatus::Rejected,
                                reason: Some("Workflow already exists".to_string()),
                            };
                        }
                        Ok(None) => {}
                        Err(e) => {
                            return CommandAcknowledgment {
                                command_id: envelope.id,
                                correlation_id: envelope.correlation_id,
                                status: CommandStatus::Rejected,
                                reason: Some(format!("Repository error: {}", e)),
                            };
                        }
                    }
                }

                // Create new workflow
                let initial_state = SimpleState::new("Start");
                let workflow = WorkflowAggregate::new(
                    *definition_id,
                    initial_state,
                    initial_context.clone(),
                );

                let workflow_id = workflow.id();

                // Save workflow
                if let Err(e) = self.repository.save(&workflow) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to save workflow: {}", e)),
                    };
                }

                // Emit event
                let event = DomainEventEnum::WorkflowStarted(crate::WorkflowStarted {
                    workflow_id,
                    definition_id: *definition_id,
                    initial_state: "Start".to_string(),
                    started_at: chrono::Utc::now(),
                });

                if let Err(e) = self.event_publisher.publish_events(vec![event], envelope.correlation_id.clone()) {
                    return CommandAcknowledgment {
                        command_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: CommandStatus::Rejected,
                        reason: Some(format!("Failed to publish event: {}", e)),
                    };
                }

                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Accepted,
                    reason: None,
                }
            }
            _ => {
                // For other workflow commands, we would need to load the workflow,
                // apply the command, save it, and emit events
                // This is a simplified implementation
                CommandAcknowledgment {
                    command_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: CommandStatus::Rejected,
                    reason: Some("Command not implemented".to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IdentityComponent, ContactComponent};

    #[test]
    fn test_register_person_handler() {
        // Setup
        let repository = InMemoryRepository::<Person>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = PersonCommandHandler::new(repository, event_publisher.clone());

        // Create command
        let person_id = uuid::Uuid::new_v4();
        let command = RegisterPerson {
            person_id,
            identity: IdentityComponent {
                legal_name: "John Doe".to_string(),
                preferred_name: Some("John".to_string()),
                date_of_birth: None,
                government_id: None,
            },
            contact: Some(ContactComponent {
                emails: vec![],
                phones: vec![],
                addresses: vec![],
            }),
            location_id: None,
        };

        let envelope = CommandEnvelope::new(command, "test-user".to_string());

        // Handle command
        let ack = handler.handle(envelope);

        // Verify acknowledgment
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        // Verify event was published
        let mock_publisher = event_publisher.as_any().downcast_ref::<MockEventPublisher>().unwrap();
        let events = mock_publisher.get_published_events();
        assert_eq!(events.len(), 1);

        match &events[0].0 {
            DomainEventEnum::PersonRegistered(event) => {
                assert_eq!(event.person_id, person_id);
                assert_eq!(event.identity.legal_name, "John Doe");
            }
            _ => panic!("Expected PersonRegistered event"),
        }
    }

    #[test]
    fn test_register_duplicate_person() {
        // Setup
        let repository = InMemoryRepository::<Person>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = PersonCommandHandler::new(repository, event_publisher);

        // Create and register first person
        let person_id = uuid::Uuid::new_v4();
        let command = RegisterPerson {
            person_id,
            identity: IdentityComponent {
                legal_name: "John Doe".to_string(),
                preferred_name: None,
                date_of_birth: None,
                government_id: None,
            },
            contact: None,
            location_id: None,
        };

        let envelope = CommandEnvelope::new(command.clone(), "test-user".to_string());
        let ack = handler.handle(envelope);
        assert_eq!(ack.status, CommandStatus::Accepted);

        // Try to register same person again
        let envelope2 = CommandEnvelope::new(command, "test-user".to_string());
        let ack2 = handler.handle(envelope2);

        // Should be rejected
        assert_eq!(ack2.status, CommandStatus::Rejected);
        assert_eq!(ack2.reason, Some("Person already exists".to_string()));
    }

    #[test]
    fn test_location_command_handler() {
        // Setup
        let repository = InMemoryRepository::<Location>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = LocationCommandHandler::new(repository, event_publisher.clone());

        // Create command
        let location_id = uuid::Uuid::new_v4();
        let command = DefineLocation {
            location_id,
            name: "Main Office".to_string(),
            location_type: crate::LocationType::Physical,
            address: Some(crate::Address::new(
                "123 Main St".to_string(),
                "Springfield".to_string(),
                "IL".to_string(),
                "USA".to_string(),
                "62701".to_string(),
            )),
            coordinates: None,
            virtual_location: None,
            parent_id: None,
            metadata: std::collections::HashMap::new(),
        };

        let envelope = CommandEnvelope::new(command, "test-user".to_string());

        // Handle command
        let ack = handler.handle(envelope);

        // Verify acknowledgment
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        // Verify event was published
        let mock_publisher = event_publisher.as_any().downcast_ref::<MockEventPublisher>().unwrap();
        let events = mock_publisher.get_published_events();
        assert_eq!(events.len(), 1);

        match &events[0].0 {
            DomainEventEnum::LocationDefined(event) => {
                assert_eq!(event.location_id, location_id);
                assert_eq!(event.name, "Main Office");
            }
            _ => panic!("Expected LocationDefined event"),
        }
    }

    #[test]
    fn test_policy_command_handler() {
        // Setup
        let repository = InMemoryRepository::<Policy>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = PolicyCommandHandler::new(repository, event_publisher.clone());

        // Create command
        let policy_id = uuid::Uuid::new_v4();
        let owner_id = uuid::Uuid::new_v4();
        let command = EnactPolicy {
            policy_id,
            policy_type: crate::PolicyType::AccessControl,
            scope: crate::PolicyScope::Global,
            owner_id,
            metadata: crate::PolicyMetadata {
                name: "Access Control Policy".to_string(),
                description: "Main access control policy".to_string(),
                tags: std::collections::HashSet::new(),
                effective_date: None,
                expiration_date: None,
                compliance_frameworks: std::collections::HashSet::new(),
            },
        };

        let envelope = CommandEnvelope::new(command, "test-user".to_string());

        // Handle command
        let ack = handler.handle(envelope);

        // Verify acknowledgment
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        // Verify event was published
        let mock_publisher = event_publisher.as_any().downcast_ref::<MockEventPublisher>().unwrap();
        let events = mock_publisher.get_published_events();
        assert_eq!(events.len(), 1);

        match &events[0].0 {
            DomainEventEnum::PolicyEnacted(event) => {
                assert_eq!(event.policy_id, policy_id);
                assert_eq!(event.policy_type, crate::PolicyType::AccessControl);
            }
            _ => panic!("Expected PolicyEnacted event"),
        }
    }

    #[test]
    fn test_document_command_handler() {
        // Setup
        let repository = InMemoryRepository::<Document>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = DocumentCommandHandler::new(repository, event_publisher.clone());

        // Create command
        let document_id = uuid::Uuid::new_v4();
        let uploaded_by = uuid::Uuid::new_v4();
        let content_cid = cid::Cid::try_from("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi").unwrap();

        let command = UploadDocument {
            document_id,
            info: crate::DocumentInfoComponent {
                title: "Test Document".to_string(),
                description: Some("A test document".to_string()),
                mime_type: "text/plain".to_string(),
                filename: Some("test.txt".to_string()),
                size_bytes: 1024,
                language: Some("en".to_string()),
            },
            content_cid,
            is_chunked: false,
            chunk_cids: vec![],
            uploaded_by,
        };

        let envelope = CommandEnvelope::new(command, "test-user".to_string());

        // Handle command
        let ack = handler.handle(envelope);

        // Verify acknowledgment
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        // Verify event was published
        let mock_publisher = event_publisher.as_any().downcast_ref::<MockEventPublisher>().unwrap();
        let events = mock_publisher.get_published_events();
        assert_eq!(events.len(), 1);

        match &events[0].0 {
            DomainEventEnum::DocumentUploaded(event) => {
                assert_eq!(event.document_id, document_id);
                assert_eq!(event.info.title, "Test Document");
            }
            _ => panic!("Expected DocumentUploaded event"),
        }
    }

    #[test]
    fn test_workflow_command_handler() {
        use crate::workflow::{WorkflowAggregate, SimpleState, SimpleInput, SimpleOutput, WorkflowCommand, WorkflowContext};

        // Setup
        let repository = InMemoryRepository::<WorkflowAggregate<SimpleState, SimpleInput, SimpleOutput>>::new();
        let event_publisher = Box::new(MockEventPublisher::new());
        let mut handler = WorkflowCommandHandler::new(repository, event_publisher.clone());

        // Create command
        let definition_id = GraphId::new();
        let command = WorkflowCommand::StartWorkflow {
            definition_id,
            initial_context: WorkflowContext::new(),
            workflow_id: None,
            start_time: None,
        };

        let envelope = CommandEnvelope::new(command, "test-user".to_string());

        // Handle command
        let ack = handler.handle(envelope);

        // Verify acknowledgment
        assert_eq!(ack.status, CommandStatus::Accepted);
        assert!(ack.reason.is_none());

        // Verify event was published
        let mock_publisher = event_publisher.as_any().downcast_ref::<MockEventPublisher>().unwrap();
        let events = mock_publisher.get_published_events();
        assert_eq!(events.len(), 1);

        match &events[0].0 {
            DomainEventEnum::WorkflowStarted(event) => {
                assert_eq!(event.initial_state, "Start");
            }
            _ => panic!("Expected WorkflowStarted event"),
        }
    }
}
