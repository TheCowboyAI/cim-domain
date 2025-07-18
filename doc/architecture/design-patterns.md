# CIM Domain Design Patterns

## Overview

This document consolidates the core design patterns and principles used throughout the CIM Domain. These patterns guide the implementation of Domain-Driven Design (DDD) in an event-driven, distributed system architecture.

## Core Design Philosophy

### 1. Event-Driven Architecture

**Principle**: All state changes are events, no CRUD operations

- **Commands and Queries return acknowledgments only** - never direct results
- **All results flow through event streams** - maintaining true asynchronous communication
- **Event sourcing for aggregates** - providing complete audit trails and time travel
- **CID chains for integrity** - cryptographic verification of event sequences

### 2. Type Safety Through Enums

**Principle**: Make invalid states unrepresentable

```rust
// Use enums to restrict choices
pub enum OrderStatus {
    Draft,
    Submitted,
    Processing,
    Completed,
    Cancelled,
}

// Use phantom types for domain separation
pub struct EntityId<T> {
    id: Uuid,
    _phantom: PhantomData<T>,
}
```

### 3. State Machines for Controlled Workflows

**Principle**: Explicit state transitions with validation

```rust
pub enum AgentStatus {
    Initializing,
    Active,
    Suspended,
    Offline,
    Decommissioned,
}

impl StateMachine for AgentStatus {
    fn valid_transitions(&self) -> Vec<Self> {
        match self {
            Self::Initializing => vec![Self::Active],
            Self::Active => vec![Self::Suspended, Self::Offline, Self::Decommissioned],
            Self::Suspended => vec![Self::Active, Self::Decommissioned],
            Self::Offline => vec![Self::Active, Self::Decommissioned],
            Self::Decommissioned => vec![], // Terminal state
        }
    }
}
```

### 4. Local-First Architecture

**Principle**: Full functionality without network connectivity

- **Offline-capable by design** - local event stores and projections
- **Hierarchical message propagation** - messages bubble up through defined layers
- **Eventual synchronization** - reconcile state when connectivity is restored
- **Conflict resolution** - deterministic rules for handling concurrent changes

## Architectural Patterns

### Command Query Responsibility Segregation (CQRS)

Strict separation of write and read models:

```rust
// Command side
pub trait CommandHandler<C: Command> {
    fn handle(&self, command: C) -> CommandAck;
}

pub enum CommandAck {
    Accepted { command_id: Uuid },
    Rejected { command_id: Uuid, reason: String },
}

// Query side
pub trait QueryHandler<Q: Query> {
    fn handle(&self, query: Q) -> QueryAck;
}

// Results come through event streams
pub struct EventStreamSubscription {
    pub correlation_id: CorrelationId,
    pub stream_subject: String,
}
```

### Event Sourcing

All state derived from events:

```rust
pub trait AggregateRoot {
    type Command;
    type Event;
    type Error;
    
    fn handle_command(&mut self, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;
    fn apply_event(&mut self, event: &Self::Event);
    
    // Rebuild from events
    fn from_events(events: Vec<Self::Event>) -> Self {
        let mut aggregate = Self::default();
        for event in events {
            aggregate.apply_event(&event);
        }
        aggregate
    }
}
```

### Component-Based Architecture

Dynamic extensibility through components:

```rust
pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn clone_box(&self) -> Box<dyn Component>;
    fn serialize(&self) -> Result<Vec<u8>, Error>;
    fn component_type(&self) -> &'static str;
}

pub struct ComponentStorage {
    components: HashMap<TypeId, Box<dyn Component>>,
    metadata: HashMap<TypeId, ComponentMetadata>,
}

impl ComponentStorage {
    pub fn add_component<C: Component + 'static>(&mut self, component: C) {
        let type_id = TypeId::of::<C>();
        self.components.insert(type_id, Box::new(component));
        self.metadata.insert(type_id, ComponentMetadata::new());
    }
    
    pub fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get(&TypeId::of::<C>())
            .and_then(|c| c.as_any().downcast_ref::<C>())
    }
}
```

## Message Flow Patterns

### Command Flow

```
1. User/System submits command
2. Command handler validates and acknowledges
3. Aggregate processes command (may use state machine)
4. Domain events are generated
5. Events published to stream
6. Subscribers react to events
```

### Query Flow

```
1. User/System submits query
2. Query handler acknowledges receipt
3. Query processed against read model/projection
4. Result event published to response stream
5. Original caller subscribes to response stream
```

### Catchup Subscription Pattern

Replay events from a specific point:

```rust
pub struct CatchupSubscription {
    pub start_after: IdType,
    pub correlation_id: Option<CorrelationId>,
    pub target_id: EntityId,
    pub end_at: Option<IdType>,
}

// Usage
pub async fn catchup_aggregate<A: Aggregate>(
    aggregate_id: AggregateId,
    last_known_event: IdType,
    event_stream: &mut EventStream,
) -> Result<A> {
    let subscription = CatchupSubscription::from_message(
        last_known_event,
        aggregate_id.into(),
    );
    
    let catchup_stream = event_stream
        .catchup_subscribe(subscription)
        .await?;
    
    let mut aggregate = A::default();
    pin_mut!(catchup_stream);
    
    while let Some(event) = catchup_stream.next().await {
        aggregate.apply_event(event)?;
    }
    
    Ok(aggregate)
}
```

## Local-First Patterns

### Hierarchical Message Propagation

Messages bubble up through layers:

```
┌─────────────────┐
│  Super-Cluster  │  ← Global coordination
└────────┬────────┘
         │
┌────────┴────────┐
│     Cluster     │  ← Regional boundary
└────────┬────────┘
         │
┌────────┴────────┐
│   Local Leaf    │  ← Edge server
└────────┬────────┘
         │
┌────────┴────────┐
│ Local Container │  ← Runtime
└────────┬────────┘
         │
┌────────┴────────┐
│   Local App     │  ← User instance
└─────────────────┘
```

### Propagation Control

```rust
pub enum PropagationScope {
    LocalOnly,      // Never leaves the app
    Container,      // Bubbles to container
    Leaf,          // Bubbles to local leaf
    Cluster,       // Bubbles to cluster
    SuperCluster,  // Global propagation
}

pub struct EventEnvelope {
    pub event: DomainEvent,
    pub scope: PropagationScope,
    pub propagation_policy: PropagationPolicy,
}

pub enum PropagationPolicy {
    Immediate,          // Propagate ASAP
    Batched(Duration),  // Batch with others
    OnConnection,       // Wait for network
    Manual,            // User-triggered
}
```

## Subject-Based Routing

### Naming Convention

```
<context>.<aggregate>.<event_type>.<version>

Examples:
people.person.registered.v1
organizations.company.created.v1
policies.access_control.updated.v1
```

### Cross-Context Communication

```rust
pub struct ContextBridge {
    context_mappings: HashMap<String, ContextMapping>,
}

impl ContextBridge {
    pub fn route_cross_context(&self, msg: NatsMessage) -> Result<Vec<NatsMessage>> {
        let subject_parts = parse_subject(&msg.subject)?;
        let source_context = &subject_parts.context;
        
        let interested_contexts = self.find_interested_contexts(
            source_context, 
            &subject_parts.event_type
        )?;
        
        interested_contexts.into_iter().map(|target_context| {
            let mapping = self.context_mappings.get(&target_context)
                .ok_or(RoutingError::NoMapping)?;
            
            let transformed_event = mapping.transform_event(&msg)?;
            
            Ok(NatsMessage {
                subject: format!("{}.{}.{}.{}",
                    target_context,
                    mapping.map_aggregate(&subject_parts.aggregate)?,
                    mapping.map_event_type(&subject_parts.event_type)?,
                    subject_parts.version
                ),
                payload: transformed_event,
                headers: msg.headers.clone(),
            })
        }).collect()
    }
}
```

## Domain Building Patterns

### Aggregate Pattern

```rust
pub struct OrderAggregate {
    entity: Entity<OrderMarker>,
    state: StateMachine<OrderState>,
    items: Vec<OrderItem>,
    version: u64,
}

impl AggregateRoot for OrderAggregate {
    type Command = OrderCommand;
    type Event = OrderEvent;
    type Error = OrderError;
    
    fn handle_command(&mut self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match cmd {
            OrderCommand::Submit { items } => {
                self.state.transition(OrderState::Submitted)?;
                Ok(vec![OrderEvent::Submitted { 
                    order_id: self.entity.id(),
                    items,
                    timestamp: Utc::now(),
                }])
            }
            // ... other commands
        }
    }
    
    fn apply_event(&mut self, event: &Self::Event) {
        match event {
            OrderEvent::Submitted { items, .. } => {
                self.items = items.clone();
                self.state = OrderState::Submitted;
            }
            // ... other events
        }
        self.version += 1;
    }
}
```

### Value Object Pattern

Immutable data with validation:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new(email: String) -> Result<Self, ValidationError> {
        if Self::is_valid(&email) {
            Ok(Self(email))
        } else {
            Err(ValidationError::InvalidEmail)
        }
    }
    
    fn is_valid(email: &str) -> bool {
        // Email validation logic
        email.contains('@') && email.len() > 3
    }
}

// Value objects are NEVER mutated
// WRONG: email.0 = "new@email.com"
// RIGHT: entity.email = EmailAddress::new("new@email.com")?
```

### Repository Pattern

Abstract persistence behind traits:

```rust
#[async_trait]
pub trait Repository<T: AggregateRoot> {
    async fn get(&self, id: EntityId<T>) -> Result<T, RepositoryError>;
    async fn save(&self, aggregate: &T) -> Result<(), RepositoryError>;
    async fn exists(&self, id: EntityId<T>) -> Result<bool, RepositoryError>;
}

// Event-sourced implementation
pub struct EventSourcedRepository<T: AggregateRoot> {
    event_store: Arc<dyn EventStore>,
}

#[async_trait]
impl<T: AggregateRoot> Repository<T> for EventSourcedRepository<T> {
    async fn get(&self, id: EntityId<T>) -> Result<T, RepositoryError> {
        let events = self.event_store.get_events(id).await?;
        Ok(T::from_events(events))
    }
    
    async fn save(&self, aggregate: &T) -> Result<(), RepositoryError> {
        let events = aggregate.pending_events();
        self.event_store.append_events(aggregate.id(), events).await?;
        Ok(())
    }
}
```

## Testing Patterns

### Aggregate Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_order_submission() {
        let mut order = OrderAggregate::new();
        
        let command = OrderCommand::Submit {
            items: vec![OrderItem::new("SKU-123", 2)],
        };
        
        let events = order.handle_command(command).unwrap();
        
        assert_eq!(events.len(), 1);
        match &events[0] {
            OrderEvent::Submitted { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].sku, "SKU-123");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
```

### Event Store Testing

```rust
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<StoredEvent>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn with_events(events: Vec<DomainEvent>) -> Self {
        let store = Self::new();
        // Add test events
        store
    }
}
```

## Anti-Patterns to Avoid

### ❌ Returning Results from Commands

```rust
// WRONG
trait CommandHandler {
    fn handle(&self, cmd: Command) -> Result<DomainObject>;
}
```

### ✅ Return Only Acknowledgments

```rust
// CORRECT
trait CommandHandler {
    fn handle(&self, cmd: Command) -> CommandAck;
}
```

### ❌ Mutable Value Objects

```rust
// WRONG
address.street = "New Street";
```

### ✅ Replace Value Objects

```rust
// CORRECT
entity.address = Address::new("New Street", city, zip);
```

### ❌ Large Aggregates

```rust
// WRONG
struct CompanyAggregate {
    employees: Vec<Employee>,  // Could be thousands
    departments: Vec<Department>,
    projects: Vec<Project>,
}
```

### ✅ Reference Other Aggregates

```rust
// CORRECT
struct CompanyAggregate {
    employee_ids: Vec<EmployeeId>,
    department_ids: Vec<DepartmentId>,
}
```

### ❌ Update Events for Value Objects

```rust
// WRONG
EdgeUpdated { old_edge, new_edge }
```

### ✅ Remove and Add Pattern

```rust
// CORRECT
EdgeRemoved { edge_id }
EdgeAdded { edge_id, source, target, relationship }
```

## Performance Patterns

### Event Batching

```rust
pub struct EventBatcher {
    batch_size: usize,
    timeout: Duration,
    buffer: Vec<DomainEvent>,
}

impl EventBatcher {
    pub async fn add(&mut self, event: DomainEvent) -> Option<Vec<DomainEvent>> {
        self.buffer.push(event);
        
        if self.buffer.len() >= self.batch_size {
            Some(self.flush())
        } else {
            None
        }
    }
    
    pub fn flush(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.buffer)
    }
}
```

### Projection Caching

```rust
pub struct CachedProjection<T> {
    cache: Arc<RwLock<HashMap<String, T>>>,
    ttl: Duration,
}

impl<T: Clone> CachedProjection<T> {
    pub async fn get_or_compute<F>(&self, key: &str, compute: F) -> Result<T>
    where
        F: FnOnce() -> Future<Output = Result<T>>,
    {
        // Check cache first
        if let Some(value) = self.cache.read().await.get(key) {
            return Ok(value.clone());
        }
        
        // Compute if not found
        let value = compute().await?;
        
        // Update cache
        self.cache.write().await.insert(key.to_string(), value.clone());
        
        Ok(value)
    }
}
```

## Security Patterns

### Subject-Based Access Control

```rust
pub struct SubjectPermissions {
    pub publish_allow: Vec<SubjectPattern>,
    pub subscribe_allow: Vec<SubjectPattern>,
    pub deny: Vec<SubjectPattern>,
}

impl SubjectPermissions {
    pub fn can_publish(&self, subject: &str) -> bool {
        !self.deny.iter().any(|p| p.matches(subject)) &&
        self.publish_allow.iter().any(|p| p.matches(subject))
    }
}
```

### Event Encryption

```rust
pub struct EncryptedEventEnvelope {
    pub header: EventHeader,
    pub encrypted_payload: Vec<u8>,
    pub encryption_metadata: EncryptionMetadata,
}

impl EncryptedEventEnvelope {
    pub fn decrypt(&self, key: &Key) -> Result<DomainEvent> {
        let payload = decrypt(&self.encrypted_payload, key)?;
        Ok(serde_json::from_slice(&payload)?)
    }
}
```

## Evolution Patterns

### Schema Evolution

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum PersonRegisteredEvent {
    #[serde(rename = "v1")]
    V1 {
        person_id: Uuid,
        name: String,
    },
    #[serde(rename = "v2")]
    V2 {
        person_id: Uuid,
        first_name: String,
        last_name: String,
        #[serde(default)]
        middle_name: Option<String>,
    },
}

impl PersonRegisteredEvent {
    pub fn upgrade(self) -> Self {
        match self {
            Self::V1 { person_id, name } => {
                let parts: Vec<&str> = name.split(' ').collect();
                Self::V2 {
                    person_id,
                    first_name: parts.get(0).unwrap_or(&"").to_string(),
                    last_name: parts.get(1).unwrap_or(&"").to_string(),
                    middle_name: None,
                }
            }
            v2 => v2,
        }
    }
}
```

## Summary

These patterns provide a foundation for building robust, scalable domains that:

- Embrace asynchronous, event-driven communication
- Support offline-first, local operation
- Enable clean separation of concerns
- Leverage Rust's type system for safety
- Scale horizontally through proper aggregate design
- Evolve gracefully over time

Consistent application of these patterns ensures domains that compose well within the larger CIM ecosystem while maintaining clarity, performance, and correctness.