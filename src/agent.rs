//! Agent aggregate - represents autonomous agents in the system
//!
//! Agents are entities that can perform actions on behalf of users or organizations.
//! They have capabilities, permissions, and can use tools/functions.

use crate::{
    component::{Component, ComponentStorage},
    entity::{AggregateRoot, Entity, EntityId},
    errors::{DomainError, DomainResult},
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

/// Marker type for Agent entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentMarker;

/// Agent aggregate root
#[derive(Debug, Clone)]
pub struct Agent {
    /// Entity base
    entity: Entity<AgentMarker>,

    /// Agent type
    agent_type: AgentType,

    /// Current status
    status: AgentStatus,

    /// Owner (person or organization)
    owner_id: Uuid,

    /// Components attached to this agent
    components: ComponentStorage,

    /// Version for optimistic concurrency
    version: u64,
}

/// Types of agents in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Human-controlled agent
    Human,
    /// AI/ML model agent
    AI,
    /// System/service agent
    System,
    /// External integration agent
    External,
}

impl fmt::Display for AgentType {
    /// Format the agent type for display
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::Human => write!(f, "Human"),
            AgentType::AI => write!(f, "AI"),
            AgentType::System => write!(f, "System"),
            AgentType::External => write!(f, "External"),
        }
    }
}

/// Agent operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is being initialized
    Initializing,
    /// Agent is active and operational
    Active,
    /// Agent is temporarily suspended
    Suspended,
    /// Agent is offline/unavailable
    Offline,
    /// Agent has been decommissioned
    Decommissioned,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        id: Uuid,
        agent_type: AgentType,
        owner_id: Uuid,
    ) -> Self {
        Self {
            entity: Entity::with_id(EntityId::from_uuid(id)),
            agent_type,
            status: AgentStatus::Initializing,
            owner_id,
            components: ComponentStorage::new(),
            version: 0,
        }
    }

    /// Get the agent's ID
    pub fn id(&self) -> Uuid {
        *self.entity.id.as_uuid()
    }

    /// Get the agent type
    pub fn agent_type(&self) -> AgentType {
        self.agent_type
    }

    /// Get the current status
    pub fn status(&self) -> AgentStatus {
        self.status
    }

    /// Get the owner ID
    pub fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    /// Add a component to the agent
    pub fn add_component<C: Component>(&mut self, component: C) -> DomainResult<()> {
        self.components.add(component)?;
        self.entity.touch();
        self.version += 1;
        Ok(())
    }

    /// Get a component by type
    pub fn get_component<C: Component>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Remove a component by type
    pub fn remove_component<C: Component>(&mut self) -> Option<Box<dyn Component>> {
        let result = self.components.remove::<C>();
        if result.is_some() {
            self.entity.touch();
            self.version += 1;
        }
        result
    }

    /// Check if the agent has a specific component
    pub fn has_component<C: Component>(&self) -> bool {
        self.components.has::<C>()
    }

    /// Activate the agent
    pub fn activate(&mut self) -> DomainResult<()> {
        match self.status {
            AgentStatus::Initializing | AgentStatus::Suspended | AgentStatus::Offline => {
                self.status = AgentStatus::Active;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            AgentStatus::Active => Err(DomainError::InvalidStateTransition {
                from: "Active".to_string(),
                to: "Active".to_string(),
            }),
            AgentStatus::Decommissioned => Err(DomainError::InvalidStateTransition {
                from: "Decommissioned".to_string(),
                to: "Active".to_string(),
            }),
        }
    }

    /// Suspend the agent
    pub fn suspend(&mut self, _reason: String) -> DomainResult<()> {
        match self.status {
            AgentStatus::Active => {
                self.status = AgentStatus::Suspended;
                self.entity.touch();
                self.version += 1;
                // Store suspension reason in metadata component if needed
                Ok(())
            }
            AgentStatus::Decommissioned => Err(DomainError::InvalidStateTransition {
                from: "Decommissioned".to_string(),
                to: "Suspended".to_string(),
            }),
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Suspended".to_string(),
            }),
        }
    }

    /// Decommission the agent
    pub fn decommission(&mut self) -> DomainResult<()> {
        if self.status == AgentStatus::Decommissioned {
            return Err(DomainError::InvalidStateTransition {
                from: "Decommissioned".to_string(),
                to: "Decommissioned".to_string(),
            });
        }
        self.status = AgentStatus::Decommissioned;
        self.entity.touch();
        self.version += 1;
        Ok(())
    }

    /// Set agent offline
    pub fn set_offline(&mut self) -> DomainResult<()> {
        match self.status {
            AgentStatus::Active | AgentStatus::Suspended => {
                self.status = AgentStatus::Offline;
                self.entity.touch();
                self.version += 1;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Offline".to_string(),
            }),
        }
    }
}

impl AggregateRoot for Agent {
    type Id = EntityId<AgentMarker>;

    fn id(&self) -> Self::Id {
        self.entity.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version += 1;
        self.entity.touch();
    }
}

// Agent Components

/// Capabilities component - what the agent can do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesComponent {
    /// Set of capability identifiers
    pub capabilities: HashSet<String>,
    /// Capability metadata (descriptions, versions, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CapabilitiesComponent {
    /// Create a new capabilities component with the given capabilities
    pub fn new(capabilities: Vec<String>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
            metadata: HashMap::new(),
        }
    }

    /// Add a new capability
    pub fn add_capability(&mut self, capability: String) {
        self.capabilities.insert(capability);
    }

    /// Remove a capability, returns true if it was present
    pub fn remove_capability(&mut self, capability: &str) -> bool {
        self.capabilities.remove(capability)
    }

    /// Check if the agent has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(capability)
    }
}

impl Component for CapabilitiesComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "CapabilitiesComponent"
    }
}

/// Authentication component - how the agent authenticates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationComponent {
    /// Authentication method
    pub auth_method: AuthMethod,
    /// Credentials or token (encrypted in production)
    pub credentials: serde_json::Value,
    /// Last authentication timestamp
    pub last_authenticated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Authentication methods supported by agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// API key authentication
    ApiKey,
    /// OAuth2 token
    OAuth2,
    /// JWT token
    JWT,
    /// Certificate-based
    Certificate,
    /// Custom authentication
    Custom(String),
}

impl Component for AuthenticationComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "AuthenticationComponent"
    }
}

/// Permissions component - what the agent is allowed to do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsComponent {
    /// Granted permissions
    pub permissions: HashSet<String>,
    /// Denied permissions (explicit denials)
    pub denials: HashSet<String>,
    /// Permission groups/roles
    pub roles: HashSet<String>,
}

impl PermissionsComponent {
    /// Create a new empty permissions component
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
            denials: HashSet::new(),
            roles: HashSet::new(),
        }
    }

    /// Grant a permission to the agent
    pub fn grant_permission(&mut self, permission: String) {
        self.permissions.insert(permission.clone());
        self.denials.remove(&permission);
    }

    /// Explicitly deny a permission
    pub fn deny_permission(&mut self, permission: String) {
        self.denials.insert(permission.clone());
        self.permissions.remove(&permission);
    }

    /// Check if the agent has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        !self.denials.contains(permission) && self.permissions.contains(permission)
    }

    /// Add a role to the agent
    pub fn add_role(&mut self, role: String) {
        self.roles.insert(role);
    }
}

impl Component for PermissionsComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "PermissionsComponent"
    }
}

/// Tool access component - tools/functions the agent can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAccessComponent {
    /// Available tools
    pub tools: HashMap<String, ToolDefinition>,
    /// Usage statistics
    pub usage_stats: HashMap<String, ToolUsageStats>,
}

/// Definition of a tool that an agent can use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name identifier
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// Tool version
    pub version: String,
    /// JSON schema for tool parameters
    pub parameters: serde_json::Value,
    /// Whether the tool is currently enabled for use
    pub enabled: bool,
}

/// Statistics about tool usage by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    /// Total number of times the tool has been invoked
    pub invocation_count: u64,
    /// When the tool was last used
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of successful invocations
    pub success_count: u64,
    /// Number of failed invocations
    pub failure_count: u64,
}

impl Component for ToolAccessComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ToolAccessComponent"
    }
}

/// Configuration component - agent-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationComponent {
    /// Configuration values
    pub config: serde_json::Value,
    /// Configuration version
    pub version: String,
    /// Last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Component for ConfigurationComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ConfigurationComponent"
    }
}

/// Metadata component for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Human-readable name
    pub name: String,
    /// Description of the agent's purpose
    pub description: String,
    /// Tags for categorization
    pub tags: HashSet<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last activity timestamp
    pub last_active: Option<chrono::DateTime<chrono::Utc>>,
}

impl Component for AgentMetadata {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "AgentMetadata"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent() {
        let agent_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let agent = Agent::new(agent_id, AgentType::AI, owner_id);

        assert_eq!(agent.id(), agent_id);
        assert_eq!(agent.agent_type(), AgentType::AI);
        assert_eq!(agent.status(), AgentStatus::Initializing);
        assert_eq!(agent.owner_id(), owner_id);
        assert_eq!(agent.version(), 0);
    }

    #[test]
    fn test_agent_status_transitions() {
        let agent_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut agent = Agent::new(agent_id, AgentType::System, owner_id);

        // Activate from initializing
        assert!(agent.activate().is_ok());
        assert_eq!(agent.status(), AgentStatus::Active);
        assert_eq!(agent.version(), 1);

        // Cannot activate when already active
        assert!(agent.activate().is_err());

        // Suspend active agent
        assert!(agent.suspend("Maintenance".to_string()).is_ok());
        assert_eq!(agent.status(), AgentStatus::Suspended);
        assert_eq!(agent.version(), 2);

        // Reactivate
        assert!(agent.activate().is_ok());
        assert_eq!(agent.status(), AgentStatus::Active);
        assert_eq!(agent.version(), 3);

        // Set offline
        assert!(agent.set_offline().is_ok());
        assert_eq!(agent.status(), AgentStatus::Offline);
        assert_eq!(agent.version(), 4);

        // Decommission
        assert!(agent.decommission().is_ok());
        assert_eq!(agent.status(), AgentStatus::Decommissioned);
        assert_eq!(agent.version(), 5);

        // Cannot activate decommissioned agent
        assert!(agent.activate().is_err());
    }

    #[test]
    fn test_agent_components() {
        let agent_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut agent = Agent::new(agent_id, AgentType::AI, owner_id);

        // Add capabilities
        let capabilities = CapabilitiesComponent::new(vec![
            "text_generation".to_string(),
            "code_analysis".to_string(),
        ]);
        assert!(agent.add_component(capabilities).is_ok());
        assert_eq!(agent.version(), 1);

        // Check component exists
        assert!(agent.has_component::<CapabilitiesComponent>());

        // Get component
        let caps = agent.get_component::<CapabilitiesComponent>().unwrap();
        assert!(caps.has_capability("text_generation"));
        assert!(caps.has_capability("code_analysis"));

        // Add metadata
        let metadata = AgentMetadata {
            name: "Code Assistant".to_string(),
            description: "AI agent for code analysis and generation".to_string(),
            tags: ["ai", "code", "assistant"].iter().map(|s| s.to_string()).collect(),
            created_at: chrono::Utc::now(),
            last_active: None,
        };
        assert!(agent.add_component(metadata).is_ok());
        assert_eq!(agent.version(), 2);

        // Remove component
        let removed = agent.remove_component::<CapabilitiesComponent>();
        assert!(removed.is_some());
        assert!(!agent.has_component::<CapabilitiesComponent>());
        assert_eq!(agent.version(), 3);
    }

    #[test]
    fn test_permissions_component() {
        let mut perms = PermissionsComponent::new();

        // Grant permissions
        perms.grant_permission("read_files".to_string());
        perms.grant_permission("write_files".to_string());

        assert!(perms.has_permission("read_files"));
        assert!(perms.has_permission("write_files"));

        // Deny permission
        perms.deny_permission("write_files".to_string());
        assert!(!perms.has_permission("write_files"));
        assert!(perms.has_permission("read_files"));

        // Add role
        perms.add_role("developer".to_string());
        assert!(perms.roles.contains("developer"));
    }

    #[test]
    fn test_aggregate_root_implementation() {
        let agent_id = Uuid::new_v4();
        let owner_id = Uuid::new_v4();

        let mut agent = Agent::new(agent_id, AgentType::Human, owner_id);

        // Test ID
        let aggregate_id = AggregateRoot::id(&agent);
        assert_eq!(*aggregate_id.as_uuid(), agent_id);

        // Test version
        assert_eq!(agent.version(), 0);

        // Test increment version
        agent.increment_version();
        assert_eq!(agent.version(), 1);

        // Verify entity was touched
        let updated_at = agent.entity.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        agent.increment_version();
        assert!(agent.entity.updated_at > updated_at);
    }
}
