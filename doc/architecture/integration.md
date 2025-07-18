# Integration Layer Architecture

## Overview

The integration layer provides the infrastructure for connecting different bounded contexts and aggregates within the CIM Domain framework. It implements patterns from Domain-Driven Design (DDD) and distributed systems to ensure consistency and proper communication between different parts of the system.

## Core Components

### 1. Aggregate Event Router

The `AggregateEventRouter` maintains consistency across aggregates by routing and transforming events between them.

**Key Features:**
- Pattern-based event matching
- Event transformation during routing
- Asynchronous event handling
- Pre-configured routes for common scenarios

**Pattern Matching:**
- `*` - matches all events
- `Person.*` - matches all Person events
- `Person.Created.*` - matches Person.Created events with any version
- `*.Created.*` - matches all Created events from any aggregate

**Example Usage:**
```rust
let router = AggregateEventRouter::new();

// Register handlers for each aggregate
router.register_handler(
    "Person",
    Box::new(PersonEventHandler::new()),
).await?;

// Configure cross-aggregate route
router.register_route(
    "Person",                    // Source aggregate
    "Organization",              // Target aggregate
    "Person.Created.*",          // Event pattern
    |event| {                    // Transformation function
        // Transform Person.Created to Organization.MemberAdded
        Some(create_member_added_event(event))
    },
).await?;

// Route an event
let events = router.route_event("Person", &person_created_event).await?;
```

### 2. Domain Bridges

Domain bridges enable communication between bounded contexts with automatic translation of commands and events.

**Components:**
- `DomainBridge` - Main bridge between two domains
- `MessageTranslator` - Translates commands and events
- `BridgeAdapter` - Handles actual message transport
- `BridgeRegistry` - Manages multiple bridges

**Property-Based Translation:**
```rust
let mut translator = PropertyBasedTranslator::new();

// Map command types and properties
translator.add_command_mapping(
    "CreatePerson".to_string(),
    "CreateEmployee".to_string(),
    vec![
        ("name".to_string(), "employee_name".to_string()),
        ("email".to_string(), "work_email".to_string()),
    ],
);

// Create and configure bridge
let mut bridge = DomainBridge::new("Person".to_string(), "HR".to_string());
bridge.set_translator(Box::new(translator));
```

### 3. Saga Orchestration

The saga pattern manages distributed transactions across multiple aggregates using state machines.

**Key Components:**
- `SagaCoordinator` - Manages saga execution
- `SagaDefinition` - Defines saga steps and transitions
- `ProcessManager` - Starts sagas based on events
- `ProcessPolicy` - Determines when to start sagas

**Saga States:**
- `Pending` - Initial state
- `Running` - Executing steps
- `Completed` - Successfully finished
- `Failed` - Failed with error
- `Compensating` - Running compensation
- `Compensated` - Compensation complete

**Example Saga Definition:**
```rust
struct EmployeeOnboardingSaga;

#[async_trait]
impl SagaDefinition for EmployeeOnboardingSaga {
    fn saga_type(&self) -> &str {
        "EmployeeOnboarding"
    }
    
    async fn create_saga(&self, context: serde_json::Value) -> Result<Saga, SagaError> {
        Ok(Saga {
            id: Uuid::new_v4(),
            name: "EmployeeOnboarding".to_string(),
            steps: vec![
                SagaStep {
                    id: "create_employee_profile".to_string(),
                    domain: "Person".to_string(),
                    command_type: "AddEmploymentComponent".to_string(),
                    depends_on: vec![],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
                SagaStep {
                    id: "add_to_organization".to_string(),
                    domain: "Organization".to_string(),
                    command_type: "AddMember".to_string(),
                    depends_on: vec!["create_employee_profile".to_string()],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
                // ... more steps
            ],
            // ... other configuration
        })
    }
}
```

### 4. Dependency Injection

The DI container manages service lifetimes and dependencies.

**Service Lifetimes:**
- `Singleton` - One instance for the entire application
- `Transient` - New instance for each resolution
- `Scoped` - One instance per scope

**Example:**
```rust
let mut builder = ContainerBuilder::new();

// Register services
builder.register_singleton::<Arc<CommandBus>>(|| {
    Arc::new(CommandBus::new())
});

builder.register_transient::<EventHandler>(|| {
    Arc::new(LoggingEventHandler::new())
});

let container = builder.build();

// Resolve services
let command_bus = container.resolve::<Arc<CommandBus>>()?.await?;
```

### 5. Service Registry

The service registry provides centralized service management with lifecycle support.

**Features:**
- Service registration with metadata
- Singleton caching
- Service discovery by tags
- Health checking

**Example:**
```rust
let registry = ServiceRegistry::new();

// Register a service
registry.register(
    ServiceDescriptor::new::<LoggingService>(
        ServiceLifetime::Singleton,
        Box::new(|_| Ok(Arc::new(LoggingService::new()))),
    )
).await?;

// Tag services for discovery
registry.tag_service::<LoggingService>("logging", "telemetry").await?;

// Discover services by tag
let logging_services = registry.find_by_tag("logging").await?;
```

### 6. Cross-Domain Search

Enables searching across multiple bounded contexts.

**Components:**
- `CrossDomainSearchEngine` - Coordinates searches
- `DomainSearcher` - Domain-specific search implementation
- `SearchResult` - Unified result format

**Example:**
```rust
let search_engine = CrossDomainSearchEngine::new();

// Register domain searchers
search_engine.register_domain_searcher(Box::new(PersonSearcher)).await;
search_engine.register_domain_searcher(Box::new(OrganizationSearcher)).await;

// Search across all domains
let results = search_engine.search("John Doe").await?;
```

### 7. Event Bridge

Routes events between domains with transformation and filtering.

**Features:**
- Pattern-based routing rules
- Event transformation
- Event filtering
- Pub/sub subscriptions

**Example:**
```rust
let event_bridge = EventBridge::new(Default::default());

// Configure routing
let mut router = EventRouter::new();
router.add_rule(
    "Person.*".to_string(),              // Source pattern
    vec!["hr.events".to_string()],       // Target streams
    Some(Box::new(PersonToHRTransformer)), // Optional transformer
);

event_bridge.set_router(router).await;

// Subscribe to events
event_bridge.subscribe(
    "hr.events".to_string(),
    Box::new(|event| {
        Box::pin(async move {
            process_hr_event(event).await
        })
    }),
).await;
```

## Integration Patterns

### 1. Event-Driven Integration

Events flow between aggregates and domains:
```
Person Aggregate -> PersonCreated Event -> Event Router -> Organization Aggregate
                                        |
                                        +-> HR Domain Bridge -> Employee System
```

### 2. Saga Pattern for Distributed Transactions

Complex operations span multiple aggregates:
```
Start: Employee Onboarding
  |
  +-> Step 1: Create Employee Profile (Person Aggregate)
  |     |
  |     +-> Success -> Step 2
  |     +-> Failure -> Compensate
  |
  +-> Step 2: Add to Organization (Organization Aggregate)
  |     |
  |     +-> Success -> Step 3
  |     +-> Failure -> Compensate Step 1
  |
  +-> Step 3: Create Agent Account (Agent Aggregate)
        |
        +-> Success -> Complete
        +-> Failure -> Compensate Steps 1 & 2
```

### 3. Anti-Corruption Layer

Domain bridges act as anti-corruption layers:
```
Internal Domain Model <-> Bridge Translator <-> External System Model
```

## Best Practices

### 1. Event Routing

- Use specific patterns rather than wildcards when possible
- Keep transformation functions pure and stateless
- Handle failures gracefully with logging
- Avoid circular dependencies between aggregates

### 2. Saga Design

- Keep saga steps idempotent
- Design compensation actions for each step
- Set appropriate timeouts
- Use retry policies with exponential backoff
- Store saga state for recovery

### 3. Domain Bridges

- Define clear contracts between domains
- Version your message formats
- Use property mappings for simple translations
- Implement custom translators for complex logic
- Monitor bridge health

### 4. Service Management

- Use appropriate lifetimes (singleton vs transient)
- Tag services for discovery
- Implement health checks
- Handle service resolution failures
- Clean up resources properly

## Configuration

### Event Router Configuration

```rust
// Pre-configured routes for common scenarios
router.configure_standard_routes().await?;

// Or configure manually
router.configure_person_org_routes().await?;
router.configure_agent_policy_routes().await?;
router.configure_location_org_routes().await?;
```

### Bridge Configuration

```rust
let config = BridgeConfig {
    buffer_size: 10000,
    event_ttl_seconds: 3600,
    enable_dlq: true,
    max_retries: 3,
    retry_backoff_multiplier: 2.0,
};

let event_bridge = EventBridge::new(config);
```

### Search Configuration

```rust
let config = SearchConfig {
    max_results: 100,
    similarity_threshold: 0.7,
    search_timeout_ms: 5000,
    include_relationships: true,
};

let search_engine = CrossDomainSearchEngine::with_config(
    event_bridge,
    config
);
```

## Error Handling

The integration layer uses `DomainError` for all error cases:

- `InvalidOperation` - Invalid operation attempted
- `NotFound` - Service/bridge/route not found
- `AlreadyExists` - Duplicate registration
- `SerializationError` - Message translation failed
- `NotImplemented` - Feature not yet implemented

Always handle errors appropriately:
```rust
match router.route_event("Person", &event).await {
    Ok(routed_events) => {
        println!("Routed {} events", routed_events.len());
    }
    Err(DomainError::InvalidOperation { reason }) => {
        eprintln!("Routing failed: {}", reason);
    }
    Err(e) => {
        eprintln!("Unexpected error: {:?}", e);
    }
}
```

## Performance Considerations

1. **Event Routing**: Routes are cached in memory for fast lookup
2. **Service Registry**: Singletons are cached after first resolution
3. **Domain Bridges**: Use connection pooling for external systems
4. **Saga Execution**: Steps can run in parallel if no dependencies
5. **Cross-Domain Search**: Results are aggregated asynchronously

## Testing

The integration layer includes comprehensive tests:

1. **Unit Tests**: Test individual components
2. **Integration Tests**: Test component interactions
3. **Example Applications**: Demonstrate real-world usage

Run tests with:
```bash
cargo test --lib integration::
```

## Future Enhancements

1. **Event Sourcing Integration**: Store all routed events
2. **Distributed Tracing**: Track events across domains
3. **Circuit Breakers**: Protect against cascading failures
4. **Message Queuing**: Durable event delivery
5. **Schema Registry**: Manage event/command schemas
6. **Monitoring Dashboard**: Real-time integration metrics