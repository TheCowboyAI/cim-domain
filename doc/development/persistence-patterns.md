<!-- Copyright 2025 Cowboy AI, LLC. -->

# Persistence Patterns in CIM Domain

This guide covers the persistence patterns and best practices for using the CIM Domain persistence layer with NATS JetStream.

## Overview

The CIM Domain persistence layer provides several repository patterns for storing and retrieving domain aggregates:

1. **Simple Repository** - Basic CRUD operations
2. **NATS KV Repository** - Key-value storage with TTL support
3. **Event-Sourced Repository** - Full event sourcing (advanced)
4. **Read Model Store** - Optimized read models for queries

## Simple Repository Pattern

The `SimpleRepository` trait provides basic persistence operations:

```rust
use cim_domain::{
    EntityId,
    DomainEntity,
    persistence::{SimpleRepository, NatsSimpleRepository},
};

// Create repository
let repo = NatsSimpleRepository::new(
    client,
    "my-bucket".to_string(),
    "MyAggregate".to_string(),
).await?;

// Save aggregate
let metadata = repo.save(&aggregate).await?;

// Load aggregate
let loaded: Option<MyAggregate> = repo.load(&id).await?;

// Check existence
let exists = repo.exists(&id).await?;
```

### When to Use
- Simple domain models without complex event sourcing needs
- Rapid prototyping
- Aggregates that don't require event history

## NATS KV Repository Pattern

The `NatsKvRepository` provides advanced features like TTL and versioning:

```rust
use cim_domain::persistence::{NatsKvRepository, NatsKvRepositoryBuilder};

let repo: NatsKvRepository<MyAggregate> = NatsKvRepositoryBuilder::new()
    .client(client)
    .bucket_name("my-bucket")
    .aggregate_type("MyAggregate")
    .history(20)              // Keep 20 versions
    .ttl_seconds(3600)        // 1 hour TTL
    .build()
    .await?;
```

### Features
- **TTL Support**: Automatically expire data after specified time
- **Version History**: Keep multiple versions of aggregates
- **Optimized Storage**: Uses NATS KV store for better performance

### When to Use
- Temporary data that should expire
- Need for version history without full event sourcing
- High-performance key-value access patterns

## Read Model Store Pattern

The `ReadModelStore` provides optimized storage for query models:

```rust
use cim_domain::persistence::{
    ReadModel, ReadModelMetadata, NatsReadModelStore, ProjectionStatus
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProductStats {
    id: String,
    total_products: u32,
    average_price: f64,
}

impl ReadModel for ProductStats {
    fn model_type() -> &'static str {
        "ProductStats"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        // Update stats based on event
        Ok(())
    }
}

// Create store
let store = NatsReadModelStore::new(client, "read-models".to_string()).await?;

// Save read model
let metadata = ReadModelMetadata {
    id: model.id().to_string(),
    model_type: ProductStats::model_type().to_string(),
    schema_version: 1,
    last_updated: Utc::now(),
    last_event_position: 100,
    metadata: HashMap::new(),
};

store.save(&model, metadata).await?;

// Update projection status
store.update_projection_status(
    ProductStats::model_type(),
    ProjectionStatus::UpToDate,
).await?;
```

### Features
- **Caching**: In-memory cache for frequently accessed models
- **Projection Status**: Track if projections are up-to-date
- **Schema Versioning**: Support for evolving read models

### When to Use
- CQRS read models
- Denormalized views for queries
- Cached aggregations and statistics

## Query Support

The persistence layer includes comprehensive query building:

```rust
use cim_domain::persistence::{QueryBuilder, SortDirection, Pagination};

// Build query
let query = QueryBuilder::new()
    .filter("status", json!("active"))
    .filter("category", json!("electronics"))
    .sort_by("created_at", SortDirection::Descending)
    .limit(20)
    .offset(40)
    .build();

// Create pagination
let pagination = Pagination::from_query(20, 40, total_items);
println!("Page {} of {}", pagination.page, pagination.total_pages);
```

## Save and Load Options

Fine-grained control over persistence operations:

```rust
use cim_domain::persistence::{SaveOptions, LoadOptions};

// Save with options
let save_options = SaveOptions {
    expected_version: Some(3),  // Optimistic concurrency control
    create_snapshot: true,      // Create snapshot for event sourcing
    metadata: None,
};

// Load with options
let load_options = LoadOptions {
    version: Some(5),          // Load specific version
    use_snapshot: true,        // Use snapshots if available
    max_events: Some(1000),    // Limit events for replay
};
```

## Error Handling

The persistence layer provides detailed error types:

```rust
use cim_domain::persistence::RepositoryError;

match repo.save(&aggregate).await {
    Ok(metadata) => println!("Saved version {}", metadata.version),
    Err(RepositoryError::VersionConflict { expected, actual }) => {
        println!("Version conflict: expected {}, was {}", expected, actual);
    }
    Err(RepositoryError::NotFound(id)) => {
        println!("Aggregate {} not found", id);
    }
    Err(e) => {
        println!("Storage error: {}", e);
    }
}
```

## Best Practices

### 1. Choose the Right Repository
- Use `SimpleRepository` for basic needs
- Use `NatsKvRepository` when you need TTL or version history
- Use event sourcing for complex domains with audit requirements

### 2. Design Read Models Carefully
- Keep read models focused on specific queries
- Update them asynchronously from events
- Use caching for frequently accessed models

### 3. Handle Concurrency
- Use `expected_version` for optimistic concurrency control
- Implement retry logic for version conflicts
- Consider using event sourcing for high-contention scenarios

### 4. Monitor Projection Status
- Track if projections are up-to-date
- Implement catch-up logic for lagging projections
- Consider rebuilding projections periodically

### 5. Use Subject-Based Routing
- Leverage NATS subjects for efficient routing
- Design subject hierarchies that match your domain
- Use wildcards for flexible subscriptions

## Example: Complete Persistence Flow

```rust
use cim_domain::{
    EntityId,
    DomainEntity,
    persistence::*,
};

// 1. Define your aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
    status: OrderStatus,
    total: f64,
}

impl DomainEntity for Order {
    type IdType = OrderMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

// 2. Create repository
let repo = NatsKvRepositoryBuilder::new()
    .client(client)
    .bucket_name("orders")
    .aggregate_type("Order")
    .build()
    .await?;

// 3. Save aggregate
let order = Order::new(customer_id, items);
let metadata = repo.save(&order).await?;

// 4. Create read model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderSummary {
    id: String,
    customer_id: String,
    total: f64,
    status: String,
}

impl ReadModel for OrderSummary {
    fn model_type() -> &'static str {
        "OrderSummary"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        // Update summary based on events
        Ok(())
    }
}

// 5. Store read model
let store = NatsReadModelStore::new(client, "order-summaries").await?;
let summary = OrderSummary::from_order(&order);
store.save(&summary, metadata).await?;

// 6. Query
let query = QueryBuilder::new()
    .filter("customer_id", json!(customer_id))
    .filter("status", json!("pending"))
    .sort_by("created_at", SortDirection::Descending)
    .limit(10)
    .build();

// In a real implementation, the repository would execute this query
```

## Migration and Evolution

When evolving your persistence layer:

1. **Schema Versioning**: Always version your aggregates and read models
2. **Backward Compatibility**: Ensure new versions can read old data
3. **Migration Scripts**: Write scripts to migrate existing data
4. **Gradual Rollout**: Deploy changes gradually with feature flags

## Performance Considerations

1. **Caching**: Use the built-in caching in `ReadModelStore`
2. **Batch Operations**: Process multiple aggregates together when possible
3. **Async Processing**: Update read models asynchronously
4. **Connection Pooling**: Reuse NATS connections
5. **Monitoring**: Track persistence metrics and latencies

## Troubleshooting

Common issues and solutions:

### Connection Errors
```bash
# Ensure NATS is running with JetStream
docker run -p 4222:4222 nats:latest -js
```

### Version Conflicts
- Implement retry logic with exponential backoff
- Consider using event sourcing for high-contention aggregates

### Performance Issues
- Enable caching for read models
- Use appropriate TTL values
- Monitor NATS server performance

### Data Consistency
- Use transactions where supported
- Implement saga patterns for distributed transactions
- Monitor projection lag