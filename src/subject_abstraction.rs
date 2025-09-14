// Copyright 2025 Cowboy AI, LLC.

//! Abstraction layer for subject functionality
//!
//! This module provides traits and types that abstract over the cim-subject
//! functionality, allowing it to be optional.

use serde::{Deserialize, Serialize};
use schemars::{JsonSchema, schema::Schema};
use std::fmt;
use std::str::FromStr;

/// A trait for subject-like types that can be used for routing
pub trait SubjectLike: fmt::Display + Send + Sync {
    /// Parse a subject from a string
    fn parse(s: &str) -> Result<Self, SubjectError>
    where
        Self: Sized;

    /// Get the parts of the subject
    fn parts(&self) -> Vec<&str>;

    /// Check if this subject matches a pattern
    fn matches_pattern(&self, pattern: &str) -> bool;
}

/// A trait for pattern-like types used for matching subjects
pub trait PatternLike: fmt::Display + Send + Sync {
    /// Parse a pattern from a string
    fn parse(s: &str) -> Result<Self, SubjectError>
    where
        Self: Sized;

    /// Check if a subject matches this pattern
    fn matches_subject(&self, subject: &str) -> bool;
}

/// Error type for subject operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum SubjectError {
    /// Error when subject format is invalid
    #[error("Invalid subject format: {0}")]
    InvalidFormat(String),
    /// Error when pattern parsing or matching fails
    #[error("Pattern error: {0}")]
    PatternError(String),
}

/// A simple subject implementation for when cim-subject is not available
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SimpleSubject {
    parts: Vec<String>,
}

impl SimpleSubject {
    /// Create a new simple subject
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }
}

impl fmt::Display for SimpleSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.parts.join("."))
    }
}

impl SubjectLike for SimpleSubject {
    fn parse(s: &str) -> Result<Self, SubjectError> {
        if s.is_empty() {
            return Err(SubjectError::InvalidFormat("Empty subject".to_string()));
        }

        let parts: Vec<String> = s.split('.').map(|s| s.to_string()).collect();

        // Validate parts
        for part in &parts {
            if part.is_empty() {
                return Err(SubjectError::InvalidFormat(
                    "Empty subject part".to_string(),
                ));
            }
        }

        Ok(Self { parts })
    }

    fn parts(&self) -> Vec<&str> {
        self.parts.iter().map(|s| s.as_str()).collect()
    }

    fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(p) = SimplePattern::parse(pattern) {
            p.matches_subject(&self.to_string())
        } else {
            false
        }
    }
}

/// A simple pattern implementation for when cim-subject is not available
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SimplePattern {
    parts: Vec<String>,
}

impl SimplePattern {
    /// Create a new simple pattern
    pub fn new(parts: Vec<String>) -> Self {
        Self { parts }
    }
}

impl fmt::Display for SimplePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.parts.join("."))
    }
}

impl PatternLike for SimplePattern {
    fn parse(s: &str) -> Result<Self, SubjectError> {
        if s.is_empty() {
            return Err(SubjectError::PatternError("Empty pattern".to_string()));
        }

        let parts: Vec<String> = s.split('.').map(|s| s.to_string()).collect();
        Ok(Self { parts })
    }

    fn matches_subject(&self, subject: &str) -> bool {
        let subject_parts: Vec<&str> = subject.split('.').collect();
        let pattern_parts = &self.parts;

        // Handle ">" wildcard at the end
        if let Some(last) = pattern_parts.last() {
            if last == ">" {
                // Pattern with ">" matches if all parts before ">" match
                let pattern_prefix = &pattern_parts[..pattern_parts.len() - 1];
                if subject_parts.len() < pattern_prefix.len() {
                    return false;
                }

                for (i, pattern_part) in pattern_prefix.iter().enumerate() {
                    if pattern_part != "*" && pattern_part.as_str() != subject_parts[i] {
                        return false;
                    }
                }
                return true;
            }
        }

        // Exact length match required without ">"
        if pattern_parts.len() != subject_parts.len() {
            return false;
        }

        // Check each part
        for (pattern_part, subject_part) in pattern_parts.iter().zip(subject_parts.iter()) {
            if pattern_part != "*" && pattern_part != subject_part {
                return false;
            }
        }

        true
    }
}

/// Type alias for subject type based on feature flags
#[cfg(feature = "subject-routing")]
pub type Subject = cim_subject::Subject;

#[cfg(not(feature = "subject-routing"))]
pub type Subject = SimpleSubject;

/// Type alias for pattern type based on feature flags
#[cfg(feature = "subject-routing")]
pub type Pattern = cim_subject::Pattern;

#[cfg(not(feature = "subject-routing"))]
pub type Pattern = SimplePattern;

// Implement the traits for the real cim-subject types
#[cfg(feature = "subject-routing")]
impl SubjectLike for cim_subject::Subject {
    fn parse(s: &str) -> Result<Self, SubjectError> {
        cim_subject::Subject::new(s).map_err(|e| SubjectError::InvalidFormat(e.to_string()))
    }

    fn parts(&self) -> Vec<&str> {
        // This is a simplification - in real impl would need access to internal parts
        vec![self.as_str()]
    }

    fn matches_pattern(&self, pattern: &str) -> bool {
        if let Ok(p) = cim_subject::Pattern::new(pattern) {
            p.matches(self)
        } else {
            false
        }
    }
}

#[cfg(feature = "subject-routing")]
impl PatternLike for cim_subject::Pattern {
    fn parse(s: &str) -> Result<Self, SubjectError> {
        cim_subject::Pattern::new(s).map_err(|e| SubjectError::PatternError(e.to_string()))
    }

    fn matches_subject(&self, subject: &str) -> bool {
        if let Ok(s) = cim_subject::Subject::new(subject) {
            self.matches(&s)
        } else {
            false
        }
    }
}

// Re-export based on features
#[cfg(feature = "subject-routing")]
pub use cim_subject::{
    CausationId,
    // CQRS correlation types
    CorrelationId,
    IdType,
    MessageFactory,
    MessageIdentity,
    MessageTranslator,
    Permissions as SubjectPermissions,
    // Other types
    SerializableCid,
    SubjectParser,
};

#[cfg(not(feature = "subject-routing"))]
pub use self::mock_types::{
    CausationId, CorrelationId, IdType, MessageFactory, MessageIdentity, MessageTranslator,
    SerializableCid, SubjectParser, SubjectPermissions,
};

#[cfg(not(feature = "subject-routing"))]
mod mock_types {
    use super::*;
    use uuid::Uuid;

    /// Mock permissions type when cim-subject is not available
    #[derive(Debug, Clone, Default)]
    pub struct SubjectPermissions;

    /// Mock subject parser when cim-subject is not available
    #[derive(Debug, Clone, Default)]
    pub struct SubjectParser;

    /// Mock message translator trait when cim-subject is not available
    pub trait MessageTranslator: Send + Sync {
        fn translate(&self, _message: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    }

    /// Mock correlation ID for message tracking
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
    pub struct CorrelationId(pub Uuid);

    impl Default for CorrelationId {
        fn default() -> Self {
            // Use UUID v7 for time-ordered correlation tracking
            Self(Uuid::now_v7())
        }
    }

    impl fmt::Display for CorrelationId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Mock causation ID for message causality tracking
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
    pub struct CausationId(pub Uuid);

    impl Default for CausationId {
        fn default() -> Self {
            // Use UUID v7 for time-ordered causation tracking
            Self(Uuid::now_v7())
        }
    }

    impl fmt::Display for CausationId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Type of ID for message identity
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
    pub enum IdType {
        /// UUID-based ID
        Uuid(Uuid),
        /// CID-based ID
        Cid(SerializableCid),
        /// Correlation ID type
        Correlation,
        /// Causation ID type
        Causation,
    }

    /// Message identity information
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
    pub struct MessageIdentity {
        /// Message ID
        pub message_id: IdType,
        /// Correlation ID for tracking related messages
        pub correlation_id: CorrelationId,
        /// Causation ID for tracking message causality
        pub causation_id: CausationId,
        /// Optional parent causation ID
        pub parent_causation_id: Option<CausationId>,
    }

    impl Default for MessageIdentity {
        fn default() -> Self {
            let id = Uuid::now_v7();
            Self {
                message_id: IdType::Uuid(id),
                correlation_id: CorrelationId::default(),
                causation_id: CausationId::default(),
                parent_causation_id: None,
            }
        }
    }

    /// Factory for creating message identities
    #[derive(Debug, Clone, Default)]
    pub struct MessageFactory;

    impl MessageFactory {
        /// Create a new message factory
        pub fn new() -> Self {
            Self
        }

        /// Create a new message identity
        pub fn create_identity(&self) -> MessageIdentity {
            MessageIdentity::default()
        }

        /// Create a child message identity
        pub fn create_child_identity(&self, parent: &MessageIdentity) -> MessageIdentity {
            let id = Uuid::now_v7();
            MessageIdentity {
                message_id: IdType::Uuid(id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId::default(),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a root command identity
        pub fn create_root_command(command_id: Uuid) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(command_id),
                correlation_id: CorrelationId(Uuid::now_v7()),
                causation_id: CausationId(command_id),
                parent_causation_id: None,
            }
        }

        /// Create a command from another command
        pub fn command_from_command(command_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(command_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(command_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a command from a query
        pub fn command_from_query(command_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(command_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(command_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a query from a command
        pub fn query_from_command(query_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(query_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(query_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a root query
        pub fn create_root_query(query_id: Uuid) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(query_id),
                correlation_id: CorrelationId(Uuid::now_v7()),
                causation_id: CausationId(query_id),
                parent_causation_id: None,
            }
        }

        /// Create a query from a query
        pub fn query_from_query(query_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(query_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(query_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create an event from a command
        pub fn event_from_command(
            event_cid: crate::Cid,
            parent: &MessageIdentity,
        ) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Cid(SerializableCid(event_cid)),
                correlation_id: parent.correlation_id.clone(),
                causation_id: match &parent.message_id {
                    IdType::Uuid(id) => CausationId(*id),
                    _ => CausationId(Uuid::now_v7()),
                },
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a command from an event
        pub fn command_from_event(command_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(command_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(command_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }

        /// Create a query from an event
        pub fn query_from_event(query_id: Uuid, parent: &MessageIdentity) -> MessageIdentity {
            MessageIdentity {
                message_id: IdType::Uuid(query_id),
                correlation_id: parent.correlation_id.clone(),
                causation_id: CausationId(query_id),
                parent_causation_id: Some(parent.causation_id.clone()),
            }
        }
    }

    /// Wrapper for CID that can be serialized
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct SerializableCid(pub crate::Cid);
    
    impl JsonSchema for SerializableCid {
        fn schema_name() -> String {
            "SerializableCid".to_string()
        }
        
        fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> Schema {
            // SerializableCid wraps the external cid::Cid type
            // It serializes as a string (base58btc encoded)
            let mut schema = schemars::schema::SchemaObject::default();
            
            schema.instance_type = Some(schemars::schema::InstanceType::String.into());
            
            schema.metadata = Some(Box::new(schemars::schema::Metadata {
                title: Some("SerializableCid".to_string()),
                description: Some("Serializable CID wrapper".to_string()),
                ..Default::default()
            }));
            
            schema.string = Some(Box::new(schemars::schema::StringValidation {
                pattern: Some("^[a-zA-Z0-9]+$".to_string()),
                min_length: Some(1),
                ..Default::default()
            }));
            
            Schema::Object(schema)
        }
    }

    impl Serialize for SerializableCid {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.0.to_string().serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for SerializableCid {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            let cid = crate::Cid::from_str(&s).map_err(serde::de::Error::custom)?;
            Ok(Self(cid))
        }
    }

    // Conversions removed - CID types are now in cim-domain, not cim_ipld
}
