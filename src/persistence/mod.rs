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
pub mod aggregate_repository_v2;
pub mod instrumented_repository;
pub mod metrics;
pub mod nats_kv_repository;
pub mod query_support;
pub mod read_model_store_v2;
pub mod simple_repository;

// Re-export the main types
pub use aggregate_repository_v2::{
    AggregateMetadata, AggregateRepository, EventSourcedRepository, LoadOptions, RepositoryError,
    SaveOptions,
};
pub use instrumented_repository::InstrumentedRepository;
pub use metrics::{
    DurationStats, MetricsInstrumented, MetricsSummary, MetricsTimer, PersistenceMetrics,
};
pub use nats_kv_repository::{NatsKvConfig, NatsKvRepository, NatsKvRepositoryBuilder};
pub use query_support::{Pagination, QueryBuilder, QueryOptions, QueryResult, SortDirection};
pub use read_model_store_v2::{NatsReadModelStore, ProjectionStatus, ReadModel, ReadModelMetadata};
pub use simple_repository::{NatsSimpleRepository, SimpleAggregateMetadata, SimpleRepository};

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
