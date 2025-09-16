# ECS (Entity-Component-System) as CIM's Core Pattern

## Why ECS for Event-Driven Systems

ECS is fundamentally superior for eventing because:
1. **Components are pure data** - No behavior, just state
2. **Systems are pure functions** - Transform components based on events
3. **Events naturally trigger systems** - Perfect impedance match
4. **Composition over inheritance** - Add/remove capabilities dynamically
5. **Cache-friendly** - Components can be stored contiguously

## The CIM ECS Pattern

### 1. Entities are just IDs
```rust
// Entity is ONLY an identifier - nothing more
pub struct Entity {
    id: EntityId,
}

// Type-safe entity IDs
pub struct EntityId<T> {
    value: Uuid,
    _phantom: PhantomData<T>,
}
```

### 2. Components are Pure Data (Invariants)
```rust
// Components have NO METHODS - just data with invariants
#[derive(Clone, Debug, Component)]
pub struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Debug, Component)]
pub struct Velocity {
    dx: f64,
    dy: f64,
    dz: f64,
}

#[derive(Clone, Debug, Component)]
pub struct Health {
    current: u32,
    max: u32,
}

// Smart constructors enforce invariants
impl Health {
    pub fn new(max: u32) -> Self {
        Health { current: max, max }
    }
}
```

### 3. Systems are Pure Functions Operating on Events
```rust
// Systems process events and return new components
pub fn movement_system(
    event: MovementEvent,
    pos: Position,
    vel: Velocity,
) -> (Position, Vec<Event>) {
    match event {
        MovementEvent::Tick { delta_time } => {
            let new_pos = Position {
                x: pos.x + vel.dx * delta_time,
                y: pos.y + vel.dy * delta_time,
                z: pos.z + vel.dz * delta_time,
            };
            
            let events = vec![
                Event::PositionChanged {
                    old: pos.clone(),
                    new: new_pos.clone(),
                }
            ];
            
            (new_pos, events)
        }
    }
}

// Systems can compose multiple components
pub fn damage_system(
    event: DamageEvent,
    health: Health,
) -> (Option<Health>, Vec<Event>) {
    match event {
        DamageEvent::TakeDamage { amount } => {
            let new_health = Health {
                current: health.current.saturating_sub(amount),
                max: health.max,
            };
            
            let mut events = vec![
                Event::HealthChanged {
                    old: health.current,
                    new: new_health.current,
                }
            ];
            
            if new_health.current == 0 {
                events.push(Event::EntityDied);
                (None, events) // Component removed
            } else {
                (Some(new_health), events)
            }
        }
    }
}
```

## Event-Driven ECS Architecture

### Events Flow Through Systems
```rust
// Events are the primary driver
#[derive(Clone, Debug)]
pub enum GameEvent {
    // Input events
    PlayerInput { entity_id: EntityId<Player>, input: Input },
    
    // System events
    PhysicsTick { delta_time: f64 },
    
    // Component events
    ComponentAdded { entity_id: EntityId<Any>, component: ComponentType },
    ComponentRemoved { entity_id: EntityId<Any>, component: ComponentType },
    
    // Domain events
    EntitySpawned { entity_id: EntityId<Any> },
    EntityDestroyed { entity_id: EntityId<Any> },
}

// Event processor routes events to systems
pub fn process_event(
    event: GameEvent,
    world: World,
) -> (World, Vec<GameEvent>) {
    match event {
        GameEvent::PhysicsTick { delta_time } => {
            // Run movement system on all entities with Position + Velocity
            let mut new_world = world.clone();
            let mut events = vec![];
            
            for entity_id in world.entities_with::<(Position, Velocity)>() {
                let pos = world.get_component::<Position>(entity_id);
                let vel = world.get_component::<Velocity>(entity_id);
                
                let (new_pos, system_events) = movement_system(
                    MovementEvent::Tick { delta_time },
                    pos,
                    vel,
                );
                
                new_world.set_component(entity_id, new_pos);
                events.extend(system_events);
            }
            
            (new_world, events)
        }
        // ... other event handlers
    }
}
```

## Component Storage Pattern

### Sparse Set Storage (Cache-Friendly)
```rust
// BREAKING FP: Mutable storage for performance
// REASON: ECS needs cache-friendly component iteration
pub struct ComponentStorage<C> {
    // Sparse set for O(1) lookup and cache-friendly iteration
    sparse: Vec<Option<usize>>,  // entity_id -> dense index
    dense: Vec<EntityId>,         // packed entity IDs
    components: Vec<C>,           // packed components
}

impl<C: Component> ComponentStorage<C> {
    // Fast iteration over components
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &C)> {
        self.dense.iter()
            .zip(self.components.iter())
            .map(|(id, comp)| (*id, comp))
    }
}
```

## Query System for Component Combinations
```rust
// Type-safe queries for component combinations
pub trait Query {
    type Item<'a>;
    fn fetch<'a>(world: &'a World, entity: EntityId) -> Option<Self::Item<'a>>;
}

// Query for entities with Position AND Velocity
impl Query for (Position, Velocity) {
    type Item<'a> = (Position, Velocity);
    
    fn fetch<'a>(world: &'a World, entity: EntityId) -> Option<Self::Item<'a>> {
        let pos = world.get_component::<Position>(entity)?;
        let vel = world.get_component::<Velocity>(entity)?;
        Some((pos.clone(), vel.clone()))
    }
}
```

## Archetypes for Efficient Storage
```rust
// Archetype = Set of component types
pub struct Archetype {
    component_types: Vec<ComponentTypeId>,
    entities: Vec<EntityId>,
    // Columnar storage for each component type
    storages: HashMap<ComponentTypeId, Box<dyn ComponentStorage>>,
}

// Moving between archetypes when components change
pub fn add_component<C: Component>(
    world: World,
    entity: EntityId,
    component: C,
) -> (World, Vec<Event>) {
    let old_archetype = world.get_archetype(entity);
    let new_archetype = old_archetype.with_component::<C>();
    
    // Move entity to new archetype
    let new_world = world.move_entity(entity, new_archetype, component);
    
    let event = Event::ComponentAdded {
        entity,
        component_type: ComponentTypeId::of::<C>(),
    };
    
    (new_world, vec![event])
}
```

## Event Sourcing with ECS

### Components from Events
```rust
// Rebuild component state from events
pub fn replay_entity_events(events: &[EntityEvent]) -> EntityState {
    events.iter().fold(EntityState::default(), |state, event| {
        match event {
            EntityEvent::ComponentAdded { component_data, .. } => {
                state.add_component_from_data(component_data)
            }
            EntityEvent::ComponentUpdated { component_data, .. } => {
                state.update_component_from_data(component_data)
            }
            EntityEvent::ComponentRemoved { component_type, .. } => {
                state.remove_component(component_type)
            }
        }
    })
}
```

## System Scheduling

### Parallel System Execution
```rust
// Systems that don't share mutable components can run in parallel
pub struct SystemSchedule {
    stages: Vec<Stage>,
}

pub struct Stage {
    systems: Vec<Box<dyn System>>,
    parallel: bool,
}

// BREAKING FP: Parallel mutation for performance
// REASON: ECS systems need to run in parallel for performance
impl SystemSchedule {
    pub async fn run(&self, world: World, event: Event) -> (World, Vec<Event>) {
        let mut current_world = world;
        let mut all_events = vec![];
        
        for stage in &self.stages {
            if stage.parallel {
                // Run systems in parallel (requires careful component access)
                let results = futures::future::join_all(
                    stage.systems.iter().map(|sys| {
                        sys.run(current_world.clone(), event.clone())
                    })
                ).await;
                
                // Merge results
                for (world_update, events) in results {
                    current_world = current_world.merge(world_update);
                    all_events.extend(events);
                }
            } else {
                // Run systems sequentially
                for system in &stage.systems {
                    let (new_world, events) = system.run(current_world, event.clone()).await;
                    current_world = new_world;
                    all_events.extend(events);
                }
            }
        }
        
        (current_world, all_events)
    }
}
```

## CIM-Specific ECS Patterns

### 1. Network Entity Replication
```rust
// Components marked for network replication
#[derive(Component, Replicated)]
pub struct NetworkedPosition {
    pos: Position,
    last_sync: Timestamp,
    authority: NodeId,
}

// Replication events
pub enum ReplicationEvent {
    ComponentChanged {
        entity: EntityId,
        component: ComponentData,
        authority: NodeId,
    }
}
```

### 2. Event-Sourced Components
```rust
// Components that track their event history
#[derive(Component)]
pub struct EventSourced<C> {
    current: C,
    events: Vec<ComponentEvent<C>>,
    version: Version,
}
```

### 3. Reactive Components
```rust
// Components that generate events when changed
#[derive(Component, Reactive)]
pub struct ReactiveHealth {
    value: Health,
    on_change: Vec<HealthChangeHandler>,
}

// Automatically generates events on change
impl ReactiveHealth {
    pub fn take_damage(&self, amount: u32) -> (Self, Vec<Event>) {
        let new_health = Health {
            current: self.value.current.saturating_sub(amount),
            max: self.value.max,
        };
        
        let events = self.on_change.iter()
            .flat_map(|handler| handler.handle(self.value, new_health))
            .collect();
        
        (Self { value: new_health, ..self.clone() }, events)
    }
}
```

## Summary: Why ECS + Events = Perfect Match

1. **Events trigger systems** - Natural flow
2. **Systems transform components** - Pure functions
3. **Components are immutable data** - Perfect for event sourcing
4. **Entity is just an ID** - Minimal coupling
5. **Dynamic composition** - Add/remove components via events
6. **Cache-friendly** - Performance without sacrificing FP
7. **Parallelizable** - Systems can run concurrently
8. **Replay-able** - Reconstruct state from events
9. **Network-ready** - Components can be serialized as events
10. **Type-safe** - Phantom types ensure correctness