//! Advanced patterns for using cim-component
//!
//! This example demonstrates more sophisticated patterns including:
//! - Component composition and relationships
//! - System-like processing patterns
//! - Event-driven component updates
//! - Performance considerations

use cim_component::{Component, ComponentError, ComponentResult};
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

// =============================================================================
// ADVANCED COMPONENT PATTERNS
// =============================================================================

/// Marker component to indicate entity relationships
#[derive(Debug, Clone)]
struct Parent {
    entity_id: u64,
}

impl Component for Parent {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn type_name(&self) -> &'static str {
        "Parent"
    }
}

/// Component that depends on other components
#[derive(Debug, Clone)]
struct Velocity {
    dx: f64,
    dy: f64,
}

impl Component for Velocity {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn type_name(&self) -> &'static str {
        "Velocity"
    }
}

/// Component with validation logic
#[derive(Debug, Clone)]
struct Temperature {
    celsius: f64,
}

impl Temperature {
    /// Constructor with validation
    fn new(celsius: f64) -> Result<Self, String> {
        if celsius < -273.15 {
            Err("Temperature cannot be below absolute zero".to_string())
        } else {
            Ok(Self { celsius })
        }
    }
}

impl Component for Temperature {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn type_name(&self) -> &'static str {
        "Temperature"
    }
}

// =============================================================================
// ADVANCED ENTITY MANAGEMENT
// =============================================================================

/// World manages all entities and provides query capabilities
struct World {
    entities: HashMap<u64, Entity>,
    next_id: u64,
    /// Index for fast component queries
    component_index: HashMap<TypeId, HashSet<u64>>,
}

struct Entity {
    id: u64,
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl World {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
            component_index: HashMap::new(),
        }
    }

    /// Create a new entity
    fn create_entity(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(id, Entity {
            id,
            components: HashMap::new(),
        });
        id
    }

    /// Add component to entity with indexing
    fn add_component<C: Component + 'static>(
        &mut self,
        entity_id: u64,
        component: C,
    ) -> ComponentResult<()> {
        let entity = self.entities.get_mut(&entity_id)
            .ok_or_else(|| ComponentError::NotFound(format!("Entity {entity_id}")))?;

        let type_id = TypeId::of::<C>();
        
        if entity.components.contains_key(&type_id) {
            return Err(ComponentError::AlreadyExists(component.type_name().to_string()));
        }

        entity.components.insert(type_id, Box::new(component));
        
        // Update index
        self.component_index
            .entry(type_id)
            .or_insert_with(HashSet::new)
            .insert(entity_id);

        Ok(())
    }

    /// Query entities with specific component
    fn query_component<C: Component + 'static>(&self) -> Vec<(u64, &C)> {
        let type_id = TypeId::of::<C>();
        
        if let Some(entity_ids) = self.component_index.get(&type_id) {
            entity_ids.iter()
                .filter_map(|&id| {
                    self.entities.get(&id)
                        .and_then(|e| e.components.get(&type_id))
                        .and_then(|c| c.as_any().downcast_ref::<C>())
                        .map(|c| (id, c))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Query entities with multiple components
    fn query_components_2<C1: Component + 'static, C2: Component + 'static>(
        &self
    ) -> Vec<(u64, &C1, &C2)> {
        let type_id_1 = TypeId::of::<C1>();
        let type_id_2 = TypeId::of::<C2>();
        
        // Get intersection of entities with both components
        if let (Some(ids1), Some(ids2)) = (
            self.component_index.get(&type_id_1),
            self.component_index.get(&type_id_2),
        ) {
            let intersection: Vec<_> = ids1.intersection(ids2).copied().collect();
            
            intersection.iter()
                .filter_map(|&id| {
                    self.entities.get(&id).and_then(|e| {
                        let c1 = e.components.get(&type_id_1)?.as_any().downcast_ref::<C1>()?;
                        let c2 = e.components.get(&type_id_2)?.as_any().downcast_ref::<C2>()?;
                        Some((id, c1, c2))
                    })
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

// =============================================================================
// SYSTEM-LIKE PROCESSING
// =============================================================================

/// Example of a "system" that processes components
fn movement_system(world: &World) {
    // Query all entities with both Position and Velocity
    #[derive(Debug, Clone)]
    struct Position { x: f64, y: f64 }
    
    impl Component for Position {
        fn as_any(&self) -> &dyn Any { self }
        fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
        fn type_name(&self) -> &'static str { "Position" }
    }

    let entities = world.query_components_2::<Position, Velocity>();
    
    println!("Movement System: Processing {entities.len(} entities"));
    
    for (id, pos, vel) in entities {
        println!("  Entity {id}: Moving from ({:.2}, {:.2}) by ({:.2}, {:.2})", pos.x, pos.y, vel.dx, vel.dy);
        // In a real system, we'd update the position here
    }
}

/// System that processes parent-child relationships
fn hierarchy_system(world: &World) {
    let parents = world.query_component::<Parent>();
    
    println!("Hierarchy System: Found {parents.len(} child entities"));
    
    for (child_id, parent) in parents {
        println!("  Entity {child_id} is a child of Entity {parent.entity_id}");
    }
}

// =============================================================================
// EVENT-DRIVEN PATTERNS
// =============================================================================

#[derive(Debug)]
enum ComponentEvent {
    Added { entity_id: u64, component_type: &'static str },
    Removed { entity_id: u64, component_type: &'static str },
    Modified { entity_id: u64, component_type: &'static str },
}

struct EventDrivenWorld {
    world: World,
    event_queue: Vec<ComponentEvent>,
    event_handlers: HashMap<&'static str, Box<dyn Fn(&ComponentEvent)>>,
}

impl EventDrivenWorld {
    fn new() -> Self {
        Self {
            world: World::new(),
            event_queue: Vec::new(),
            event_handlers: HashMap::new(),
        }
    }

    fn add_component_with_event<C: Component + 'static>(
        &mut self,
        entity_id: u64,
        component: C,
    ) -> ComponentResult<()> {
        let component_type = component.type_name();
        self.world.add_component(entity_id, component)?;
        
        self.event_queue.push(ComponentEvent::Added {
            entity_id,
            component_type,
        });
        
        Ok(())
    }

    fn process_events(&mut self) {
        let events = std::mem::take(&mut self.event_queue);
        
        for event in events {
            match &event {
                ComponentEvent::Added { component_type, .. } => {
                    if let Some(handler) = self.event_handlers.get(component_type) {
                        handler(&event);
                    }
                }
                _ => {}
            }
        }
    }
}

// =============================================================================
// MAIN EXAMPLE
// =============================================================================

fn main() {
    println!("=== Advanced CIM Component Patterns ===\n");

    // 1. Basic world usage
    let mut world = World::new();
    
    // Create entities
    let player_id = world.create_entity();
    let enemy_id = world.create_entity();
    let child_id = world.create_entity();
    
    // Add components
    #[derive(Debug, Clone)]
    struct Position { x: f64, y: f64 }
    
    impl Component for Position {
        fn as_any(&self) -> &dyn Any { self }
        fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
        fn type_name(&self) -> &'static str { "Position" }
    }

    world.add_component(player_id, Position { x: 0.0, y: 0.0 }).unwrap();
    world.add_component(player_id, Velocity { dx: 1.0, dy: 0.5 }).unwrap();
    
    world.add_component(enemy_id, Position { x: 10.0, y: 10.0 }).unwrap();
    world.add_component(enemy_id, Velocity { dx: -0.5, dy: -0.5 }).unwrap();
    
    world.add_component(child_id, Position { x: 5.0, y: 5.0 }).unwrap();
    world.add_component(child_id, Parent { entity_id: player_id }).unwrap();
    
    // 2. Run systems
    println!("Running systems:\n");
    movement_system(&world);
    println!();
    hierarchy_system(&world);
    
    // 3. Component validation
    println!("\nComponent validation example:");
    match Temperature::new(100.0) {
        Ok(temp) => {
            world.add_component(player_id, temp).unwrap();
            println!("  Added valid temperature to player");
        }
        Err(e) => println!("  Error: {e}"),
    }
    
    match Temperature::new(-300.0) {
        Ok(_) => println!("  Should not reach here"),
        Err(e) => println!("  Validation prevented invalid temperature: {e}"),
    }
    
    // 4. Event-driven example
    println!("\nEvent-driven pattern:");
    let mut event_world = EventDrivenWorld::new();
    
    // Register event handler
    event_world.event_handlers.insert("Position", Box::new(|event| {
        if let ComponentEvent::Added { entity_id, .. } = event {
            println!("  Position component added to entity {entity_id}");
        }
    }));
    
    let entity = event_world.world.create_entity();
    event_world.add_component_with_event(entity, Position { x: 0.0, y: 0.0 }).unwrap();
    event_world.process_events();
    
    // 5. Performance considerations
    println!("\nPerformance patterns demonstrated:");
    println!("  - Component indexing for O(1) lookups");
    println!("  - Efficient entity queries with type-based indexing");
    println!("  - Event batching to reduce processing overhead");
    println!("  - Component validation at construction time");
} 