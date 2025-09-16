# Pragmatic FP with Domain Patterns

## Core Philosophy
**Prefer FP, but be pragmatic.** When we break from FP, we document WHY.

## Domain Pattern: Entity-Component Architecture

### Every Entity MUST Follow This Pattern:
```rust
// Entity = ID + Components
#[derive(Clone, Debug)]
struct User {
    id: EntityId<User>,
    components: UserComponents,
}

// Components are immutable invariants
#[derive(Clone, Debug)]
struct UserComponents {
    name: Name,
    email: Email,
    created_at: Timestamp,
    // Each component enforces its own invariants
}

// Components are newtype wrappers with invariants
#[derive(Clone, Debug)]
struct Email(String);

impl Email {
    // Constructor enforces invariants
    pub fn new(s: String) -> Result<Self, ValidationError> {
        if s.contains('@') {
            Ok(Email(s))
        } else {
            Err(ValidationError::InvalidEmail)
        }
    }
}
```

## When to Prefer FP

### 1. State Transformations
```rust
// PREFER: Pure transformation
fn apply_event(user: User, event: UserEvent) -> User {
    match event {
        UserEvent::EmailChanged { new_email } => User {
            components: UserComponents {
                email: new_email,
                ..user.components
            },
            ..user
        }
    }
}
```

### 2. Business Logic
```rust
// PREFER: Return effects as data
fn handle_command(cmd: Command, state: State) -> (State, Vec<Event>) {
    // Pure logic, no side effects
    match cmd {
        Command::UpdateEmail { user_id, email } => {
            let new_state = update_user_email(state, user_id, email.clone());
            let events = vec![Event::EmailUpdated { user_id, email }];
            (new_state, events)
        }
    }
}
```

### 3. Validation
```rust
// PREFER: Validation as pure functions returning Result
fn validate_user_components(components: &UserComponents) -> Result<(), Vec<ValidationError>> {
    let mut errors = vec![];
    
    if components.name.is_empty() {
        errors.push(ValidationError::EmptyName);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
```

## When to Break FP (With Documentation)

### 1. Performance-Critical Paths
```rust
// BREAKING FP: Mutation for performance in hot path
// REASON: This processes 1M events/sec and allocation would kill performance
impl EventBuffer {
    pub fn append_unchecked(&mut self, event: Event) {
        // Direct mutation for performance
        self.events.push(event);
        self.count += 1;
    }
}
```

### 2. System Boundaries (I/O)
```rust
// BREAKING FP: Mutable connection state
// REASON: NATS client requires mutable state for connection management
pub struct NatsConnection {
    client: Arc<Mutex<Client>>, // Shared mutable state
}

impl NatsConnection {
    // Async I/O inherently involves effects
    pub async fn publish(&self, subject: String, payload: Vec<u8>) -> Result<()> {
        let client = self.client.lock().await;
        client.publish(subject, payload).await
    }
}
```

### 3. Resource Management
```rust
// BREAKING FP: RAII pattern for resource cleanup
// REASON: Must guarantee cleanup even on panic
pub struct ConnectionGuard {
    conn: Option<Connection>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            conn.close(); // Side effect on drop
        }
    }
}
```

### 4. Caching/Memoization
```rust
// BREAKING FP: Internal caching for expensive computations
// REASON: CID calculation is expensive and idempotent
pub struct CidCache {
    cache: Arc<RwLock<HashMap<Vec<u8>, Cid>>>,
}

impl CidCache {
    pub async fn get_or_compute(&self, data: &[u8]) -> Cid {
        {
            let cache = self.cache.read().await;
            if let Some(cid) = cache.get(data) {
                return cid.clone();
            }
        }
        
        let cid = compute_cid(data); // Expensive
        
        let mut cache = self.cache.write().await;
        cache.insert(data.to_vec(), cid.clone());
        cid
    }
}
```

## Domain Patterns to Follow

### 1. Aggregate Roots
```rust
// Aggregate = Entity that ensures consistency boundary
pub struct Order {
    id: EntityId<Order>,
    components: OrderComponents,
    // Aggregate includes related entities
    items: Vec<OrderItem>,
}

impl Order {
    // Aggregate methods return new aggregate maintaining invariants
    pub fn add_item(self, item: OrderItem) -> Result<Self, DomainError> {
        // Check invariants
        if self.items.len() >= 100 {
            return Err(DomainError::TooManyItems);
        }
        
        Ok(Order {
            items: [self.items, vec![item]].concat(),
            ..self
        })
    }
}
```

### 2. Value Objects (Components)
```rust
// Components are immutable value objects
#[derive(Clone, Debug, PartialEq)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub fn add(&self, other: &Money) -> Result<Money, DomainError> {
        if self.currency != other.currency {
            return Err(DomainError::CurrencyMismatch);
        }
        
        Ok(Money {
            amount: self.amount + other.amount,
            currency: self.currency,
        })
    }
}
```

### 3. Domain Events
```rust
// Events are immutable facts
#[derive(Clone, Debug)]
pub enum OrderEvent {
    Created {
        id: EntityId<Order>,
        customer_id: EntityId<Customer>,
        created_at: Timestamp,
    },
    ItemAdded {
        order_id: EntityId<Order>,
        item: OrderItem,
        added_at: Timestamp,
    },
}

// Event sourcing with fold
pub fn replay_order(events: &[OrderEvent]) -> Order {
    events.iter().fold(Order::default(), |order, event| {
        apply_order_event(order, event.clone())
    })
}
```

### 4. Repository Pattern (When Needed)
```rust
// BREAKING FP: Repository for I/O boundary
// REASON: Abstracts storage details from domain
#[async_trait]
pub trait OrderRepository {
    async fn find(&self, id: EntityId<Order>) -> Result<Order>;
    async fn save(&self, order: Order) -> Result<()>;
    async fn events_for(&self, id: EntityId<Order>) -> Result<Vec<OrderEvent>>;
}

// But domain logic stays pure
pub fn process_order_command(
    cmd: OrderCommand,
    order: Order,
) -> Result<(Order, Vec<OrderEvent>), DomainError> {
    // Pure transformation
    match cmd {
        OrderCommand::AddItem { item } => {
            let new_order = order.add_item(item.clone())?;
            let event = OrderEvent::ItemAdded {
                order_id: new_order.id,
                item,
                added_at: Timestamp::now(),
            };
            Ok((new_order, vec![event]))
        }
    }
}
```

## Type-Safe IDs
```rust
use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityId<T> {
    value: Uuid,
    _phantom: PhantomData<T>,
}

impl<T> EntityId<T> {
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }
    
    pub fn from_string(s: &str) -> Result<Self, ParseError> {
        Ok(Self {
            value: Uuid::parse_str(s)?,
            _phantom: PhantomData,
        })
    }
}
```

## Documentation Template for Breaking FP

When you must break from FP, document it:

```rust
// BREAKING FP: [What you're doing - e.g., "Using mutable state"]
// REASON: [Why it's necessary - e.g., "NATS client requires it"]
// SAFETY: [How you ensure safety - e.g., "Protected by Mutex"]
// ALTERNATIVE: [What FP approach you considered - e.g., "Tried State monad but too slow"]
```

## Summary Rules

1. **DEFAULT TO FP** - Start with pure functions and immutable data
2. **ENTITIES ARE ID + COMPONENTS** - Follow the domain pattern
3. **DOCUMENT BREAKS** - When you break FP, say why
4. **PERFORMANCE MATTERS** - It's OK to mutate in hot paths
5. **I/O IS IMPURE** - Accept it at system boundaries
6. **KEEP DOMAIN PURE** - Business logic should be pure functions
7. **EVENTS AS DATA** - Prefer event sourcing patterns
8. **TYPE SAFETY** - Use phantom types for IDs
9. **INVARIANTS IN TYPES** - Make illegal states unrepresentable
10. **COMPOSITION** - Prefer function composition over inheritance