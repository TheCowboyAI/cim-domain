//! Tests for cim-component functionality aligned with user stories

use cim_component::{component_type_id, Component, ComponentError};
use std::any::{Any, TypeId};

#[derive(Debug, Clone, PartialEq)]
struct TestComponent(String);

impl Component for TestComponent {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn clone_box(&self) -> Box<dyn Component> {
        Box::new(self.clone())
    }
    fn type_name(&self) -> &'static str {
        "TestComponent"
    }
}

#[derive(Debug, Clone, PartialEq)]
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

/// User Story: F1 - Extensible Domain Objects
///
/// As a domain modeler
/// I want to attach typed components to any domain object
/// So that I can extend entities without modifying their core structure
///
/// ```mermaid
/// graph TD
///     Component[Component Trait]
///     TestComp[TestComponent]
///     Position[Position3D]
///
///     TestComp -->|implements| Component
///     Position -->|implements| Component
///
///     Component -->|as_any| Downcast[Type-safe Downcast]
///     Component -->|clone_box| Clone[Cloneable]
///     Component -->|type_name| Debug[Debuggable]
/// ```
///
/// Acceptance Criteria:
/// - Components are type-safe and cannot be confused at compile time
/// - Components can be cloned when duplicating entities
/// - Component type names are available for debugging and serialization
#[test]
fn test_component_type_safety_and_cloning() {
    // Given a component with data
    let comp = TestComponent("test data".to_string());

    // When I use it as a trait object
    let any_ref = comp.as_any();

    // Then I can downcast to the correct type
    let downcast = any_ref.downcast_ref::<TestComponent>();
    assert!(downcast.is_some());
    assert_eq!(downcast.unwrap(), &comp);

    // And I cannot downcast to an incorrect type
    let wrong_downcast = any_ref.downcast_ref::<Position3D>();
    assert!(wrong_downcast.is_none());

    // When I clone the component
    let cloned = comp.clone_box();

    // Then the clone has the same data
    let cloned_comp = cloned.as_any().downcast_ref::<TestComponent>().unwrap();
    assert_eq!(cloned_comp, &comp);

    // And the type name is available for debugging
    assert_eq!(comp.type_name(), "TestComponent");
}

/// User Story: F2 - Component Discovery
///
/// As a system developer
/// I want to discover what components are attached to an entity
/// So that I can build generic systems that work with any component
///
/// ```mermaid
/// graph LR
///     Entity[Entity]
///     CompA[Component A]
///     CompB[Component B]
///     TypeId[TypeId Registry]
///
///     Entity -->|has| CompA
///     Entity -->|has| CompB
///     CompA -->|type_id| TypeId
///     CompB -->|type_id| TypeId
///
///     TypeId -->|enables| Discovery[Component Discovery]
/// ```
///
/// Acceptance Criteria:
/// - Component type names are accessible for debugging
/// - TypeId can be used for component lookups
/// - Different component types have different TypeIds
#[test]
fn test_component_discovery_and_type_identification() {
    // Given different component types
    let text_comp = TestComponent("label".to_string());
    let pos_comp = Position3D {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    // When I get their type names
    let text_name = text_comp.type_name();
    let pos_name = pos_comp.type_name();

    // Then they are distinct and meaningful
    assert_eq!(text_name, "TestComponent");
    assert_eq!(pos_name, "Position3D");
    assert_ne!(text_name, pos_name);

    // When I get their TypeIds
    let text_id = component_type_id::<TestComponent>();
    let pos_id = component_type_id::<Position3D>();

    // Then they are unique
    assert_ne!(text_id, pos_id);

    // And consistent for the same type
    let text_id2 = TypeId::of::<TestComponent>();
    assert_eq!(text_id, text_id2);
}

/// User Story: F1 - Extensible Domain Objects
///
/// Error handling for component operations
///
/// ```mermaid
/// graph TD
///     Op[Component Operation]
///     Success[Success]
///     AlreadyExists[AlreadyExists Error]
///     NotFound[NotFound Error]
///
///     Op -->|attach existing| AlreadyExists
///     Op -->|get missing| NotFound
///     Op -->|normal| Success
/// ```
///
/// Acceptance Criteria:
/// - Errors include context about what went wrong
/// - Errors can be converted to user-friendly messages
#[test]
fn test_component_error_handling() {
    // Given component errors
    let already_exists = ComponentError::AlreadyExists("Position3D".to_string());
    let not_found = ComponentError::NotFound("Velocity".to_string());

    // When I format them as strings
    let exists_msg = already_exists.to_string();
    let not_found_msg = not_found.to_string();

    // Then they provide clear error messages
    assert_eq!(exists_msg, "Component already exists: Position3D");
    assert_eq!(not_found_msg, "Component not found: Velocity");

    // And they implement Error trait
    fn assert_error<E: std::error::Error>(_: &E) {}
    assert_error(&already_exists);
    assert_error(&not_found);
}
