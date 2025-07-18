# Persistence Layer Architecture

## Overview

The persistence layer provides durable storage for the CIM Domain framework using NATS JetStream as the underlying infrastructure. It leverages cim-subject for intelligent routing and cim-ipld for content-addressed storage, creating a distributed, event-sourced persistence system.

## Design Principles

1. **Event Sourcing First**: All state changes are captured as events
2. **Content Addressing**: Use IPLD for immutable, verifiable storage
3. **Subject-Based Routing**: Leverage NATS subjects for efficient querying
4. **Read/Write Separation**: Optimize for different access patterns
5. **Schema Evolution**: Support for migrations and versioning

## Core Components

### 1. Aggregate Repository

The `AggregateRepository` trait provides the main interface for persisting domain aggregates:

```rust
#[async_trait]
pub trait AggregateRepository<T: Entity>: Send + Sync {
    async fn save(
        &self,
        aggregate: &T,
        events: Vec<Box<dyn DomainEvent>>,
        options: SaveOptions,
    ) -> Result<AggregateMetadata, RepositoryError>;
    
    async fn load(
        &self,
        id: &EntityId<T>,
        options: LoadOptions,
    ) -> Result<(T, AggregateMetadata), RepositoryError>;
}
```

**Key Features:**
- Optimistic concurrency control
- Event and snapshot storage
- Version tracking
- IPLD content addressing

### 2. NATS Repository

The `NatsRepository` implements persistence using NATS JetStream:

```rust
let config = NatsRepositoryConfig {
    event_stream_name: "CIM-EVENTS",
    aggregate_stream_name: "CIM-AGGREGATES",
    read_model_bucket: "cim-read-models",
    snapshot_bucket: "cim-snapshots",
    // ...
};

let repository = NatsRepository::<Product>::new(
    client,
    config,
    "Product".to_string(),
).await?;
```

**Storage Strategy:**
- Events stored in JetStream streams
- Aggregates stored with subject-based keys
- Snapshots in KV buckets
- Content chains tracked with IPLD

### 3. Read Model Store

Optimized storage for query models using NATS KV:

```rust
#[async_trait]
pub trait ReadModelStore: Send + Sync {
    async fn save<T: ReadModel>(
        &self,
        model: &T,
        metadata: ReadModelMetadata,
    ) -> Result<(), ReadModelError>;
    
    async fn query<T: ReadModel>(
        &self,
        pattern: Option<&str>,
        filters: HashMap<String, Value>,
    ) -> Result<Vec<(T, ReadModelMetadata)>, ReadModelError>;
}
```

**Features:**
- Materialized views
- Projection status tracking
- Schema versioning
- Caching support

### 4. Query Optimizer

Subject-based query optimization for efficient data retrieval:

```rust
let optimizer = NatsQueryOptimizer::new();

// Create indexes
optimizer.create_index(
    "product_index",
    vec!["products.*", "inventory.*"],
).await?;

// Create optimized query plan
let plan = optimizer.create_plan(
    "subject:products.*.created",
    QueryHint {
        strategy: Some(IndexStrategy::SubjectPattern),
        expected_size: Some(100),
        use_cache: true,
        ..Default::default()
    },
).await?;
```

**Optimization Strategies:**
- Subject pattern matching
- Index-based lookups
- Time-series optimization
- Query plan caching

### 5. Subject Router

Intelligent routing based on NATS subjects:

```rust
let router = SubjectRouter::new(RoutingStrategy::PriorityBased);

router.add_route(RoutePattern {
    name: "aggregate_events",
    pattern: "domain.*.event.*",
    handler: "event_processor",
    priority: 10,
    metadata: HashMap::new(),
}).await?;
```

**Routing Strategies:**
- FirstMatch: First matching route wins
- AllMatches: Execute all matching routes
- PriorityBased: Highest priority wins
- RoundRobin: Distribute among matches

### 6. IPLD Serializer

Content-addressed storage using IPLD:

```rust
let serializer = IpldSerializer::new();

// Add to content chain
let cid = serializer.add_to_chain(
    "aggregate-history",
    content,
    metadata,
)?;

// Verify chain integrity
serializer.verify_chain("aggregate-history")?;
```

**Benefits:**
- Immutable history
- Content verification
- Distributed storage
- Efficient deduplication

### 7. Migration Support

Schema evolution and data migrations:

```rust
let mut runner = MigrationRunner::new(current_version);

runner.add_migration(Box::new(MyMigration));

// Migrate to target version
runner.migrate_to(SchemaVersion::new(2, 0, 0)).await?;
```

## Subject Hierarchy

The persistence layer uses a structured subject hierarchy:

```
cim.events.{context}.{aggregate}.{entity}.{event}.{version}
cim.aggregates.{context}.{aggregate}.{entity}.state.{version}
cim.readmodels.{type}.{id}.{operation}
cim.snapshots.{aggregate}.{entity}.{version}
```

Examples:
- `cim.events.domain.product.123.created.v1`
- `cim.aggregates.domain.product.123.state.v1`
- `cim.readmodels.catalog.main.query`

## Storage Patterns

### Event Storage

Events are stored in JetStream with:
- Stream per aggregate type
- Subject encoding for routing
- CID chains for integrity
- Automatic retention policies

### Aggregate Storage

Aggregates use hybrid storage:
1. Current state in KV store
2. Event history in streams
3. Snapshots for performance
4. IPLD chains for verification

### Read Model Storage

Read models optimize for queries:
- Denormalized projections
- Subject-based indexing
- Materialized views
- Cache-friendly access

## Query Patterns

### 1. Event Sourcing Queries

```rust
// Get aggregate history
let events = repository.get_history(
    &aggregate_id,
    Some(from_version),
    Some(to_version),
).await?;

// Rebuild at specific version
let (aggregate, metadata) = repository.get_at_version(
    &aggregate_id,
    version,
).await?;
```

### 2. Subject Pattern Queries

```rust
// Query by pattern
let results = read_store.query::<ProductView>(
    Some("products.electronics.*"),
    filters,
).await?;
```

### 3. Optimized Queries

```rust
// Create and execute query plan
let plan = optimizer.create_plan(query, hints).await?;
let (results, performance) = optimizer.execute_plan(&plan).await?;
```

## Performance Considerations

### Caching

- In-memory aggregate cache
- Query result caching
- Subject index caching
- Configurable TTLs

### Indexing

- Subject-based indexes
- Time-series indexes
- Custom composite indexes
- Automatic index updates

### Batching

- Event batch appends
- Bulk read operations
- Parallel query execution
- Connection pooling

## Configuration

### Repository Configuration

```rust
NatsRepositoryConfig {
    event_stream_name: "EVENTS",
    aggregate_stream_name: "AGGREGATES",
    read_model_bucket: "readmodels",
    snapshot_bucket: "snapshots",
    cache_size: 1000,
    enable_ipld: true,
}
```

### Stream Configuration

JetStream streams are configured with:
- Retention policies
- Replication factors
- Storage types (File/Memory)
- Subject filters

### KV Configuration

KV buckets use:
- TTL settings
- History depth
- Replication
- Watch support

## Error Handling

The persistence layer provides specific error types:

```rust
pub enum RepositoryError {
    NotFound(String),
    VersionConflict { expected: u64, actual: u64 },
    SerializationError(String),
    StorageError(String),
    SubjectError(String),
    IpldError(String),
}
```

## Best Practices

### 1. Aggregate Design

- Keep aggregates small
- Use snapshots for large histories
- Version all changes
- Include metadata

### 2. Event Design

- Use descriptive event types
- Include correlation IDs
- Version event schemas
- Keep events immutable

### 3. Read Model Design

- Denormalize for queries
- Update asynchronously
- Handle eventual consistency
- Version schemas

### 4. Subject Design

- Use hierarchical structure
- Include version information
- Follow naming conventions
- Plan for wildcards

### 5. Migration Design

- Test migrations thoroughly
- Provide rollback capability
- Version all schemas
- Document changes

## Testing

The persistence layer includes comprehensive tests:

```bash
# Run persistence tests
cargo test --lib persistence::

# Run integration tests
cargo test --test persistence_integration

# Run example
cargo run --example persistence_example
```

## Security Considerations

1. **Access Control**: Use NATS permissions and ACLs
2. **Encryption**: Enable TLS for NATS connections
3. **Audit Trail**: All changes tracked via events
4. **Data Integrity**: IPLD provides verification

## Future Enhancements

1. **Multi-Region Support**: Geo-distributed persistence
2. **Advanced Indexing**: Full-text search, graph queries
3. **Compression**: Event and snapshot compression
4. **Archival**: Long-term storage strategies
5. **Analytics**: Built-in analytical queries