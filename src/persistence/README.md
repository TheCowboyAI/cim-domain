<!-- Copyright 2025 Cowboy AI, LLC. -->

# Persistence Layer

The persistence layer provides durable storage for CIM Domain using NATS JetStream, with intelligent routing via cim-subject and content addressing through cim-ipld.

## Quick Start

```rust
use cim_domain::persistence::*;
use async_nats::Client;

// Connect to NATS
let client = Client::connect("nats://localhost:4222").await?;

// Create repository
let config = NatsRepositoryConfig::default();
let repository = NatsRepository::<MyAggregate>::new(
    client.clone(),
    config,
    "MyAggregate".to_string(),
).await?;

// Save aggregate with events
let metadata = repository.save(
    &aggregate,
    events,
    SaveOptions::default(),
).await?;

// Load aggregate
let (aggregate, metadata) = repository.load(
    &aggregate_id,
    LoadOptions::default(),
).await?;
```

## Components

### Aggregate Repository
Core trait for aggregate persistence with event sourcing:
- Save aggregates with events
- Load by ID or version
- Query with patterns
- Snapshot support

### NATS Repository
Production implementation using NATS JetStream:
- Event streams for history
- KV store for current state
- IPLD content addressing
- Subject-based routing

### Read Model Store
Optimized storage for query models:
- NATS KV backed
- Materialized views
- Projection tracking
- Schema versioning

### Query Optimizer
Subject-based query optimization:
- Pattern matching
- Index strategies
- Query planning
- Performance metrics

### Subject Router
Intelligent message routing:
- Pattern-based routing
- Multiple strategies
- Priority handling
- Permission checks

### IPLD Serializer
Content-addressed storage:
- Immutable content
- Chain verification
- CID generation
- Metadata support

### Migration Support
Schema evolution tools:
- Version tracking
- Migration planning
- Rollback support
- Validation checks

## Usage Patterns

### Event Sourcing
```rust
// Append events
repository.save(&aggregate, events, SaveOptions {
    expected_version: Some(current_version),
    create_snapshot: true,
    ..Default::default()
}).await?;

// Get history
let events = repository.get_history(
    &aggregate_id,
    Some(from_version),
    Some(to_version),
).await?;
```

### Read Models
```rust
// Save read model
read_store.save(&model, metadata).await?;

// Query with filters
let results = read_store.query::<MyModel>(
    Some("pattern.*"),
    filters,
).await?;
```

### Query Optimization
```rust
// Create optimized plan
let plan = optimizer.create_plan(
    "subject:domain.*.event",
    QueryHint {
        strategy: Some(IndexStrategy::SubjectPattern),
        use_cache: true,
        ..Default::default()
    },
).await?;

// Execute plan
let (results, performance) = optimizer.execute_plan(&plan).await?;
```

## Configuration

### Repository Config
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

### Subject Patterns
- Events: `cim.events.{context}.{aggregate}.{id}.{event}.{version}`
- Aggregates: `cim.aggregates.{context}.{aggregate}.{id}.state`
- Read Models: `cim.readmodels.{type}.{id}`

## Best Practices

1. **Keep aggregates small** - Use snapshots for large histories
2. **Version everything** - Events, aggregates, and read models
3. **Use proper subjects** - Follow naming conventions
4. **Handle consistency** - Read models are eventually consistent
5. **Plan migrations** - Version schemas from the start

## Testing

```bash
# Run tests
cargo test --lib persistence::

# Run example
cargo run --example persistence_example
```

## Error Handling

```rust
match repository.save(&aggregate, events, options).await {
    Ok(metadata) => println!("Saved version {}", metadata.version),
    Err(RepositoryError::VersionConflict { expected, actual }) => {
        println!("Version conflict: expected {}, found {}", expected, actual);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Performance Tips

- Enable caching for frequently accessed aggregates
- Use snapshots for aggregates with long histories  
- Create indexes for common query patterns
- Batch event appends when possible
- Use read models for complex queries

## See Also

- [Architecture Documentation](../../doc/architecture/persistence.md)
- [NATS JetStream Docs](https://docs.nats.io/jetstream)
- [IPLD Specification](https://ipld.io/)