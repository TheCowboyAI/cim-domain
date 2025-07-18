# CIM Domain Testing Guide

## Overview

The CIM Domain maintains comprehensive test coverage with 196+ tests across all modules. This document describes the testing philosophy, patterns, and practices used throughout the codebase.

## Testing Philosophy

### Core Principles

1. **Test-First Development** - Write tests before implementation
2. **100% Coverage Required** - Every public API must be tested
3. **Domain Isolation** - No infrastructure dependencies in domain tests
4. **Living Documentation** - Tests serve as usage examples
5. **Fast Feedback** - All tests complete in < 0.01s

### Testing Metrics

| Metric | Value | Target |
|--------|-------|--------|
| Total Tests | 196 | Continuous growth |
| Module Coverage | 94% | 100% |
| Test Execution | < 0.01s | < 1s |
| Domain Isolation | 100% | 100% |
| Pass Rate | 100% | 100% |

## Test Organization

### Directory Structure

```
cim-domain/
├── src/
│   └── *.rs files with unit tests
├── tests/
│   ├── infrastructure_tests.rs
│   ├── jetstream_event_store_tests.rs
│   └── infrastructure/
│       ├── test_cqrs_flow.rs
│       ├── test_event_store.rs
│       └── test_nats_connection.rs
└── crates/
    └── */tests/
        └── integration tests per crate
```

### Test Categories

#### 1. Unit Tests

Located in `src/*.rs` files alongside implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entity_creation() {
        let entity = Entity::<PersonMarker>::new();
        assert_eq!(entity.version(), 0);
    }
}
```

#### 2. Integration Tests

Located in `tests/` directory:

```rust
#[tokio::test]
async fn test_command_to_event_flow() {
    let command = RegisterPerson { /* ... */ };
    let events = handle_command(command).await.unwrap();
    assert_eq!(events.len(), 1);
}
```

#### 3. Documentation Tests

In code documentation:

```rust
/// Creates a new person entity
/// 
/// # Example
/// ```
/// use cim_domain::Person;
/// 
/// let person = Person::new();
/// assert!(person.id().to_string().len() > 0);
/// ```
pub fn new() -> Self { /* ... */ }
```

## Testing Patterns

### Aggregate Testing

Test aggregates through their command/event interface:

```rust
#[test]
fn test_person_registration() {
    let mut person = Person::new();
    
    let command = RegisterPerson {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    let events = person.handle_command(command).unwrap();
    
    // Verify events
    assert_eq!(events.len(), 1);
    match &events[0] {
        PersonEvent::Registered { name, email, .. } => {
            assert_eq!(name, "Alice");
            assert_eq!(email, "alice@example.com");
        }
        _ => panic!("Wrong event type"),
    }
    
    // Verify state change
    person.apply_event(&events[0]);
    assert_eq!(person.version(), 1);
}
```

### Component Testing

Test component lifecycle and type safety:

```rust
#[test]
fn test_component_storage() {
    let mut storage = ComponentStorage::new();
    
    let identity = IdentityComponent {
        legal_name: "John Doe".to_string(),
        preferred_name: Some("John".to_string()),
        date_of_birth: None,
        government_id: None,
    };
    
    // Add component
    storage.add_component(identity.clone());
    
    // Retrieve component
    let retrieved = storage.get_component::<IdentityComponent>().unwrap();
    assert_eq!(retrieved.legal_name, "John Doe");
    
    // Type safety
    assert!(storage.get_component::<ContactComponent>().is_none());
}
```

### State Machine Testing

Test all state transitions and invariants:

```rust
#[test]
fn test_agent_state_transitions() {
    let mut state = AgentStatus::Initializing;
    
    // Valid transition
    assert!(state.can_transition_to(&AgentStatus::Active));
    state = state.transition_to(AgentStatus::Active).unwrap();
    
    // Invalid transition
    assert!(state.transition_to(AgentStatus::Initializing).is_err());
    
    // Terminal state
    state = AgentStatus::Decommissioned;
    assert!(state.valid_transitions().is_empty());
}
```

### Value Object Testing

Test immutability and validation:

```rust
#[test]
fn test_email_validation() {
    // Valid email
    let email = EmailAddress::new("test@example.com".to_string());
    assert!(email.is_ok());
    
    // Invalid emails
    assert!(EmailAddress::new("".to_string()).is_err());
    assert!(EmailAddress::new("notanemail".to_string()).is_err());
    assert!(EmailAddress::new("@example.com".to_string()).is_err());
}

#[test]
fn test_address_immutability() {
    let address = Address::new(
        "123 Main St",
        "New York",
        "NY",
        "US",
        "10001"
    ).unwrap();
    
    // Create new address instead of mutating
    let new_address = address.with_street("456 Oak Ave");
    
    assert_eq!(address.street(), "123 Main St");
    assert_eq!(new_address.street(), "456 Oak Ave");
}
```

### CQRS Testing

Test command and query handling separately:

```rust
#[test]
fn test_command_acknowledgment() {
    let command = CreateOrganization {
        name: "Acme Corp".to_string(),
        org_type: OrganizationType::Company,
    };
    
    let ack = handle_command(command);
    
    match ack {
        CommandAck::Accepted { command_id } => {
            assert!(!command_id.is_nil());
        }
        CommandAck::Rejected { reason, .. } => {
            panic!("Command rejected: {}", reason);
        }
    }
}

#[test]
fn test_query_subscription() {
    let query = GetPersonById {
        person_id: EntityId::new(),
    };
    
    let ack = handle_query(query);
    
    match ack {
        QueryAck::Accepted { query_id, subscription } => {
            assert!(!query_id.is_nil());
            assert!(subscription.stream_subject.contains("person"));
        }
        QueryAck::Rejected { .. } => panic!("Query rejected"),
    }
}
```

## Test Utilities

### Mock Event Publisher

```rust
pub struct MockEventPublisher {
    published_events: Arc<Mutex<Vec<DomainEvent>>>,
}

impl EventPublisher for MockEventPublisher {
    async fn publish(&self, event: DomainEvent) -> Result<()> {
        self.published_events.lock().unwrap().push(event);
        Ok(())
    }
}
```

### In-Memory Repository

```rust
pub struct InMemoryRepository<T: AggregateRoot> {
    storage: Arc<RwLock<HashMap<EntityId<T>, T>>>,
}

impl<T: AggregateRoot> Repository<T> for InMemoryRepository<T> {
    async fn save(&self, aggregate: &T) -> Result<()> {
        self.storage.write().await.insert(aggregate.id(), aggregate.clone());
        Ok(())
    }
    
    async fn get(&self, id: EntityId<T>) -> Result<Option<T>> {
        Ok(self.storage.read().await.get(&id).cloned())
    }
}
```

### Test Fixtures

```rust
pub mod fixtures {
    pub fn valid_person() -> Person {
        let mut person = Person::new();
        person.add_component(IdentityComponent {
            legal_name: "Test User".to_string(),
            preferred_name: None,
            date_of_birth: None,
            government_id: None,
        }).unwrap();
        person
    }
    
    pub fn valid_organization() -> Organization {
        Organization::new(
            "Test Org",
            OrganizationType::Company,
        ).unwrap()
    }
}
```

## Testing Best Practices

### 1. Arrange-Act-Assert

```rust
#[test]
fn test_policy_approval() {
    // Arrange
    let mut policy = Policy::new(
        PolicyType::AccessControl,
        PolicyScope::Global,
    );
    
    // Act
    let events = policy.approve(approver_id).unwrap();
    
    // Assert
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], PolicyEvent::Approved { .. }));
}
```

### 2. Test Edge Cases

```rust
#[test]
fn test_empty_component_storage() {
    let storage = ComponentStorage::new();
    assert!(storage.get_component::<IdentityComponent>().is_none());
    assert_eq!(storage.component_count(), 0);
}

#[test]
fn test_max_hierarchy_depth() {
    let mut org = Organization::new("Root", OrganizationType::Company).unwrap();
    
    // Test deep hierarchy
    for i in 0..MAX_DEPTH {
        let child_id = EntityId::new();
        org.add_child_unit(child_id).unwrap();
    }
    
    // Should fail at max depth
    let too_deep = EntityId::new();
    assert!(org.add_child_unit(too_deep).is_err());
}
```

### 3. Property-Based Testing

```rust
#[quickcheck]
fn prop_entity_id_uniqueness(count: u8) -> bool {
    let ids: HashSet<EntityId<PersonMarker>> = (0..count)
        .map(|_| EntityId::new())
        .collect();
    
    ids.len() == count as usize
}

#[quickcheck]
fn prop_event_ordering(events: Vec<TestEvent>) -> bool {
    let mut last_timestamp = None;
    
    for event in events {
        if let Some(last) = last_timestamp {
            if event.occurred_at < last {
                return false;
            }
        }
        last_timestamp = Some(event.occurred_at);
    }
    
    true
}
```

### 4. Async Testing

```rust
#[tokio::test]
async fn test_async_command_handling() {
    let handler = CommandHandler::new();
    
    let command = RegisterPerson {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    let ack = handler.handle(command).await;
    
    assert!(matches!(ack, CommandAck::Accepted { .. }));
}
```

## Visual Test Documentation

Many tests include Mermaid diagrams for clarity:

```rust
#[test]
fn test_state_machine_transitions() {
    // Test validates this state machine:
    // ```mermaid
    // stateDiagram-v2
    //     [*] --> Initializing
    //     Initializing --> Active
    //     Active --> Suspended
    //     Active --> Offline
    //     Active --> Decommissioned
    //     Suspended --> Active
    //     Suspended --> Decommissioned
    //     Offline --> Active
    //     Offline --> Decommissioned
    //     Decommissioned --> [*]
    // ```
    
    // Test implementation...
}
```

## Performance Testing

### Benchmarks

```rust
#[bench]
fn bench_component_lookup(b: &mut Bencher) {
    let mut storage = ComponentStorage::new();
    
    // Add 100 components
    for i in 0..100 {
        storage.add_component(TestComponent { id: i });
    }
    
    b.iter(|| {
        storage.get_component::<TestComponent>()
    });
}
```

### Load Testing

```rust
#[test]
fn test_high_volume_events() {
    let publisher = MockEventPublisher::new();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    runtime.block_on(async {
        let handles: Vec<_> = (0..1000)
            .map(|i| {
                let pub_clone = publisher.clone();
                tokio::spawn(async move {
                    pub_clone.publish(test_event(i)).await
                })
            })
            .collect();
        
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
    });
    
    assert_eq!(publisher.event_count(), 1000);
}
```

## Continuous Integration

### Test Execution

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_person_registration

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench

# Check coverage
cargo tarpaulin
```

### Pre-commit Checks

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run tests
cargo test || exit 1

# Check formatting
cargo fmt -- --check || exit 1

# Run clippy
cargo clippy -- -D warnings || exit 1
```

## Future Testing Enhancements

### Planned Improvements

1. **Mutation Testing** - Ensure test quality with mutation analysis
2. **Fuzz Testing** - Find edge cases with property-based fuzzing
3. **Integration Test Suite** - End-to-end scenarios with full stack
4. **Performance Regression** - Track performance over time
5. **Visual Test Reports** - HTML reports with coverage visualization

### Research Areas

1. **Formal Verification** - Prove correctness of critical invariants
2. **Model-Based Testing** - Generate tests from specifications
3. **Chaos Testing** - Test resilience to failures
4. **Contract Testing** - Verify inter-domain contracts

## Summary

The CIM Domain testing approach ensures:

- **Correctness** through comprehensive coverage
- **Maintainability** through clear test organization
- **Documentation** through example-driven tests
- **Performance** through fast execution
- **Quality** through multiple testing strategies

This robust testing foundation enables confident development and refactoring while maintaining system reliability.