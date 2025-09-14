// Copyright 2025 Cowboy AI, LLC.

//! Infrastructure layer for cim-domain
//!
//! This module contains all infrastructure concerns including:
//! - NATS client and JetStream integration
//! - Event store implementation
//! - Snapshot storage
//! - Event replay services
//! - Event streams as first-class objects

pub mod event_replay;
pub mod event_store;
pub mod event_stream;
pub mod event_stream_service;
/// Event versioning and schema evolution support
pub mod event_versioning;
// jetstream_event_store moved to backup - needs cim-ipld types
pub mod nats_client;
/// Projection checkpoint storage for fault-tolerant event processing
pub mod projection_checkpoint;
/// Saga pattern implementation for distributed transactions
pub mod saga;
/// Automatic snapshot policies for performance optimization
pub mod snapshot_policy;
pub mod snapshot_store;

pub use event_replay::{
    AggregateEventProcessor, AggregateRebuilder, EventHandler, EventReplayService,
    ProjectionBuilder, ProjectionHandler, ReplayError, ReplayOptions, ReplayStats,
};
pub use event_store::{EventMetadata, EventStore, EventStoreError, StoredEvent};
pub use event_stream::{
    CausationOrder, EventFilter, EventOrdering, EventQuery, EventStream, EventStreamError,
    EventStreamId, EventStreamMetadata, EventStreamOperations, GroupingCriteria, StreamComposition,
    StreamTransformation, WindowSpec,
};
pub use event_stream_service::EventStreamService;
pub use event_versioning::{
    EventTypeMetadata, EventUpcaster, EventVersioningError, EventVersioningService, SimpleUpcaster,
    VersionedEvent,
};
// JetStreamEventStore moved to cim-ipld (uses storage-specific types)
// pub use jetstream_event_store::JetStreamEventStore;
pub use nats_client::{NatsClient, NatsConfig, NatsError};
pub use projection_checkpoint::{
    CheckpointError, CheckpointManager, CheckpointStore, EventPosition, InMemoryCheckpointStore,
    JetStreamCheckpointStore, ProjectionCheckpoint,
};
pub use saga::{
    CommandBus, ProcessManager, ProcessPolicy, SagaCommand, SagaCoordinator, SagaDefinition,
    SagaError, SagaInstance, SagaMarker,
};
pub use snapshot_policy::{
    AutoSnapshotService, SnapshotMetrics, SnapshotPolicy, SnapshotPolicyEngine,
};
pub use snapshot_store::{AggregateSnapshot, SnapshotError, SnapshotStore};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod event_stream_tests;
