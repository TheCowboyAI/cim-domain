# Isomorphic Component Architecture

This document describes the isomorphic mapping between Domain-Driven Design (DDD) components and Entity Component System (ECS) components in the CIM architecture.

## Overview

The isomorphic architecture ensures that:
1. **EVERYTHING runs on ECS** (Bevy Entity Component System)
2. **Some things enhance ECS with DDD** patterns
3. **NATS is the ONLY transport** allowed between process boundaries
4. **1:1 mapping** exists between DDD components and ECS components

## Architecture Components

### 1. Component Trait (cim-component)
The base `Component` trait provides:
- Type erasure for attachable components
- Serialization/deserialization support
- Conversion to ECS-compatible data format
- Event generation for component state changes

### 2. Domain Component Sync (cim-domain)
- `DomainComponentSync`: Manages component synchronization via NATS
- `DomainComponentBridge`: Bridges domain events to component events
- Publishes `ComponentEvent` messages to NATS topics

### 3. NATS Component Bridge (cim-domain-bevy)
- `NatsComponentBridge`: Receives component events from NATS
- `NatsSyncedEntity`: Marks entities synchronized via NATS
- `PendingComponentUpdate`: Queues component updates for application
- Systems to process events and apply updates to Bevy ECS

## Data Flow

1. **Domain Layer** (DDD):
   - Domain events trigger component updates
   - Components implement the `Component` trait
   - `DomainComponentSync` publishes events to NATS

2. **Transport Layer** (NATS):
   - Component events published to `cim.component.*` topics
   - Events contain serialized component data
   - Guarantees delivery across process boundaries

3. **Visualization Layer** (Bevy ECS):
   - `NatsComponentBridge` subscribes to component events
   - Events mapped to Bevy components
   - ECS systems apply updates to entities

## Component Event Types

```rust
pub enum ComponentEvent {
    Added { entity_id: Uuid, component_data: EcsComponentData },
    Updated { entity_id: Uuid, component_data: EcsComponentData },
    Removed { entity_id: Uuid, component_type: String },
}
```

## Example Usage

### Domain Side (DDD)
```rust
// Define a domain component
#[derive(Component, Serialize, Deserialize)]
struct WorkflowStateComponent {
    state: String,
    definition_id: String,
}

// Component automatically synced when attached to entity
entity.add_component(WorkflowStateComponent {
    state: "Running".to_string(),
    definition_id: "order-processing".to_string(),
});
```

### Bevy Side (ECS)
```rust
// Component updates automatically received and applied
fn apply_component_updates(
    mut commands: Commands,
    query: Query<(Entity, &PendingComponentUpdate), With<NatsSyncedEntity>>,
) {
    for (entity, pending) in query.iter() {
        match pending.component_data.component_type.as_str() {
            "WorkflowStateComponent" => {
                // Apply workflow visualization
            }
            _ => {}
        }
    }
}
```

## Benefits

1. **Decoupling**: Domain logic separated from visualization
2. **Scalability**: Components sync across distributed systems
3. **Flexibility**: New component types added without changing transport
4. **Performance**: Async NATS messaging prevents blocking
5. **Debugging**: All component changes traceable through events

## Testing

See `/cim-domain-bevy/tests/isomorphic_component_sync_test.rs` for comprehensive tests of the isomorphic architecture.