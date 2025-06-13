//! # CIM Domain
//!
//! Core Domain-Driven Design (DDD) components and traits for the Composable Information Machine.
//!
//! This crate provides the fundamental building blocks for implementing DDD patterns:
//! - **Component**: Trait for attachable components with type erasure
//! - **Entity**: Types with identity and lifecycle
//! - **Value Objects**: Immutable types defined by their attributes
//! - **Aggregates**: Consistency boundaries with root entities
//! - **Domain Events**: Things that happen in the domain
//! - **Commands**: Requests to change state (return only acknowledgments)
//! - **Queries**: Requests to read state (return only acknowledgments)
//! - **State Machines**: Enum-based state management with controlled transitions
//!
//! ## Design Principles
//!
//! 1. **Type Safety**: Use phantom types for compile-time guarantees
//! 2. **Immutability**: Value objects are immutable by design
//! 3. **Identity**: Entities have globally unique, persistent identities
//! 4. **Composition**: Build complex types from simple components
//! 5. **Domain Alignment**: Types reflect business concepts, not technical details
//! 6. **Event-Driven**: Commands and queries produce event streams, not direct results
//! 7. **Controlled State**: Enums restrict states and transitions to valid options

#![warn(missing_docs)]

mod component;
mod entity;
mod identifiers;
mod node_types;
mod relationship_types;
mod context_types;
mod composition_types;
mod cqrs;
mod errors;
mod state_machine;
mod events;
mod domain_events;
mod commands;
mod command_handlers;
mod query_handlers;
mod bevy_bridge;
mod location;
mod agent;
mod policy;
mod document;
mod concept_graph;
pub mod domain_graph;
pub mod workflow;
pub mod infrastructure;
pub mod projections;

// Re-export core types
pub use component::{Component, ComponentStorage};
pub use entity::{Entity, EntityId, AggregateRoot};
pub use identifiers::{NodeId, EdgeId, GraphId, StateId, TransitionId, WorkflowId};
pub use node_types::NodeType;
pub use relationship_types::RelationshipType;
pub use context_types::{ContextType, SubdomainType, ServiceType};
pub use composition_types::{CompositionType, DomainCompositionType};
pub use cqrs::{
    Command, Query, CommandId, QueryId, EventId,
    CommandEnvelope, QueryEnvelope,
    CommandHandler, QueryHandler as CqrsQueryHandler,
    CorrelationId, CausationId, IdType,
    CommandStatus, QueryStatus,
    CommandAcknowledgment, QueryAcknowledgment,
    EventStreamSubscription,
};
pub use errors::{DomainError, DomainResult};
pub use state_machine::{
    State, MooreStateTransitions, MealyStateTransitions,
    MooreMachine, MealyMachine,
    StateTransition, TransitionInput, TransitionOutput,
    EventOutput, EmptyInput, CommandInput,
    DocumentState,
};

// Re-export from cim-subject crate
pub use cim_subject::{
    Subject as SubjectParts, // Maintain backward compatibility
    Pattern as SubjectPattern,
    Permissions as SubjectPermissions,
    SubjectParser,
    MessageTranslator,
};

// Keep these types that are specific to cim-domain
pub use events::{
    PropagationScope, EventEnvelope,
};

pub use events::{
    DomainEvent, EventMetadata, DomainEventEnvelope,
    DomainEventEnvelopeWithMetadata,
    AgentDeployed,
    LocationDefined, PolicyEnacted,
    AgentActivated, AgentSuspended, AgentWentOffline, AgentDecommissioned,
    AgentCapabilitiesAdded, AgentCapabilitiesRemoved,
    AgentPermissionsGranted, AgentPermissionsRevoked,
    AgentToolsEnabled, AgentToolsDisabled,
    AgentConfigurationRemoved, AgentConfigurationSet,
    PolicySubmittedForApproval, PolicyApproved, PolicyRejected,
    PolicySuspended, PolicyReactivated, PolicySuperseded, PolicyArchived,
    PolicyExternalApprovalRequested, PolicyExternalApprovalReceived,
    DocumentUploaded, DocumentClassified, DocumentOwnershipAssigned,
    DocumentAccessControlSet, DocumentStatusSet, DocumentProcessed,
    DocumentRelationshipAdded, DocumentRelationshipRemoved,
    DocumentVersionCreated, DocumentArchived,
};
pub use commands::{
    DeployAgent, UpdateAgentCapabilities,
    DefineLocation, EnactPolicy, UpdatePolicyRules,
    ActivateAgent, SuspendAgent, SetAgentOffline, DecommissionAgent,
    GrantAgentPermissions, RevokeAgentPermissions,
    EnableAgentTools, DisableAgentTools, UpdateAgentConfiguration,
    SubmitPolicyForApproval, ApprovePolicy, RejectPolicy,
    SuspendPolicy, ReactivatePolicy, SupersedePolicy, ArchivePolicy,
    RequestPolicyExternalApproval, RecordPolicyExternalApproval,
    UploadDocument, ClassifyDocument, AssignDocumentOwnership,
    SetDocumentAccessControl, SetDocumentStatus, ProcessDocument,
    AddDocumentRelationship, RemoveDocumentRelationship,
    CreateDocumentVersion, ArchiveDocument,
};
pub use command_handlers::{
    EventPublisher, MockEventPublisher,
    AggregateRepository, InMemoryRepository,
    AgentCommandHandler,
    LocationCommandHandler, PolicyCommandHandler, DocumentCommandHandler,
    WorkflowCommandHandler,
};
pub use query_handlers::{
    DirectQueryHandler, QueryResult, ReadModelStorage, InMemoryReadModel, QueryCriteria,

    LocationView, FindLocationsByType, LocationQueryHandler,
    PolicyView, FindActivePolicies, PolicyQueryHandler,
    DocumentView, SearchDocuments, DocumentQueryHandler,
    AgentView, FindAgentsByCapability, AgentQueryHandler,
    WorkflowView, FindWorkflowsByStatus, WorkflowQueryHandler,
};
pub use bevy_bridge::{
    BevyCommand, BevyEvent, ComponentData,
    NatsToBevyTranslator, BevyEventRouter,
    NatsMessage, TranslationError,
};
pub use location::{
    Location, LocationMarker, LocationType,
    Address, GeoCoordinates, VirtualLocation,
};


pub use agent::{
    Agent, AgentMarker,
    AgentType, AgentStatus,
    CapabilitiesComponent, AuthenticationComponent, AuthMethod,
    PermissionsComponent, ToolAccessComponent, ToolDefinition, ToolUsageStats,
    ConfigurationComponent, AgentMetadata,
};
pub use policy::{
    Policy, PolicyMarker,
    PolicyType, PolicyStatus, PolicyScope,
    RulesComponent, ApprovalRequirementsComponent, ExternalApprovalRequirement,
    ApprovalStateComponent, Approval, PendingExternalApproval, ExternalVerification, Rejection,
    EnforcementComponent, EnforcementMode, ViolationAction, ViolationSeverity, PolicyException,
    PolicyMetadata,
};
pub use document::{
    Document, DocumentMarker,
    DocumentInfoComponent, ContentAddressComponent, ClassificationComponent,
    ConfidentialityLevel, OwnershipComponent, LifecycleComponent, DocumentStatus,
    AccessControlComponent, RelationshipsComponent, DocumentRelation, RelationType,
    ExternalReference, ProcessingComponent, ThumbnailInfo,
    PublicDocumentView, SearchIndexProjection,
};
pub use concept_graph::{
    ConceptGraph, ConceptGraphMarker,
    GraphMetadataComponent, GraphPurpose, ConceptNodeComponent, ConceptType,
    SourceReference, ConceptRelationshipComponent, ConceptRelationshipType,
    TemporalRelation, CausalRelation, ConceptualSpaceMappingComponent,
    ConceptualDimension, DimensionType, ConceptualPosition,
    LayoutConfigComponent, LayoutAlgorithm, AssemblyRulesComponent,
    InclusionRule, Condition, ConditionOperator, ConceptMapping,
    RelationshipRule, RelationshipDetection, FilterCriteria, FilterTarget,
    ConceptGraphView, ConceptNodeView, ConceptRelationshipView,
    LayoutInfo, BoundingBox,
};
pub use domain_events::{
    DomainEventEnum,
    WorkflowStarted, WorkflowTransitionExecuted, WorkflowCompleted,
    WorkflowSuspended, WorkflowResumed, WorkflowCancelled, WorkflowFailed,
};

// Re-export common marker types
pub mod markers {
    //! Marker types for phantom type parameters
    pub use crate::entity::{
        GraphMarker, AggregateMarker, BoundedContextMarker,
        EntityMarker, ValueObjectMarker, ServiceMarker,
        EventMarker, CommandMarker, QueryMarker
    };
    pub use crate::location::LocationMarker;
    pub use crate::agent::AgentMarker;
    pub use crate::policy::PolicyMarker;
    pub use crate::document::DocumentMarker;
    pub use crate::concept_graph::ConceptGraphMarker;
}


