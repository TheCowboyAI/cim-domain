//! Example demonstrating how cim-component is used in the wider CIM architecture
//! 
//! This example shows the expected usage patterns without taking on any dependencies.
//! It demonstrates how other modules in CIM would use the Component trait.

use cim_component::{Component, ComponentError, ComponentResult};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;

// =============================================================================
// EXAMPLE COMPONENTS - These would be defined in various domain modules
// =============================================================================

/// Visual component for 3D positioning (used in cim-domain-graph)
#[derive(Debug, Clone)]
struct Position3D {
    x: f32,
    y: f32,
    z: f32,
}

impl Component for Position3D {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Position3D"
    }
}

/// Semantic component for conceptual spaces (used in cim-domain-conceptualspaces)
#[derive(Debug, Clone)]
struct ConceptualCoordinates {
    dimensions: Vec<f64>,
    space_id: uuid::Uuid,
}

impl Component for ConceptualCoordinates {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "ConceptualCoordinates"
    }
}

/// Metadata component (used across multiple domains)
#[derive(Debug, Clone)]
struct Metadata {
    name: String,
    description: Option<String>,
    tags: Vec<String>,
    created_at: std::time::SystemTime,
}

impl Component for Metadata {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Metadata"
    }
}

/// Identity component (used in cim-domain-identity)
#[derive(Debug, Clone)]
struct IdentityInfo {
    user_id: uuid::Uuid,
    roles: Vec<String>,
    permissions: Vec<String>,
}

impl Component for IdentityInfo {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "IdentityInfo"
    }
}

// =============================================================================
// COMPONENT STORAGE - This pattern would be implemented in domain modules
// =============================================================================

/// Example of how domains implement component storage
struct ComponentStorage {
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl ComponentStorage {
    fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    fn add<C: Component + 'static>(&mut self, component: C) -> ComponentResult<()> {
        let type_id = TypeId::of::<C>();
        if self.components.contains_key(&type_id) {
            return Err(ComponentError::AlreadyExists(component.type_name().to_string()));
        }
        self.components.insert(type_id, Box::new(component));
        Ok(())
    }

    fn get<C: Component + 'static>(&self) -> Option<&C> {
        self.components
            .get(&TypeId::of::<C>())
            .and_then(|c| c.as_any().downcast_ref::<C>())
    }

    fn remove<C: Component + 'static>(&mut self) -> ComponentResult<Box<dyn Component>> {
        self.components
            .remove(&TypeId::of::<C>())
            .ok_or_else(|| ComponentError::NotFound(std::any::type_name::<C>().to_string()))
    }

    fn has<C: Component + 'static>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<C>())
    }
}

// =============================================================================
// DOMAIN ENTITY EXAMPLE - Shows how entities use components
// =============================================================================

/// Example entity that uses components (simplified version of what domains implement)
struct Entity {
    id: uuid::Uuid,
    components: ComponentStorage,
}

impl Entity {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            components: ComponentStorage::new(),
        }
    }

    fn add_component<C: Component + 'static>(&mut self, component: C) -> ComponentResult<()> {
        self.components.add(component)
    }

    fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }

    fn remove_component<C: Component + 'static>(&mut self) -> ComponentResult<Box<dyn Component>> {
        self.components.remove::<C>()
    }
}

// =============================================================================
// USAGE EXAMPLES - Demonstrating architectural patterns
// =============================================================================

fn main() {
    println!("=== CIM Component Architecture Usage Examples ===\n");

    // Example 1: Graph node with visual and semantic components
    example_graph_node();
    
    // Example 2: Identity entity with permissions
    example_identity_entity();
    
    // Example 3: Cross-domain entity composition
    example_cross_domain_composition();
    
    // Example 4: Component queries and filtering
    example_component_queries();
}

fn example_graph_node() {
    println!("1. Graph Node Example (cim-domain-graph usage):");
    println!("   Creating a graph node with position and metadata...\n");

    let mut node = Entity::new();

    // Add visual position
    node.add_component(Position3D {
        x: 10.0,
        y: 20.0,
        z: 0.0,
    }).unwrap();

    // Add metadata
    node.add_component(Metadata {
        name: "StartNode".to_string(),
        description: Some("The entry point of the workflow".to_string()),
        tags: vec!["workflow".to_string(), "entry".to_string()],
        created_at: std::time::SystemTime::now(),
    }).unwrap();

    // Access components
    if let Some(pos) = node.get_component::<Position3D>() {
        println!("   Node position: ({pos.x}, {pos.y}, {pos.z})");
    }

    if let Some(meta) = node.get_component::<Metadata>() {
        println!("   Node name: {meta.name}");
        println!("   Tags: {:?}\n", meta.tags);
    }
}

fn example_identity_entity() {
    println!("2. Identity Entity Example (cim-domain-identity usage):");
    println!("   Creating a user entity with identity components...\n");

    let mut user = Entity::new();

    // Add identity information
    user.add_component(IdentityInfo {
        user_id: uuid::Uuid::new_v4(),
        roles: vec!["admin".to_string(), "developer".to_string()],
        permissions: vec!["read".to_string(), "write".to_string(), "delete".to_string()],
    }).unwrap();

    // Add metadata
    user.add_component(Metadata {
        name: "John Doe".to_string(),
        description: Some("System administrator".to_string()),
        tags: vec!["staff".to_string()],
        created_at: std::time::SystemTime::now(),
    }).unwrap();

    if let Some(identity) = user.get_component::<IdentityInfo>() {
        println!("   User ID: {identity.user_id}");
        println!("   Roles: {:?}", identity.roles);
        println!("   Permissions: {:?}\n", identity.permissions);
    }
}

fn example_cross_domain_composition() {
    println!("3. Cross-Domain Composition Example:");
    println!("   Creating an entity with components from multiple domains...\n");

    let mut entity = Entity::new();

    // Add components from different domains
    entity.add_component(Position3D { x: 0.0, y: 0.0, z: 0.0 }).unwrap();
    
    entity.add_component(ConceptualCoordinates {
        dimensions: vec![0.5, 0.8, 0.2, 0.9],
        space_id: uuid::Uuid::new_v4(),
    }).unwrap();

    entity.add_component(Metadata {
        name: "Hybrid Entity".to_string(),
        description: Some("Entity with both visual and conceptual representation".to_string()),
        tags: vec!["hybrid".to_string(), "multi-domain".to_string()],
        created_at: std::time::SystemTime::now(),
    }).unwrap();

    // The entity now has components from:
    // - Visual domain (Position3D)
    // - Conceptual domain (ConceptualCoordinates)
    // - Common metadata

    println!("   Entity has Position3D: {entity.components.has::<Position3D>(}"));
    println!("   Entity has ConceptualCoordinates: {entity.components.has::<ConceptualCoordinates>(}"));
    println!("   Entity has Metadata: {entity.components.has::<Metadata>(}\n"));
}

fn example_component_queries() {
    println!("4. Component Query Example:");
    println!("   Demonstrating component-based filtering...\n");

    // Create multiple entities
    let mut entities = Vec::new();

    // Entity 1: Only position
    let mut e1 = Entity::new();
    e1.add_component(Position3D { x: 1.0, y: 1.0, z: 1.0 }).unwrap();
    entities.push(e1);

    // Entity 2: Position and metadata
    let mut e2 = Entity::new();
    e2.add_component(Position3D { x: 2.0, y: 2.0, z: 2.0 }).unwrap();
    e2.add_component(Metadata {
        name: "Named Entity".to_string(),
        description: None,
        tags: vec![],
        created_at: std::time::SystemTime::now(),
    }).unwrap();
    entities.push(e2);

    // Entity 3: All components
    let mut e3 = Entity::new();
    e3.add_component(Position3D { x: 3.0, y: 3.0, z: 3.0 }).unwrap();
    e3.add_component(ConceptualCoordinates {
        dimensions: vec![1.0, 0.0, 0.5],
        space_id: uuid::Uuid::new_v4(),
    }).unwrap();
    e3.add_component(Metadata {
        name: "Full Entity".to_string(),
        description: Some("Has all component types".to_string()),
        tags: vec!["complete".to_string()],
        created_at: std::time::SystemTime::now(),
    }).unwrap();
    entities.push(e3);

    // Query: Find entities with both Position3D and Metadata
    let with_pos_and_meta: Vec<_> = entities
        .iter()
        .filter(|e| e.components.has::<Position3D>() && e.components.has::<Metadata>())
        .collect();

    println!("   Entities with Position3D and Metadata: {with_pos_and_meta.len(}"));

    // Query: Find entities with ConceptualCoordinates
    let with_conceptual: Vec<_> = entities
        .iter()
        .filter(|e| e.components.has::<ConceptualCoordinates>())
        .collect();

    println!("   Entities with ConceptualCoordinates: {with_conceptual.len(}"));

    // Process entities with specific components
    for entity in &entities {
        if let Some(meta) = entity.get_component::<Metadata>() {
            if let Some(pos) = entity.get_component::<Position3D>() {
                println!("   Entity '{meta.name}' at position ({pos.x}, {pos.y}, {pos.z})");
            }
        }
    }
}

// =============================================================================
// ARCHITECTURAL PATTERNS DEMONSTRATED
// =============================================================================

// This example demonstrates several key architectural patterns:
//
// 1. **Component Definition**: Each domain defines its own components that
//    implement the Component trait. These are simple data structures.
//
// 2. **Type Safety**: The component system uses Rust's type system for
//    compile-time safety while allowing runtime flexibility.
//
// 3. **Storage Pattern**: Domains implement storage using TypeId-based
//    HashMap lookups for O(1) access.
//
// 4. **Entity Composition**: Entities can have components from multiple
//    domains, enabling cross-domain functionality.
//
// 5. **Query Patterns**: Systems can query entities based on component
//    presence, enabling data-driven behavior.
//
// 6. **No Dependencies**: The cim-component crate remains dependency-free,
//    while consuming crates bring their own dependencies. 