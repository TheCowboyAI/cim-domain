// Copyright (c) 2025 - Cowboy AI, LLC.

//! # CIM Domain
//!
//! Core Domain-Driven Design (DDD) components and traits for the Composable Information Machine.
//!
//! This crate provides the fundamental building blocks for implementing DDD patterns:
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
//! 4. **Composition**: Build complex types from simple, immutable values
//! 5. **Domain Alignment**: Types reflect business concepts, not technical details
//! 6. **Event-Driven**: Commands and queries produce event streams, not direct results
//! 7. **Controlled State**: Enums restrict states and transitions to valid options

#![warn(missing_docs)]

mod command_handlers;
mod commands;
mod composition_types;
mod context_types;
mod cqrs;
mod entity;
mod errors;
mod event_handler;
mod events;
pub mod identifiers;
mod node_types;
mod query_handlers;
mod relationship_types;
pub mod state_machine;
// Subject abstraction removed - cim-domain is standalone
// Location module has been extracted to cim-domain-location
// Graph modules have been extracted to cim-domain-graph
// Workflow module has been extracted to cim-domain-workflow
pub mod category;
pub mod composition;
pub mod concept_naming;
pub mod concepts;
pub mod core_concepts;
pub mod domain;
pub mod ontology_quality;
pub mod ul_classifier;
// Infrastructure removed - belongs in infrastructure layer (cim-ipld or separate crate)
// Integration removed - belongs in infrastructure layer
// Persistence removed - belongs in infrastructure layer (cim-ipld)
pub mod projections;

// FP FOUNDATION - Entity as MONAD and Formal Domain Structure
pub mod formal_domain;
pub mod fp_adts;
pub mod fp_monad;

// Core domain concepts (content addressing optional, may be turned off downstream)
pub mod cid;
pub mod domain_path;
pub mod object_store;
pub mod subject;

/// FP-aligned JSON schemas for domain primitives
pub mod fp_schemas;
pub mod saga;
pub mod transaction_state;
pub mod vector_clock;

// Re-export core types
pub use composition_types::{CompositionType, DomainCompositionType};
pub use context_types::{ContextType, ServiceType, SubdomainType};
pub use cqrs::{
    AggregateTransactionId, CausationId, Command, CommandAcknowledgment, CommandEnvelope,
    CommandHandler, CommandId, CommandStatus, CorrelationId, EventId, EventStreamSubscription,
    IdType, MessageIdentity, Query, QueryAcknowledgment, QueryEnvelope,
    QueryHandler as CqrsQueryHandler, QueryId, QueryResponse, QueryStatus,
};
pub use domain_path::{DomainArtifactKind, DomainPath, DomainPathError, DomainPathSegment};
pub use entity::{AggregateRoot, DomainEntity, Entity, EntityId};
pub use identifiers::{EdgeId, GraphId, NodeId, StateId, TransitionId};
pub use node_types::NodeType;
pub use object_store::{BucketEntry, BucketLog, BucketRootKind, CidIndexEntry, MoveHistoryEntry};
pub use relationship_types::RelationshipType;
pub use saga::{Participant, Saga};
pub use subject::{Subject, SubjectError, SubjectPattern, SubjectPatternSegment, SubjectSegment};
pub use transaction_state::{TransactionInput, TransactionState, TxOutput};
pub use vector_clock::{ActorId, ClockCmp, VectorClock};

// Export QueryHandler without alias for compatibility
pub use cqrs::QueryHandler;

pub use errors::{DomainError, DomainResult};
pub use state_machine::{
    CommandInput, DocumentState, EmptyInput, EventOutput, MealyMachine, MealyStateTransitions,
    MooreMachine, MooreStateTransitions, State, StateTransition, TransitionInput, TransitionOutput,
};

// Subject types removed - cim-domain is standalone
// Subject routing belongs in infrastructure/transport layer, not domain

// Transport concerns removed from events - routing belongs in infrastructure layer

pub use events::{DomainEvent, DomainEventEnvelope, PayloadMetadata};
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
// Workflow events removed from core domain; see cim-domain-workflow

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

// Export event handler trait
pub use event_handler::EventHandler;

/// Type alias for aggregate identifiers using EntityId with AggregateMarker
pub type AggregateId = EntityId<markers::AggregateMarker>;

// ============================================================================
// FP FOUNDATION EXPORTS
// ============================================================================

// Re-export Entity monad and helpers
pub use fp_monad::{run_entity, Components, Entity as EntityMonad, KleisliArrow};

// Re-export CID types
pub use cid::{generate_cid, CidChain, CidError, CidImpl as Cid, ContentType, DomainCid};
pub use fp_adts::Either;

// Re-export formal domain traits
pub use formal_domain::{
    Aggregate,
    AggregateState,
    // Supporting types
    DomainCommand as FormalCommand,
    // Marker traits (REQUIRED for all domain concepts)
    DomainConcept,
    DomainEvent as FormalEvent,
    DomainQuery as FormalQuery,
    FormalDomainEntity,
    FormalEntityId,
    // Validation
    Invariant,
    // State machines
    MealyStateMachine,

    Policy,
    Saga as SagaTrait,

    SagaState,
    SagaStepResult,

    Specification,
    // ECS bridge
    System,

    ValueObject,
};

// Re-export concepts
pub use concept_naming::{suggest_by_prototypes, vector_from_features};
pub use concepts::{Concept, ConceptEdge, ConceptGraph, ConceptRelationshipType, HasConcept};
pub use core_concepts::{core_concepts, CoreConceptId};
pub use ontology_quality::{
    OntologyQualifier, QualityDimension, QualitySchema, QualityVector, ScaleType,
};
pub use ul_classifier::classify_object;
