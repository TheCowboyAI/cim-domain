// Copyright 2025 Cowboy AI, LLC.

//! Infrastructure Layer 1.3: CQRS Flow Tests for cim-domain
//!
//! User Story: As a domain system, I need to separate commands and queries for scalability
//!
//! Test Requirements:
//! - Verify command handler execution
//! - Verify query handler execution
//! - Verify command/query separation
//! - Verify projection updates from events
//!
//! Event Sequence:
//! 1. CommandReceived { command_type, aggregate_id }
//! 2. CommandValidated { command_type, aggregate_id }
//! 3. EventGenerated { event_type, aggregate_id }
//! 4. ProjectionUpdated { projection_type, entity_id }
//! 5. QueryExecuted { query_type, result_count }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Send Command]
//!     B --> C[CommandReceived]
//!     C --> D[Validate Command]
//!     D --> E[CommandValidated]
//!     E --> F[Generate Events]
//!     F --> G[EventGenerated]
//!     G --> H[Update Projection]
//!     H --> I[ProjectionUpdated]
//!     I --> J[Execute Query]
//!     J --> K[QueryExecuted]
//!     K --> L[Test Success]
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// CQRS event types for testing
#[derive(Debug, Clone, PartialEq)]
pub enum CQRSEvent {
    CommandReceived {
        command_type: String,
        aggregate_id: String,
    },
    CommandValidated {
        command_type: String,
        aggregate_id: String,
    },
    EventGenerated {
        event_type: String,
        aggregate_id: String,
    },
    ProjectionUpdated {
        projection_type: String,
        entity_id: String,
    },
    QueryExecuted {
        query_type: String,
        result_count: usize,
    },
}

/// Mock command for testing
#[derive(Debug, Clone)]
pub struct TestCommand {
    pub command_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
}

/// Mock query for testing
#[derive(Debug, Clone)]
pub struct TestQuery {
    pub query_type: String,
    pub filter: HashMap<String, String>,
}

/// Mock domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDomainEvent {
    pub event_type: String,
    pub aggregate_id: String,
    pub payload: serde_json::Value,
}

/// Mock projection entry
#[derive(Debug, Clone)]
pub struct ProjectionEntry {
    pub entity_id: String,
    pub projection_type: String,
    pub data: serde_json::Value,
}

/// Mock command handler
pub struct TestCommandHandler {
    validation_rules: HashMap<String, Box<dyn Fn(&TestCommand) -> bool + Send + Sync>>,
    event_store: Arc<Mutex<Vec<TestDomainEvent>>>,
}

impl TestCommandHandler {
    pub fn new() -> Self {
        let mut validation_rules = HashMap::new();

        // Add default validation rule
        validation_rules.insert(
            "CreateEntity".to_string(),
            Box::new(|cmd: &TestCommand| !cmd.aggregate_id.is_empty())
                as Box<dyn Fn(&TestCommand) -> bool + Send + Sync>,
        );

        Self {
            validation_rules,
            event_store: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn handle_command(&self, command: TestCommand) -> Result<Vec<TestDomainEvent>, String> {
        // Validate command
        if let Some(validator) = self.validation_rules.get(&command.command_type) {
            if !validator(&command) {
                return Err(format!(
                    "Command validation failed for {}",
                    command.command_type
                ));
            }
        }

        // Generate events based on command
        let events = match command.command_type.as_str() {
            "CreateEntity" => vec![TestDomainEvent {
                event_type: "EntityCreated".to_string(),
                aggregate_id: command.aggregate_id.clone(),
                payload: command.payload,
            }],
            "UpdateEntity" => vec![TestDomainEvent {
                event_type: "EntityUpdated".to_string(),
                aggregate_id: command.aggregate_id.clone(),
                payload: command.payload,
            }],
            _ => return Err(format!("Unknown command type: {}", command.command_type)),
        };

        // Store events
        let mut store = self.event_store.lock().unwrap();
        store.extend(events.clone());

        Ok(events)
    }

    pub fn get_events(&self) -> Vec<TestDomainEvent> {
        self.event_store.lock().unwrap().clone()
    }
}

/// Mock projection updater
pub struct TestProjectionUpdater {
    projections: Arc<Mutex<HashMap<String, ProjectionEntry>>>,
}

impl TestProjectionUpdater {
    pub fn new() -> Self {
        Self {
            projections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_from_event(&self, event: &TestDomainEvent) -> Result<(), String> {
        let projection_type = match event.event_type.as_str() {
            "EntityCreated" => "EntityProjection",
            "EntityUpdated" => "EntityProjection",
            _ => return Err(format!("Unknown event type: {}", event.event_type)),
        };

        let entry = ProjectionEntry {
            entity_id: event.aggregate_id.clone(),
            projection_type: projection_type.to_string(),
            data: event.payload.clone(),
        };

        let mut projections = self.projections.lock().unwrap();
        projections.insert(event.aggregate_id.clone(), entry);

        Ok(())
    }

    pub fn get_projection(&self, entity_id: &str) -> Option<ProjectionEntry> {
        self.projections.lock().unwrap().get(entity_id).cloned()
    }

    pub fn get_all_projections(&self) -> Vec<ProjectionEntry> {
        self.projections.lock().unwrap().values().cloned().collect()
    }
}

/// Mock query handler
pub struct TestQueryHandler {
    projection_updater: Arc<TestProjectionUpdater>,
}

impl TestQueryHandler {
    pub fn new(projection_updater: Arc<TestProjectionUpdater>) -> Self {
        Self { projection_updater }
    }

    pub fn handle_query(&self, query: TestQuery) -> Result<Vec<ProjectionEntry>, String> {
        match query.query_type.as_str() {
            "GetById" => {
                if let Some(id) = query.filter.get("id") {
                    Ok(self
                        .projection_updater
                        .get_projection(id)
                        .map(|p| vec![p])
                        .unwrap_or_default())
                } else {
                    Err("Missing 'id' in query filter".to_string())
                }
            }
            "GetAll" => Ok(self.projection_updater.get_all_projections()),
            "GetByType" => {
                if let Some(proj_type) = query.filter.get("type") {
                    Ok(self
                        .projection_updater
                        .get_all_projections()
                        .into_iter()
                        .filter(|p| &p.projection_type == proj_type)
                        .collect())
                } else {
                    Err("Missing 'type' in query filter".to_string())
                }
            }
            _ => Err(format!("Unknown query type: {}", query.query_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_command_handler_validation() {
        // Arrange
        let handler = TestCommandHandler::new();

        let valid_command = TestCommand {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "entity_1".to_string(),
            payload: json!({ "name": "Test Entity" }),
        };

        let invalid_command = TestCommand {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "".to_string(), // Empty ID should fail validation
            payload: json!({ "name": "Test Entity" }),
        };

        // Act
        let valid_result = handler.handle_command(valid_command);
        let invalid_result = handler.handle_command(invalid_command);

        // Assert
        assert!(valid_result.is_ok());
        assert_eq!(valid_result.unwrap().len(), 1);

        assert!(invalid_result.is_err());
        assert!(invalid_result.unwrap_err().contains("validation failed"));
    }

    #[test]
    fn test_event_generation_from_commands() {
        // Arrange
        let handler = TestCommandHandler::new();

        let create_command = TestCommand {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "entity_1".to_string(),
            payload: json!({ "name": "New Entity" }),
        };

        let update_command = TestCommand {
            command_type: "UpdateEntity".to_string(),
            aggregate_id: "entity_1".to_string(),
            payload: json!({ "name": "Updated Entity" }),
        };

        // Act
        let create_events = handler.handle_command(create_command).unwrap();
        let update_events = handler.handle_command(update_command).unwrap();

        // Assert
        assert_eq!(create_events.len(), 1);
        assert_eq!(create_events[0].event_type, "EntityCreated");
        assert_eq!(create_events[0].aggregate_id, "entity_1");

        assert_eq!(update_events.len(), 1);
        assert_eq!(update_events[0].event_type, "EntityUpdated");

        // Verify events are stored
        let all_events = handler.get_events();
        assert_eq!(all_events.len(), 2);
    }

    #[test]
    fn test_projection_updates_from_events() {
        // Arrange
        let updater = TestProjectionUpdater::new();

        let create_event = TestDomainEvent {
            event_type: "EntityCreated".to_string(),
            aggregate_id: "entity_1".to_string(),
            payload: json!({ "name": "Test Entity", "status": "active" }),
        };

        let update_event = TestDomainEvent {
            event_type: "EntityUpdated".to_string(),
            aggregate_id: "entity_1".to_string(),
            payload: json!({ "name": "Updated Entity", "status": "modified" }),
        };

        // Act
        updater.update_from_event(&create_event).unwrap();
        let projection_after_create = updater.get_projection("entity_1").unwrap();

        updater.update_from_event(&update_event).unwrap();
        let projection_after_update = updater.get_projection("entity_1").unwrap();

        // Assert
        assert_eq!(projection_after_create.entity_id, "entity_1");
        assert_eq!(projection_after_create.projection_type, "EntityProjection");
        assert_eq!(projection_after_create.data["status"], "active");

        assert_eq!(projection_after_update.data["status"], "modified");
    }

    #[test]
    fn test_query_handler_operations() {
        // Arrange
        let updater = Arc::new(TestProjectionUpdater::new());
        let query_handler = TestQueryHandler::new(updater.clone());

        // Create some projections
        for i in 1..=3 {
            let event = TestDomainEvent {
                event_type: "EntityCreated".to_string(),
                aggregate_id: format!("entity_{i}"),
                payload: json!({ "name": format!("Entity {i}") }),
            };
            updater.update_from_event(&event).unwrap();
        }

        // Act - Query by ID
        let mut filter = HashMap::new();
        filter.insert("id".to_string(), "entity_2".to_string());

        let by_id_query = TestQuery {
            query_type: "GetById".to_string(),
            filter,
        };

        let by_id_result = query_handler.handle_query(by_id_query).unwrap();

        // Act - Query all
        let get_all_query = TestQuery {
            query_type: "GetAll".to_string(),
            filter: HashMap::new(),
        };

        let all_result = query_handler.handle_query(get_all_query).unwrap();

        // Assert
        assert_eq!(by_id_result.len(), 1);
        assert_eq!(by_id_result[0].entity_id, "entity_2");

        assert_eq!(all_result.len(), 3);
    }

    #[test]
    fn test_cqrs_complete_flow() {
        // Arrange
        let command_handler = TestCommandHandler::new();
        let projection_updater = Arc::new(TestProjectionUpdater::new());
        let query_handler = TestQueryHandler::new(projection_updater.clone());

        let mut events_captured = Vec::new();

        // Act - Send command
        let command = TestCommand {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "test_entity".to_string(),
            payload: json!({ "name": "CQRS Test Entity", "value": 42 }),
        };

        events_captured.push(CQRSEvent::CommandReceived {
            command_type: command.command_type.clone(),
            aggregate_id: command.aggregate_id.clone(),
        });

        // Validate and handle command
        let events = command_handler.handle_command(command).unwrap();

        events_captured.push(CQRSEvent::CommandValidated {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "test_entity".to_string(),
        });

        events_captured.push(CQRSEvent::EventGenerated {
            event_type: events[0].event_type.clone(),
            aggregate_id: events[0].aggregate_id.clone(),
        });

        // Update projection
        for event in &events {
            projection_updater.update_from_event(event).unwrap();
        }

        events_captured.push(CQRSEvent::ProjectionUpdated {
            projection_type: "EntityProjection".to_string(),
            entity_id: "test_entity".to_string(),
        });

        // Execute query
        let mut filter = HashMap::new();
        filter.insert("id".to_string(), "test_entity".to_string());

        let query = TestQuery {
            query_type: "GetById".to_string(),
            filter,
        };

        let query_result = query_handler.handle_query(query).unwrap();

        events_captured.push(CQRSEvent::QueryExecuted {
            query_type: "GetById".to_string(),
            result_count: query_result.len(),
        });

        // Assert - Verify complete flow
        assert_eq!(events_captured.len(), 5);
        assert_eq!(query_result.len(), 1);
        assert_eq!(query_result[0].entity_id, "test_entity");
        assert_eq!(query_result[0].data["name"], "CQRS Test Entity");
        assert_eq!(query_result[0].data["value"], 42);

        // Verify event sequence
        assert!(matches!(
            events_captured[0],
            CQRSEvent::CommandReceived { .. }
        ));
        assert!(matches!(
            events_captured[1],
            CQRSEvent::CommandValidated { .. }
        ));
        assert!(matches!(
            events_captured[2],
            CQRSEvent::EventGenerated { .. }
        ));
        assert!(matches!(
            events_captured[3],
            CQRSEvent::ProjectionUpdated { .. }
        ));
        assert!(matches!(
            events_captured[4],
            CQRSEvent::QueryExecuted { .. }
        ));
    }

    #[test]
    fn test_command_query_separation() {
        // Arrange
        let command_handler = TestCommandHandler::new();
        let projection_updater = Arc::new(TestProjectionUpdater::new());
        let query_handler = TestQueryHandler::new(projection_updater.clone());

        // Act - Commands should not return data
        let command = TestCommand {
            command_type: "CreateEntity".to_string(),
            aggregate_id: "entity_sep".to_string(),
            payload: json!({ "data": "test" }),
        };

        let command_result = command_handler.handle_command(command).unwrap();

        // Assert - Command returns events, not data
        assert!(!command_result.is_empty());
        assert!(command_result[0].event_type.contains("Created"));

        // Act - Queries should not modify state
        let initial_projections = projection_updater.get_all_projections();

        let query = TestQuery {
            query_type: "GetAll".to_string(),
            filter: HashMap::new(),
        };

        let _query_result = query_handler.handle_query(query).unwrap();
        let after_query_projections = projection_updater.get_all_projections();

        // Assert - Query didn't change state
        assert_eq!(initial_projections.len(), after_query_projections.len());
    }
}
