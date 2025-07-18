// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating Bevy ECS integration patterns
//!
//! This example shows:
//! - Component trait implementation
//! - Entity-component architecture
//! - Component storage patterns
//! - Type-safe component access

use cim_domain::{
    // Core types
    EntityId, DomainEntity,
    markers::AggregateMarker,
    
    // Component system
    Component, ComponentStorage,
    
    // Errors
    DomainError,
};
use serde::{Deserialize, Serialize};

/// Example DDD component: Transform
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransformComponent {
    position: [f32; 3],
    rotation: [f32; 4], // Quaternion
    scale: [f32; 3],
}

impl Component for TransformComponent {
    fn type_name(&self) -> &'static str {
        "TransformComponent"
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Example DDD component: Health
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthComponent {
    current: f32,
    maximum: f32,
}

impl Component for HealthComponent {
    fn type_name(&self) -> &'static str {
        "HealthComponent"
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Example DDD component: Player tag
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerComponent {
    name: String,
    level: u32,
}

impl Component for PlayerComponent {
    fn type_name(&self) -> &'static str {
        "PlayerComponent"
    }
    
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Example entity: GameObject
#[derive(Debug, Clone)]
struct GameObject {
    id: EntityId<AggregateMarker>,
    entity_type: String,
    components: ComponentStorage,
}

impl DomainEntity for GameObject {
    type IdType = AggregateMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl GameObject {
    fn new(entity_type: String) -> Self {
        Self {
            id: EntityId::new(),
            entity_type,
            components: ComponentStorage::new(),
        }
    }
    
    fn add_component<C: Component + 'static>(&mut self, component: C) -> Result<(), DomainError> {
        self.components.add(component)
    }
    
    fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components.get::<C>()
    }
    
    fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.has::<C>()
    }
}

/// Simulate a Bevy system that processes entities with specific components
fn movement_system(entities: &mut [GameObject]) {
    for entity in entities {
        // Check if entity has both Transform and Player components
        if let (Some(transform), Some(player)) = (
            entity.get_component::<TransformComponent>(),
            entity.get_component::<PlayerComponent>()
        ) {
            println!("   Moving player '{}' at position {:?}", 
                player.name, transform.position);
        }
    }
}

/// Simulate a Bevy system that processes health
fn health_system(entities: &[GameObject]) {
    for entity in entities {
        if let Some(health) = entity.get_component::<HealthComponent>() {
            let percentage = (health.current / health.maximum * 100.0) as u32;
            println!("   Entity {} health: {}% ({}/{})", 
                entity.id, percentage, health.current, health.maximum);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Bevy Integration Example");
    println!("=======================\n");
    
    // Example 1: Create entities with components
    println!("1. Creating game entities...");
    
    let mut player = GameObject::new("Player".to_string());
    let mut enemy = GameObject::new("Enemy".to_string());
    let mut npc = GameObject::new("NPC".to_string());
    
    println!("   Created player: {}", player.id);
    println!("   Created enemy: {}", enemy.id);
    println!("   Created NPC: {}", npc.id);
    
    // Example 2: Add components to player
    println!("\n2. Adding components to player...");
    
    player.add_component(TransformComponent {
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    })?;
    
    player.add_component(HealthComponent {
        current: 100.0,
        maximum: 100.0,
    })?;
    
    player.add_component(PlayerComponent {
        name: "Hero".to_string(),
        level: 1,
    })?;
    
    println!("   ✓ Added Transform, Health, and Player components");
    
    // Example 3: Add components to enemy
    println!("\n3. Adding components to enemy...");
    
    enemy.add_component(TransformComponent {
        position: [10.0, 0.0, 5.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    })?;
    
    enemy.add_component(HealthComponent {
        current: 50.0,
        maximum: 50.0,
    })?;
    
    println!("   ✓ Added Transform and Health components");
    
    // Example 4: Add components to NPC
    println!("\n4. Adding components to NPC...");
    
    npc.add_component(TransformComponent {
        position: [-5.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    })?;
    
    npc.add_component(PlayerComponent {
        name: "Merchant".to_string(),
        level: 5,
    })?;
    
    println!("   ✓ Added Transform and Player components");
    
    // Example 5: Query components
    println!("\n5. Querying components...");
    
    // Check what components each entity has
    println!("   Player components:");
    println!("     - Transform: {}", player.has_component::<TransformComponent>());
    println!("     - Health: {}", player.has_component::<HealthComponent>());
    println!("     - Player: {}", player.has_component::<PlayerComponent>());
    
    println!("   Enemy components:");
    println!("     - Transform: {}", enemy.has_component::<TransformComponent>());
    println!("     - Health: {}", enemy.has_component::<HealthComponent>());
    println!("     - Player: {}", enemy.has_component::<PlayerComponent>());
    
    // Example 6: Run systems
    println!("\n6. Running Bevy-style systems...");
    
    let mut entities = vec![player, enemy, npc];
    
    println!("\n   Movement System:");
    movement_system(&mut entities);
    
    println!("\n   Health System:");
    health_system(&entities);
    
    // Example 7: Modify components
    println!("\n7. Modifying components...");
    
    // Update player position
    if let Some(player_entity) = entities.iter_mut().find(|e| e.entity_type == "Player") {
        // In a real system, we'd modify the component directly
        // Here we demonstrate the pattern
        if let Some(transform) = player_entity.get_component::<TransformComponent>() {
            println!("   Current player position: {:?}", transform.position);
            // In Bevy, you'd modify through a mutable reference
            println!("   Would move player to [5.0, 0.0, 3.0]");
        }
    }
    
    // Damage enemy
    if let Some(enemy_entity) = entities.iter().find(|e| e.entity_type == "Enemy") {
        if let Some(health) = enemy_entity.get_component::<HealthComponent>() {
            println!("   Current enemy health: {}/{}", health.current, health.maximum);
            println!("   Would damage enemy for 25 HP");
        }
    }
    
    // Example 8: Component patterns
    println!("\n8. Component patterns...");
    
    // Count entities with health
    let entities_with_health = entities.iter()
        .filter(|e| e.has_component::<HealthComponent>())
        .count();
    println!("   Entities with health: {}", entities_with_health);
    
    // Find all players
    let players: Vec<_> = entities.iter()
        .filter(|e| e.has_component::<PlayerComponent>())
        .collect();
    println!("   Players found: {}", players.len());
    
    for player_entity in players {
        if let Some(player) = player_entity.get_component::<PlayerComponent>() {
            println!("     - {} (Level {})", player.name, player.level);
        }
    }
    
    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Component trait implementation");
    println!("  • Entity-component architecture");
    println!("  • Type-safe component storage");
    println!("  • Bevy-style system patterns");
    println!("  • Component queries and filtering");
    
    println!("\nKey Concepts:");
    println!("  • Components are data (Transform, Health, Player)");
    println!("  • Entities are IDs with components (GameObject)");
    println!("  • Systems are functions that process entities");
    println!("  • Type safety through generics");
    
    Ok(())
} 