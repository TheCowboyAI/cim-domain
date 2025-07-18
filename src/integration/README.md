<!-- Copyright 2025 Cowboy AI, LLC. -->

# Integration Layer

The integration layer provides infrastructure for connecting bounded contexts and aggregates within the CIM Domain framework. It implements Domain-Driven Design patterns to ensure proper communication and consistency across the system.

## Quick Start

```rust
use cim_domain::integration::*;

// Set up aggregate event routing
let router = AggregateEventRouter::new();
router.configure_standard_routes().await?;

// Configure domain bridges
let mut bridge = DomainBridge::new("Person", "HR");
bridge.set_translator(Box::new(PropertyBasedTranslator::new()));

// Set up dependency injection
let container = ContainerBuilder::new()
    .register_singleton::<CommandBus>(|| Arc::new(CommandBus::new()))
    .build();
```

## Core Components

### Event Routing
Routes events between aggregates with pattern matching and transformation:
- `AggregateEventRouter` - Cross-aggregate event consistency
- Pattern-based routing (`Person.*`, `*.Created.*`)
- Event transformation during routing

### Domain Bridges
Enable communication between bounded contexts:
- `DomainBridge` - Anti-corruption layer between domains
- `PropertyBasedTranslator` - Property mapping for messages
- `BridgeRegistry` - Manages multiple bridges

### Saga Orchestration
Manages distributed transactions using state machines:
- `SagaCoordinator` - Executes saga workflows
- `ProcessManager` - Starts sagas based on events
- Built on existing state machine infrastructure

### Dependency Injection
Manages service lifetimes and dependencies:
- `ContainerBuilder` - Configures services
- `ServiceRegistry` - Centralized service management
- Singleton, Transient, and Scoped lifetimes

### Cross-Domain Search
Enables searching across multiple bounded contexts:
- `CrossDomainSearchEngine` - Coordinates searches
- `DomainSearcher` - Domain-specific search implementation
- Relationship traversal support

### Event Bridge
Routes events between domains with pub/sub:
- `EventBridge` - Main event routing infrastructure
- `EventRouter` - Configures routing rules
- Pattern-based subscriptions

## Examples

### Event Routing
```rust
// Register handlers
router.register_handler("Person", Box::new(PersonEventHandler)).await?;

// Configure routes
router.register_route(
    "Person",                    // Source
    "Organization",              // Target
    "Person.Created.*",          // Pattern
    |event| {                    // Transform
        Some(create_member_added_event(event))
    },
).await?;

// Route events
let events = router.route_event("Person", &person_created).await?;
```

### Domain Translation
```rust
// Configure translator
let mut translator = PropertyBasedTranslator::new();
translator.add_command_mapping(
    "CreatePerson",
    "CreateEmployee",
    vec![
        ("name", "employee_name"),
        ("email", "work_email"),
    ],
);

// Set up bridge
let mut bridge = DomainBridge::new("Person", "HR");
bridge.set_translator(Box::new(translator));
```

### Saga Workflow
```rust
// Define saga
struct EmployeeOnboardingSaga;

impl SagaDefinition for EmployeeOnboardingSaga {
    fn saga_type(&self) -> &str {
        "EmployeeOnboarding"
    }
    
    async fn create_saga(&self, context: Value) -> Result<Saga, SagaError> {
        // Define workflow steps
    }
}

// Start saga
let coordinator = SagaCoordinator::new(command_bus);
coordinator.register_saga(Arc::new(EmployeeOnboardingSaga)).await;
coordinator.start_saga("EmployeeOnboarding", context).await?;
```

### Service Management
```rust
// Register services
let mut builder = ContainerBuilder::new();
builder.register_singleton::<Logger>(|| Arc::new(Logger::new()));
builder.register_transient::<Handler>(|| Arc::new(Handler::new()));

let container = builder.build();

// Resolve services
let logger = container.resolve::<Logger>()?.await?;
```

## Integration Patterns

### Event-Driven Flow
```
Person → PersonCreated → Router → Organization
                           ↓
                      HR Bridge → Employee System
```

### Saga Pattern
```
Start → Step 1 → Success → Step 2 → Success → Complete
            ↓                  ↓
         Failure          Failure
            ↓                  ↓
      Compensate        Compensate All
```

## Testing

Run integration tests:
```bash
cargo test --lib integration::
```

Run examples:
```bash
cargo run --example integration_example
```

## Best Practices

1. **Event Routing**
   - Use specific patterns over wildcards
   - Keep transformations pure and stateless
   - Handle failures gracefully

2. **Domain Bridges**
   - Define clear contracts between domains
   - Version message formats
   - Monitor bridge health

3. **Saga Design**
   - Make steps idempotent
   - Design compensation for each step
   - Set appropriate timeouts

4. **Service Management**
   - Choose appropriate lifetimes
   - Tag services for discovery
   - Handle resolution failures

## Architecture

See [doc/architecture/integration.md](../../doc/architecture/integration.md) for detailed architecture documentation.

## Status

The integration layer is complete and includes:
- ✅ Aggregate event routing
- ✅ Domain bridges with translation
- ✅ Saga orchestration
- ✅ Dependency injection
- ✅ Service registry
- ✅ Cross-domain search
- ✅ Event bridge
- ✅ Comprehensive tests
- ✅ Full documentation