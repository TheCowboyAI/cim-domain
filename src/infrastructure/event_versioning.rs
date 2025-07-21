// Copyright 2025 Cowboy AI, LLC.

//! Event versioning and schema evolution support
//!
//! This module provides infrastructure for handling event schema changes over time,
//! allowing systems to evolve without breaking existing stored events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

// Type alias for transformer function
type EventTransformerFn = Box<
    dyn Fn(&serde_json::Value) -> Result<serde_json::Value, EventVersioningError> + Send + Sync,
>;

/// Errors that can occur during event versioning operations
#[derive(Debug, Error)]
pub enum EventVersioningError {
    /// The event type is not registered with the versioning service
    #[error("Unknown event type: {0}")]
    UnknownEventType(String),

    /// No upcaster is available for the requested version transition
    #[error("No upcaster registered for version {from} to {to}")]
    NoUpcaster {
        /// Source version
        from: u32,
        /// Target version
        to: u32,
    },

    /// The upcasting transformation failed
    #[error("Upcasting failed: {0}")]
    UpcastingFailed(String),

    /// Failed to serialize or deserialize event data
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Trait for transforming events from one version to another
pub trait EventUpcaster: Send + Sync {
    /// Transform event data from an older version to a newer version
    fn upcast(
        &self,
        event_data: &serde_json::Value,
    ) -> Result<serde_json::Value, EventVersioningError>;

    /// Get the source version this upcaster transforms from
    fn source_version(&self) -> u32;

    /// Get the target version this upcaster transforms to
    fn to_version(&self) -> u32;
}

/// Metadata about an event type and its versioning information
#[derive(Clone)]
pub struct EventTypeMetadata {
    /// The name of the event type
    pub event_type: String,
    /// The current version of this event type
    pub current_version: u32,
    /// The schema version (for future use)
    pub schema_version: u32,
}

/// Service for managing event versioning and upcasting
pub struct EventVersioningService {
    event_metadata: HashMap<String, EventTypeMetadata>,
    upcasters: HashMap<String, Vec<Box<dyn EventUpcaster>>>,
}

impl Default for EventVersioningService {
    fn default() -> Self {
        Self::new()
    }
}

impl EventVersioningService {
    /// Create a new event versioning service
    pub fn new() -> Self {
        Self {
            event_metadata: HashMap::new(),
            upcasters: HashMap::new(),
        }
    }

    /// Register an event type with its current version
    pub fn register_event_type(&mut self, event_type: String, current_version: u32) {
        self.event_metadata.insert(
            event_type.clone(),
            EventTypeMetadata {
                event_type,
                current_version,
                schema_version: current_version,
            },
        );
    }

    /// Register an upcaster for transforming events between versions
    pub fn register_upcaster(&mut self, event_type: String, upcaster: Box<dyn EventUpcaster>) {
        self.upcasters.entry(event_type).or_default().push(upcaster);
    }

    /// Upcast an event from an older version to the current version
    pub fn upcast_event(
        &self,
        event_type: &str,
        event_data: serde_json::Value,
        from_version: u32,
    ) -> Result<serde_json::Value, EventVersioningError> {
        let metadata = self
            .event_metadata
            .get(event_type)
            .ok_or_else(|| EventVersioningError::UnknownEventType(event_type.to_string()))?;

        if from_version == metadata.current_version {
            return Ok(event_data);
        }

        let upcasters =
            self.upcasters
                .get(event_type)
                .ok_or(EventVersioningError::NoUpcaster {
                    from: from_version,
                    to: metadata.current_version,
                })?;

        let mut current_data = event_data;
        let mut current_version = from_version;

        while current_version < metadata.current_version {
            let upcaster = upcasters
                .iter()
                .find(|u| u.source_version() == current_version)
                .ok_or(EventVersioningError::NoUpcaster {
                    from: current_version,
                    to: metadata.current_version,
                })?;

            current_data = upcaster.upcast(&current_data)?;
            current_version = upcaster.to_version();
        }

        Ok(current_data)
    }

    /// Get the current version of an event type
    pub fn get_current_version(&self, event_type: &str) -> Option<u32> {
        self.event_metadata
            .get(event_type)
            .map(|m| m.current_version)
    }
}

/// A versioned event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedEvent {
    /// The type name of the event
    pub event_type: String,
    /// The version of the event schema
    pub version: u32,
    /// The event data as JSON
    pub data: serde_json::Value,
    /// Additional metadata about the event
    pub metadata: EventMetadata,
}

/// Metadata associated with an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// When the event occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Correlation ID for tracking related events
    pub correlation_id: Option<String>,
    /// ID of the event that caused this event
    pub causation_id: Option<String>,
    /// ID of the user who triggered the event
    pub user_id: Option<String>,
    /// Source system or service that generated the event
    pub source: Option<String>,
}

/// A simple implementation of EventUpcaster using a closure
pub struct SimpleUpcaster {
    from: u32,
    to: u32,
    transformer: EventTransformerFn,
}

impl SimpleUpcaster {
    /// Create a new simple upcaster with a transformation function
    pub fn new<F>(from: u32, to: u32, transformer: F) -> Self
    where
        F: Fn(&serde_json::Value) -> Result<serde_json::Value, EventVersioningError>
            + Send
            + Sync
            + 'static,
    {
        Self {
            from,
            to,
            transformer: Box::new(transformer),
        }
    }
}

impl EventUpcaster for SimpleUpcaster {
    fn upcast(
        &self,
        event_data: &serde_json::Value,
    ) -> Result<serde_json::Value, EventVersioningError> {
        (self.transformer)(event_data)
    }

    fn source_version(&self) -> u32 {
        self.from
    }

    fn to_version(&self) -> u32 {
        self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_versioning() {
        let mut service = EventVersioningService::new();

        service.register_event_type("UserCreated".to_string(), 2);

        let upcaster_v1_to_v2 = SimpleUpcaster::new(1, 2, |data| {
            let mut new_data = data.clone();
            if let Some(obj) = new_data.as_object_mut() {
                let name_value = obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                if let Some(name) = name_value {
                    let parts: Vec<&str> = name.split_whitespace().collect();
                    obj.insert(
                        "first_name".to_string(),
                        json!(parts.first().unwrap_or(&"")),
                    );
                    obj.insert("last_name".to_string(), json!(parts.get(1).unwrap_or(&"")));
                    obj.remove("name");
                }
            }
            Ok(new_data)
        });

        service.register_upcaster("UserCreated".to_string(), Box::new(upcaster_v1_to_v2));

        let old_event = json!({
            "id": "123",
            "name": "John Doe",
            "email": "john@example.com"
        });

        let result = service.upcast_event("UserCreated", old_event, 1).unwrap();

        assert_eq!(result["first_name"], "John");
        assert_eq!(result["last_name"], "Doe");
        assert_eq!(result["email"], "john@example.com");
        assert!(result.get("name").is_none());
    }

    #[test]
    fn test_event_versioning_service_new() {
        let service = EventVersioningService::new();
        assert!(service.event_metadata.is_empty());
        assert!(service.upcasters.is_empty());
    }

    #[test]
    fn test_register_event_type() {
        let mut service = EventVersioningService::new();

        service.register_event_type("OrderCreated".to_string(), 3);
        service.register_event_type("OrderUpdated".to_string(), 2);

        assert_eq!(service.event_metadata.len(), 2);
        assert_eq!(service.get_current_version("OrderCreated"), Some(3));
        assert_eq!(service.get_current_version("OrderUpdated"), Some(2));
        assert_eq!(service.get_current_version("NonExistent"), None);
    }

    #[test]
    fn test_register_upcaster() {
        let mut service = EventVersioningService::new();

        let upcaster1 = SimpleUpcaster::new(1, 2, |data| Ok(data.clone()));
        let upcaster2 = SimpleUpcaster::new(2, 3, |data| Ok(data.clone()));

        service.register_upcaster("TestEvent".to_string(), Box::new(upcaster1));
        service.register_upcaster("TestEvent".to_string(), Box::new(upcaster2));

        assert_eq!(service.upcasters.len(), 1);
        assert_eq!(service.upcasters.get("TestEvent").unwrap().len(), 2);
    }

    #[test]
    fn test_upcast_event_same_version() {
        let mut service = EventVersioningService::new();
        service.register_event_type("TestEvent".to_string(), 1);

        let event_data = json!({"field": "value"});
        let result = service
            .upcast_event("TestEvent", event_data.clone(), 1)
            .unwrap();

        assert_eq!(result, event_data);
    }

    #[test]
    fn test_upcast_event_unknown_type() {
        let service = EventVersioningService::new();

        let event_data = json!({"field": "value"});
        let result = service.upcast_event("UnknownEvent", event_data, 1);

        assert!(result.is_err());
        match result.unwrap_err() {
            EventVersioningError::UnknownEventType(event_type) => {
                assert_eq!(event_type, "UnknownEvent");
            }
            _ => panic!("Expected UnknownEventType error"),
        }
    }

    #[test]
    fn test_upcast_event_no_upcaster() {
        let mut service = EventVersioningService::new();
        service.register_event_type("TestEvent".to_string(), 3);

        let event_data = json!({"field": "value"});
        let result = service.upcast_event("TestEvent", event_data, 1);

        assert!(result.is_err());
        match result.unwrap_err() {
            EventVersioningError::NoUpcaster { from, to } => {
                assert_eq!(from, 1);
                assert_eq!(to, 3);
            }
            _ => panic!("Expected NoUpcaster error"),
        }
    }

    #[test]
    fn test_upcast_event_missing_intermediate_upcaster() {
        let mut service = EventVersioningService::new();
        service.register_event_type("TestEvent".to_string(), 3);

        // Only register upcaster from v1 to v2, missing v2 to v3
        let upcaster = SimpleUpcaster::new(1, 2, |data| Ok(data.clone()));
        service.register_upcaster("TestEvent".to_string(), Box::new(upcaster));

        let event_data = json!({"field": "value"});
        let result = service.upcast_event("TestEvent", event_data, 1);

        assert!(result.is_err());
        match result.unwrap_err() {
            EventVersioningError::NoUpcaster { from, to } => {
                assert_eq!(from, 2);
                assert_eq!(to, 3);
            }
            _ => panic!("Expected NoUpcaster error"),
        }
    }

    #[test]
    fn test_upcast_event_chain() {
        let mut service = EventVersioningService::new();
        service.register_event_type("TestEvent".to_string(), 3);

        // v1 to v2: add "version" field
        let upcaster_v1_to_v2 = SimpleUpcaster::new(1, 2, |data| {
            let mut new_data = data.clone();
            if let Some(obj) = new_data.as_object_mut() {
                obj.insert("version".to_string(), json!(2));
            }
            Ok(new_data)
        });

        // v2 to v3: add "upgraded" field
        let upcaster_v2_to_v3 = SimpleUpcaster::new(2, 3, |data| {
            let mut new_data = data.clone();
            if let Some(obj) = new_data.as_object_mut() {
                obj.insert("upgraded".to_string(), json!(true));
            }
            Ok(new_data)
        });

        service.register_upcaster("TestEvent".to_string(), Box::new(upcaster_v1_to_v2));
        service.register_upcaster("TestEvent".to_string(), Box::new(upcaster_v2_to_v3));

        let old_event = json!({"id": "123"});
        let result = service.upcast_event("TestEvent", old_event, 1).unwrap();

        assert_eq!(result["id"], "123");
        assert_eq!(result["version"], 2);
        assert_eq!(result["upgraded"], true);
    }

    #[test]
    fn test_upcaster_error_propagation() {
        let mut service = EventVersioningService::new();
        service.register_event_type("TestEvent".to_string(), 2);

        let failing_upcaster = SimpleUpcaster::new(1, 2, |_| {
            Err(EventVersioningError::UpcastingFailed(
                "Test error".to_string(),
            ))
        });

        service.register_upcaster("TestEvent".to_string(), Box::new(failing_upcaster));

        let event_data = json!({"field": "value"});
        let result = service.upcast_event("TestEvent", event_data, 1);

        assert!(result.is_err());
        match result.unwrap_err() {
            EventVersioningError::UpcastingFailed(msg) => {
                assert_eq!(msg, "Test error");
            }
            _ => panic!("Expected UpcastingFailed error"),
        }
    }

    #[test]
    fn test_event_versioning_error_display() {
        let unknown = EventVersioningError::UnknownEventType("TestEvent".to_string());
        assert_eq!(unknown.to_string(), "Unknown event type: TestEvent");

        let no_upcaster = EventVersioningError::NoUpcaster { from: 1, to: 3 };
        assert_eq!(
            no_upcaster.to_string(),
            "No upcaster registered for version 1 to 3"
        );

        let upcasting_failed = EventVersioningError::UpcastingFailed("Parse error".to_string());
        assert_eq!(
            upcasting_failed.to_string(),
            "Upcasting failed: Parse error"
        );

        let serialization = EventVersioningError::SerializationError("Invalid JSON".to_string());
        assert_eq!(
            serialization.to_string(),
            "Serialization error: Invalid JSON"
        );
    }

    #[test]
    fn test_versioned_event_serialization() {
        use chrono::Utc;

        let event = VersionedEvent {
            event_type: "UserCreated".to_string(),
            version: 2,
            data: json!({"id": "123", "name": "Test User"}),
            metadata: EventMetadata {
                timestamp: Utc::now(),
                correlation_id: Some("corr-123".to_string()),
                causation_id: Some("cause-456".to_string()),
                user_id: Some("user-789".to_string()),
                source: Some("api".to_string()),
            },
        };

        // Test serialization
        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: VersionedEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.version, event.version);
        assert_eq!(deserialized.data, event.data);
        assert_eq!(
            deserialized.metadata.correlation_id,
            event.metadata.correlation_id
        );
    }

    #[test]
    fn test_event_metadata_optional_fields() {
        use chrono::Utc;

        let metadata = EventMetadata {
            timestamp: Utc::now(),
            correlation_id: None,
            causation_id: None,
            user_id: None,
            source: None,
        };

        // All optional fields should be None
        assert!(metadata.correlation_id.is_none());
        assert!(metadata.causation_id.is_none());
        assert!(metadata.user_id.is_none());
        assert!(metadata.source.is_none());
    }

    #[test]
    fn test_simple_upcaster_trait_implementation() {
        let upcaster = SimpleUpcaster::new(5, 6, |data| {
            let mut new_data = data.clone();
            if let Some(obj) = new_data.as_object_mut() {
                obj.insert("test".to_string(), json!("added"));
            }
            Ok(new_data)
        });

        assert_eq!(upcaster.source_version(), 5);
        assert_eq!(upcaster.to_version(), 6);

        let input = json!({"existing": "value"});
        let output = upcaster.upcast(&input).unwrap();

        assert_eq!(output["existing"], "value");
        assert_eq!(output["test"], "added");
    }

    #[test]
    fn test_event_type_metadata_clone() {
        let metadata = EventTypeMetadata {
            event_type: "TestEvent".to_string(),
            current_version: 3,
            schema_version: 2,
        };

        let cloned = metadata.clone();
        assert_eq!(cloned.event_type, metadata.event_type);
        assert_eq!(cloned.current_version, metadata.current_version);
        assert_eq!(cloned.schema_version, metadata.schema_version);
    }
}
