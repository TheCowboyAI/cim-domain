//! Infrastructure layer for cim-domain
//!
//! This module contains all infrastructure concerns including:
//! - NATS client and JetStream integration
//! - Event store implementation
//! - CID chain management
//! - Snapshot storage
//! - Event replay services
//! - Event streams as first-class objects

pub mod nats_client;
pub mod event_store;
pub mod cid_chain;
pub mod jetstream_event_store;
pub mod event_replay;
pub mod snapshot_store;
pub mod event_stream;
pub mod event_stream_service;

pub use nats_client::{NatsClient, NatsConfig, NatsError};
pub use event_store::{EventStore, EventStoreError, StoredEvent, EventMetadata};
pub use cid_chain::{EventWithCid, calculate_event_cid, verify_event_chain, CidError, ChainVerificationError};
pub use jetstream_event_store::JetStreamEventStore;
pub use event_replay::{
    EventReplayService, ReplayError, ReplayStats, ReplayOptions,
    EventHandler, AggregateRebuilder, AggregateEventProcessor,
    ProjectionBuilder, ProjectionHandler,
};
pub use snapshot_store::{SnapshotStore, SnapshotError, AggregateSnapshot};
pub use event_stream::{
    EventStream, EventStreamId, EventStreamMetadata, EventQuery, EventStreamOperations,
    EventStreamError, CausationOrder, EventFilter, EventOrdering, StreamTransformation,
    StreamComposition, GroupingCriteria, WindowSpec,
};
pub use event_stream_service::EventStreamService;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod event_stream_tests;
