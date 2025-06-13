//! Workflow state definitions and traits
//!
//! States are the objects in our workflow category. They are fully injectable
//! by users and can represent any domain concept.

use crate::identifiers::StateId;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::collections::HashMap;

/// Core trait for workflow states
///
/// States are the objects in our workflow category. They must be:
/// - Cloneable for state transitions
/// - Debuggable for logging
/// - Comparable for state matching
/// - Thread-safe for concurrent execution
pub trait WorkflowState: Clone + Debug + PartialEq + Send + Sync + 'static {
    /// Unique identifier for this state
    fn id(&self) -> StateId;

    /// Whether this is a terminal state (no outgoing transitions allowed)
    fn is_terminal(&self) -> bool {
        false
    }

    /// Human-readable name for this state
    fn name(&self) -> &str;

    /// Optional description of what this state represents
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Context for workflow execution
///
/// Contains runtime data that can influence transition guards and outputs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Key-value pairs of context data
    data: HashMap<String, serde_json::Value>,

    /// Current user or system executing the workflow
    actor: Option<String>,

    /// Correlation ID for distributed tracing
    correlation_id: Option<String>,
}

impl WorkflowContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            actor: None,
            correlation_id: None,
        }
    }

    /// Create a context with an actor
    pub fn with_actor(actor: String) -> Self {
        Self {
            data: HashMap::new(),
            actor: Some(actor),
            correlation_id: None,
        }
    }

    /// Set a value in the context
    pub fn set<T: Serialize>(&mut self, key: &str, value: T) -> Result<(), serde_json::Error> {
        self.data.insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Get a value from the context
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get the actor
    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }

    /// Set the actor
    pub fn set_actor(&mut self, actor: String) {
        self.actor = Some(actor);
    }

    /// Get the correlation ID
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Set the correlation ID
    pub fn set_correlation_id(&mut self, id: String) {
        self.correlation_id = Some(id);
    }

    /// Get the data as a HashMap
    pub fn data(&self) -> &HashMap<String, serde_json::Value> {
        &self.data
    }

    /// Convert to HashMap (for event serialization)
    pub fn into_data(self) -> HashMap<String, serde_json::Value> {
        self.data
    }
}

impl Default for WorkflowContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Example implementation of a simple state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SimpleState {
    /// Unique identifier for this state
    pub id: String,
    /// Human-readable name for this state
    pub name: String,
    /// Whether this is a terminal state (no outgoing transitions allowed)
    pub is_terminal: bool,
}

impl SimpleState {
    /// Create a new non-terminal state with the given name
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: StateId::from(name.clone()).to_string(),
            name,
            is_terminal: false,
        }
    }

    /// Create a new terminal state with the given name
    pub fn terminal(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: StateId::from(name.clone()).to_string(),
            name,
            is_terminal: true,
        }
    }
}

impl WorkflowState for SimpleState {
    fn id(&self) -> StateId {
        StateId::from(self.id.clone())
    }

    fn is_terminal(&self) -> bool {
        self.is_terminal
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_state() {
        let state = SimpleState::new("Draft");
        assert_eq!(state.name(), "Draft");
        assert!(!state.is_terminal());

        let terminal = SimpleState::terminal("Archived");
        assert_eq!(terminal.name(), "Archived");
        assert!(terminal.is_terminal());
    }

    #[test]
    fn test_workflow_context() {
        let mut ctx = WorkflowContext::with_actor("user123".to_string());

        // Set and get values
        ctx.set("document_id", "doc456").unwrap();
        ctx.set("version", 2).unwrap();

        assert_eq!(ctx.get::<String>("document_id"), Some("doc456".to_string()));
        assert_eq!(ctx.get::<i32>("version"), Some(2));
        assert_eq!(ctx.actor(), Some("user123"));
    }
}
