//! Bridge between NATS domain events and Bevy ECS components
//!
//! This module provides translation between domain events (NATS messages)
//! and Bevy ECS commands/components. It handles the impedance mismatch
//! between event-driven domain logic and ECS component updates.

use crate::{
    OrganizationCreated, AgentDeployed, PolicyEnacted,
    DomainEventEnvelope,
};
use cim_subject::{Subject as SubjectParts, MessageTranslator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Bevy-compatible component data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    /// Component type identifier
    pub component_type: String,
    /// Component data as JSON
    pub data: serde_json::Value,
}

/// Commands that Bevy systems can process
#[derive(Debug, Clone)]
pub enum BevyCommand {
    /// Spawn a new entity with components
    SpawnEntity {
        /// Entity ID (maps to domain aggregate ID)
        entity_id: Uuid,
        /// Components to attach
        components: Vec<ComponentData>,
        /// Optional parent entity
        parent: Option<Uuid>,
    },

    /// Update entity components
    UpdateEntity {
        /// Entity to update
        entity_id: Uuid,
        /// Components to add or update
        components: Vec<ComponentData>,
    },

    /// Remove entity
    DespawnEntity {
        /// Entity to remove
        entity_id: Uuid,
    },

    /// Create a relationship between entities
    CreateRelationship {
        /// Source entity
        source: Uuid,
        /// Target entity
        target: Uuid,
        /// Relationship type
        relationship_type: String,
    },
}

/// Events that Bevy can emit back to NATS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BevyEvent {
    /// Entity was clicked/selected
    EntitySelected {
        /// The ID of the selected entity
        entity_id: Uuid,
        /// World position of click
        position: [f32; 3],
    },

    /// Entity was moved
    EntityMoved {
        /// The ID of the moved entity
        entity_id: Uuid,
        /// Previous position before movement
        old_position: [f32; 3],
        /// New position after movement
        new_position: [f32; 3],
    },

    /// User requested entity creation
    EntityCreationRequested {
        /// Type of entity to create
        entity_type: String,
        /// Position where entity should be created
        position: [f32; 3],
        /// Additional metadata for creation
        metadata: serde_json::Value,
    },
}

/// Maps component types to their Bevy representations
pub struct ComponentMapper {
}

impl ComponentMapper {
    /// Create a new component mapper
    pub fn new() -> Self {
        Self { }
    }



    /// Map an organization to Bevy components
    pub fn map_organization(&self, org: &OrganizationCreated) -> Vec<ComponentData> {
        vec![
            ComponentData {
                component_type: "OrganizationEntity".to_string(),
                data: serde_json::json!({
                    "id": org.organization_id,
                    "org_type": org.org_type,
                }),
            },
            ComponentData {
                component_type: "Name".to_string(),
                data: serde_json::json!({
                    "name": org.name,
                }),
            },
            ComponentData {
                component_type: "Transform".to_string(),
                data: serde_json::json!({
                    "translation": [0.0, 1.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.5, 1.5, 1.5],
                }),
            },
        ]
    }

    /// Map an agent to Bevy components
    pub fn map_agent(&self, agent: &AgentDeployed) -> Vec<ComponentData> {
        vec![
            ComponentData {
                component_type: "AgentEntity".to_string(),
                data: serde_json::json!({
                    "id": agent.agent_id,
                    "agent_type": agent.agent_type,
                    "owner_id": agent.owner_id,
                }),
            },
            ComponentData {
                component_type: "Name".to_string(),
                data: serde_json::json!({
                    "name": agent.metadata.name,
                }),
            },
            ComponentData {
                component_type: "Description".to_string(),
                data: serde_json::json!({
                    "description": agent.metadata.description,
                }),
            },
            ComponentData {
                component_type: "Owner".to_string(),
                data: serde_json::json!({
                    "owner_id": agent.owner_id,
                }),
            },
            ComponentData {
                component_type: "Transform".to_string(),
                data: serde_json::json!({
                    "translation": [1.0, 0.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [0.8, 0.8, 0.8],
                }),
            },
        ]
    }

    /// Map a policy to Bevy components
    pub fn map_policy(&self, policy: &PolicyEnacted) -> Vec<ComponentData> {
        vec![
            ComponentData {
                component_type: "PolicyEntity".to_string(),
                data: serde_json::json!({
                    "id": policy.policy_id,
                    "policy_type": policy.policy_type,
                    "owner_id": policy.owner_id,
                }),
            },
            ComponentData {
                component_type: "Name".to_string(),
                data: serde_json::json!({
                    "name": policy.metadata.name,
                }),
            },
            ComponentData {
                component_type: "Description".to_string(),
                data: serde_json::json!({
                    "description": policy.metadata.description,
                }),
            },
            ComponentData {
                component_type: "PolicyScope".to_string(),
                data: serde_json::to_value(&policy.scope).unwrap_or(serde_json::Value::Null),
            },
            ComponentData {
                component_type: "Transform".to_string(),
                data: serde_json::json!({
                    "translation": [0.0, 2.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.2, 1.2, 1.2],
                }),
            },
        ]
    }
}

/// NATS message for Bevy translation
#[derive(Debug, Clone)]
pub struct NatsMessage {
    /// NATS subject
    pub subject: String,
    /// Message payload
    pub payload: Vec<u8>,
    /// Optional headers
    pub headers: HashMap<String, String>,
}

/// Translation error types
#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    /// Unknown event type
    #[error("Unknown event type: {0}")]
    UnknownEventType(String),

    /// Deserialization failed
    #[error("Failed to deserialize: {0}")]
    DeserializationError(String),

    /// Unsupported command
    #[error("Unsupported command type")]
    UnsupportedCommand,

    /// Unsupported event type
    #[error("Unsupported event type: {0}")]
    UnsupportedEventType(String),
}

impl From<serde_json::Error> for TranslationError {
    fn from(err: serde_json::Error) -> Self {
        TranslationError::DeserializationError(err.to_string())
    }
}

/// NATS to Bevy translator implementation
pub struct NatsToBevyTranslator {
    /// Component mapper
    component_mapper: ComponentMapper,
}

impl NatsToBevyTranslator {
    /// Create a new translator
    pub fn new() -> Self {
        Self {
            component_mapper: ComponentMapper::new(),
        }
    }
}

impl MessageTranslator<NatsMessage, BevyCommand> for NatsToBevyTranslator {
    type Error = TranslationError;

    fn translate(&self, msg: NatsMessage) -> Result<BevyCommand, Self::Error> {
        // Parse subject to understand domain context
        let subject_parts = SubjectParts::new(&msg.subject)
            .map_err(|e| TranslationError::DeserializationError(e.to_string()))?;

        // Deserialize the domain event envelope
        let envelope: DomainEventEnvelope<serde_json::Value> =
            serde_json::from_slice(&msg.payload)
                .map_err(|e| TranslationError::DeserializationError(e.to_string()))?;

        // Route based on subject pattern
        match (subject_parts.context(), subject_parts.event_type()) {
            ("organizations", "created") => {
                let event: OrganizationCreated = serde_json::from_value(envelope.event)?;
                Ok(BevyCommand::SpawnEntity {
                    entity_id: event.organization_id,
                    components: self.component_mapper.map_organization(&event),
                    parent: None,
                })
            }
            ("agents", "deployed") => {
                let event: AgentDeployed = serde_json::from_value(envelope.event)?;
                Ok(BevyCommand::SpawnEntity {
                    entity_id: event.agent_id,
                    components: self.component_mapper.map_agent(&event),
                    parent: Some(event.owner_id),
                })
            }
            ("policies", "enacted") => {
                let event: PolicyEnacted = serde_json::from_value(envelope.event)?;
                Ok(BevyCommand::SpawnEntity {
                    entity_id: event.policy_id,
                    components: self.component_mapper.map_policy(&event),
                    parent: Some(event.owner_id),
                })
            }
            _ => Err(TranslationError::UnsupportedEventType(
                format!("{}.{}", subject_parts.context(), subject_parts.event_type())
            )),
        }
    }

    fn reverse(&self, cmd: BevyCommand) -> Result<NatsMessage, Self::Error> {
        // Translate Bevy commands back to NATS messages
        match cmd {
            BevyCommand::SpawnEntity { entity_id, components, .. } => {
                // Determine entity type from components
                let entity_type = components.iter()
                    .find(|c| c.component_type.ends_with("Entity"))
                    .map(|c| &c.component_type)
                    .ok_or(TranslationError::UnsupportedCommand)?;

                // Generate appropriate subject based on entity type
                let subject = match entity_type.as_str() {
                    "OrganizationEntity" => "organizations.organization.created.v1",
                    "AgentEntity" => "agents.agent.created.v1",
                    "PolicyEntity" => "policies.policy.created.v1",
                    _ => return Err(TranslationError::UnsupportedCommand),
                };

                // Create event payload
                let event = serde_json::json!({
                    "entity_id": entity_id,
                    "components": components,
                });

                Ok(NatsMessage {
                    subject: subject.to_string(),
                    payload: serde_json::to_vec(&event)
                        .map_err(|e| TranslationError::DeserializationError(e.to_string()))?,
                    headers: HashMap::new(),
                })
            }

            _ => Err(TranslationError::UnsupportedCommand),
        }
    }
}

/// Subject-based routing for Bevy events
pub struct BevyEventRouter {
}

impl BevyEventRouter {
    /// Create a new event router
    pub fn new() -> Self {
        Self { }
    }

    /// Route a Bevy event to appropriate NATS subject
    pub fn route_event(&self, event: &BevyEvent) -> String {
        match event {
            BevyEvent::EntitySelected { .. } => "ui.entity.selected.v1".to_string(),
            BevyEvent::EntityMoved { .. } => "ui.entity.moved.v1".to_string(),
            BevyEvent::EntityCreationRequested { entity_type, .. } => {
                format!("ui.{}.creation_requested.v1", entity_type.to_lowercase())
            }
        }
    }
}

impl Default for BevyEventRouter {
    fn default() -> Self {
        Self { }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventMetadata, PropagationScope};

    #[test]
    fn test_bevy_event_routing() {
        let router = BevyEventRouter::new();

        let event = BevyEvent::EntitySelected {
            entity_id: Uuid::new_v4(),
            position: [10.0, 20.0, 0.0],
        };

        let subject = router.route_event(&event);
        assert_eq!(subject, "ui.entity.selected.v1");
    }
}
