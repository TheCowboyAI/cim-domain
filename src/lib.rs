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
mod component_sync;
mod domain_component_bridge;
mod entity;
pub mod identifiers;
mod node_types;
mod relationship_types;
mod context_types;
mod composition_types;
mod cqrs;
mod errors;
pub mod state_machine;
mod events;
mod domain_events;
mod commands;
mod command_handlers;
mod query_handlers;
mod event_handler;
// Location module has been extracted to cim-domain-location
// Graph modules have been extracted to cim-domain-graph
// Workflow module has been extracted to cim-domain-workflow
pub mod infrastructure;
pub mod projections;
pub mod category;
pub mod composition;
pub mod domain;
pub mod integration;

// Re-export core types
pub use component::{Component, ComponentStorage, ComponentExt, EcsComponentData, ComponentEvent};
pub use component_sync::DomainComponentSync;
pub use domain_component_bridge::DomainComponentBridge;
pub use entity::{Entity, EntityId, AggregateRoot};
pub use identifiers::{NodeId, EdgeId, GraphId, StateId, TransitionId, WorkflowId, WorkflowIdExt};
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
    CommandAcknowledgment, QueryAcknowledgment, QueryResponse,
    EventStreamSubscription,
};

// Export QueryHandler without alias for compatibility
pub use cqrs::QueryHandler;

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
};
// Location commands have been extracted to cim-domain-location
pub use commands::{DomainCommand, AcknowledgeCommand};
pub use command_handlers::{
    EventPublisher, MockEventPublisher,
    AggregateRepository, InMemoryRepository,
};
pub use query_handlers::{
    DirectQueryHandler, QueryResult, ReadModelStorage, InMemoryReadModel, QueryCriteria,
};
// Location types have been extracted to cim-domain-location

// ConceptGraph types have been extracted to cim-domain-graph
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
    // LocationMarker has been moved to cim-domain-location
    // ConceptGraphMarker has been moved to cim-domain-graph
}

// Export infrastructure types that domains need
pub use infrastructure::event_replay::EventHandler as ReplayEventHandler;
pub use event_handler::EventHandler;

/// Type alias for aggregate identifiers using EntityId with AggregateMarker
pub type AggregateId = EntityId<markers::AggregateMarker>;


