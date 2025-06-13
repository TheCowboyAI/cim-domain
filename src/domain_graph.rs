//! Domain model graph generation
//!
//! This module provides functionality to analyze the domain model and generate
//! graph visualizations showing relationships between aggregates, entities,
//! value objects, commands, events, and other DDD components.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Type of domain element
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DomainElementType {
    /// Aggregate root that enforces business invariants
    Aggregate,
    /// Entity with unique identity within an aggregate
    Entity,
    /// Immutable value object defined by its attributes
    ValueObject,
    /// Command that expresses intent to change state
    Command,
    /// Event that records something that happened
    Event,
    /// Reusable component that can be attached to entities
    Component,
    /// Read model projection built from events
    Projection,
    /// Domain service that encapsulates business logic
    Service,
    /// Repository for aggregate persistence
    Repository,
    /// Command or event handler
    Handler,
}

/// A node in the domain graph
#[derive(Debug, Clone)]
pub struct DomainNode {
    /// Unique identifier for the node
    pub id: String,
    /// Display name of the domain element
    pub name: String,
    /// Type of domain element this node represents
    pub element_type: DomainElementType,
    /// Module where this element is defined
    pub module: String,
    /// Fields/properties of this element
    pub fields: Vec<FieldInfo>,
    /// Methods defined on this element
    pub methods: Vec<String>,
    /// Traits implemented by this element
    pub traits: Vec<String>,
}

/// Field information
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Name of the field
    pub name: String,
    /// Type of the field (e.g., "String", "Uuid", "Vec<T>")
    pub field_type: String,
    /// Whether the field is optional (Option<T>)
    pub is_optional: bool,
    /// Whether the field is a collection (Vec, HashSet, etc.)
    pub is_collection: bool,
}

/// An edge in the domain graph
#[derive(Debug, Clone)]
pub struct DomainEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Type of relationship between nodes
    pub relationship: RelationshipType,
    /// Optional label for the edge
    pub label: Option<String>,
}

/// Type of relationship between domain elements
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    /// Aggregate contains entity or value object
    Contains,
    /// One element references another by ID
    References,
    /// Element implements a trait or interface
    Implements,
    /// Handler processes a specific command
    HandlesCommand,
    /// Command or handler emits an event
    EmitsEvent,
    /// System or handler listens to an event
    ListensTo,
    /// General dependency relationship
    DependsOn,
    /// Creates or maintains a projection
    Projects,
    /// Uses a component
    Uses,
}

/// Domain model graph
pub struct DomainGraph {
    /// All nodes in the graph, indexed by their ID
    pub nodes: HashMap<String, DomainNode>,
    /// All edges representing relationships between nodes
    pub edges: Vec<DomainEdge>,
}

impl DomainGraph {
    /// Create a new empty domain graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: DomainNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: DomainEdge) {
        self.edges.push(edge);
    }

    /// Generate Mermaid diagram
    pub fn to_mermaid(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "graph TD").unwrap();
        writeln!(&mut output, "    %% Domain Model Graph").unwrap();
        writeln!(&mut output, "    %% Generated from cim-domain source code").unwrap();
        writeln!(&mut output).unwrap();

        // Define node styles based on element type
        writeln!(&mut output, "    %% Node Styles").unwrap();
        writeln!(&mut output, "    classDef aggregate fill:#f9f,stroke:#333,stroke-width:4px;").unwrap();
        writeln!(&mut output, "    classDef entity fill:#bbf,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output, "    classDef valueObject fill:#bfb,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output, "    classDef command fill:#fbb,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output, "    classDef event fill:#fbf,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output, "    classDef component fill:#ffb,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output, "    classDef projection fill:#bff,stroke:#333,stroke-width:2px;").unwrap();
        writeln!(&mut output).unwrap();

        // Group nodes by module
        let mut modules: HashMap<String, Vec<&DomainNode>> = HashMap::new();
        for node in self.nodes.values() {
            modules.entry(node.module.clone()).or_default().push(node);
        }

        // Generate subgraphs for each module
        for (module, nodes) in modules {
            writeln!(&mut output, "    subgraph {}", module).unwrap();
            for node in nodes {
                let node_def = self.format_node(node);
                writeln!(&mut output, "        {}", node_def).unwrap();
            }
            writeln!(&mut output, "    end").unwrap();
            writeln!(&mut output).unwrap();
        }

        // Generate edges
        writeln!(&mut output, "    %% Relationships").unwrap();
        for edge in &self.edges {
            let edge_def = self.format_edge(edge);
            writeln!(&mut output, "    {}", edge_def).unwrap();
        }

        // Apply styles
        writeln!(&mut output).unwrap();
        writeln!(&mut output, "    %% Apply Styles").unwrap();
        for node in self.nodes.values() {
            let class = match node.element_type {
                DomainElementType::Aggregate => "aggregate",
                DomainElementType::Entity => "entity",
                DomainElementType::ValueObject => "valueObject",
                DomainElementType::Command => "command",
                DomainElementType::Event => "event",
                DomainElementType::Component => "component",
                DomainElementType::Projection => "projection",
                _ => continue,
            };
            writeln!(&mut output, "    class {} {};", node.id, class).unwrap();
        }

        output
    }

    /// Generate GraphViz DOT format
    pub fn to_dot(&self) -> String {
        let mut output = String::new();
        writeln!(&mut output, "digraph DomainModel {{").unwrap();
        writeln!(&mut output, "    rankdir=TB;").unwrap();
        writeln!(&mut output, "    node [shape=record];").unwrap();
        writeln!(&mut output).unwrap();

        // Group by module
        let mut modules: HashMap<String, Vec<&DomainNode>> = HashMap::new();
        for node in self.nodes.values() {
            modules.entry(node.module.clone()).or_default().push(node);
        }

        // Generate subgraphs
        for (idx, (module, nodes)) in modules.iter().enumerate() {
            writeln!(&mut output, "    subgraph cluster_{} {{", idx).unwrap();
            writeln!(&mut output, "        label=\"{}\";", module).unwrap();
            writeln!(&mut output, "        style=filled;").unwrap();
            writeln!(&mut output, "        color=lightgrey;").unwrap();
            writeln!(&mut output).unwrap();

            for node in nodes {
                let color = match node.element_type {
                    DomainElementType::Aggregate => "lightpink",
                    DomainElementType::Entity => "lightblue",
                    DomainElementType::ValueObject => "lightgreen",
                    DomainElementType::Command => "lightsalmon",
                    DomainElementType::Event => "plum",
                    DomainElementType::Component => "lightyellow",
                    DomainElementType::Projection => "lightcyan",
                    _ => "white",
                };

                let mut label = format!("{{<b>{}</b>", node.name);
                if !node.fields.is_empty() {
                    label.push_str("|");
                    for field in &node.fields {
                        label.push_str(&format!("{}:{}", field.name, field.field_type));
                        if field.is_optional {
                            label.push('?');
                        }
                        label.push_str("\\l");
                    }
                }
                label.push('}');

                writeln!(
                    &mut output,
                    "        {} [label=\"{}\" fillcolor={} style=filled];",
                    node.id, label, color
                ).unwrap();
            }
            writeln!(&mut output, "    }}").unwrap();
            writeln!(&mut output).unwrap();
        }

        // Generate edges
        for edge in &self.edges {
            let style = match edge.relationship {
                RelationshipType::Contains => "style=bold",
                RelationshipType::References => "style=dashed",
                RelationshipType::Implements => "style=dotted",
                RelationshipType::HandlesCommand => "color=red",
                RelationshipType::EmitsEvent => "color=blue",
                RelationshipType::ListensTo => "color=green",
                _ => "",
            };

            let label = edge.label.as_deref().unwrap_or("");
            writeln!(
                &mut output,
                "    {} -> {} [label=\"{}\" {}];",
                edge.from, edge.to, label, style
            ).unwrap();
        }

        writeln!(&mut output, "}}").unwrap();
        output
    }

    fn format_node(&self, node: &DomainNode) -> String {
        let mut label = node.name.clone();

        // Add field count if there are many fields
        if node.fields.len() > 3 {
            label.push_str(&format!("<br/>{} fields", node.fields.len()));
        } else if !node.fields.is_empty() {
            // Show first few fields
            for field in node.fields.iter().take(3) {
                label.push_str(&format!("<br/>- {}: {}", field.name, field.field_type));
            }
        }

        format!("{}[{}]", node.id, label)
    }

    fn format_edge(&self, edge: &DomainEdge) -> String {
        let arrow = match edge.relationship {
            RelationshipType::Contains => "-->",
            RelationshipType::References => "-.->",
            RelationshipType::Implements => "-.->",
            RelationshipType::HandlesCommand => "==>",
            RelationshipType::EmitsEvent => "==>",
            RelationshipType::ListensTo => "-.->",
            _ => "-->",
        };

        if let Some(label) = &edge.label {
            format!("{} {}|{}| {}", edge.from, arrow, label, edge.to)
        } else {
            format!("{} {} {}", edge.from, arrow, edge.to)
        }
    }

    /// Analyze the current domain model
    pub fn analyze_current_model() -> Self {
        let mut graph = Self::new();

        // Add Aggregates
        graph.add_node(DomainNode {
            id: "Location".to_string(),
            name: "Location".to_string(),
            element_type: DomainElementType::Aggregate,
            module: "location".to_string(),
            fields: vec![
                FieldInfo {
                    name: "entity".to_string(),
                    field_type: "Entity<LocationMarker>".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "name".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "location_type".to_string(),
                    field_type: "LocationType".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "address".to_string(),
                    field_type: "Address".to_string(),
                    is_optional: true,
                    is_collection: false,
                },
                FieldInfo {
                    name: "coordinates".to_string(),
                    field_type: "GeoCoordinates".to_string(),
                    is_optional: true,
                    is_collection: false,
                },
            ],
            methods: vec!["new_physical".to_string(), "new_virtual".to_string()],
            traits: vec!["AggregateRoot".to_string()],
        });

        graph.add_node(DomainNode {
            id: "Person".to_string(),
            name: "Person".to_string(),
            element_type: DomainElementType::Aggregate,
            module: "person".to_string(),
            fields: vec![
                FieldInfo {
                    name: "entity".to_string(),
                    field_type: "Entity<PersonMarker>".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "components".to_string(),
                    field_type: "ComponentStorage".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec!["add_component".to_string(), "remove_component".to_string()],
            traits: vec!["AggregateRoot".to_string()],
        });

        // Add Value Objects
        graph.add_node(DomainNode {
            id: "Address".to_string(),
            name: "Address".to_string(),
            element_type: DomainElementType::ValueObject,
            module: "location".to_string(),
            fields: vec![
                FieldInfo {
                    name: "street1".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "locality".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "region".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "country".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec!["validate".to_string()],
            traits: vec![],
        });

        graph.add_node(DomainNode {
            id: "GeoCoordinates".to_string(),
            name: "GeoCoordinates".to_string(),
            element_type: DomainElementType::ValueObject,
            module: "location".to_string(),
            fields: vec![
                FieldInfo {
                    name: "latitude".to_string(),
                    field_type: "f64".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "longitude".to_string(),
                    field_type: "f64".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec!["distance_to".to_string()],
            traits: vec![],
        });

        // Add Components
        graph.add_node(DomainNode {
            id: "IdentityComponent".to_string(),
            name: "IdentityComponent".to_string(),
            element_type: DomainElementType::Component,
            module: "person".to_string(),
            fields: vec![
                FieldInfo {
                    name: "legal_name".to_string(),
                    field_type: "String".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "preferred_name".to_string(),
                    field_type: "String".to_string(),
                    is_optional: true,
                    is_collection: false,
                },
            ],
            methods: vec![],
            traits: vec!["Component".to_string()],
        });

        graph.add_node(DomainNode {
            id: "ContactComponent".to_string(),
            name: "ContactComponent".to_string(),
            element_type: DomainElementType::Component,
            module: "person".to_string(),
            fields: vec![
                FieldInfo {
                    name: "emails".to_string(),
                    field_type: "EmailAddress".to_string(),
                    is_optional: false,
                    is_collection: true,
                },
                FieldInfo {
                    name: "addresses".to_string(),
                    field_type: "Uuid".to_string(),
                    is_optional: false,
                    is_collection: true,
                },
            ],
            methods: vec![],
            traits: vec!["Component".to_string()],
        });

        // Add Commands
        graph.add_node(DomainNode {
            id: "RegisterPerson".to_string(),
            name: "RegisterPerson".to_string(),
            element_type: DomainElementType::Command,
            module: "commands".to_string(),
            fields: vec![
                FieldInfo {
                    name: "person_id".to_string(),
                    field_type: "Uuid".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "identity".to_string(),
                    field_type: "IdentityComponent".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec![],
            traits: vec!["Command".to_string()],
        });

        graph.add_node(DomainNode {
            id: "DefineLocation".to_string(),
            name: "DefineLocation".to_string(),
            element_type: DomainElementType::Command,
            module: "commands".to_string(),
            fields: vec![
                FieldInfo {
                    name: "location_id".to_string(),
                    field_type: "Uuid".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "address".to_string(),
                    field_type: "Address".to_string(),
                    is_optional: true,
                    is_collection: false,
                },
            ],
            methods: vec![],
            traits: vec!["Command".to_string()],
        });

        // Add Events
        graph.add_node(DomainNode {
            id: "PersonRegistered".to_string(),
            name: "PersonRegistered".to_string(),
            element_type: DomainElementType::Event,
            module: "events".to_string(),
            fields: vec![
                FieldInfo {
                    name: "person_id".to_string(),
                    field_type: "Uuid".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "identity".to_string(),
                    field_type: "IdentityComponent".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec![],
            traits: vec!["DomainEvent".to_string()],
        });

        graph.add_node(DomainNode {
            id: "LocationDefined".to_string(),
            name: "LocationDefined".to_string(),
            element_type: DomainElementType::Event,
            module: "events".to_string(),
            fields: vec![
                FieldInfo {
                    name: "location_id".to_string(),
                    field_type: "Uuid".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "address".to_string(),
                    field_type: "Address".to_string(),
                    is_optional: true,
                    is_collection: false,
                },
            ],
            methods: vec![],
            traits: vec!["DomainEvent".to_string()],
        });

        // Add Projections
        graph.add_node(DomainNode {
            id: "EmployeeView".to_string(),
            name: "EmployeeView".to_string(),
            element_type: DomainElementType::Projection,
            module: "person".to_string(),
            fields: vec![
                FieldInfo {
                    name: "person_id".to_string(),
                    field_type: "EntityId<PersonMarker>".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "identity".to_string(),
                    field_type: "IdentityComponent".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
                FieldInfo {
                    name: "employment".to_string(),
                    field_type: "EmploymentComponent".to_string(),
                    is_optional: false,
                    is_collection: false,
                },
            ],
            methods: vec!["from_person".to_string()],
            traits: vec![],
        });

        // Add relationships
        graph.add_edge(DomainEdge {
            from: "Location".to_string(),
            to: "Address".to_string(),
            relationship: RelationshipType::Contains,
            label: Some("has".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "Location".to_string(),
            to: "GeoCoordinates".to_string(),
            relationship: RelationshipType::Contains,
            label: Some("has".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "Person".to_string(),
            to: "IdentityComponent".to_string(),
            relationship: RelationshipType::Uses,
            label: Some("stores".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "Person".to_string(),
            to: "ContactComponent".to_string(),
            relationship: RelationshipType::Uses,
            label: Some("stores".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "ContactComponent".to_string(),
            to: "Location".to_string(),
            relationship: RelationshipType::References,
            label: Some("addresses".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "RegisterPerson".to_string(),
            to: "PersonRegistered".to_string(),
            relationship: RelationshipType::EmitsEvent,
            label: Some("produces".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "DefineLocation".to_string(),
            to: "LocationDefined".to_string(),
            relationship: RelationshipType::EmitsEvent,
            label: Some("produces".to_string()),
        });

        graph.add_edge(DomainEdge {
            from: "EmployeeView".to_string(),
            to: "Person".to_string(),
            relationship: RelationshipType::Projects,
            label: Some("from".to_string()),
        });

        graph
    }

    /// Find unused elements
    pub fn find_unused_elements(&self) -> Vec<&DomainNode> {
        let mut referenced = HashSet::new();

        // Collect all referenced nodes
        for edge in &self.edges {
            referenced.insert(&edge.to);
            referenced.insert(&edge.from);
        }

        // Find nodes that are not referenced
        self.nodes.values()
            .filter(|node| !referenced.contains(&node.id))
            .collect()
    }

    /// Find missing elements (mentioned in edges but not defined)
    pub fn find_missing_elements(&self) -> Vec<String> {
        let mut missing = Vec::new();

        for edge in &self.edges {
            if !self.nodes.contains_key(&edge.from) {
                missing.push(edge.from.clone());
            }
            if !self.nodes.contains_key(&edge.to) {
                missing.push(edge.to.clone());
            }
        }

        missing.sort();
        missing.dedup();
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_graph_creation() {
        let graph = DomainGraph::analyze_current_model();

        // Verify we have the expected aggregates
        assert!(graph.nodes.contains_key("Location"));
        assert!(graph.nodes.contains_key("Person"));

        // Verify we have value objects
        assert!(graph.nodes.contains_key("Address"));
        assert!(graph.nodes.contains_key("GeoCoordinates"));

        // Verify relationships exist
        assert!(graph.edges.iter().any(|e|
            e.from == "Location" && e.to == "Address"
        ));
    }

    #[test]
    fn test_mermaid_generation() {
        let graph = DomainGraph::analyze_current_model();
        let mermaid = graph.to_mermaid();

        // Verify basic structure
        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("subgraph location"));
        assert!(mermaid.contains("subgraph person"));
        assert!(mermaid.contains("Location[Location"));
        assert!(mermaid.contains("Person[Person"));
    }

    #[test]
    fn test_dot_generation() {
        let graph = DomainGraph::analyze_current_model();
        let dot = graph.to_dot();

        // Verify basic structure
        assert!(dot.contains("digraph DomainModel"));
        assert!(dot.contains("subgraph cluster_"));
        assert!(dot.contains("Location [label="));
        assert!(dot.contains("Person [label="));
    }
}
