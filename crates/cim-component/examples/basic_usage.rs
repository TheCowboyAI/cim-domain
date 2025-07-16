//! Basic usage example for cim-component without external dependencies
//!
//! This example shows the fundamental patterns of the Component trait
//! using only standard library types.

use cim_component::{Component, ComponentError, ComponentResult};
use std::any::{Any, TypeId};
use std::collections::HashMap;

// Simple components using only std types
#[derive(Debug, Clone)]
struct Label {
    text: String,
}

impl Component for Label {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Label"
    }
}

#[derive(Debug, Clone)]
struct Health {
    current: u32,
    max: u32,
}

impl Component for Health {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Health"
    }
}

#[derive(Debug, Clone)]
struct Position {
    x: f64,
    y: f64,
}

impl Component for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }

    fn type_name(&self) -> &'static str {
        "Position"
    }
}

// Simple entity with components
struct GameObject {
    id: u64,
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl GameObject {
    fn new(id: u64) -> Self {
        Self {
            id,
            components: HashMap::new(),
        }
    }

    fn add_component<C: Component + 'static>(&mut self, component: C) -> ComponentResult<()> {
        let type_id = TypeId::of::<C>();
        if self.components.contains_key(&type_id) {
            return Err(ComponentError::AlreadyExists(component.type_name().to_string()));
        }
        self.components.insert(type_id, Box::new(component));
        Ok(())
    }

    fn get_component<C: Component + 'static>(&self) -> Option<&C> {
        self.components
            .get(&TypeId::of::<C>())
            .and_then(|c| c.as_any().downcast_ref::<C>())
    }

    fn has_component<C: Component + 'static>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<C>())
    }
}

fn main() {
    println!("=== Basic CIM Component Usage ===\n");

    // Create a game object
    let mut player = GameObject::new(1);
    
    // Add components
    player.add_component(Label {
        text: "Player One".to_string(),
    }).unwrap();
    
    player.add_component(Health {
        current: 100,
        max: 100,
    }).unwrap();
    
    player.add_component(Position {
        x: 50.0,
        y: 75.0,
    }).unwrap();

    // Access components
    if let Some(label) = player.get_component::<Label>() {
        println!("Player name: {label.text}");
    }

    if let Some(health) = player.get_component::<Health>() {
        println!("Health: {health.current}/{health.max}");
    }

    if let Some(pos) = player.get_component::<Position>() {
        println!("Position: ({pos.x}, {pos.y})");
    }

    // Check for components
    println!("\nComponent checks:");
    println!("Has Label: {player.has_component::<Label>(}"));
    println!("Has Health: {player.has_component::<Health>(}"));
    println!("Has Position: {player.has_component::<Position>(}"));

    // Error handling
    println!("\nError handling:");
    match player.add_component(Label { text: "Duplicate".to_string() }) {
        Ok(_) => println!("Added duplicate label (shouldn't happen)"),
        Err(ComponentError::AlreadyExists(name)) => {
            println!("Cannot add duplicate component: {name}");
        }
        Err(_) => println!("Other error"),
    }

    // Create another object with different components
    let mut enemy = GameObject::new(2);
    enemy.add_component(Label {
        text: "Goblin".to_string(),
    }).unwrap();
    enemy.add_component(Health {
        current: 30,
        max: 30,
    }).unwrap();
    // Note: No position component for this enemy

    // Query pattern example
    println!("\nQuery example - finding objects with health:");
    let objects = vec![player, enemy];
    
    for obj in &objects {
        if let Some(label) = obj.get_component::<Label>() {
            if let Some(health) = obj.get_component::<Health>() {
                println!("  {label.text} has {health.current} health");
            }
        }
    }
} 