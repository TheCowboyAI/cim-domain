//! Event versioning and schema evolution support
//!
//! This module provides infrastructure for handling event schema changes over time,
//! allowing systems to evolve without breaking existing stored events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

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
        to: u32 
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
    fn upcast(&self, event_data: &serde_json::Value) -> Result<serde_json::Value, EventVersioningError>;
    
    /// Get the source version this upcaster transforms from
    fn from_version(&self) -> u32;
    
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
        self.upcasters
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(upcaster);
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

        let upcasters = self
            .upcasters
            .get(event_type)
            .ok_or_else(|| EventVersioningError::NoUpcaster {
                from: from_version,
                to: metadata.current_version,
            })?;

        let mut current_data = event_data;
        let mut current_version = from_version;

        while current_version < metadata.current_version {
            let upcaster = upcasters
                .iter()
                .find(|u| u.from_version() == current_version)
                .ok_or_else(|| EventVersioningError::NoUpcaster {
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
        self.event_metadata.get(event_type).map(|m| m.current_version)
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
    transformer: Box<dyn Fn(&serde_json::Value) -> Result<serde_json::Value, EventVersioningError> + Send + Sync>,
}

impl SimpleUpcaster {
    /// Create a new simple upcaster with a transformation function
    pub fn new<F>(from: u32, to: u32, transformer: F) -> Self
    where
        F: Fn(&serde_json::Value) -> Result<serde_json::Value, EventVersioningError> + Send + Sync + 'static,
    {
        Self {
            from,
            to,
            transformer: Box::new(transformer),
        }
    }
}

impl EventUpcaster for SimpleUpcaster {
    fn upcast(&self, event_data: &serde_json::Value) -> Result<serde_json::Value, EventVersioningError> {
        (self.transformer)(event_data)
    }

    fn from_version(&self) -> u32 {
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
                let name_value = obj.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                if let Some(name) = name_value {
                    let parts: Vec<&str> = name.split_whitespace().collect();
                    obj.insert("first_name".to_string(), json!(parts.get(0).unwrap_or(&"")));
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
}