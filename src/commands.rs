// Copyright 2025 Cowboy AI, LLC.

//! Domain commands for CIM
//!
//! Commands represent requests to change state. They are processed by command handlers
//! which validate business rules and emit events. Commands return only acknowledgments,
//! not data - use queries for data retrieval.

use crate::Command;

/// A domain command that can be serialized and deserialized
/// This trait extends the base Command trait for cross-domain communication
///
/// # Examples
///
/// ```rust
/// use cim_domain::{DomainCommand, Command, EntityId};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct CreateUserCommand {
///     user_id: String,
///     email: String,
///     name: String,
/// }
///
/// impl Command for CreateUserCommand {
///     type Aggregate = ();
///     
///     fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
///         None
///     }
/// }
///
/// impl DomainCommand for CreateUserCommand {
///     fn command_type(&self) -> &'static str {
///         "CreateUserCommand"
///     }
///     
///     fn aggregate_id(&self) -> String {
///         self.user_id.clone()
///     }
/// }
///
/// let cmd = CreateUserCommand {
///     user_id: "user-123".to_string(),
///     email: "user@example.com".to_string(),
///     name: "John Doe".to_string(),
/// };
///
/// assert_eq!(cmd.command_type(), "CreateUserCommand");
/// assert_eq!(DomainCommand::aggregate_id(&cmd), "user-123");
/// ```
pub trait DomainCommand:
    serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + Sync + 'static
{
    /// Get the command type name
    fn command_type(&self) -> &'static str;

    /// Get the aggregate ID this command targets
    fn aggregate_id(&self) -> String;
}

// All domain-specific commands have been moved to their respective domain submodules:
// - Person commands: cim-domain-person
// - Organization commands: cim-domain-organization
// - Agent commands: cim-domain-agent
// - Workflow commands: cim-domain-workflow
// - Location commands: cim-domain-location
// - Document commands: cim-domain-document
// - Policy commands: cim-domain-policy

/// An acknowledgment command for testing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcknowledgeCommand;

impl Command for AcknowledgeCommand {
    type Aggregate = ();

    fn aggregate_id(&self) -> Option<crate::entity::EntityId<Self::Aggregate>> {
        None
    }
}

impl DomainCommand for AcknowledgeCommand {
    fn command_type(&self) -> &'static str {
        "AcknowledgeCommand"
    }

    fn aggregate_id(&self) -> String {
        "test".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityId;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestCommand {
        id: String,
        value: String,
    }

    impl Command for TestCommand {
        type Aggregate = ();

        fn aggregate_id(&self) -> Option<EntityId<Self::Aggregate>> {
            None
        }
    }

    impl DomainCommand for TestCommand {
        fn command_type(&self) -> &'static str {
            "TestCommand"
        }

        fn aggregate_id(&self) -> String {
            self.id.clone()
        }
    }

    #[test]
    fn test_domain_command_trait() {
        let cmd = TestCommand {
            id: "test-123".to_string(),
            value: "test-value".to_string(),
        };

        assert_eq!(cmd.command_type(), "TestCommand");
        assert_eq!(DomainCommand::aggregate_id(&cmd), "test-123");
    }

    #[test]
    fn test_acknowledge_command() {
        let cmd = AcknowledgeCommand;

        assert_eq!(cmd.command_type(), "AcknowledgeCommand");
        assert_eq!(DomainCommand::aggregate_id(&cmd), "test");
        assert!(Command::aggregate_id(&cmd).is_none());
    }

    #[test]
    fn test_command_serialization() {
        let cmd = TestCommand {
            id: "serialize-test".to_string(),
            value: "serialize-value".to_string(),
        };

        // Test serialization
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("serialize-test"));
        assert!(json.contains("serialize-value"));

        // Test deserialization
        let deserialized: TestCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, cmd.id);
        assert_eq!(deserialized.value, cmd.value);
    }

    #[test]
    fn test_acknowledge_command_serialization() {
        let cmd = AcknowledgeCommand;

        // Test serialization
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, "null"); // Unit struct serializes to null

        // Test deserialization
        let deserialized: AcknowledgeCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.command_type(), cmd.command_type());
    }

    #[test]
    fn test_command_debug_trait() {
        let cmd = TestCommand {
            id: "debug-test".to_string(),
            value: "debug-value".to_string(),
        };

        let debug_str = format!("{cmd:?}");
        assert!(debug_str.contains("TestCommand"));
        assert!(debug_str.contains("debug-test"));
        assert!(debug_str.contains("debug-value"));
    }

    #[test]
    fn test_command_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TestCommand>();
        assert_send_sync::<AcknowledgeCommand>();
    }
}
