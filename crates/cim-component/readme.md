# CIM Component

The foundational component trait for the Composable Information Machine (CIM). This crate provides the core abstraction for attaching data to domain objects in a type-safe, composable manner.

## Overview

`cim-component` defines the fundamental `Component` trait that enables:
- Type-safe component storage and retrieval
- Runtime component composition
- Cross-domain data attachment
- Foundation for Entity-Component-System (ECS) architectures

## Features

- **Zero Dependencies**: Pure Rust implementation with no external dependencies
- **Type Safety**: Compile-time type checking with runtime flexibility
- **Cloneable**: All components can be cloned for easy duplication
- **Serializable**: Built-in support for serialization via Serde
- **Error Handling**: Comprehensive error types for component operations

## Usage

### Basic Example

```rust
use cim_component::{Component, ComponentError, ComponentResult};
use std::any::Any;

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
```

### Storing Components

```rust
use std::any::TypeId;
use std::collections::HashMap;

struct Entity {
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl Entity {
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
}
```

## Examples

The crate includes three comprehensive examples demonstrating different usage patterns:

### 1. Basic Usage (`cargo run --example basic_usage`)
Demonstrates fundamental component patterns using only standard library types:
- Creating and implementing simple components
- Component storage and retrieval
- Error handling
- Basic entity queries

### 2. Architecture Usage (`cargo run --example architecture_usage`)
Shows how cim-component integrates with the wider CIM architecture:
- Components from different domains (Graph, Identity, ConceptualSpaces)
- Cross-domain entity composition
- Component storage patterns used by domain modules
- Query patterns for finding entities with specific components

### 3. Advanced Patterns (`cargo run --example advanced_patterns`)
Demonstrates sophisticated usage patterns:
- Component relationships and dependencies
- System-like processing of components
- Event-driven component updates
- Performance optimizations with indexing
- Component validation at construction time

## Integration with CIM

`cim-component` serves as the foundation for data composition across all CIM domains:

- **Graph Domain**: Uses components for node positions, metadata, and visual properties
- **Identity Domain**: Attaches identity, roles, and permissions as components
- **ConceptualSpaces Domain**: Represents semantic coordinates as components
- **Workflow Domain**: Tracks workflow state and progress through components

## Design Philosophy

The component system follows these principles:

1. **Data-Behavior Separation**: Components are pure data; systems provide behavior
2. **Composition over Inheritance**: Build complex entities by combining simple components
3. **Type Safety with Flexibility**: Compile-time safety with runtime composition
4. **Domain Agnostic**: Components can represent any domain concept

## Error Handling

The crate provides comprehensive error types:

```rust
pub enum ComponentError {
    NotFound(String),
    AlreadyExists(String),
    TypeMismatch { expected: String, actual: String },
    Custom(String),
}
```

## Performance Considerations

- Components are stored in a `HashMap` with `TypeId` keys for O(1) access
- The `clone_box` method enables efficient component duplication
- Type erasure through `Box<dyn Component>` allows heterogeneous storage

## License

This project is licensed under the MIT License - see the LICENSE file for details. 