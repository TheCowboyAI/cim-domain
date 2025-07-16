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
pub trait DomainCommand: serde::Serialize + for<'de> serde::Deserialize<'de> + std::fmt::Debug + Send + Sync + 'static {
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

