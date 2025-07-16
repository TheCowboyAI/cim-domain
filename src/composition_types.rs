//! Composition types for building complex domain structures

use serde::{Deserialize, Serialize};

/// Types of graph compositions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompositionType {
    /// Single node, no edges - represents a value
    Atomic {
        /// Type of the atomic value
        value_type: String
    },

    /// Multiple nodes/edges - represents a structure
    Composite {
        /// Type of the composite structure
        structure_type: String
    },

    /// Maps one graph to another - represents transformation
    Functor {
        /// Source graph type
        source_type: String,
        /// Target graph type
        target_type: String,
    },

    /// Wraps a graph-returning computation - represents context
    Monad {
        /// Type of monadic context
        context_type: String
    },

    /// Represents a DDD concept
    Domain(DomainCompositionType),
}

/// Domain-specific composition types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCompositionType {
    /// Entity composition
    Entity {
        /// Type of entity
        entity_type: String
    },

    /// Value object composition
    ValueObject {
        /// Type of value object
        value_type: String
    },

    /// Aggregate composition
    Aggregate {
        /// Type of aggregate
        aggregate_type: String
    },

    /// Service composition
    Service {
        /// Type of service
        service_type: String
    },

    /// Event composition
    Event {
        /// Type of event
        event_type: String
    },

    /// Command composition
    Command {
        /// Type of command
        command_type: String
    },

    /// Query composition
    Query {
        /// Type of query
        query_type: String
    },

    /// Bounded context composition
    BoundedContext {
        /// Domain name
        domain: String
    },

    /// Policy composition
    Policy {
        /// Type of policy
        policy_type: String
    },

    /// Workflow composition
    Workflow {
        /// Type of workflow
        workflow_type: String
    },
    
    /// Composite structure (for limits/colimits)
    Composite {
        /// Type of composite
        composite_type: String,
        /// Component domains
        components: Vec<String>,
    },
}

impl CompositionType {
    /// Check if this is an atomic composition
    pub fn is_atomic(&self) -> bool {
        matches!(self, CompositionType::Atomic { .. })
    }

    /// Check if this is a composite composition
    pub fn is_composite(&self) -> bool {
        matches!(self, CompositionType::Composite { .. })
    }

    /// Check if this is a domain composition
    pub fn is_domain(&self) -> bool {
        matches!(self, CompositionType::Domain(_))
    }

    /// Get a human-readable name for the composition type
    pub fn display_name(&self) -> String {
        match self {
            CompositionType::Atomic { value_type } => format!("Atomic {value_type}"),
            CompositionType::Composite { structure_type } => format!("Composite {structure_type}"),
            CompositionType::Functor { source_type, target_type } => {
                format!("Functor {source_type} → {target_type}")
            }
            CompositionType::Monad { context_type } => format!("Monad {context_type}"),
            CompositionType::Domain(domain_type) => domain_type.display_name(),
        }
    }
}

impl DomainCompositionType {
    /// Check if this represents an entity type
    pub fn is_entity_type(&self) -> bool {
        matches!(self,
            DomainCompositionType::Entity { .. } |
            DomainCompositionType::Aggregate { .. } |
            DomainCompositionType::Event { .. } |
            DomainCompositionType::Command { .. } |
            DomainCompositionType::Query { .. }
        )
    }

    /// Check if this represents a value object type
    pub fn is_value_object_type(&self) -> bool {
        matches!(self,
            DomainCompositionType::ValueObject { .. } |
            DomainCompositionType::Policy { .. }
        )
    }

    /// Check if this represents a service type
    pub fn is_service_type(&self) -> bool {
        matches!(self, DomainCompositionType::Service { .. })
    }

    /// Check if this represents a boundary type
    pub fn is_boundary_type(&self) -> bool {
        matches!(self,
            DomainCompositionType::BoundedContext { .. } |
            DomainCompositionType::Aggregate { .. }
        )
    }

    /// Get a human-readable name
    pub fn display_name(&self) -> String {
        match self {
            DomainCompositionType::Entity { entity_type } => format!("Entity: {entity_type}"),
            DomainCompositionType::ValueObject { value_type } => format!("Value Object: {value_type}"),
            DomainCompositionType::Aggregate { aggregate_type } => format!("Aggregate: {aggregate_type}"),
            DomainCompositionType::Service { service_type } => format!("Service: {service_type}"),
            DomainCompositionType::Event { event_type } => format!("Event: {event_type}"),
            DomainCompositionType::Command { command_type } => format!("Command: {command_type}"),
            DomainCompositionType::Query { query_type } => format!("Query: {query_type}"),
            DomainCompositionType::BoundedContext { domain } => format!("Bounded Context: {domain}"),
            DomainCompositionType::Policy { policy_type } => format!("Policy: {policy_type}"),
            DomainCompositionType::Workflow { workflow_type } => format!("Workflow: {workflow_type}"),
            DomainCompositionType::Composite { composite_type, components } => format!("Composite: {} ({})", composite_type, components.join(", ")),
        }
    }

    /// Get the base type name without the specific subtype
    pub fn base_type_name(&self) -> &str {
        match self {
            DomainCompositionType::Entity { .. } => "Entity",
            DomainCompositionType::ValueObject { .. } => "ValueObject",
            DomainCompositionType::Aggregate { .. } => "Aggregate",
            DomainCompositionType::Service { .. } => "Service",
            DomainCompositionType::Event { .. } => "Event",
            DomainCompositionType::Command { .. } => "Command",
            DomainCompositionType::Query { .. } => "Query",
            DomainCompositionType::BoundedContext { .. } => "BoundedContext",
            DomainCompositionType::Policy { .. } => "Policy",
            DomainCompositionType::Workflow { .. } => "Workflow",
            DomainCompositionType::Composite { .. } => "Composite",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test CompositionType classification methods
    ///
    /// ```mermaid
    /// graph TD
    ///     A[CompositionType] -->|is_atomic| B{Check Type}
    ///     B -->|Atomic| C[true]
    ///     B -->|Other| D[false]
    ///     A -->|is_composite| E{Check Type}
    ///     E -->|Composite| F[true]
    ///     E -->|Other| G[false]
    /// ```
    #[test]
    fn test_composition_type_classification() {
        let atomic = CompositionType::Atomic {
            value_type: "Integer".to_string(),
        };
        assert!(atomic.is_atomic());
        assert!(!atomic.is_composite());
        assert!(!atomic.is_domain());

        let composite = CompositionType::Composite {
            structure_type: "Tree".to_string(),
        };
        assert!(!composite.is_atomic());
        assert!(composite.is_composite());
        assert!(!composite.is_domain());

        let domain = CompositionType::Domain(DomainCompositionType::Entity {
            entity_type: "Order".to_string(),
        });
        assert!(!domain.is_atomic());
        assert!(!domain.is_composite());
        assert!(domain.is_domain());
    }

    /// Test CompositionType display names
    #[test]
    fn test_composition_type_display_names() {
        assert_eq!(
            CompositionType::Atomic {
                value_type: "String".to_string(),
            }.display_name(),
            "Atomic String"
        );

        assert_eq!(
            CompositionType::Composite {
                structure_type: "Graph".to_string(),
            }.display_name(),
            "Composite Graph"
        );

        assert_eq!(
            CompositionType::Functor {
                source_type: "List".to_string(),
                target_type: "Tree".to_string(),
            }.display_name(),
            "Functor List → Tree"
        );

        assert_eq!(
            CompositionType::Monad {
                context_type: "Maybe".to_string(),
            }.display_name(),
            "Monad Maybe"
        );

        assert_eq!(
            CompositionType::Domain(DomainCompositionType::Entity {
                entity_type: "Customer".to_string(),
            }).display_name(),
            "Entity: Customer"
        );
    }

    /// Test DomainCompositionType entity classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[DomainCompositionType] -->|is_entity_type| B{Check Type}
    ///     B -->|Entity| C[true]
    ///     B -->|Aggregate| C
    ///     B -->|Event| C
    ///     B -->|Command| C
    ///     B -->|Query| C
    ///     B -->|ValueObject| D[false]
    ///     B -->|Service| D
    /// ```
    #[test]
    fn test_domain_composition_entity_classification() {
        // Entity types
        assert!(DomainCompositionType::Entity { entity_type: "Order".to_string() }.is_entity_type());
        assert!(DomainCompositionType::Aggregate { aggregate_type: "Cart".to_string() }.is_entity_type());
        assert!(DomainCompositionType::Event { event_type: "OrderPlaced".to_string() }.is_entity_type());
        assert!(DomainCompositionType::Command { command_type: "PlaceOrder".to_string() }.is_entity_type());
        assert!(DomainCompositionType::Query { query_type: "GetOrder".to_string() }.is_entity_type());

        // Non-entity types
        assert!(!DomainCompositionType::ValueObject { value_type: "Money".to_string() }.is_entity_type());
        assert!(!DomainCompositionType::Service { service_type: "PaymentService".to_string() }.is_entity_type());
        assert!(!DomainCompositionType::Policy { policy_type: "RefundPolicy".to_string() }.is_entity_type());
        assert!(!DomainCompositionType::BoundedContext { domain: "Sales".to_string() }.is_entity_type());
        assert!(!DomainCompositionType::Workflow { workflow_type: "OrderFlow".to_string() }.is_entity_type());
    }

    /// Test DomainCompositionType value object classification
    #[test]
    fn test_domain_composition_value_object_classification() {
        // Value object types
        assert!(DomainCompositionType::ValueObject { value_type: "Address".to_string() }.is_value_object_type());
        assert!(DomainCompositionType::Policy { policy_type: "DiscountPolicy".to_string() }.is_value_object_type());

        // Non-value object types
        assert!(!DomainCompositionType::Entity { entity_type: "Customer".to_string() }.is_value_object_type());
        assert!(!DomainCompositionType::Service { service_type: "EmailService".to_string() }.is_value_object_type());
        assert!(!DomainCompositionType::Event { event_type: "CustomerCreated".to_string() }.is_value_object_type());
    }

    /// Test DomainCompositionType service classification
    #[test]
    fn test_domain_composition_service_classification() {
        // Service types
        assert!(DomainCompositionType::Service { service_type: "PaymentService".to_string() }.is_service_type());

        // Non-service types
        assert!(!DomainCompositionType::Entity { entity_type: "Order".to_string() }.is_service_type());
        assert!(!DomainCompositionType::ValueObject { value_type: "Money".to_string() }.is_service_type());
        assert!(!DomainCompositionType::Event { event_type: "OrderShipped".to_string() }.is_service_type());
    }

    /// Test DomainCompositionType boundary classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[DomainCompositionType] -->|is_boundary_type| B{Check Type}
    ///     B -->|BoundedContext| C[true]
    ///     B -->|Aggregate| C
    ///     B -->|Entity| D[false]
    ///     B -->|Service| D
    /// ```
    #[test]
    fn test_domain_composition_boundary_classification() {
        // Boundary types
        assert!(DomainCompositionType::BoundedContext { domain: "Inventory".to_string() }.is_boundary_type());
        assert!(DomainCompositionType::Aggregate { aggregate_type: "ShoppingCart".to_string() }.is_boundary_type());

        // Non-boundary types
        assert!(!DomainCompositionType::Entity { entity_type: "Product".to_string() }.is_boundary_type());
        assert!(!DomainCompositionType::Service { service_type: "PricingService".to_string() }.is_boundary_type());
        assert!(!DomainCompositionType::ValueObject { value_type: "Price".to_string() }.is_boundary_type());
    }

    /// Test DomainCompositionType display names
    #[test]
    fn test_domain_composition_display_names() {
        assert_eq!(
            DomainCompositionType::Entity { entity_type: "Customer".to_string() }.display_name(),
            "Entity: Customer"
        );

        assert_eq!(
            DomainCompositionType::ValueObject { value_type: "Email".to_string() }.display_name(),
            "Value Object: Email"
        );

        assert_eq!(
            DomainCompositionType::Aggregate { aggregate_type: "Order".to_string() }.display_name(),
            "Aggregate: Order"
        );

        assert_eq!(
            DomainCompositionType::Service { service_type: "NotificationService".to_string() }.display_name(),
            "Service: NotificationService"
        );

        assert_eq!(
            DomainCompositionType::Event { event_type: "PaymentReceived".to_string() }.display_name(),
            "Event: PaymentReceived"
        );

        assert_eq!(
            DomainCompositionType::Command { command_type: "CancelOrder".to_string() }.display_name(),
            "Command: CancelOrder"
        );

        assert_eq!(
            DomainCompositionType::Query { query_type: "FindOrdersByCustomer".to_string() }.display_name(),
            "Query: FindOrdersByCustomer"
        );

        assert_eq!(
            DomainCompositionType::BoundedContext { domain: "Shipping".to_string() }.display_name(),
            "Bounded Context: Shipping"
        );

        assert_eq!(
            DomainCompositionType::Policy { policy_type: "ReturnPolicy".to_string() }.display_name(),
            "Policy: ReturnPolicy"
        );

        assert_eq!(
            DomainCompositionType::Workflow { workflow_type: "FulfillmentFlow".to_string() }.display_name(),
            "Workflow: FulfillmentFlow"
        );
    }

    /// Test DomainCompositionType base type names
    #[test]
    fn test_domain_composition_base_type_names() {
        assert_eq!(
            DomainCompositionType::Entity { entity_type: "Any".to_string() }.base_type_name(),
            "Entity"
        );
        assert_eq!(
            DomainCompositionType::ValueObject { value_type: "Any".to_string() }.base_type_name(),
            "ValueObject"
        );
        assert_eq!(
            DomainCompositionType::Aggregate { aggregate_type: "Any".to_string() }.base_type_name(),
            "Aggregate"
        );
        assert_eq!(
            DomainCompositionType::Service { service_type: "Any".to_string() }.base_type_name(),
            "Service"
        );
        assert_eq!(
            DomainCompositionType::Event { event_type: "Any".to_string() }.base_type_name(),
            "Event"
        );
        assert_eq!(
            DomainCompositionType::Command { command_type: "Any".to_string() }.base_type_name(),
            "Command"
        );
        assert_eq!(
            DomainCompositionType::Query { query_type: "Any".to_string() }.base_type_name(),
            "Query"
        );
        assert_eq!(
            DomainCompositionType::BoundedContext { domain: "Any".to_string() }.base_type_name(),
            "BoundedContext"
        );
        assert_eq!(
            DomainCompositionType::Policy { policy_type: "Any".to_string() }.base_type_name(),
            "Policy"
        );
        assert_eq!(
            DomainCompositionType::Workflow { workflow_type: "Any".to_string() }.base_type_name(),
            "Workflow"
        );
    }

    /// Test serialization and deserialization
    #[test]
    fn test_serde() {
        // Test CompositionType variants
        let compositions = vec![
            CompositionType::Atomic { value_type: "Boolean".to_string() },
            CompositionType::Composite { structure_type: "List".to_string() },
            CompositionType::Functor {
                source_type: "Option".to_string(),
                target_type: "Result".to_string(),
            },
            CompositionType::Monad { context_type: "IO".to_string() },
            CompositionType::Domain(DomainCompositionType::Entity {
                entity_type: "User".to_string(),
            }),
        ];

        for comp in compositions {
            let json = serde_json::to_string(&comp).unwrap();
            let deserialized: CompositionType = serde_json::from_str(&json).unwrap();
            assert_eq!(comp, deserialized);
        }

        // Test DomainCompositionType variants
        let domain_types = vec![
            DomainCompositionType::Entity { entity_type: "Product".to_string() },
            DomainCompositionType::ValueObject { value_type: "SKU".to_string() },
            DomainCompositionType::Aggregate { aggregate_type: "Inventory".to_string() },
            DomainCompositionType::Service { service_type: "StockService".to_string() },
            DomainCompositionType::Event { event_type: "StockDepleted".to_string() },
            DomainCompositionType::Command { command_type: "RestockItem".to_string() },
            DomainCompositionType::Query { query_type: "GetStockLevel".to_string() },
            DomainCompositionType::BoundedContext { domain: "Warehouse".to_string() },
            DomainCompositionType::Policy { policy_type: "RestockingPolicy".to_string() },
            DomainCompositionType::Workflow { workflow_type: "RestockingFlow".to_string() },
        ];

        for domain_type in domain_types {
            let json = serde_json::to_string(&domain_type).unwrap();
            let deserialized: DomainCompositionType = serde_json::from_str(&json).unwrap();
            assert_eq!(domain_type, deserialized);
        }
    }

    /// Test equality and hashing
    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        // Add different composition types
        set.insert(CompositionType::Atomic { value_type: "Int".to_string() });
        set.insert(CompositionType::Composite { structure_type: "Map".to_string() });
        set.insert(CompositionType::Domain(DomainCompositionType::Entity {
            entity_type: "Order".to_string(),
        }));

        assert_eq!(set.len(), 3);

        // Same composition should not increase size
        set.insert(CompositionType::Atomic { value_type: "Int".to_string() });
        assert_eq!(set.len(), 3);

        // Different value type should increase size
        set.insert(CompositionType::Atomic { value_type: "Float".to_string() });
        assert_eq!(set.len(), 4);
    }

    /// Test functor composition type
    #[test]
    fn test_functor_composition() {
        let functor = CompositionType::Functor {
            source_type: "List<A>".to_string(),
            target_type: "Option<B>".to_string(),
        };

        assert!(!functor.is_atomic());
        assert!(!functor.is_composite());
        assert!(!functor.is_domain());
        assert_eq!(functor.display_name(), "Functor List<A> → Option<B>");
    }

    /// Test monad composition type
    #[test]
    fn test_monad_composition() {
        let monad = CompositionType::Monad {
            context_type: "Result<T, E>".to_string(),
        };

        assert!(!monad.is_atomic());
        assert!(!monad.is_composite());
        assert!(!monad.is_domain());
        assert_eq!(monad.display_name(), "Monad Result<T, E>");
    }

    /// Test overlapping classifications
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Aggregate] -->|is_entity_type| B[true]
    ///     A -->|is_boundary_type| C[true]
    ///     D[Policy] -->|is_value_object_type| E[true]
    ///     D -->|is_entity_type| F[false]
    /// ```
    #[test]
    fn test_overlapping_classifications() {
        let aggregate = DomainCompositionType::Aggregate {
            aggregate_type: "ShoppingCart".to_string(),
        };

        // Aggregate is both entity type and boundary type
        assert!(aggregate.is_entity_type());
        assert!(aggregate.is_boundary_type());
        assert!(!aggregate.is_value_object_type());
        assert!(!aggregate.is_service_type());

        let policy = DomainCompositionType::Policy {
            policy_type: "PricingPolicy".to_string(),
        };

        // Policy is value object type
        assert!(policy.is_value_object_type());
        assert!(!policy.is_entity_type());
        assert!(!policy.is_boundary_type());
        assert!(!policy.is_service_type());
    }
}
