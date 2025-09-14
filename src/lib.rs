// Copyright 2025 Cowboy AI, LLC.

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

mod command_handlers;
mod commands;
mod component;
mod composition_types;
mod context_types;
mod cqrs;
mod domain_events;
mod entity;
mod errors;
mod event_handler;
mod events;
pub mod identifiers;
mod node_types;
mod query_handlers;
mod relationship_types;
pub mod state_machine;
// Subject abstraction layer
pub mod subject_abstraction;
// Location module has been extracted to cim-domain-location
// Graph modules have been extracted to cim-domain-graph
// Workflow module has been extracted to cim-domain-workflow
pub mod category;
pub mod composition;
pub mod domain;
pub mod infrastructure;
pub mod integration;
pub mod persistence;
pub mod projections;

// FP FOUNDATION - Entity as MONAD and Formal Domain Structure
pub mod fp_monad;
pub mod formal_domain;

// Core domain concepts
pub mod cid;

// FP-aligned JSON schemas
pub mod fp_schemas;

// Re-export core types
pub use component::{Component, ComponentEvent, ComponentExt, ComponentStorage, EcsComponentData};
pub use composition_types::{CompositionType, DomainCompositionType};
pub use context_types::{ContextType, ServiceType, SubdomainType};
pub use cqrs::{
    CausationId, Command, CommandAcknowledgment, CommandEnvelope, CommandHandler, CommandId,
    CommandStatus, CorrelationId, EventId, EventStreamSubscription, IdType, Query,
    QueryAcknowledgment, QueryEnvelope, QueryHandler as CqrsQueryHandler, QueryId, QueryResponse,
    QueryStatus,
};
pub use entity::{AggregateRoot, DomainEntity, Entity, EntityId};
pub use identifiers::{EdgeId, GraphId, NodeId, StateId, TransitionId, WorkflowId, WorkflowIdExt};
pub use node_types::NodeType;
pub use relationship_types::RelationshipType;

// Export QueryHandler without alias for compatibility
pub use cqrs::QueryHandler;

pub use errors::{DomainError, DomainResult};
pub use state_machine::{
    CommandInput, DocumentState, EmptyInput, EventOutput, MealyMachine, MealyStateTransitions,
    MooreMachine, MooreStateTransitions, State, StateTransition, TransitionInput, TransitionOutput,
};

// Re-export subject types from abstraction layer
pub use subject_abstraction::{
    MessageTranslator,
    Pattern as SubjectPattern,
    Subject as SubjectParts, // Maintain backward compatibility
    SubjectParser,
    SubjectPermissions,
};

// Keep these types that are specific to cim-domain
pub use events::{EventEnvelope, PropagationScope};

pub use events::{
    DomainEvent, DomainEventEnvelope, DomainEventEnvelopeWithMetadata, EventMetadata,
};
// Location commands have been extracted to cim-domain-location
pub use command_handlers::{
    AggregateRepository, EventPublisher, InMemoryRepository, MockEventPublisher,
};
pub use commands::{AcknowledgeCommand, DomainCommand};
pub use query_handlers::{
    DirectQueryHandler, InMemoryReadModel, QueryCriteria, QueryResult, ReadModelStorage,
};
// Location types have been extracted to cim-domain-location

// ConceptGraph types have been extracted to cim-domain-graph
pub use domain_events::{
    DomainEventEnum, WorkflowCancelled, WorkflowCompleted, WorkflowFailed, WorkflowResumed,
    WorkflowStarted, WorkflowSuspended, WorkflowTransitionExecuted,
};

// Re-export common marker types
pub mod markers {
    //! Marker types for phantom type parameters
    pub use crate::entity::{
        AggregateMarker, BoundedContextMarker, CommandMarker, EntityMarker, EventMarker,
        GraphMarker, QueryMarker, ServiceMarker, ValueObjectMarker,
    };
    // LocationMarker has been moved to cim-domain-location
    // ConceptGraphMarker has been moved to cim-domain-graph
}

// Export infrastructure types that domains need
pub use event_handler::EventHandler;
pub use infrastructure::event_replay::EventHandler as ReplayEventHandler;

/// Type alias for aggregate identifiers using EntityId with AggregateMarker
pub type AggregateId = EntityId<markers::AggregateMarker>;

// ============================================================================
// FP FOUNDATION EXPORTS
// ============================================================================

// Re-export Entity monad and helpers
pub use fp_monad::{Entity as EntityMonad, Components, run_entity, KleisliArrow};

// Re-export CID types
pub use cid::{DomainCid, ContentType, CidChain, generate_cid, CidImpl as Cid, CidError};

// Re-export formal domain traits
pub use formal_domain::{
    // Marker traits (REQUIRED for all domain concepts)
    DomainConcept, ValueObject, FormalDomainEntity, FormalEntityId,
    Aggregate, Policy, Saga,
    
    // State machines
    MealyStateMachine,
    
    // Supporting types
    DomainCommand as FormalCommand,
    DomainEvent as FormalEvent,
    DomainQuery as FormalQuery,
    AggregateState, SagaState, SagaStepResult,
    
    // ECS bridge
    System,
    
    // Validation
    Invariant, Specification,
};
