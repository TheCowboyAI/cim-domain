// Copyright 2025 Cowboy AI, LLC.

//! # Persistence Layer
//!
//! This module provides persistence capabilities for the CIM Domain framework
//! using NATS JetStream as the underlying storage mechanism. It leverages
//! cim-subject for routing and cim-ipld for content-addressed storage.
//!
//! ## Components
//!
//! - **Aggregate Persistence**: Store and retrieve domain aggregates
//! - **Read Model Storage**: Optimized storage for query models
//! - **Query Optimization**: Subject-based indexing and routing
//! - **Migration Support**: Schema evolution and data migrations

// Core modules that compile correctly
pub mod simple_repository;
pub mod aggregate_repository_v2;
pub mod nats_kv_repository;
pub mod read_model_store_v2;
pub mod query_support;
pub mod metrics;
pub mod instrumented_repository;

// Re-export the main types
pub use simple_repository::{
    SimpleRepository, NatsSimpleRepository, SimpleAggregateMetadata,
};
pub use aggregate_repository_v2::{
    AggregateRepository, AggregateMetadata, EventSourcedRepository,
    SaveOptions, LoadOptions, RepositoryError,
};
pub use nats_kv_repository::{
    NatsKvRepository, NatsKvRepositoryBuilder, NatsKvConfig,
};
pub use read_model_store_v2::{
    ReadModel, ReadModelMetadata, ProjectionStatus, NatsReadModelStore,
};
pub use query_support::{
    QueryOptions, QueryResult, QueryBuilder, SortDirection, Pagination,
};
pub use metrics::{
    PersistenceMetrics, MetricsSummary, DurationStats, MetricsTimer, MetricsInstrumented,
};
pub use instrumented_repository::InstrumentedRepository;

// The following modules contain advanced features but have compilation issues
// that need to be resolved. They are temporarily disabled:
//
// pub mod aggregate_repository;
// pub mod read_model_store;
// pub mod query_optimizer;
// pub mod nats_repository;
// pub mod ipld_serializer;
// pub mod subject_router;
// pub mod migration;

#[cfg(test)]
mod tests;