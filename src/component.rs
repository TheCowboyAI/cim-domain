// Copyright 2025 Cowboy AI, LLC.

//! Component storage for attaching data to domain objects
//!
//! This module provides a pure FP implementation of components
//! that work with the Entity monad. Components are type-erased
//! data that can be attached to entities.

use crate::DomainError;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Trait for components that can be attached to entities
/// 
/// Components are immutable data that follow the ECS pattern.
/// They should be value objects with no behavior.
pub trait Component: Any + Send + Sync + fmt::Debug {
    /// Get the component as Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Clone the component into a box
    fn clone_box(&self) -> Box<dyn Component>;
    
    /// Get type name for debugging
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Implementation helper for components
impl<T> Component for T
where
    T: Any + Send + Sync + fmt::Debug + Clone + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
}

/// Events that can occur on components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentEvent {
    /// Component was added
    Added {
        entity_id: String,
        component_type: String,
    },
    /// Component was removed
    Removed {
        entity_id: String,
        component_type: String,
    },
    /// Component was updated
    Updated {
        entity_id: String,
        component_type: String,
    },
}

/// Extension trait for component operations
pub trait ComponentExt {
    /// Add a component
    fn with_component<C: Component>(self, component: C) -> Self;
    
    /// Get a component by type
    fn get_component<C: Component>(&self) -> Option<&C>;
    
    /// Check if has component
    fn has_component<C: Component>(&self) -> bool;
}

/// ECS component data wrapper
/// 
/// This wraps arbitrary data for use in Entity-Component-System patterns
#[derive(Clone)]
pub struct EcsComponentData {
    data: Arc<dyn Any + Send + Sync>,
}

impl EcsComponentData {
    /// Create new component data
    pub fn new<T: Any + Send + Sync + 'static>(data: T) -> Self {
        Self {
            data: Arc::new(data),
        }
    }
    
    /// Try to downcast to specific type
    pub fn downcast_ref<T: Any + 'static>(&self) -> Option<&T> {
        self.data.downcast_ref::<T>()
    }
}

impl fmt::Debug for EcsComponentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EcsComponentData")
            .field("type_id", &(*self.data).type_id())
            .finish()
    }
}

/// Storage for components attached to a domain object
///
/// Components are stored by their TypeId and can only have one instance
/// of each type. Components are immutable once added (following FP principles).
#[derive(Default)]
pub struct ComponentStorage {
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl ComponentStorage {
    /// Create a new empty component storage
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// Add a component to storage
    /// 
    /// BREAKING FP: Mutable operation at storage boundary
    /// REASON: Performance optimization for component management
    pub fn add<C: Component + 'static>(&mut self, component: C) -> Result<(), DomainError> {
        let type_id = TypeId::of::<C>();
        
        if self.components.contains_key(&type_id) {
            return Err(DomainError::AlreadyExists(format!(
                "Component {} already exists",
                component.type_name()
            )));
        }

        self.components.insert(type_id, Box::new(component));
        Ok(())
    }

    /// Get a component by type
    pub fn get<C: Component + 'static>(&self) -> Option<&C> {
        let type_id = TypeId::of::<C>();
        self.components
            .get(&type_id)
            .and_then(|c| c.as_any().downcast_ref::<C>())
    }

    /// Remove a component by type
    /// 
    /// BREAKING FP: Mutable operation at storage boundary
    /// REASON: Performance optimization for component management
    pub fn remove<C: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        let type_id = TypeId::of::<C>();
        self.components.remove(&type_id)
    }

    /// Check if a component type exists
    pub fn has<C: Component + 'static>(&self) -> bool {
        let type_id = TypeId::of::<C>();
        self.components.contains_key(&type_id)
    }

    /// Get the number of components
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Iterate over all components
    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Component>> {
        self.components.values()
    }
}

impl Clone for ComponentStorage {
    fn clone(&self) -> Self {
        let mut storage = Self::new();
        for (type_id, component) in &self.components {
            storage.components.insert(*type_id, component.clone_box());
        }
        storage
    }
}

impl fmt::Debug for ComponentStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentStorage")
            .field("component_count", &self.components.len())
            .field(
                "component_types",
                &self
                    .components
                    .values()
                    .map(|c| c.type_name())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, Clone)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }

    #[test]
    fn test_component_storage() {
        let mut storage = ComponentStorage::new();
        
        // Add components
        let pos = Position { x: 10.0, y: 20.0 };
        let vel = Velocity { dx: 1.0, dy: 2.0 };
        
        storage.add(pos.clone()).unwrap();
        storage.add(vel.clone()).unwrap();
        
        // Get components
        assert_eq!(storage.get::<Position>().unwrap().x, 10.0);
        assert_eq!(storage.get::<Velocity>().unwrap().dx, 1.0);
        
        // Check existence
        assert!(storage.has::<Position>());
        assert!(storage.has::<Velocity>());
        
        // Remove component
        storage.remove::<Position>();
        assert!(!storage.has::<Position>());
        assert!(storage.has::<Velocity>());
    }
    
    #[test]
    fn test_duplicate_component() {
        let mut storage = ComponentStorage::new();
        
        let pos1 = Position { x: 10.0, y: 20.0 };
        let pos2 = Position { x: 30.0, y: 40.0 };
        
        storage.add(pos1).unwrap();
        
        // Should fail to add duplicate
        assert!(storage.add(pos2).is_err());
    }
}