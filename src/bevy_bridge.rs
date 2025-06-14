//! Bridge between NATS domain events and Bevy ECS components
//!
//! This module provides translation between domain events (NATS messages)
//! and Bevy ECS commands/components. It handles the impedance mismatch
//! between event-driven domain logic and ECS component updates.

use crate::{
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
}

impl NatsToBevyTranslator {
    /// Create a new translator
    pub fn new() -> Self {
        Self {
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
        let _envelope: DomainEventEnvelope<serde_json::Value> =
            serde_json::from_slice(&msg.payload)
                .map_err(|e| TranslationError::DeserializationError(e.to_string()))?;

        // Route based on subject pattern
        match (subject_parts.context(), subject_parts.event_type()) {
            _ => Err(TranslationError::UnsupportedEventType(
                format!("{}.{}", subject_parts.context(), subject_parts.event_type())
            )),
        }
    }

    fn reverse(&self, _cmd: BevyCommand) -> Result<NatsMessage, Self::Error> {
        // Translate Bevy commands back to NATS messages
        // Currently no supported reverse translations
        Err(TranslationError::UnsupportedCommand)
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
