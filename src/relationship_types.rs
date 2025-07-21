// Copyright 2025 Cowboy AI, LLC.

//! Relationship types for edges in domain graphs

use serde::{Deserialize, Serialize};

/// Types of relationships (edges) between nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    // Structural relationships
    /// One object contains another
    Contains,
    /// One object is part of another
    PartOf,
    /// One object references another
    References,
    /// One object depends on another
    DependsOn,
    /// One object uses another
    Uses,
    /// One object implements another
    Implements,
    /// One object extends another
    Extends,
    /// One object inherits from another
    InheritsFrom,

    // Behavioral relationships
    /// One object calls/invokes another
    Calls,
    /// One object sends message to another
    SendsTo,
    /// One object receives from another
    ReceivesFrom,
    /// One object triggers another
    Triggers,
    /// One object produces another
    Produces,
    /// One object consumes another
    Consumes,
    /// One object transforms into another
    TransformsTo,

    // Temporal relationships
    /// One thing happens before another
    Before,
    /// One thing happens after another
    After,
    /// Things happen concurrently
    Concurrent,
    /// One thing blocks another
    Blocks,
    /// One thing waits for another
    WaitsFor,

    // Domain relationships
    /// Aggregate contains entity
    AggregateContains,
    /// Entity has value object
    HasValue,
    /// Command affects aggregate
    CommandTargets,
    /// Event sourced from aggregate
    EventFrom,
    /// Query reads from
    QueryReads,
    /// Policy applies to
    PolicyAppliesTo,

    // Workflow relationships
    /// Sequential flow
    FlowsTo,
    /// Conditional flow
    ConditionalFlow {
        /// The condition that must be met for this flow to be taken
        condition: String,
    },
    /// Parallel flow
    ParallelFlow,
    /// Loop back
    LoopBack,

    // Context relationships
    /// Shared kernel between contexts
    SharedKernel,
    /// Customer-supplier relationship
    CustomerSupplier,
    /// Conformist relationship
    Conformist,
    /// Anti-corruption layer
    AntiCorruptionLayer,
    /// Open host service
    OpenHostService,
    /// Published language
    PublishedLanguage,

    // Development relationships
    /// Feature depends on another
    FeatureDependency,
    /// Task blocks another
    TaskBlocks,
    /// Milestone contains tasks
    MilestoneContains,

    // Custom relationship
    /// Domain-specific relationship
    Custom(String),
}

impl RelationshipType {
    /// Check if this is a containment relationship
    pub fn is_containment(&self) -> bool {
        matches!(
            self,
            RelationshipType::Contains
                | RelationshipType::PartOf
                | RelationshipType::AggregateContains
                | RelationshipType::HasValue
                | RelationshipType::MilestoneContains
        )
    }

    /// Check if this is a dependency relationship
    pub fn is_dependency(&self) -> bool {
        matches!(
            self,
            RelationshipType::DependsOn
                | RelationshipType::Uses
                | RelationshipType::References
                | RelationshipType::WaitsFor
                | RelationshipType::FeatureDependency
        )
    }

    /// Check if this is a behavioral relationship
    pub fn is_behavioral(&self) -> bool {
        matches!(
            self,
            RelationshipType::Calls
                | RelationshipType::SendsTo
                | RelationshipType::ReceivesFrom
                | RelationshipType::Triggers
                | RelationshipType::Produces
                | RelationshipType::Consumes
                | RelationshipType::TransformsTo
        )
    }

    /// Check if this is a temporal relationship
    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            RelationshipType::Before
                | RelationshipType::After
                | RelationshipType::Concurrent
                | RelationshipType::Blocks
                | RelationshipType::WaitsFor
                | RelationshipType::TaskBlocks
        )
    }

    /// Check if this is a context boundary relationship
    pub fn is_context_boundary(&self) -> bool {
        matches!(
            self,
            RelationshipType::SharedKernel
                | RelationshipType::CustomerSupplier
                | RelationshipType::Conformist
                | RelationshipType::AntiCorruptionLayer
                | RelationshipType::OpenHostService
                | RelationshipType::PublishedLanguage
        )
    }

    /// Get a human-readable name for this relationship
    pub fn display_name(&self) -> String {
        match self {
            RelationshipType::Contains => "contains".to_string(),
            RelationshipType::PartOf => "part of".to_string(),
            RelationshipType::References => "references".to_string(),
            RelationshipType::DependsOn => "depends on".to_string(),
            RelationshipType::Uses => "uses".to_string(),
            RelationshipType::Implements => "implements".to_string(),
            RelationshipType::Extends => "extends".to_string(),
            RelationshipType::InheritsFrom => "inherits from".to_string(),
            RelationshipType::Calls => "calls".to_string(),
            RelationshipType::SendsTo => "sends to".to_string(),
            RelationshipType::ReceivesFrom => "receives from".to_string(),
            RelationshipType::Triggers => "triggers".to_string(),
            RelationshipType::Produces => "produces".to_string(),
            RelationshipType::Consumes => "consumes".to_string(),
            RelationshipType::TransformsTo => "transforms to".to_string(),
            RelationshipType::Before => "before".to_string(),
            RelationshipType::After => "after".to_string(),
            RelationshipType::Concurrent => "concurrent with".to_string(),
            RelationshipType::Blocks => "blocks".to_string(),
            RelationshipType::WaitsFor => "waits for".to_string(),
            RelationshipType::AggregateContains => "aggregate contains".to_string(),
            RelationshipType::HasValue => "has value".to_string(),
            RelationshipType::CommandTargets => "command targets".to_string(),
            RelationshipType::EventFrom => "event from".to_string(),
            RelationshipType::QueryReads => "query reads".to_string(),
            RelationshipType::PolicyAppliesTo => "policy applies to".to_string(),
            RelationshipType::FlowsTo => "flows to".to_string(),
            RelationshipType::ConditionalFlow { condition } => format!("flows to if {condition}"),
            RelationshipType::ParallelFlow => "parallel flow".to_string(),
            RelationshipType::LoopBack => "loops back".to_string(),
            RelationshipType::SharedKernel => "shared kernel".to_string(),
            RelationshipType::CustomerSupplier => "customer-supplier".to_string(),
            RelationshipType::Conformist => "conformist".to_string(),
            RelationshipType::AntiCorruptionLayer => "anti-corruption layer".to_string(),
            RelationshipType::OpenHostService => "open host service".to_string(),
            RelationshipType::PublishedLanguage => "published language".to_string(),
            RelationshipType::FeatureDependency => "feature depends on".to_string(),
            RelationshipType::TaskBlocks => "task blocks".to_string(),
            RelationshipType::MilestoneContains => "milestone contains".to_string(),
            RelationshipType::Custom(name) => name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test containment relationship classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[RelationshipType] -->|is_containment| B{Containment?}
    ///     B -->|Yes| C[Containment Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[Contains, PartOf, AggregateContains, HasValue, MilestoneContains]
    /// ```
    #[test]
    fn test_is_containment() {
        // Containment relationships
        assert!(RelationshipType::Contains.is_containment());
        assert!(RelationshipType::PartOf.is_containment());
        assert!(RelationshipType::AggregateContains.is_containment());
        assert!(RelationshipType::HasValue.is_containment());
        assert!(RelationshipType::MilestoneContains.is_containment());

        // Non-containment relationships
        assert!(!RelationshipType::References.is_containment());
        assert!(!RelationshipType::DependsOn.is_containment());
        assert!(!RelationshipType::Calls.is_containment());
        assert!(!RelationshipType::Before.is_containment());
        assert!(!RelationshipType::SharedKernel.is_containment());
        assert!(!RelationshipType::Custom("Test".to_string()).is_containment());
    }

    /// Test dependency relationship classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[RelationshipType] -->|is_dependency| B{Dependency?}
    ///     B -->|Yes| C[Dependency Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[DependsOn, Uses, References, WaitsFor, FeatureDependency]
    /// ```
    #[test]
    fn test_is_dependency() {
        // Dependency relationships
        assert!(RelationshipType::DependsOn.is_dependency());
        assert!(RelationshipType::Uses.is_dependency());
        assert!(RelationshipType::References.is_dependency());
        assert!(RelationshipType::WaitsFor.is_dependency());
        assert!(RelationshipType::FeatureDependency.is_dependency());

        // Non-dependency relationships
        assert!(!RelationshipType::Contains.is_dependency());
        assert!(!RelationshipType::Calls.is_dependency());
        assert!(!RelationshipType::Before.is_dependency());
        assert!(!RelationshipType::SharedKernel.is_dependency());
        assert!(!RelationshipType::Custom("Test".to_string()).is_dependency());
    }

    /// Test behavioral relationship classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[RelationshipType] -->|is_behavioral| B{Behavioral?}
    ///     B -->|Yes| C[Behavioral Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[Calls, SendsTo, ReceivesFrom, Triggers, Produces, Consumes, TransformsTo]
    /// ```
    #[test]
    fn test_is_behavioral() {
        // Behavioral relationships
        assert!(RelationshipType::Calls.is_behavioral());
        assert!(RelationshipType::SendsTo.is_behavioral());
        assert!(RelationshipType::ReceivesFrom.is_behavioral());
        assert!(RelationshipType::Triggers.is_behavioral());
        assert!(RelationshipType::Produces.is_behavioral());
        assert!(RelationshipType::Consumes.is_behavioral());
        assert!(RelationshipType::TransformsTo.is_behavioral());

        // Non-behavioral relationships
        assert!(!RelationshipType::Contains.is_behavioral());
        assert!(!RelationshipType::DependsOn.is_behavioral());
        assert!(!RelationshipType::Before.is_behavioral());
        assert!(!RelationshipType::SharedKernel.is_behavioral());
        assert!(!RelationshipType::Custom("Test".to_string()).is_behavioral());
    }

    /// Test temporal relationship classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[RelationshipType] -->|is_temporal| B{Temporal?}
    ///     B -->|Yes| C[Temporal Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[Before, After, Concurrent, Blocks, WaitsFor, TaskBlocks]
    /// ```
    #[test]
    fn test_is_temporal() {
        // Temporal relationships
        assert!(RelationshipType::Before.is_temporal());
        assert!(RelationshipType::After.is_temporal());
        assert!(RelationshipType::Concurrent.is_temporal());
        assert!(RelationshipType::Blocks.is_temporal());
        assert!(RelationshipType::WaitsFor.is_temporal());
        assert!(RelationshipType::TaskBlocks.is_temporal());

        // Non-temporal relationships
        assert!(!RelationshipType::Contains.is_temporal());
        assert!(!RelationshipType::DependsOn.is_temporal());
        assert!(!RelationshipType::Calls.is_temporal());
        assert!(!RelationshipType::SharedKernel.is_temporal());
        assert!(!RelationshipType::Custom("Test".to_string()).is_temporal());
    }

    /// Test context boundary relationship classification
    ///
    /// ```mermaid
    /// graph TD
    ///     A[RelationshipType] -->|is_context_boundary| B{Context Boundary?}
    ///     B -->|Yes| C[Context Types]
    ///     B -->|No| D[Other Types]
    ///     C --> E[SharedKernel, CustomerSupplier, Conformist, etc.]
    /// ```
    #[test]
    fn test_is_context_boundary() {
        // Context boundary relationships
        assert!(RelationshipType::SharedKernel.is_context_boundary());
        assert!(RelationshipType::CustomerSupplier.is_context_boundary());
        assert!(RelationshipType::Conformist.is_context_boundary());
        assert!(RelationshipType::AntiCorruptionLayer.is_context_boundary());
        assert!(RelationshipType::OpenHostService.is_context_boundary());
        assert!(RelationshipType::PublishedLanguage.is_context_boundary());

        // Non-context boundary relationships
        assert!(!RelationshipType::Contains.is_context_boundary());
        assert!(!RelationshipType::DependsOn.is_context_boundary());
        assert!(!RelationshipType::Calls.is_context_boundary());
        assert!(!RelationshipType::Before.is_context_boundary());
        assert!(!RelationshipType::Custom("Test".to_string()).is_context_boundary());
    }

    /// Test display names
    #[test]
    fn test_display_name() {
        // Structural relationships
        assert_eq!(RelationshipType::Contains.display_name(), "contains");
        assert_eq!(RelationshipType::PartOf.display_name(), "part of");
        assert_eq!(RelationshipType::References.display_name(), "references");
        assert_eq!(RelationshipType::DependsOn.display_name(), "depends on");

        // Behavioral relationships
        assert_eq!(RelationshipType::Calls.display_name(), "calls");
        assert_eq!(RelationshipType::SendsTo.display_name(), "sends to");
        assert_eq!(RelationshipType::Triggers.display_name(), "triggers");

        // Temporal relationships
        assert_eq!(RelationshipType::Before.display_name(), "before");
        assert_eq!(RelationshipType::After.display_name(), "after");
        assert_eq!(
            RelationshipType::Concurrent.display_name(),
            "concurrent with"
        );

        // Domain relationships
        assert_eq!(
            RelationshipType::AggregateContains.display_name(),
            "aggregate contains"
        );
        assert_eq!(
            RelationshipType::CommandTargets.display_name(),
            "command targets"
        );

        // Workflow relationships
        assert_eq!(RelationshipType::FlowsTo.display_name(), "flows to");
        assert_eq!(
            RelationshipType::ConditionalFlow {
                condition: "x > 0".to_string()
            }
            .display_name(),
            "flows to if x > 0"
        );

        // Context relationships
        assert_eq!(
            RelationshipType::SharedKernel.display_name(),
            "shared kernel"
        );
        assert_eq!(
            RelationshipType::CustomerSupplier.display_name(),
            "customer-supplier"
        );

        // Custom relationship
        assert_eq!(
            RelationshipType::Custom("MyRelation".to_string()).display_name(),
            "MyRelation"
        );
    }

    /// Test serialization and deserialization
    #[test]
    fn test_serde() {
        let test_cases = vec![
            RelationshipType::Contains,
            RelationshipType::DependsOn,
            RelationshipType::Calls,
            RelationshipType::Before,
            RelationshipType::SharedKernel,
            RelationshipType::ConditionalFlow {
                condition: "test".to_string(),
            },
            RelationshipType::Custom("TestRelation".to_string()),
        ];

        for relationship in test_cases {
            // Serialize
            let json = serde_json::to_string(&relationship).unwrap();

            // Deserialize
            let deserialized: RelationshipType = serde_json::from_str(&json).unwrap();

            // Verify equality
            assert_eq!(relationship, deserialized);
        }
    }

    /// Test conditional flow with different conditions
    #[test]
    fn test_conditional_flow() {
        let flow1 = RelationshipType::ConditionalFlow {
            condition: "x > 0".to_string(),
        };
        let flow2 = RelationshipType::ConditionalFlow {
            condition: "x > 0".to_string(),
        };
        let flow3 = RelationshipType::ConditionalFlow {
            condition: "y < 10".to_string(),
        };

        // Equality
        assert_eq!(flow1, flow2);
        assert_ne!(flow1, flow3);

        // Display names
        assert_eq!(flow1.display_name(), "flows to if x > 0");
        assert_eq!(flow3.display_name(), "flows to if y < 10");

        // Classification
        assert!(!flow1.is_containment());
        assert!(!flow1.is_dependency());
        assert!(!flow1.is_behavioral());
        assert!(!flow1.is_temporal());
        assert!(!flow1.is_context_boundary());
    }

    /// Test custom relationship behavior
    #[test]
    fn test_custom_relationship() {
        let custom1 = RelationshipType::Custom("RelationA".to_string());
        let custom2 = RelationshipType::Custom("RelationA".to_string());
        let custom3 = RelationshipType::Custom("RelationB".to_string());

        // Equality
        assert_eq!(custom1, custom2);
        assert_ne!(custom1, custom3);

        // Display name
        assert_eq!(custom1.display_name(), "RelationA");
        assert_eq!(custom3.display_name(), "RelationB");

        // Classification - custom relationships don't fall into predefined categories
        assert!(!custom1.is_containment());
        assert!(!custom1.is_dependency());
        assert!(!custom1.is_behavioral());
        assert!(!custom1.is_temporal());
        assert!(!custom1.is_context_boundary());
    }

    /// Test classification coverage
    #[test]
    fn test_classification_coverage() {
        let all_relationships = vec![
            // Structural
            RelationshipType::Contains,
            RelationshipType::PartOf,
            RelationshipType::References,
            RelationshipType::DependsOn,
            RelationshipType::Uses,
            RelationshipType::Implements,
            RelationshipType::Extends,
            RelationshipType::InheritsFrom,
            // Behavioral
            RelationshipType::Calls,
            RelationshipType::SendsTo,
            RelationshipType::ReceivesFrom,
            RelationshipType::Triggers,
            RelationshipType::Produces,
            RelationshipType::Consumes,
            RelationshipType::TransformsTo,
            // Temporal
            RelationshipType::Before,
            RelationshipType::After,
            RelationshipType::Concurrent,
            RelationshipType::Blocks,
            RelationshipType::WaitsFor,
            // Domain
            RelationshipType::AggregateContains,
            RelationshipType::HasValue,
            RelationshipType::CommandTargets,
            RelationshipType::EventFrom,
            RelationshipType::QueryReads,
            RelationshipType::PolicyAppliesTo,
            // Workflow
            RelationshipType::FlowsTo,
            RelationshipType::ConditionalFlow {
                condition: "test".to_string(),
            },
            RelationshipType::ParallelFlow,
            RelationshipType::LoopBack,
            // Context
            RelationshipType::SharedKernel,
            RelationshipType::CustomerSupplier,
            RelationshipType::Conformist,
            RelationshipType::AntiCorruptionLayer,
            RelationshipType::OpenHostService,
            RelationshipType::PublishedLanguage,
            // Development
            RelationshipType::FeatureDependency,
            RelationshipType::TaskBlocks,
            RelationshipType::MilestoneContains,
            // Custom
            RelationshipType::Custom("Test".to_string()),
        ];

        // Verify each relationship has a display name
        for relationship in &all_relationships {
            assert!(!relationship.display_name().is_empty());
        }

        // Count classifications
        let containment_count = all_relationships
            .iter()
            .filter(|r| r.is_containment())
            .count();
        let dependency_count = all_relationships
            .iter()
            .filter(|r| r.is_dependency())
            .count();
        let behavioral_count = all_relationships
            .iter()
            .filter(|r| r.is_behavioral())
            .count();
        let temporal_count = all_relationships.iter().filter(|r| r.is_temporal()).count();
        let context_count = all_relationships
            .iter()
            .filter(|r| r.is_context_boundary())
            .count();

        // Verify expected counts
        assert_eq!(containment_count, 5);
        assert_eq!(dependency_count, 5);
        assert_eq!(behavioral_count, 7);
        assert_eq!(temporal_count, 6);
        assert_eq!(context_count, 6);
    }

    /// Test hash and equality implementation
    #[test]
    fn test_hash_and_eq() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        // Add various relationships
        set.insert(RelationshipType::Contains);
        set.insert(RelationshipType::DependsOn);
        set.insert(RelationshipType::Custom("Relation1".to_string()));

        // Verify contains
        assert!(set.contains(&RelationshipType::Contains));
        assert!(set.contains(&RelationshipType::DependsOn));
        assert!(set.contains(&RelationshipType::Custom("Relation1".to_string())));

        // Verify not contains
        assert!(!set.contains(&RelationshipType::PartOf));
        assert!(!set.contains(&RelationshipType::Custom("Relation2".to_string())));

        // Add duplicate - should not increase size
        let original_len = set.len();
        set.insert(RelationshipType::Contains);
        assert_eq!(set.len(), original_len);
    }

    /// Test overlapping classifications
    #[test]
    fn test_overlapping_classifications() {
        // WaitsFor appears in both dependency and temporal
        assert!(RelationshipType::WaitsFor.is_dependency());
        assert!(RelationshipType::WaitsFor.is_temporal());

        // TaskBlocks is only temporal
        assert!(RelationshipType::TaskBlocks.is_temporal());
        assert!(!RelationshipType::TaskBlocks.is_dependency());

        // Most relationships belong to only one category
        assert!(RelationshipType::Contains.is_containment());
        assert!(!RelationshipType::Contains.is_dependency());
        assert!(!RelationshipType::Contains.is_behavioral());
        assert!(!RelationshipType::Contains.is_temporal());
        assert!(!RelationshipType::Contains.is_context_boundary());
    }
}
