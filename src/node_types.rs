//! Node types representing DDD concepts

use serde::{Deserialize, Serialize};

/// Types of nodes that can exist in a domain graph
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    // Core DDD types
    /// Abstract concept or idea
    Concept,
    /// Grouping or classification
    Category,
    /// Concrete instance of a concept
    Instance,
    /// Domain entity (has identity)
    Entity,
    /// Immutable value (no identity)
    ValueObject,
    /// Aggregate root (consistency boundary)
    Aggregate,

    // Event sourcing types
    /// Domain event (something that happened)
    Event,
    /// Command (request to change state)
    Command,
    /// Query (request to read state)
    Query,

    // Workflow types
    /// Workflow activity or step
    Activity,
    /// Decision point in a workflow
    Gateway,
    /// Data flowing through workflow
    DataObject,

    // Development tracking types
    /// Project milestone
    Milestone,
    /// Development phase
    Phase,
    /// Work task
    Task,

    // Version control types
    /// Git commit
    Commit,
    /// Git branch
    Branch,
    /// Git tag
    Tag,

    // Service types
    /// Domain service
    DomainService,
    /// Application service
    ApplicationService,
    /// Infrastructure service
    InfrastructureService,

    // Context types
    /// Bounded context
    BoundedContext,
    /// Module within a context
    Module,
    /// Subdomain
    Subdomain,

    // Policy types
    /// Business rule or policy
    Policy,
    /// Invariant that must be maintained
    Invariant,
    /// Constraint on the domain
    Constraint,

    // Generic/Unknown
    /// Type not yet identified
    Unidentified,
    /// Domain-specific custom type
    Custom(String),
}

impl NodeType {
    /// Check if this is an entity type (has identity)
    pub fn is_entity(&self) -> bool {
        matches!(self,
            NodeType::Entity |
            NodeType::Aggregate |
            NodeType::Event |
            NodeType::Command |
            NodeType::Query
        )
    }

    /// Check if this is a value object type (no identity)
    pub fn is_value_object(&self) -> bool {
        matches!(self,
            NodeType::ValueObject |
            NodeType::DataObject |
            NodeType::Policy |
            NodeType::Invariant |
            NodeType::Constraint
        )
    }

    /// Check if this is a service type
    pub fn is_service(&self) -> bool {
        matches!(self,
            NodeType::DomainService |
            NodeType::ApplicationService |
            NodeType::InfrastructureService
        )
    }

    /// Check if this is a context boundary type
    pub fn is_context_boundary(&self) -> bool {
        matches!(self,
            NodeType::BoundedContext |
            NodeType::Module |
            NodeType::Subdomain
        )
    }

    /// Get a human-readable name for this node type
    pub fn display_name(&self) -> &str {
        match self {
            NodeType::Concept => "Concept",
            NodeType::Category => "Category",
            NodeType::Instance => "Instance",
            NodeType::Entity => "Entity",
            NodeType::ValueObject => "Value Object",
            NodeType::Aggregate => "Aggregate",
            NodeType::Event => "Event",
            NodeType::Command => "Command",
            NodeType::Query => "Query",
            NodeType::Activity => "Activity",
            NodeType::Gateway => "Gateway",
            NodeType::DataObject => "Data Object",
            NodeType::Milestone => "Milestone",
            NodeType::Phase => "Phase",
            NodeType::Task => "Task",
            NodeType::Commit => "Commit",
            NodeType::Branch => "Branch",
            NodeType::Tag => "Tag",
            NodeType::DomainService => "Domain Service",
            NodeType::ApplicationService => "Application Service",
            NodeType::InfrastructureService => "Infrastructure Service",
            NodeType::BoundedContext => "Bounded Context",
            NodeType::Module => "Module",
            NodeType::Subdomain => "Subdomain",
            NodeType::Policy => "Policy",
            NodeType::Invariant => "Invariant",
            NodeType::Constraint => "Constraint",
            NodeType::Unidentified => "Unidentified",
            NodeType::Custom(name) => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test entity type classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[NodeType] -->|is_entity| B{Has Identity?}
    ///     B -->|Yes| C[Entity Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[Entity, Aggregate, Event, Command, Query]
    /// ```
    #[test]
    fn test_is_entity() {
        // Entity types
        assert!(NodeType::Entity.is_entity());
        assert!(NodeType::Aggregate.is_entity());
        assert!(NodeType::Event.is_entity());
        assert!(NodeType::Command.is_entity());
        assert!(NodeType::Query.is_entity());

        // Non-entity types
        assert!(!NodeType::ValueObject.is_entity());
        assert!(!NodeType::Concept.is_entity());
        assert!(!NodeType::Category.is_entity());
        assert!(!NodeType::Instance.is_entity());
        assert!(!NodeType::Activity.is_entity());
        assert!(!NodeType::Gateway.is_entity());
        assert!(!NodeType::DataObject.is_entity());
        assert!(!NodeType::DomainService.is_entity());
        assert!(!NodeType::Policy.is_entity());
        assert!(!NodeType::Custom("Test".to_string()).is_entity());
    }

    /// Test value object type classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[NodeType] -->|is_value_object| B{No Identity?}
    ///     B -->|Yes| C[Value Object Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[ValueObject, DataObject, Policy, Invariant, Constraint]
    /// ```
    #[test]
    fn test_is_value_object() {
        // Value object types
        assert!(NodeType::ValueObject.is_value_object());
        assert!(NodeType::DataObject.is_value_object());
        assert!(NodeType::Policy.is_value_object());
        assert!(NodeType::Invariant.is_value_object());
        assert!(NodeType::Constraint.is_value_object());

        // Non-value object types
        assert!(!NodeType::Entity.is_value_object());
        assert!(!NodeType::Aggregate.is_value_object());
        assert!(!NodeType::Event.is_value_object());
        assert!(!NodeType::Command.is_value_object());
        assert!(!NodeType::Query.is_value_object());
        assert!(!NodeType::Concept.is_value_object());
        assert!(!NodeType::DomainService.is_value_object());
        assert!(!NodeType::Custom("Test".to_string()).is_value_object());
    }

    /// Test service type classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[NodeType] -->|is_service| B{Service Type?}
    ///     B -->|Yes| C[Service Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[DomainService]
    ///     C --> F[ApplicationService]
    ///     C --> G[InfrastructureService]
    /// ```
    #[test]
    fn test_is_service() {
        // Service types
        assert!(NodeType::DomainService.is_service());
        assert!(NodeType::ApplicationService.is_service());
        assert!(NodeType::InfrastructureService.is_service());

        // Non-service types
        assert!(!NodeType::Entity.is_service());
        assert!(!NodeType::ValueObject.is_service());
        assert!(!NodeType::Aggregate.is_service());
        assert!(!NodeType::Event.is_service());
        assert!(!NodeType::Command.is_service());
        assert!(!NodeType::Policy.is_service());
        assert!(!NodeType::BoundedContext.is_service());
        assert!(!NodeType::Custom("Test".to_string()).is_service());
    }

    /// Test context boundary type classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[NodeType] -->|is_context_boundary| B{Boundary Type?}
    ///     B -->|Yes| C[Context Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[BoundedContext, Module, Subdomain]
    /// ```
    #[test]
    fn test_is_context_boundary() {
        // Context boundary types
        assert!(NodeType::BoundedContext.is_context_boundary());
        assert!(NodeType::Module.is_context_boundary());
        assert!(NodeType::Subdomain.is_context_boundary());

        // Non-context boundary types
        assert!(!NodeType::Entity.is_context_boundary());
        assert!(!NodeType::ValueObject.is_context_boundary());
        assert!(!NodeType::Aggregate.is_context_boundary());
        assert!(!NodeType::DomainService.is_context_boundary());
        assert!(!NodeType::Event.is_context_boundary());
        assert!(!NodeType::Policy.is_context_boundary());
        assert!(!NodeType::Custom("Test".to_string()).is_context_boundary());
    }

    /// Test display names
    #[test]
    fn test_display_name() {
        // Core DDD types
        assert_eq!(NodeType::Concept.display_name(), "Concept");
        assert_eq!(NodeType::Category.display_name(), "Category");
        assert_eq!(NodeType::Instance.display_name(), "Instance");
        assert_eq!(NodeType::Entity.display_name(), "Entity");
        assert_eq!(NodeType::ValueObject.display_name(), "Value Object");
        assert_eq!(NodeType::Aggregate.display_name(), "Aggregate");

        // Event sourcing types
        assert_eq!(NodeType::Event.display_name(), "Event");
        assert_eq!(NodeType::Command.display_name(), "Command");
        assert_eq!(NodeType::Query.display_name(), "Query");

        // Workflow types
        assert_eq!(NodeType::Activity.display_name(), "Activity");
        assert_eq!(NodeType::Gateway.display_name(), "Gateway");
        assert_eq!(NodeType::DataObject.display_name(), "Data Object");

        // Development tracking types
        assert_eq!(NodeType::Milestone.display_name(), "Milestone");
        assert_eq!(NodeType::Phase.display_name(), "Phase");
        assert_eq!(NodeType::Task.display_name(), "Task");

        // Version control types
        assert_eq!(NodeType::Commit.display_name(), "Commit");
        assert_eq!(NodeType::Branch.display_name(), "Branch");
        assert_eq!(NodeType::Tag.display_name(), "Tag");

        // Service types
        assert_eq!(NodeType::DomainService.display_name(), "Domain Service");
        assert_eq!(NodeType::ApplicationService.display_name(), "Application Service");
        assert_eq!(NodeType::InfrastructureService.display_name(), "Infrastructure Service");

        // Context types
        assert_eq!(NodeType::BoundedContext.display_name(), "Bounded Context");
        assert_eq!(NodeType::Module.display_name(), "Module");
        assert_eq!(NodeType::Subdomain.display_name(), "Subdomain");

        // Policy types
        assert_eq!(NodeType::Policy.display_name(), "Policy");
        assert_eq!(NodeType::Invariant.display_name(), "Invariant");
        assert_eq!(NodeType::Constraint.display_name(), "Constraint");

        // Generic types
        assert_eq!(NodeType::Unidentified.display_name(), "Unidentified");
        assert_eq!(NodeType::Custom("MyCustomType".to_string()).display_name(), "MyCustomType");
    }

    /// Test serialization and deserialization
    #[test]
    fn test_serde() {
        let test_cases = vec![
            NodeType::Entity,
            NodeType::ValueObject,
            NodeType::Aggregate,
            NodeType::Event,
            NodeType::DomainService,
            NodeType::BoundedContext,
            NodeType::Custom("TestType".to_string()),
        ];

        for node_type in test_cases {
            // Serialize
            let json = serde_json::to_string(&node_type).unwrap();

            // Deserialize
            let deserialized: NodeType = serde_json::from_str(&json).unwrap();

            // Verify equality
            assert_eq!(node_type, deserialized);
        }
    }

    /// Test custom type behavior
    #[test]
    fn test_custom_type() {
        let custom1 = NodeType::Custom("TypeA".to_string());
        let custom2 = NodeType::Custom("TypeA".to_string());
        let custom3 = NodeType::Custom("TypeB".to_string());

        // Equality
        assert_eq!(custom1, custom2);
        assert_ne!(custom1, custom3);

        // Display name
        assert_eq!(custom1.display_name(), "TypeA");
        assert_eq!(custom3.display_name(), "TypeB");

        // Classification
        assert!(!custom1.is_entity());
        assert!(!custom1.is_value_object());
        assert!(!custom1.is_service());
        assert!(!custom1.is_context_boundary());
    }

    /// Test all node types are covered by classification methods
    #[test]
    fn test_classification_coverage() {
        let all_types = vec![
            NodeType::Concept,
            NodeType::Category,
            NodeType::Instance,
            NodeType::Entity,
            NodeType::ValueObject,
            NodeType::Aggregate,
            NodeType::Event,
            NodeType::Command,
            NodeType::Query,
            NodeType::Activity,
            NodeType::Gateway,
            NodeType::DataObject,
            NodeType::Milestone,
            NodeType::Phase,
            NodeType::Task,
            NodeType::Commit,
            NodeType::Branch,
            NodeType::Tag,
            NodeType::DomainService,
            NodeType::ApplicationService,
            NodeType::InfrastructureService,
            NodeType::BoundedContext,
            NodeType::Module,
            NodeType::Subdomain,
            NodeType::Policy,
            NodeType::Invariant,
            NodeType::Constraint,
            NodeType::Unidentified,
            NodeType::Custom("Test".to_string()),
        ];

        // Verify each type has a display name
        for node_type in &all_types {
            assert!(!node_type.display_name().is_empty());
        }

        // Count classifications
        let entity_count = all_types.iter().filter(|t| t.is_entity()).count();
        let value_object_count = all_types.iter().filter(|t| t.is_value_object()).count();
        let service_count = all_types.iter().filter(|t| t.is_service()).count();
        let context_count = all_types.iter().filter(|t| t.is_context_boundary()).count();

        // Verify reasonable distribution
        assert_eq!(entity_count, 5);
        assert_eq!(value_object_count, 5);
        assert_eq!(service_count, 3);
        assert_eq!(context_count, 3);

        // Many types don't fall into these categories, which is expected
        let classified_count = entity_count + value_object_count + service_count + context_count;
        assert!(classified_count < all_types.len());
    }

    /// Test hash and equality implementation
    #[test]
    fn test_hash_and_eq() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        // Add various types
        set.insert(NodeType::Entity);
        set.insert(NodeType::ValueObject);
        set.insert(NodeType::Custom("Type1".to_string()));

        // Verify contains
        assert!(set.contains(&NodeType::Entity));
        assert!(set.contains(&NodeType::ValueObject));
        assert!(set.contains(&NodeType::Custom("Type1".to_string())));

        // Verify not contains
        assert!(!set.contains(&NodeType::Aggregate));
        assert!(!set.contains(&NodeType::Custom("Type2".to_string())));

        // Add duplicate - should not increase size
        let original_len = set.len();
        set.insert(NodeType::Entity);
        assert_eq!(set.len(), original_len);
    }
}
