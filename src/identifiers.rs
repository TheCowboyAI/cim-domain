//! Identifier types for graphs, nodes, and edges

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use crate::entity::{EntityId, GraphMarker};

/// Node ID - only meaningful within a graph context
///
/// Nodes are not entities - they're local identifiers within a graph.
/// They don't have global identity or lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new random node ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from a UUID
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NodeId> for Uuid {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

impl From<&NodeId> for Uuid {
    fn from(id: &NodeId) -> Self {
        id.0
    }
}

/// Edge ID - only meaningful within a graph context
///
/// Edges are not entities - they're local identifiers within a graph.
/// They represent relationships between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(Uuid);

impl EdgeId {
    /// Create a new random edge ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from a UUID
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EdgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<EdgeId> for Uuid {
    fn from(id: EdgeId) -> Self {
        id.0
    }
}

impl From<&EdgeId> for Uuid {
    fn from(id: &EdgeId) -> Self {
        id.0
    }
}

/// State ID - identifies a state within a workflow
///
/// States are not entities - they're local identifiers within a workflow.
/// They represent possible states in a state machine or workflow.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StateId(String);

impl StateId {
    /// Create from a string
    pub fn from(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for StateId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for StateId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Transition ID - identifies a transition within a workflow
///
/// Transitions are not entities - they're local identifiers within a workflow.
/// They represent allowed state changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransitionId(String);

impl TransitionId {
    /// Create from a string
    pub fn from(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TransitionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TransitionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TransitionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Type alias for graph entity IDs
///
/// Graphs are entities with global identity and lifecycle.
/// This is just a convenience alias for EntityId<GraphMarker>.
pub type GraphId = EntityId<GraphMarker>;

/// Type alias for workflow entity IDs
///
/// Workflows are entities with global identity and lifecycle.
/// This is just a convenience alias for EntityId<WorkflowMarker>.
pub type WorkflowId = EntityId<markers::WorkflowMarker>;

/// Marker types for different entity kinds
pub mod markers {
    /// Marker for Workflow entities
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WorkflowMarker;
}

/// Extension trait for WorkflowId to provide convenient methods
pub trait WorkflowIdExt {
    /// Convert to UUID
    fn to_uuid(&self) -> Uuid;
}

impl WorkflowIdExt for EntityId<markers::WorkflowMarker> {
    fn to_uuid(&self) -> Uuid {
        Uuid::from(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test NodeId creation and uniqueness
    ///
    /// ```mermaid
    /// graph LR
    ///     A[NodeId::new] -->|UUID v4| B[Unique ID]
    ///     C[NodeId::new] -->|UUID v4| D[Different ID]
    ///     B -->|Not Equal| D
    /// ```
    #[test]
    fn test_node_id_new() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();

        // IDs should be unique
        assert_ne!(id1, id2);

        // IDs should not be nil
        assert!(!id1.as_uuid().is_nil());
        assert!(!id2.as_uuid().is_nil());
    }

    /// Test NodeId from UUID
    #[test]
    fn test_node_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = NodeId::from_uuid(uuid);

        assert_eq!(id.as_uuid(), &uuid);
    }

    /// Test NodeId default implementation
    #[test]
    fn test_node_id_default() {
        let id1 = NodeId::default();
        let id2 = NodeId::default();

        // Default should create unique IDs
        assert_ne!(id1, id2);
        assert!(!id1.as_uuid().is_nil());
    }

    /// Test NodeId display formatting
    #[test]
    fn test_node_id_display() {
        let uuid = Uuid::new_v4();
        let id = NodeId::from_uuid(uuid);

        assert_eq!(format!("{}", id), format!("{}", uuid));
    }

    /// Test NodeId serialization/deserialization
    #[test]
    fn test_node_id_serde() {
        let original = NodeId::new();

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: NodeId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    /// Test EdgeId creation and uniqueness
    ///
    /// ```mermaid
    /// graph LR
    ///     A[EdgeId::new] -->|UUID v4| B[Unique ID]
    ///     C[EdgeId::new] -->|UUID v4| D[Different ID]
    ///     B -->|Not Equal| D
    /// ```
    #[test]
    fn test_edge_id_new() {
        let id1 = EdgeId::new();
        let id2 = EdgeId::new();

        // IDs should be unique
        assert_ne!(id1, id2);

        // IDs should not be nil
        assert!(!id1.as_uuid().is_nil());
        assert!(!id2.as_uuid().is_nil());
    }

    /// Test EdgeId from UUID
    #[test]
    fn test_edge_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EdgeId::from_uuid(uuid);

        assert_eq!(id.as_uuid(), &uuid);
    }

    /// Test EdgeId default implementation
    #[test]
    fn test_edge_id_default() {
        let id1 = EdgeId::default();
        let id2 = EdgeId::default();

        // Default should create unique IDs
        assert_ne!(id1, id2);
        assert!(!id1.as_uuid().is_nil());
    }

    /// Test EdgeId display formatting
    #[test]
    fn test_edge_id_display() {
        let uuid = Uuid::new_v4();
        let id = EdgeId::from_uuid(uuid);

        assert_eq!(format!("{}", id), format!("{}", uuid));
    }

    /// Test EdgeId serialization/deserialization
    #[test]
    fn test_edge_id_serde() {
        let original = EdgeId::new();

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: EdgeId = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    /// Test GraphId type alias
    ///
    /// ```mermaid
    /// graph TD
    ///     A[GraphId] -->|Is| B[EntityId<GraphMarker>]
    ///     B -->|Has| C[Global Identity]
    ///     B -->|Has| D[Lifecycle]
    /// ```
    #[test]
    fn test_graph_id() {
        let graph_id: GraphId = GraphId::new();

        // Should behave like EntityId
        assert!(!graph_id.as_uuid().is_nil());

        // Should be displayable
        let display = format!("{}", graph_id);
        assert!(!display.is_empty());
    }

    /// Test ID types are distinct
    #[test]
    fn test_id_types_distinct() {
        let node_id = NodeId::new();
        let edge_id = EdgeId::new();
        let graph_id: GraphId = GraphId::new();

        // All should have unique UUIDs
        assert_ne!(node_id.as_uuid(), edge_id.as_uuid());
        assert_ne!(node_id.as_uuid(), graph_id.as_uuid());
        assert_ne!(edge_id.as_uuid(), graph_id.as_uuid());
    }

    /// Test IDs as hash map keys
    #[test]
    fn test_ids_as_keys() {
        use std::collections::HashMap;

        // NodeId as key
        let mut node_map = HashMap::new();
        let node_id1 = NodeId::new();
        let node_id2 = NodeId::new();
        node_map.insert(node_id1, "node1");
        node_map.insert(node_id2, "node2");
        assert_eq!(node_map.get(&node_id1), Some(&"node1"));
        assert_eq!(node_map.get(&node_id2), Some(&"node2"));

        // EdgeId as key
        let mut edge_map = HashMap::new();
        let edge_id1 = EdgeId::new();
        let edge_id2 = EdgeId::new();
        edge_map.insert(edge_id1, "edge1");
        edge_map.insert(edge_id2, "edge2");
        assert_eq!(edge_map.get(&edge_id1), Some(&"edge1"));
        assert_eq!(edge_map.get(&edge_id2), Some(&"edge2"));

        // GraphId as key
        let mut graph_map = HashMap::new();
        let graph_id1: GraphId = GraphId::new();
        let graph_id2: GraphId = GraphId::new();
        graph_map.insert(graph_id1, "graph1");
        graph_map.insert(graph_id2, "graph2");
        assert_eq!(graph_map.get(&graph_id1), Some(&"graph1"));
        assert_eq!(graph_map.get(&graph_id2), Some(&"graph2"));
    }

    /// Test ID equality and hashing
    #[test]
    fn test_id_equality() {
        let uuid = Uuid::new_v4();

        // NodeId equality
        let node1 = NodeId::from_uuid(uuid);
        let node2 = NodeId::from_uuid(uuid);
        assert_eq!(node1, node2);

        // EdgeId equality
        let edge1 = EdgeId::from_uuid(uuid);
        let edge2 = EdgeId::from_uuid(uuid);
        assert_eq!(edge1, edge2);

        // Different types with same UUID are not comparable
        // (This is enforced at compile time by Rust's type system)
    }
}
