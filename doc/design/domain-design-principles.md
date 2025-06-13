# CIM Domain Design Principles

## Overview

This document establishes the foundational design principles and patterns for building domains in the Composable Information Machine (CIM). These principles guide the implementation of Domain-Driven Design (DDD) in an event-driven, distributed system architecture.

## Core Design Philosophy

### 1. Event-Driven Architecture
- **Commands and Queries return acknowledgments only** - never direct results
- **All results flow through event streams** - maintaining true asynchronous communication
- **Event sourcing for aggregates** - providing complete audit trails and time travel
- **CID chains for integrity** - cryptographic verification of event sequences

### 2. Type Safety Through Enums
- **Use enums wherever possible** to restrict choices and enforce valid options
- **Compile-time guarantees** over runtime validation when feasible
- **Self-documenting code** through expressive enum variants
- **Explicit over implicit** - make invalid states unrepresentable

### 3. State Machines for Controlled Workflows
- **Optional by default** - not everything needs a state machine
- **Required for Aggregates** - transactional boundaries need controlled state
- **Required for known procedures** - multi-step workflows with defined paths
- **Enum-based states** with explicit transition rules

### 4. Local-First Architecture
- **Offline-capable by design** - full functionality without network connectivity
- **Hierarchical message propagation** - messages bubble up through defined layers
- **Eventual synchronization** - reconcile state when connectivity is restored
- **Conflict resolution** - deterministic rules for handling concurrent changes

## Architectural Layers

### Domain Layer
The heart of the system, containing pure business logic:
- **Aggregates**: Transactional consistency boundaries with state machines
- **Entities**: Objects with identity and lifecycle
- **Value Objects**: Immutable data with no identity
- **Domain Events**: Facts that have occurred
- **Domain Services**: Pure business logic operations

### Application Layer
Orchestrates domain operations:
- **Command Handlers**: Process commands, return acknowledgments
- **Query Handlers**: Process queries, return acknowledgments
- **Event Publishers**: Emit domain events to streams
- **Projections**: Build read models from event streams

### Infrastructure Layer
Technical concerns and external integrations:
- **Event Store**: NATS JetStream with CID chains
- **Message Bus**: NATS for all inter-service communication
- **Repositories**: Abstract persistence (never exposed to domain)
- **External Services**: Third-party integrations

### Presentation Layer
User and system interfaces:
- **Bevy ECS**: Real-time visualization and interaction
- **NATS APIs**: All External system interfaces flow through NATS
- **Event Subscriptions**: React to domain events

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

A special type of query that brings an object up to date by replaying events from a specific point:

```rust
pub struct CatchupSubscription {
    /// Starting point - the last known message ID
    pub start_after: IdType,

    /// Correlation to follow (if specific chain needed)
    pub correlation_id: Option<CorrelationId>,

    /// Target aggregate or entity to update
    pub target_id: EntityId,

    /// Optional end point (for bounded replay)
    pub end_at: Option<IdType>,
}

impl CatchupSubscription {
    /// Create a catchup from a specific message
    pub fn from_message(message_id: IdType, target: EntityId) -> Self {
        Self {
            start_after: message_id,
            correlation_id: None,
            target_id: target,
            end_at: None,
        }
    }

    /// Create a catchup for a specific correlation chain
    pub fn from_correlation(
        start_after: IdType,
        correlation: CorrelationId,
        target: EntityId
    ) -> Self {
        Self {
            start_after,
            correlation_id: Some(correlation),
            target_id: target,
            end_at: None,
        }
    }
}
```

#### Catchup Flow
```
1. Client submits CatchupSubscription query
2. Query handler acknowledges and starts streaming
3. Event store retrieves all events after start_after
4. If correlation_id specified, filter to that correlation chain
5. Events are streamed to client in order
6. Client applies events to rebuild current state
7. Once caught up, can transition to live subscription
```

#### Implementation Pattern
```rust
pub async fn handle_catchup_subscription(
    subscription: CatchupSubscription,
    event_store: &EventStore,
) -> Result<impl Stream<Item = DomainEvent>> {
    // Retrieve events from the event store
    let events = event_store
        .get_events_after(subscription.start_after)
        .await?;

    // Filter by correlation if specified
    let filtered = match subscription.correlation_id {
        Some(correlation) => events
            .filter(|e| e.correlation_id == correlation)
            .collect(),
        None => events,
    };

    // Return as async stream
    Ok(stream::iter(filtered))
}

// Client-side usage
pub async fn catchup_aggregate<A: Aggregate>(
    aggregate_id: AggregateId,
    last_known_event: IdType,
    event_stream: &mut EventStream,
) -> Result<A> {
    let subscription = CatchupSubscription::from_message(
        last_known_event,
        aggregate_id.into(),
    );

    // Submit catchup query
    let catchup_stream = event_stream
        .catchup_subscribe(subscription)
        .await?;

    // Rebuild aggregate from events
    let mut aggregate = A::default();
    pin_mut!(catchup_stream);

    while let Some(event) = catchup_stream.next().await {
        aggregate.apply_event(event)?;
    }

    Ok(aggregate)
}
```

#### Use Cases for Catchup Subscriptions

1. **Reconnection After Offline Period**
   - Client stores last processed event ID
   - On reconnect, catchup from that point
   - Seamlessly resume without full reload

2. **Projection Rebuilding**
   - Start from specific snapshot point
   - Apply only events after snapshot
   - Faster than full event replay

3. **Debugging and Auditing**
   - Replay events from specific point
   - Follow correlation chain through system
   - Understand sequence of changes

4. **Partial Synchronization**
   - Sync only specific correlation chains
   - Reduce bandwidth for large datasets
   - Targeted updates for specific workflows

## Local-First Message Propagation

### Hierarchical Layers
Messages bubble up through a defined hierarchy, enabling offline operation and gradual synchronization:

```
┌─────────────────┐
│  Super-Cluster  │  ← Global coordination
└────────┬────────┘
         │
┌────────┴────────┐
│     Cluster     │  ← Regional/organizational boundary
└────────┬────────┘
         │
┌────────┴────────┐
│   Local Leaf    │  ← Edge server/gateway
└────────┬────────┘
         │
┌────────┴────────┐
│ Local Container │  ← Application container/runtime
└────────┬────────┘
         │
┌────────┴────────┐
│   Local App     │  ← User application instance
└─────────────────┘
```

### Message Bubbling Rules

#### 1. Local Processing First
- Commands are processed locally when possible
- Local event store maintains recent history
- Projections built from local events
- No network dependency for core operations

#### 2. Selective Propagation
Messages bubble up based on scope and relevance:
- **Local-only**: UI state, draft changes, temporary data
- **Container-scoped**: User preferences, session data
- **Leaf-scoped**: Team/department data, local policies
- **Cluster-scoped**: Organization-wide events, shared resources
- **Super-cluster**: Global events, cross-organization coordination

#### 3. Propagation Triggers
```rust
pub enum PropagationScope {
    LocalOnly,           // Never leaves the app
    Container,           // Bubbles to container
    Leaf,               // Bubbles to local leaf
    Cluster,            // Bubbles to cluster
    SuperCluster,       // Global propagation
}

pub struct EventEnvelope {
    pub event: DomainEvent,
    pub scope: PropagationScope,
    pub propagation_policy: PropagationPolicy,
}

pub enum PropagationPolicy {
    Immediate,          // Propagate as soon as possible
    Batched(Duration),  // Batch with other events
    OnConnection,       // Wait for connectivity
    Manual,            // User-triggered sync
}
```

#### 4. Conflict Resolution
When messages from different layers conflict:
- **Last-Write-Wins**: For simple value updates
- **CRDT-based**: For collaborative data structures
- **Domain-specific**: Custom resolution based on business rules
- **User-mediated**: Present conflicts for user resolution

### Implementation Patterns

#### Local Event Store
```rust
pub struct LocalEventStore {
    // In-memory recent events
    recent_events: RingBuffer<DomainEvent>,

    // Persistent local storage
    local_db: LocalDatabase,

    // Pending propagation queue
    propagation_queue: PriorityQueue<EventEnvelope>,
}
```

#### Synchronization Service
```rust
pub struct SyncService {
    local_store: LocalEventStore,
    parent_connection: Option<ParentConnection>,
    sync_state: SyncState,
}

impl SyncService {
    pub async fn sync_with_parent(&mut self) -> Result<SyncReport> {
        // 1. Send pending local events
        let pending = self.local_store.get_pending_propagation();

        // 2. Receive events from parent
        let parent_events = self.parent_connection
            .as_ref()
            .map(|conn| conn.fetch_events_since(self.sync_state.last_sync))
            .transpose()?;

        // 3. Resolve conflicts
        let resolved = self.resolve_conflicts(pending, parent_events)?;

        // 4. Update local state
        self.apply_sync_results(resolved).await
    }
}
```

### Benefits of Local-First

1. **Offline Capability**: Full functionality without network
2. **Low Latency**: Immediate local processing
3. **Resilience**: Network failures don't break the app
4. **Privacy**: Data stays local until explicitly shared
5. **Scalability**: Reduced server load through edge processing

### Challenges and Solutions

#### Challenge: Event Ordering
**Solution**: Hybrid logical clocks combining local time and logical counters

#### Challenge: Storage Limits
**Solution**: Sliding window of events with periodic snapshots

#### Challenge: Conflict Detection
**Solution**: Vector clocks or interval tree clocks for causality tracking

#### Challenge: Schema Evolution
**Solution**: Event versioning with upgrade paths

## Subject-Based Domain Organization

### Subjects as Context Routing Mechanism

Subjects are the primary mechanism for organizing domain boundaries and controlling message routing between contexts. They act as the "address system" for our distributed domain, independent of the propagation layer.

#### Subject Naming Convention
```
<context>.<aggregate>.<event_type>.<version>

Examples:
people.person.registered.v1
organizations.company.created.v1
policies.access_control.updated.v1
agents.ai_assistant.deployed.v1
locations.region.merged.v1
graph.node.added.v1
workflow.process.started.v1
```

#### Subject Components
1. **Context**: Bounded context name (people, organizations, agents, etc.)
2. **Aggregate**: The aggregate root type
3. **Event Type**: What happened (created, updated, deleted, etc.)
4. **Version**: Schema version for evolution

#### Layer Escalation (Orthogonal to Subjects)
The propagation layer is determined by the event envelope, not the subject:

```rust
pub struct EventEnvelope {
    pub event: DomainEvent,
    pub subject: String,  // e.g., "people.person.registered.v1"
    pub propagation: PropagationScope,  // Determines if/how to escalate
}

pub enum PropagationScope {
    LocalOnly,           // Never leaves the app
    Container,           // May bubble to container
    Leaf,               // May bubble to local leaf
    Cluster,            // May bubble to cluster
    SuperCluster,       // May bubble globally
}
```

### Message Translation Bridge

The bridge pattern translates between different message systems while preserving domain semantics:

```rust
/// Trait for translating between message systems
pub trait MessageTranslator<From, To> {
    type Error;

    fn translate(&self, from: From) -> Result<To, Self::Error>;
    fn reverse(&self, to: To) -> Result<From, Self::Error>;
}

/// NATS to Bevy translator
pub struct NatsToBevyTranslator {
    subject_parser: SubjectParser,
    component_mapper: ComponentMapper,
}

impl MessageTranslator<NatsMessage, BevyCommand> for NatsToBevyTranslator {
    type Error = TranslationError;

    fn translate(&self, msg: NatsMessage) -> Result<BevyCommand, Self::Error> {
        // Parse subject to understand domain context
        let subject_parts = self.subject_parser.parse(&msg.subject)?;

        // Extract domain event from payload
        let domain_event: DomainEvent = serde_json::from_slice(&msg.payload)?;

        // Map to appropriate Bevy command based on subject
        match (subject_parts.context.as_str(), subject_parts.event_type.as_str()) {
            ("people", "registered") => {
                let event: PersonRegistered = serde_json::from_value(domain_event.payload)?;
                Ok(BevyCommand::SpawnEntity {
                    components: vec![
                        self.component_mapper.to_person_component(event.person_id),
                        self.component_mapper.to_name_component(event.name),
                        self.component_mapper.to_position_component(event.location),
                    ],
                })
            }
            ("graph", "node_added") => {
                let event: NodeAdded = serde_json::from_value(domain_event.payload)?;
                Ok(BevyCommand::SpawnNode {
                    node_id: event.node_id,
                    position: event.position,
                    components: self.component_mapper.map_node_components(event.components),
                })
            }
            _ => Err(TranslationError::UnknownEventType)
        }
    }

    fn reverse(&self, cmd: BevyCommand) -> Result<NatsMessage, Self::Error> {
        // Translate Bevy commands back to NATS messages
        match cmd {
            BevyCommand::UpdatePosition { entity_id, new_position } => {
                let subject = "graph.node.moved.v1".to_string();
                let event = NodeMoved {
                    node_id: entity_id.into(),
                    old_position: None, // Would need to track this
                    new_position,
                    timestamp: SystemTime::now(),
                };

                Ok(NatsMessage {
                    subject,
                    payload: serde_json::to_vec(&event)?,
                    headers: Default::default(),
                })
            }
            _ => Err(TranslationError::UnsupportedCommand)
        }
    }
}
```

### Context-Based Access Control

Subjects naturally provide context-based access control:

```rust
pub struct SubjectPermissions {
    /// What subjects can this actor publish to
    pub publish_allow: Vec<SubjectPattern>,

    /// What subjects can this actor subscribe to
    pub subscribe_allow: Vec<SubjectPattern>,

    /// Explicitly denied patterns (override allows)
    pub deny: Vec<SubjectPattern>,
}

impl SubjectPermissions {
    pub fn can_publish(&self, subject: &str) -> bool {
        !self.deny.iter().any(|p| p.matches(subject)) &&
        self.publish_allow.iter().any(|p| p.matches(subject))
    }

    pub fn can_subscribe(&self, subject: &str) -> bool {
        !self.deny.iter().any(|p| p.matches(subject)) &&
        self.subscribe_allow.iter().any(|p| p.matches(subject))
    }
}

// Example: Agent can only operate in specific contexts
let agent_permissions = SubjectPermissions {
    publish_allow: vec![
        SubjectPattern::new("agents.ai_assistant.>"),
        SubjectPattern::new("workflow.process.>"),
    ],
    subscribe_allow: vec![
        SubjectPattern::new("agents.>"),
        SubjectPattern::new("people.*.query_result.>"),
        SubjectPattern::new("organizations.*.query_result.>"),
    ],
    deny: vec![
        SubjectPattern::new("*.*.deleted.>"), // Can't delete anything
        SubjectPattern::new("policies.>"),    // Can't modify policies
    ],
};
```

### Cross-Context Communication

Subjects enable clean cross-context communication patterns:

```rust
pub struct ContextBridge {
    context_mappings: HashMap<String, ContextMapping>,
}

impl ContextBridge {
    /// Route messages between contexts based on business rules
    pub fn route_cross_context(&self, msg: NatsMessage) -> Result<Vec<NatsMessage>> {
        let subject_parts = parse_subject(&msg.subject)?;
        let source_context = &subject_parts.context;

        // Find contexts that need to know about this event
        let interested_contexts = self.find_interested_contexts(source_context, &subject_parts.event_type)?;

        // Create appropriate messages for each context
        interested_contexts.into_iter().map(|target_context| {
            let mapping = self.context_mappings.get(&target_context)
                .ok_or(RoutingError::NoMapping)?;

            // Transform the event for the target context's language
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

// Example: When a person is registered, notify the agent context
let person_to_agent_mapping = ContextMapping {
    source_context: "people".to_string(),
    target_context: "agents".to_string(),
    event_mappings: vec![
        EventMapping {
            source_event: "person.registered",
            target_event: "potential_user.identified",
            transformer: Box::new(|event| {
                // Transform PersonRegistered to PotentialUserIdentified
                // preserving relevant information for agent context
            }),
        },
    ],
};
```

### Subject Evolution Patterns

As domains evolve, subjects must support versioning and migration:

```rust
pub struct SubjectEvolution {
    pub from_version: String,
    pub to_version: String,
    pub translator: Box<dyn MessageTranslator<DomainEvent, DomainEvent>>,
}

pub struct SubjectMigrationService {
    evolutions: HashMap<(String, String), SubjectEvolution>,
}

impl SubjectMigrationService {
    pub async fn migrate_subscriber(
        &self,
        old_subject: &str,
        new_subject: &str,
    ) -> Result<()> {
        // Parse versions from subjects
        let old_version = extract_version(old_subject)?;
        let new_version = extract_version(new_subject)?;

        // Find evolution path
        let evolution = self.evolutions
            .get(&(old_version.clone(), new_version.clone()))
            .ok_or(MigrationError::NoEvolutionPath)?;

        // Set up translation proxy
        let translator = TranslationProxy::new(
            old_subject,
            new_subject,
            evolution.translator.clone(),
        );

        // Start proxying messages with translation
        translator.start().await
    }
}
```

### Subject Hierarchies for Domain Organization

While subjects don't include layer information, they can express domain hierarchies:

```rust
// Hierarchical contexts
people.employee.hired.v1
people.employee.manager.assigned.v1
people.contractor.engaged.v1

// Aggregate hierarchies
organizations.company.created.v1
organizations.company.department.added.v1
organizations.company.department.team.formed.v1

// Workflow stages
workflow.order.initiated.v1
workflow.order.payment.processed.v1
workflow.order.fulfillment.shipped.v1
workflow.order.completed.v1
```

These hierarchies are about domain relationships, not propagation layers. The decision to escalate an event from local to container/leaf/cluster is orthogonal to the subject structure and controlled by the event envelope's propagation scope.

## Correlation and Causation

### CorrelationId
- **First message**: Self-referential (the message IS the correlation)
- **Subsequent messages**: Share the same correlation ID
- **Purpose**: Group all related messages in a workflow

### CausationId
- **Always references existing message**: Cannot be generated
- **Optional**: Only present when caused by another message
- **Purpose**: Track what specifically triggered this message

### ID Types
```rust
pub enum IdType {
    Uuid(Uuid),  // Commands and Queries
    Cid(Cid),    // Events (content-addressed)
}
```

## Domain Building Blocks

### Aggregates
**When to use**:
- Need transactional consistency
- Multiple related entities that must change together
- Business invariants span multiple objects

**Requirements**:
- MUST have a state machine for lifecycle management
- MUST enforce all business invariants
- MUST publish events for all state changes
- SHOULD be small and focused

**Example**:
```rust
pub struct OrderAggregate {
    id: OrderId,
    state: StateMachine<OrderState>,
    items: Vec<OrderItem>,
    // ... other fields
}
```

### Entities
**When to use**:
- Object has unique identity
- Identity persists across state changes
- Need to track lifecycle

**Requirements**:
- MUST have unique identifier (UUID v7 preferred)
- MAY have state machine if lifecycle is complex
- SHOULD be part of an aggregate

### Value Objects
**When to use**:
- Data defined by attributes, not identity
- Immutable by nature
- Can be freely copied/shared

**Key Principle**: Value Objects are NEVER updated - they are replaced
```rust
// WRONG
EdgeUpdated { old_edge, new_edge }

// CORRECT
EdgeRemoved { edge_id }
EdgeAdded { edge_id, source, target, relationship }
```

### Domain Events
**When to use**:
- Something significant happened in the domain
- Other contexts need to know
- Need audit trail

**Requirements**:
- MUST be named in past tense
- MUST be immutable
- MUST contain all relevant data (no entity references)
- SHOULD be small and focused

### State Machines
**When to use**:
- Aggregates (always)
- Complex entity lifecycles
- Multi-step workflows
- Need to enforce valid transitions

**When NOT to use**:
- Simple CRUD operations
- Stateless calculations
- Value object changes

## Core Domain Entities

CIM identifies five essential entities that any information system needs:

### 1. People
- Human actors in the system
- Identity, authentication, profiles
- Relationships to organizations

### 2. Agents
- Automated actors (AI, bots, services)
- Bounded capabilities and permissions
- Act on behalf of people or organizations

### 3. Organizations
- Groups, companies, teams
- Hierarchical structures
- Collective ownership and permissions

### 4. Locations
- Physical and virtual spaces
- Spatial context for activities
- Geographic and logical boundaries

### 5. Policies
- Rules and constraints
- Governance and compliance
- Dynamic behavior modification

## Implementation Guidelines

### 1. Start with Events
- Identify domain events first
- Events drive aggregate design
- Events define integration points

### 2. Keep Aggregates Small
- One aggregate per transaction
- Minimize aggregate size
- Reference other aggregates by ID only

### 3. Embrace Eventual Consistency
- Between aggregates
- Across bounded contexts
- Through event propagation

### 4. Use Enums for Domain Concepts
```rust
pub enum OrderStatus {
    Draft,
    Submitted,
    Processing,
    Completed,
    Cancelled,
}

pub enum PaymentMethod {
    CreditCard { last_four: String },
    BankTransfer { account_suffix: String },
    Cryptocurrency { wallet_type: String },
}
```

### 5. Leverage Type System
- Phantom types for compile-time guarantees
- Newtype pattern for domain primitives
- Sealed traits for extensibility control

## Testing Strategy

### Unit Tests
- Test aggregates with in-memory event store
- Test state machines for all transitions
- Test value object invariants

### Integration Tests
- Test command/event flow
- Test projections and read models
- Test cross-context integration

### Property-Based Tests
- Test aggregate invariants hold
- Test event ordering properties
- Test state machine properties

## Anti-Patterns to Avoid

### ❌ Returning Results from Commands/Queries
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
    fn handle(&self, cmd: Command) -> CommandAcknowledgment;
}
```

### ❌ Updating Value Objects
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

## Future Considerations

### Conceptual Spaces
- Integration with knowledge representation
- Semantic positioning of entities
- AI-driven relationship discovery

### Graph Workflows
- Visual workflow design
- Self-documenting processes
- Runtime workflow modification

### Dog-fooding
- System visualizes its own development
- Self-improvement through analysis
- Meta-circular architecture

## Summary

This design provides a foundation for building robust, scalable domains that:
- Embrace asynchronous, event-driven patterns
- Support offline-first, local operation
- Enable hierarchical message propagation
- Use subjects as the primary domain organization mechanism
- Provide clean translation between message systems
- Leverage Rust's type system for safety
- Maintain clear boundaries and responsibilities
- Support evolution and extension
- Enable both human and AI understanding

Follow these principles consistently to create domains that compose well within the larger CIM ecosystem.
