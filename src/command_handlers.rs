//! Command handlers for CIM domain aggregates
//!
//! Command handlers process commands, validate business rules, and emit events.
//! They return only acknowledgments, not data - use queries for data retrieval.

use crate::{
    commands::*,
    cqrs::{CommandAcknowledgment, CommandEnvelope, CommandHandler, CommandStatus, CorrelationId},
    entity::EntityId,
    domain_events::DomainEventEnum,
    location::Location,
    AggregateRoot,
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





// Workflow Command Handler has been moved to cim-domain-workflow

#[cfg(test)]
mod tests {
    use super::*;

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





    // Workflow command handler tests have been moved to cim-domain-workflow
}
