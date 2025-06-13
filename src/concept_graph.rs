//! ConceptGraph aggregate and related components
//!
//! A ConceptGraph represents a semantic network of domain concepts and their relationships,
//! assembled from various domain objects to provide a unified view of the conceptual space.

use crate::{
    AggregateRoot, DomainResult, Component, ComponentStorage,
    NodeId, EdgeId, GraphId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use std::any::Any;

/// ConceptGraph aggregate - represents a semantic network of domain concepts
#[derive(Debug, Clone)]
pub struct ConceptGraph {
    /// Graph ID
    id: GraphId,

    /// Version for optimistic concurrency control
    version: u64,

    /// Components attached to this concept graph
    components: ComponentStorage,

    /// Nodes in the graph
    nodes: HashMap<NodeId, ConceptNodeComponent>,

    /// Relationships in the graph
    relationships: HashMap<EdgeId, ConceptRelationshipComponent>,

    /// Last update timestamp
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// Marker type for ConceptGraph entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConceptGraphMarker;

impl ConceptGraph {
    /// Create a new ConceptGraph
    pub fn new(id: GraphId) -> Self {
        Self {
            id,
            version: 0,
            components: ComponentStorage::new(),
            nodes: HashMap::new(),
            relationships: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    /// Get the graph's ID
    pub fn id(&self) -> &GraphId {
        &self.id
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Add a component to the graph
    pub fn add_component<C: Component + 'static>(&mut self, component: C) -> DomainResult<()> {
        self.components.add(component)?;
        self.last_updated = chrono::Utc::now();
        self.version += 1;
        Ok(())
    }

    /// Get a component from the graph
    pub fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    /// Remove a component from the graph
    pub fn remove_component<C: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        let result = self.components.remove::<C>();
        if result.is_some() {
            self.last_updated = chrono::Utc::now();
            self.version += 1;
        }
        result
    }

    /// Check if the graph has a specific component
    pub fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.has::<C>()
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: ConceptNodeComponent) -> DomainResult<()> {
        let node_id = NodeId::new();
        self.nodes.insert(node_id, node);
        self.last_updated = chrono::Utc::now();
        self.version += 1;
        Ok(())
    }

    /// Add a relationship to the graph
    pub fn add_relationship(&mut self, relationship: ConceptRelationshipComponent) -> DomainResult<()> {
        let edge_id = EdgeId::new();
        self.relationships.insert(edge_id, relationship);
        self.last_updated = chrono::Utc::now();
        self.version += 1;
        Ok(())
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<NodeId, ConceptNodeComponent> {
        &self.nodes
    }

    /// Get all relationships
    pub fn relationships(&self) -> &HashMap<EdgeId, ConceptRelationshipComponent> {
        &self.relationships
    }
}

impl AggregateRoot for ConceptGraph {
    type Id = GraphId;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn increment_version(&mut self) {
        self.version += 1;
        self.last_updated = chrono::Utc::now();
    }
}

// Components

/// Graph metadata component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphMetadataComponent {
    /// Name of the concept graph
    pub name: String,
    /// Description of what this graph represents
    pub description: Option<String>,
    /// Purpose of the graph (e.g., "domain_overview", "workflow_visualization")
    pub purpose: GraphPurpose,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Created by
    pub created_by: Uuid,
    /// Created at
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified at
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

impl Component for GraphMetadataComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "GraphMetadataComponent"
    }
}

/// Purpose of the concept graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GraphPurpose {
    /// Overview of domain concepts
    DomainOverview,
    /// Workflow visualization
    WorkflowVisualization,
    /// Knowledge representation
    KnowledgeRepresentation,
    /// Event flow diagram
    EventFlowDiagram,
    /// Aggregate relationships
    AggregateRelationships,
    /// Custom purpose
    Custom(String),
}

/// Concept node component - represents a domain concept in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptNodeComponent {
    /// Type of domain object this represents
    pub concept_type: ConceptType,
    /// Reference to the source domain object
    pub source_reference: SourceReference,
    /// Display label for the node
    pub label: String,
    /// Additional properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl Component for ConceptNodeComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ConceptNodeComponent"
    }
}

/// Type of concept in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConceptType {
    /// Aggregate root
    Aggregate(String),
    /// Entity
    Entity(String),
    /// Value object
    ValueObject(String),
    /// Domain event
    Event(String),
    /// Command
    Command(String),
    /// Policy
    Policy(String),
    /// Process/Workflow
    Process(String),
    /// Actor/Agent
    Actor(String),
    /// Custom concept
    Custom(String),
}

/// Reference to the source domain object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceReference {
    /// Type of the source (e.g., "Person", "Organization")
    pub source_type: String,
    /// ID of the source object
    pub source_id: Uuid,
    /// Optional context (e.g., bounded context name)
    pub context: Option<String>,
}

/// Concept relationship component - represents relationships between concepts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptRelationshipComponent {
    /// Source concept node ID
    pub source_node_id: NodeId,
    /// Target concept node ID
    pub target_node_id: NodeId,
    /// Type of relationship
    pub relationship_type: ConceptRelationshipType,
    /// Strength or weight of the relationship (0.0 to 1.0)
    pub strength: f32,
    /// Additional properties
    pub properties: HashMap<String, serde_json::Value>,
}

impl Component for ConceptRelationshipComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ConceptRelationshipComponent"
    }
}

/// Type of relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConceptRelationshipType {
    /// Containment (e.g., Aggregate contains Entity)
    Contains,
    /// Reference (e.g., Entity references another)
    References,
    /// Dependency
    DependsOn,
    /// Inheritance/Specialization
    IsA,
    /// Association
    AssociatedWith,
    /// Temporal (e.g., happens before/after)
    Temporal(TemporalRelation),
    /// Causal (e.g., triggers, causes)
    Causal(CausalRelation),
    /// Semantic similarity
    Similar(f32),
    /// Custom relationship
    Custom(String),
}

/// Temporal relationship types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemporalRelation {
    /// Occurs before another event
    Before,
    /// Occurs after another event
    After,
    /// Occurs during another event
    During,
    /// Overlaps with another event
    Overlaps,
    /// Meets another event (ends when other begins)
    Meets,
    /// Starts at the same time as another event
    Starts,
    /// Finishes at the same time as another event
    Finishes,
}

/// Causal relationship types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CausalRelation {
    /// Directly triggers another event
    Triggers,
    /// Causes another event to occur
    Causes,
    /// Enables another event to occur
    Enables,
    /// Prevents another event from occurring
    Prevents,
    /// Is required for another event to occur
    Requires,
}

/// Conceptual space mapping component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptualSpaceMappingComponent {
    /// ID of the conceptual space
    pub space_id: Uuid,
    /// Dimensions of the space
    pub dimensions: Vec<ConceptualDimension>,
    /// Node positions in the conceptual space
    pub node_positions: HashMap<NodeId, ConceptualPosition>,
}

impl Component for ConceptualSpaceMappingComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ConceptualSpaceMappingComponent"
    }
}

/// Dimension in conceptual space
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptualDimension {
    /// Name of the dimension
    pub name: String,
    /// Type of dimension
    pub dimension_type: DimensionType,
    /// Range of values
    pub range: (f32, f32),
}

/// Type of conceptual dimension
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DimensionType {
    /// Continuous numeric values
    Continuous,
    /// Discrete categories
    Categorical,
    /// Ordered categories
    Ordinal,
    /// Circular values (e.g., angles, time of day)
    Circular,
}

/// Position in conceptual space
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptualPosition {
    /// Coordinates in each dimension
    pub coordinates: Vec<f32>,
    /// Confidence in this position (0.0 to 1.0)
    pub confidence: f32,
}

/// Layout configuration component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutConfigComponent {
    /// Layout algorithm to use
    pub algorithm: LayoutAlgorithm,
    /// Layout parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Fixed node positions (if any)
    pub fixed_positions: HashMap<NodeId, (f32, f32, f32)>,
}

impl Component for LayoutConfigComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "LayoutConfigComponent"
    }
}

/// Layout algorithm types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LayoutAlgorithm {
    /// Force-directed layout
    ForceDirected,
    /// Hierarchical layout
    Hierarchical,
    /// Circular layout
    Circular,
    /// Grid layout
    Grid,
    /// Conceptual space mapping
    ConceptualSpace,
    /// Custom algorithm
    Custom(String),
}

/// Assembly rules component - defines how to assemble concepts from domain objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssemblyRulesComponent {
    /// Rules for including domain objects
    pub inclusion_rules: Vec<InclusionRule>,
    /// Rules for creating relationships
    pub relationship_rules: Vec<RelationshipRule>,
    /// Filtering criteria
    pub filters: Vec<FilterCriteria>,
}

impl Component for AssemblyRulesComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "AssemblyRulesComponent"
    }
}

/// Rule for including domain objects in the graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InclusionRule {
    /// Type of domain object to include
    pub object_type: String,
    /// Conditions for inclusion
    pub conditions: Vec<Condition>,
    /// How to map to concept node
    pub mapping: ConceptMapping,
}

/// Condition for inclusion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Condition {
    /// Field to check
    pub field: String,
    /// Operator
    pub operator: ConditionOperator,
    /// Value to compare
    pub value: serde_json::Value,
}

/// Condition operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionOperator {
    /// Equal to value
    Equals,
    /// Not equal to value
    NotEquals,
    /// Greater than value
    GreaterThan,
    /// Less than value
    LessThan,
    /// Contains substring or element
    Contains,
    /// Value is in a list
    In,
    /// Matches a pattern
    Matches,
}

/// Mapping from domain object to concept node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptMapping {
    /// How to derive the label
    pub label_field: String,
    /// Additional fields to include as properties
    pub property_fields: Vec<String>,
    /// Concept type mapping
    pub concept_type: String,
}

/// Rule for creating relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelationshipRule {
    /// Source object type
    pub source_type: String,
    /// Target object type
    pub target_type: String,
    /// How to determine if relationship exists
    pub detection: RelationshipDetection,
    /// Type of relationship to create
    pub relationship_type: String,
}

/// How to detect relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipDetection {
    /// Direct reference (field contains ID)
    DirectReference {
        /// Name of the field containing the reference ID
        field: String
    },
    /// Shared property
    SharedProperty {
        /// Name of the property that must match between entities
        property: String
    },
    /// Event correlation
    EventCorrelation {
        /// Type of event that correlates the entities
        event_type: String
    },
    /// Custom logic
    Custom(String),
}

/// Filter criteria
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterCriteria {
    /// What to filter
    pub target: FilterTarget,
    /// Filter condition
    pub condition: Condition,
}

/// What can be filtered
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilterTarget {
    /// Filter applies to nodes
    Node,
    /// Filter applies to relationships
    Relationship,
    /// Filter applies to both nodes and relationships
    Both,
}

// View projections

/// Public view of a concept graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptGraphView {
    /// Unique identifier of the graph
    pub graph_id: GraphId,
    /// Graph metadata including name, purpose, and tags
    pub metadata: GraphMetadataComponent,
    /// All nodes in the graph
    pub nodes: Vec<ConceptNodeView>,
    /// All relationships between nodes
    pub relationships: Vec<ConceptRelationshipView>,
    /// Optional layout information for visualization
    pub layout: Option<LayoutInfo>,
}

/// View of a concept node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptNodeView {
    /// Unique identifier of the node
    pub node_id: NodeId,
    /// Type of concept this node represents (e.g., "Aggregate", "Entity")
    pub concept_type: String,
    /// Display label for the node
    pub label: String,
    /// Additional properties as key-value pairs
    pub properties: HashMap<String, serde_json::Value>,
    /// Optional 3D position for visualization
    pub position: Option<(f32, f32, f32)>,
}

/// View of a concept relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelationshipView {
    /// Unique identifier of the edge
    pub edge_id: EdgeId,
    /// ID of the source node
    pub source_node_id: NodeId,
    /// ID of the target node
    pub target_node_id: NodeId,
    /// Type of relationship (e.g., "Contains", "DependsOn")
    pub relationship_type: String,
    /// Strength or weight of the relationship (0.0 to 1.0)
    pub strength: f32,
    /// Additional properties as key-value pairs
    pub properties: HashMap<String, serde_json::Value>,
}

/// Layout information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutInfo {
    /// Name of the layout algorithm used
    pub algorithm: String,
    /// Bounding box containing all nodes
    pub bounds: BoundingBox,
    /// Center point of the graph layout
    pub center: (f32, f32, f32),
}

/// Bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Minimum coordinates (x, y, z)
    pub min: (f32, f32, f32),
    /// Maximum coordinates (x, y, z)
    pub max: (f32, f32, f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_graph_creation() {
        let id = GraphId::new();
        let graph = ConceptGraph::new(id);

        assert_eq!(graph.id(), &id);
        assert_eq!(graph.version(), 0);
    }

    #[test]
    fn test_graph_metadata_component() {
        let mut graph = ConceptGraph::new(GraphId::new());

        let metadata = GraphMetadataComponent {
            name: "Domain Overview".to_string(),
            description: Some("High-level view of domain concepts".to_string()),
            purpose: GraphPurpose::DomainOverview,
            tags: vec!["domain".to_string(), "overview".to_string()],
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };

        graph.add_component(metadata.clone()).unwrap();

        let retrieved = graph.get_component::<GraphMetadataComponent>().unwrap();
        assert_eq!(retrieved.name, "Domain Overview");
        assert_eq!(retrieved.purpose, GraphPurpose::DomainOverview);
    }

    #[test]
    fn test_concept_node_component() {
        let mut graph = ConceptGraph::new(GraphId::new());

        let node = ConceptNodeComponent {
            concept_type: ConceptType::Aggregate("Person".to_string()),
            source_reference: SourceReference {
                source_type: "Person".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("PersonnelContext".to_string()),
            },
            label: "Person Aggregate".to_string(),
            properties: HashMap::new(),
        };

        graph.add_node(node.clone()).unwrap();
        assert_eq!(graph.nodes().len(), 1);
    }

    #[test]
    fn test_concept_relationship_component() {
        let mut graph = ConceptGraph::new(GraphId::new());

        let relationship = ConceptRelationshipComponent {
            source_node_id: NodeId::new(),
            target_node_id: NodeId::new(),
            relationship_type: ConceptRelationshipType::Contains,
            strength: 1.0,
            properties: HashMap::new(),
        };

        graph.add_relationship(relationship.clone()).unwrap();
        assert_eq!(graph.relationships().len(), 1);
    }

    #[test]
    fn test_assembly_rules() {
        let mut graph = ConceptGraph::new(GraphId::new());

        let rules = AssemblyRulesComponent {
            inclusion_rules: vec![
                InclusionRule {
                    object_type: "Person".to_string(),
                    conditions: vec![],
                    mapping: ConceptMapping {
                        label_field: "name".to_string(),
                        property_fields: vec!["role".to_string()],
                        concept_type: "Actor".to_string(),
                    },
                },
            ],
            relationship_rules: vec![
                RelationshipRule {
                    source_type: "Person".to_string(),
                    target_type: "Organization".to_string(),
                    detection: RelationshipDetection::DirectReference {
                        field: "organization_id".to_string(),
                    },
                    relationship_type: "MemberOf".to_string(),
                },
            ],
            filters: vec![],
        };

        graph.add_component(rules).unwrap();
        assert!(graph.has_component::<AssemblyRulesComponent>());
    }

    #[test]
    fn test_conceptual_space_mapping() {
        let mut graph = ConceptGraph::new(GraphId::new());

        let mapping = ConceptualSpaceMappingComponent {
            space_id: Uuid::new_v4(),
            dimensions: vec![
                ConceptualDimension {
                    name: "Complexity".to_string(),
                    dimension_type: DimensionType::Continuous,
                    range: (0.0, 1.0),
                },
                ConceptualDimension {
                    name: "Abstraction".to_string(),
                    dimension_type: DimensionType::Ordinal,
                    range: (0.0, 5.0),
                },
            ],
            node_positions: HashMap::new(),
        };

        graph.add_component(mapping).unwrap();

        let retrieved = graph.get_component::<ConceptualSpaceMappingComponent>().unwrap();
        assert_eq!(retrieved.dimensions.len(), 2);
    }

    /// Test creating a concept graph for the KECO Capital domain model
    ///
    /// ```mermaid
    /// graph TD
    ///     subgraph "Core Domain"
    ///         LP[Loan Processing]
    ///         UW[Underwriting]
    ///         DM[Document Management]
    ///     end
    ///     subgraph "Supporting Domains"
    ///         BM[Borrower Management]
    ///         UM[User Management]
    ///         NS[Notification Service]
    ///     end
    ///     LP --> UW
    ///     LP --> DM
    ///     UW --> DM
    ///     BM --> LP
    /// ```
    #[test]
    fn test_keco_domain_overview_graph() {
        let mut graph = ConceptGraph::new(GraphId::new());

        // Add metadata
        let metadata = GraphMetadataComponent {
            name: "KECO Capital Domain Overview".to_string(),
            description: Some("High-level view of KECO Capital lending platform domains".to_string()),
            purpose: GraphPurpose::DomainOverview,
            tags: vec!["keco".to_string(), "lending".to_string(), "domain".to_string()],
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        graph.add_component(metadata).unwrap();

        // Core domain nodes
        let loan_processing = ConceptNodeComponent {
            concept_type: ConceptType::Process("LoanProcessing".to_string()),
            source_reference: SourceReference {
                source_type: "Domain".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("CoreDomain".to_string()),
            },
            label: "Loan Processing".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("domain_type".to_string(), serde_json::json!("core"));
                props.insert("description".to_string(), serde_json::json!("Handles loan application workflow"));
                props
            },
        };
        graph.add_node(loan_processing).unwrap();

        let underwriting = ConceptNodeComponent {
            concept_type: ConceptType::Process("Underwriting".to_string()),
            source_reference: SourceReference {
                source_type: "Domain".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("CoreDomain".to_string()),
            },
            label: "Underwriting".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("domain_type".to_string(), serde_json::json!("core"));
                props.insert("description".to_string(), serde_json::json!("Risk assessment and approval"));
                props
            },
        };
        graph.add_node(underwriting).unwrap();

        let document_mgmt = ConceptNodeComponent {
            concept_type: ConceptType::Process("DocumentManagement".to_string()),
            source_reference: SourceReference {
                source_type: "Domain".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("CoreDomain".to_string()),
            },
            label: "Document Management".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("domain_type".to_string(), serde_json::json!("core"));
                props.insert("description".to_string(), serde_json::json!("Document collection and verification"));
                props
            },
        };
        graph.add_node(document_mgmt).unwrap();

        // Supporting domain nodes
        let borrower_mgmt = ConceptNodeComponent {
            concept_type: ConceptType::Process("BorrowerManagement".to_string()),
            source_reference: SourceReference {
                source_type: "Domain".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("SupportingDomain".to_string()),
            },
            label: "Borrower Management".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("domain_type".to_string(), serde_json::json!("supporting"));
                props.insert("description".to_string(), serde_json::json!("Borrower information management"));
                props
            },
        };
        graph.add_node(borrower_mgmt).unwrap();

        // Verify nodes were added
        assert_eq!(graph.nodes().len(), 4);

        // Add relationships
        let node_ids: Vec<NodeId> = graph.nodes().keys().cloned().collect();

        // Loan Processing -> Underwriting
        let rel1 = ConceptRelationshipComponent {
            source_node_id: node_ids[0],
            target_node_id: node_ids[1],
            relationship_type: ConceptRelationshipType::DependsOn,
            strength: 0.9,
            properties: {
                let mut props = HashMap::new();
                props.insert("interaction".to_string(), serde_json::json!("submits_to"));
                props
            },
        };
        graph.add_relationship(rel1).unwrap();

        // Loan Processing -> Document Management
        let rel2 = ConceptRelationshipComponent {
            source_node_id: node_ids[0],
            target_node_id: node_ids[2],
            relationship_type: ConceptRelationshipType::DependsOn,
            strength: 0.8,
            properties: {
                let mut props = HashMap::new();
                props.insert("interaction".to_string(), serde_json::json!("uses"));
                props
            },
        };
        graph.add_relationship(rel2).unwrap();

        // Verify relationships
        assert_eq!(graph.relationships().len(), 2);
    }

    /// Test creating a bounded context graph for KECO
    ///
    /// ```mermaid
    /// graph TB
    ///     subgraph "Loan Origination Context"
    ///         B[Borrower]
    ///         DF[Deal File]
    ///         LA[Loan Application]
    ///     end
    ///     subgraph "Underwriting Context"
    ///         LE[Loan Evaluation]
    ///         RA[Risk Assessment]
    ///         AD[Approval Decision]
    ///     end
    ///     B --> DF
    ///     DF --> LA
    ///     LA -.-> LE
    /// ```
    #[test]
    fn test_keco_bounded_contexts_graph() {
        let mut graph = ConceptGraph::new(GraphId::new());

        // Add metadata
        let metadata = GraphMetadataComponent {
            name: "KECO Bounded Contexts".to_string(),
            description: Some("Bounded contexts and their relationships".to_string()),
            purpose: GraphPurpose::AggregateRelationships,
            tags: vec!["keco".to_string(), "bounded-context".to_string(), "ddd".to_string()],
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        graph.add_component(metadata).unwrap();

        // Loan Origination Context entities
        let borrower = ConceptNodeComponent {
            concept_type: ConceptType::Entity("Borrower".to_string()),
            source_reference: SourceReference {
                source_type: "Entity".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("LoanOriginationContext".to_string()),
            },
            label: "Borrower".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("context".to_string(), serde_json::json!("LoanOrigination"));
                props.insert("aggregate_root".to_string(), serde_json::json!(true));
                props
            },
        };
        graph.add_node(borrower).unwrap();

        let deal_file = ConceptNodeComponent {
            concept_type: ConceptType::Entity("DealFile".to_string()),
            source_reference: SourceReference {
                source_type: "Entity".to_string(),
                source_id: Uuid::new_v4(),
                context: Some("LoanOriginationContext".to_string()),
            },
            label: "Deal File".to_string(),
            properties: {
                let mut props = HashMap::new();
                props.insert("context".to_string(), serde_json::json!("LoanOrigination"));
                props.insert("aggregate_root".to_string(), serde_json::json!(true));
                props
            },
        };
        graph.add_node(deal_file).unwrap();

        // Add conceptual space mapping
        let mut node_positions = HashMap::new();
        let node_ids: Vec<NodeId> = graph.nodes().keys().cloned().collect();

        // Position nodes in conceptual space
        node_positions.insert(node_ids[0], ConceptualPosition {
            coordinates: vec![0.2, 0.8], // Low complexity, high abstraction
            confidence: 0.9,
        });
        node_positions.insert(node_ids[1], ConceptualPosition {
            coordinates: vec![0.5, 0.6], // Medium complexity, medium-high abstraction
            confidence: 0.85,
        });

        let conceptual_mapping = ConceptualSpaceMappingComponent {
            space_id: Uuid::new_v4(),
            dimensions: vec![
                ConceptualDimension {
                    name: "Business Complexity".to_string(),
                    dimension_type: DimensionType::Continuous,
                    range: (0.0, 1.0),
                },
                ConceptualDimension {
                    name: "Domain Abstraction".to_string(),
                    dimension_type: DimensionType::Continuous,
                    range: (0.0, 1.0),
                },
            ],
            node_positions,
        };
        graph.add_component(conceptual_mapping).unwrap();

        // Verify conceptual space mapping
        let mapping = graph.get_component::<ConceptualSpaceMappingComponent>().unwrap();
        assert_eq!(mapping.dimensions.len(), 2);
        assert_eq!(mapping.node_positions.len(), 2);
    }

    /// Test assembly rules for KECO domain
    #[test]
    fn test_keco_assembly_rules() {
        let mut graph = ConceptGraph::new(GraphId::new());

        // Define assembly rules for KECO domain
        let rules = AssemblyRulesComponent {
            inclusion_rules: vec![
                InclusionRule {
                    object_type: "Borrower".to_string(),
                    conditions: vec![
                        Condition {
                            field: "status".to_string(),
                            operator: ConditionOperator::NotEquals,
                            value: serde_json::json!("archived"),
                        },
                    ],
                    mapping: ConceptMapping {
                        label_field: "name".to_string(),
                        property_fields: vec!["type".to_string(), "status".to_string()],
                        concept_type: "Entity".to_string(),
                    },
                },
                InclusionRule {
                    object_type: "DealFile".to_string(),
                    conditions: vec![],
                    mapping: ConceptMapping {
                        label_field: "deal_id".to_string(),
                        property_fields: vec!["status".to_string(), "loan_amount".to_string()],
                        concept_type: "Entity".to_string(),
                    },
                },
                InclusionRule {
                    object_type: "Document".to_string(),
                    conditions: vec![
                        Condition {
                            field: "required".to_string(),
                            operator: ConditionOperator::Equals,
                            value: serde_json::json!(true),
                        },
                    ],
                    mapping: ConceptMapping {
                        label_field: "document_type".to_string(),
                        property_fields: vec!["status".to_string(), "category".to_string()],
                        concept_type: "ValueObject".to_string(),
                    },
                },
            ],
            relationship_rules: vec![
                RelationshipRule {
                    source_type: "Borrower".to_string(),
                    target_type: "DealFile".to_string(),
                    detection: RelationshipDetection::DirectReference {
                        field: "borrower_id".to_string(),
                    },
                    relationship_type: "Owns".to_string(),
                },
                RelationshipRule {
                    source_type: "DealFile".to_string(),
                    target_type: "Document".to_string(),
                    detection: RelationshipDetection::DirectReference {
                        field: "deal_file_id".to_string(),
                    },
                    relationship_type: "Contains".to_string(),
                },
                RelationshipRule {
                    source_type: "DealFile".to_string(),
                    target_type: "DealFile".to_string(),
                    detection: RelationshipDetection::SharedProperty {
                        property: "property_address".to_string(),
                    },
                    relationship_type: "RelatedTo".to_string(),
                },
            ],
            filters: vec![
                FilterCriteria {
                    target: FilterTarget::Node,
                    condition: Condition {
                        field: "visibility".to_string(),
                        operator: ConditionOperator::NotEquals,
                        value: serde_json::json!("internal"),
                    },
                },
            ],
        };

        graph.add_component(rules).unwrap();

        // Verify rules
        let retrieved = graph.get_component::<AssemblyRulesComponent>().unwrap();
        assert_eq!(retrieved.inclusion_rules.len(), 3);
        assert_eq!(retrieved.relationship_rules.len(), 3);
        assert_eq!(retrieved.filters.len(), 1);
    }

    /// Test workflow visualization for KECO loan process
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Application Received] --> B[Document Collection]
    ///     B --> C[Initial Review]
    ///     C --> D{Complete?}
    ///     D -->|No| B
    ///     D -->|Yes| E[Underwriting]
    ///     E --> F{Approved?}
    ///     F -->|Yes| G[Closing]
    ///     F -->|No| H[Declined]
    /// ```
    #[test]
    fn test_keco_workflow_graph() {
        let mut graph = ConceptGraph::new(GraphId::new());

        // Add metadata
        let metadata = GraphMetadataComponent {
            name: "KECO Loan Processing Workflow".to_string(),
            description: Some("End-to-end loan processing workflow".to_string()),
            purpose: GraphPurpose::WorkflowVisualization,
            tags: vec!["keco".to_string(), "workflow".to_string(), "loan-process".to_string()],
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        graph.add_component(metadata).unwrap();

        // Workflow steps
        let steps = vec![
            ("ApplicationReceived", "Application Received", "start"),
            ("DocumentCollection", "Document Collection", "process"),
            ("InitialReview", "Initial Review", "process"),
            ("CompletenessCheck", "Complete?", "decision"),
            ("Underwriting", "Underwriting", "process"),
            ("ApprovalDecision", "Approved?", "decision"),
            ("Closing", "Closing", "process"),
            ("Declined", "Declined", "end"),
        ];

        for (id, label, step_type) in steps {
            let node = ConceptNodeComponent {
                concept_type: ConceptType::Process(id.to_string()),
                source_reference: SourceReference {
                    source_type: "WorkflowStep".to_string(),
                    source_id: Uuid::new_v4(),
                    context: Some("LoanProcessingWorkflow".to_string()),
                },
                label: label.to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("step_type".to_string(), serde_json::json!(step_type));
                    props
                },
            };
            graph.add_node(node).unwrap();
        }

        // Add temporal relationships
        let node_ids: Vec<NodeId> = graph.nodes().keys().cloned().collect();

        // Application -> Document Collection
        graph.add_relationship(ConceptRelationshipComponent {
            source_node_id: node_ids[0],
            target_node_id: node_ids[1],
            relationship_type: ConceptRelationshipType::Temporal(TemporalRelation::Before),
            strength: 1.0,
            properties: HashMap::new(),
        }).unwrap();

        // Document Collection -> Initial Review
        graph.add_relationship(ConceptRelationshipComponent {
            source_node_id: node_ids[1],
            target_node_id: node_ids[2],
            relationship_type: ConceptRelationshipType::Temporal(TemporalRelation::Before),
            strength: 1.0,
            properties: HashMap::new(),
        }).unwrap();

        // Configure hierarchical layout
        let layout = LayoutConfigComponent {
            algorithm: LayoutAlgorithm::Hierarchical,
            parameters: {
                let mut params = HashMap::new();
                params.insert("direction".to_string(), serde_json::json!("left-to-right"));
                params.insert("node_spacing".to_string(), serde_json::json!(100));
                params.insert("level_spacing".to_string(), serde_json::json!(150));
                params
            },
            fixed_positions: HashMap::new(),
        };
        graph.add_component(layout).unwrap();

        // Verify workflow structure
        assert_eq!(graph.nodes().len(), 8);
        assert!(graph.relationships().len() >= 2);
        assert!(graph.has_component::<LayoutConfigComponent>());
    }

    /// Test creating a knowledge graph for KECO ubiquitous language
    #[test]
    fn test_keco_ubiquitous_language_graph() {
        let mut graph = ConceptGraph::new(GraphId::new());

        // Add metadata
        let metadata = GraphMetadataComponent {
            name: "KECO Ubiquitous Language".to_string(),
            description: Some("Key terms and concepts in the KECO domain".to_string()),
            purpose: GraphPurpose::KnowledgeRepresentation,
            tags: vec!["keco".to_string(), "ubiquitous-language".to_string(), "glossary".to_string()],
            created_by: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        graph.add_component(metadata).unwrap();

        // Key terms
        let terms = vec![
            ("Borrower", "Individual or entity seeking financing", "actor"),
            ("DealFile", "Collection of loan request information", "aggregate"),
            ("ITO", "Initial Term Outline - preliminary loan terms", "document"),
            ("CLA", "Commitment Letter Agreement", "document"),
            ("ARV", "After Repair Value", "value"),
            ("SOW", "Scope of Work for improvements", "document"),
        ];

        let mut term_nodes = Vec::new();
        for (term, definition, category) in terms {
            let node = ConceptNodeComponent {
                concept_type: ConceptType::Custom(category.to_string()),
                source_reference: SourceReference {
                    source_type: "Term".to_string(),
                    source_id: Uuid::new_v4(),
                    context: Some("UbiquitousLanguage".to_string()),
                },
                label: term.to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("definition".to_string(), serde_json::json!(definition));
                    props.insert("category".to_string(), serde_json::json!(category));
                    props
                },
            };
            graph.add_node(node).unwrap();
            term_nodes.push(term);
        }

        // Add semantic relationships
        let node_ids: Vec<NodeId> = graph.nodes().keys().cloned().collect();

        // Borrower -> DealFile (creates)
        graph.add_relationship(ConceptRelationshipComponent {
            source_node_id: node_ids[0],
            target_node_id: node_ids[1],
            relationship_type: ConceptRelationshipType::Causal(CausalRelation::Triggers),
            strength: 0.9,
            properties: {
                let mut props = HashMap::new();
                props.insert("action".to_string(), serde_json::json!("creates"));
                props
            },
        }).unwrap();

        // DealFile -> ITO (contains)
        graph.add_relationship(ConceptRelationshipComponent {
            source_node_id: node_ids[1],
            target_node_id: node_ids[2],
            relationship_type: ConceptRelationshipType::Contains,
            strength: 0.8,
            properties: HashMap::new(),
        }).unwrap();

        // SOW -> ARV (influences)
        graph.add_relationship(ConceptRelationshipComponent {
            source_node_id: node_ids[5],
            target_node_id: node_ids[4],
            relationship_type: ConceptRelationshipType::Causal(CausalRelation::Enables),
            strength: 0.7,
            properties: {
                let mut props = HashMap::new();
                props.insert("relationship".to_string(), serde_json::json!("determines"));
                props
            },
        }).unwrap();

        // Verify knowledge graph
        assert_eq!(graph.nodes().len(), 6);
        assert_eq!(graph.relationships().len(), 3);
    }
}
