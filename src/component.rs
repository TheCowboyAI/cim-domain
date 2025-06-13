//! Component trait and storage for attaching data to domain objects

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use crate::DomainError;

/// Trait for components that can be attached to domain objects
///
/// Components are immutable data that can be attached to entities, nodes, or edges.
/// They provide a way to extend domain objects with additional data without modifying
/// their core structure.
///
/// # Example
///
/// ```
/// use cim_domain::Component;
/// use std::any::Any;
///
/// #[derive(Debug, Clone)]
/// struct Label(String);
///
/// impl Component for Label {
///     fn as_any(&self) -> &dyn Any { self }
///     fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
///     fn type_name(&self) -> &'static str { "Label" }
/// }
/// ```
pub trait Component: Any + Send + Sync {
    /// Get the component as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Clone the component into a box
    fn clone_box(&self) -> Box<dyn Component>;

    /// Get the name of this component type
    fn type_name(&self) -> &'static str;
}

/// Storage for components attached to a domain object
///
/// Components are stored by their TypeId and can only have one instance
/// of each type. Components are immutable once added.
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

    /// Add a component (can only be done once per type)
    ///
    /// # Errors
    ///
    /// Returns an error if a component of the same type already exists
    pub fn add<T: Component + 'static>(&mut self, component: T) -> Result<(), DomainError> {
        let type_id = TypeId::of::<T>();
        if self.components.contains_key(&type_id) {
            return Err(DomainError::ComponentAlreadyExists(component.type_name().to_string()));
        }
        self.components.insert(type_id, Box::new(component));
        Ok(())
    }

    /// Get a component by type (immutable access only)
    pub fn get<T: Component + 'static>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }

    /// Remove a component by type (returns the component)
    pub fn remove<T: Component + 'static>(&mut self) -> Option<Box<dyn Component>> {
        self.components.remove(&TypeId::of::<T>())
    }

    /// Check if a component type exists
    pub fn has<T: Component + 'static>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    /// Iterate over all components
    pub fn iter(&self) -> impl Iterator<Item = (&TypeId, &Box<dyn Component>)> {
        self.components.iter()
    }

    /// Get the number of components
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

impl Clone for ComponentStorage {
    fn clone(&self) -> Self {
        let mut storage = ComponentStorage::new();
        for (type_id, component) in &self.components {
            storage.components.insert(*type_id, component.clone_box());
        }
        storage
    }
}

impl fmt::Debug for ComponentStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let component_names: Vec<&str> = self.components
            .values()
            .map(|c| c.type_name())
            .collect();
        f.debug_struct("ComponentStorage")
            .field("components", &component_names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test components
    #[derive(Debug, Clone, PartialEq)]
    struct TestLabel(String);

    impl Component for TestLabel {
        fn as_any(&self) -> &dyn Any { self }
        fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
        fn type_name(&self) -> &'static str { "TestLabel" }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestMetadata {
        key: String,
        value: i32,
    }

    impl Component for TestMetadata {
        fn as_any(&self) -> &dyn Any { self }
        fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
        fn type_name(&self) -> &'static str { "TestMetadata" }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestTag;

    impl Component for TestTag {
        fn as_any(&self) -> &dyn Any { self }
        fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
        fn type_name(&self) -> &'static str { "TestTag" }
    }

    /// Test component trait implementation
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Component] -->|as_any| B[&dyn Any]
    ///     A -->|clone_box| C[Box<dyn Component>]
    ///     A -->|type_name| D[&'static str]
    /// ```
    #[test]
    fn test_component_trait() {
        let label = TestLabel("test".to_string());

        // Test as_any
        let any_ref = label.as_any();
        assert!(any_ref.downcast_ref::<TestLabel>().is_some());

        // Test clone_box
        let cloned = label.clone_box();
        let cloned_label = cloned.as_any().downcast_ref::<TestLabel>().unwrap();
        assert_eq!(cloned_label, &label);

        // Test type_name
        assert_eq!(label.type_name(), "TestLabel");
    }

    /// Test ComponentStorage creation
    #[test]
    fn test_component_storage_new() {
        let storage = ComponentStorage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }

    /// Test adding components
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Empty Storage] -->|add| B[Storage with Component]
    ///     B -->|add same type| C[Error: Already Exists]
    ///     B -->|add different type| D[Storage with 2 Components]
    /// ```
    #[test]
    fn test_add_component() {
        let mut storage = ComponentStorage::new();

        // Add first component
        let label = TestLabel("test".to_string());
        assert!(storage.add(label.clone()).is_ok());
        assert_eq!(storage.len(), 1);

        // Try to add same type again - should fail
        let label2 = TestLabel("test2".to_string());
        let result = storage.add(label2);
        assert!(result.is_err());
        match result {
            Err(DomainError::ComponentAlreadyExists(name)) => {
                assert_eq!(name, "TestLabel");
            }
            _ => panic!("Expected ComponentAlreadyExists error"),
        }

        // Add different type - should succeed
        let metadata = TestMetadata {
            key: "key".to_string(),
            value: 42,
        };
        assert!(storage.add(metadata).is_ok());
        assert_eq!(storage.len(), 2);
    }

    /// Test getting components
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Storage] -->|get<T>| B{Type Exists?}
    ///     B -->|Yes| C[Some(&T)]
    ///     B -->|No| D[None]
    /// ```
    #[test]
    fn test_get_component() {
        let mut storage = ComponentStorage::new();

        // Add component
        let label = TestLabel("test".to_string());
        storage.add(label.clone()).unwrap();

        // Get existing component
        let retrieved = storage.get::<TestLabel>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), &label);

        // Try to get non-existent component
        let missing = storage.get::<TestMetadata>();
        assert!(missing.is_none());
    }

    /// Test removing components
    #[test]
    fn test_remove_component() {
        let mut storage = ComponentStorage::new();

        // Add components
        let label = TestLabel("test".to_string());
        let metadata = TestMetadata {
            key: "key".to_string(),
            value: 42,
        };
        storage.add(label.clone()).unwrap();
        storage.add(metadata.clone()).unwrap();
        assert_eq!(storage.len(), 2);

        // Remove one component
        let removed = storage.remove::<TestLabel>();
        assert!(removed.is_some());
        assert_eq!(storage.len(), 1);

        // Verify it's gone
        assert!(!storage.has::<TestLabel>());
        assert!(storage.get::<TestLabel>().is_none());

        // Verify other component still exists
        assert!(storage.has::<TestMetadata>());
        assert!(storage.get::<TestMetadata>().is_some());

        // Try to remove non-existent component
        let removed_again = storage.remove::<TestLabel>();
        assert!(removed_again.is_none());
    }

    /// Test has component check
    #[test]
    fn test_has_component() {
        let mut storage = ComponentStorage::new();

        // Initially empty
        assert!(!storage.has::<TestLabel>());
        assert!(!storage.has::<TestMetadata>());

        // Add component
        storage.add(TestLabel("test".to_string())).unwrap();
        assert!(storage.has::<TestLabel>());
        assert!(!storage.has::<TestMetadata>());

        // Remove component
        storage.remove::<TestLabel>();
        assert!(!storage.has::<TestLabel>());
    }

    /// Test component storage iteration
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Storage] -->|iter| B[Iterator]
    ///     B -->|next| C[(&TypeId, &Component)]
    ///     C -->|next| D[(&TypeId, &Component)]
    ///     D -->|next| E[None]
    /// ```
    #[test]
    fn test_component_iteration() {
        let mut storage = ComponentStorage::new();

        // Add multiple components
        storage.add(TestLabel("test".to_string())).unwrap();
        storage.add(TestMetadata {
            key: "key".to_string(),
            value: 42,
        }).unwrap();
        storage.add(TestTag).unwrap();

        // Iterate and count
        let mut count = 0;
        let mut type_names = Vec::new();
        for (_, component) in storage.iter() {
            count += 1;
            type_names.push(component.type_name());
        }

        assert_eq!(count, 3);
        assert!(type_names.contains(&"TestLabel"));
        assert!(type_names.contains(&"TestMetadata"));
        assert!(type_names.contains(&"TestTag"));
    }

    /// Test component storage cloning
    #[test]
    fn test_component_storage_clone() {
        let mut storage = ComponentStorage::new();

        // Add components
        storage.add(TestLabel("test".to_string())).unwrap();
        storage.add(TestMetadata {
            key: "key".to_string(),
            value: 42,
        }).unwrap();

        // Clone storage
        let cloned = storage.clone();

        // Verify cloned storage has same components
        assert_eq!(cloned.len(), storage.len());
        assert!(cloned.has::<TestLabel>());
        assert!(cloned.has::<TestMetadata>());

        // Verify components are equal
        let original_label = storage.get::<TestLabel>().unwrap();
        let cloned_label = cloned.get::<TestLabel>().unwrap();
        assert_eq!(original_label, cloned_label);

        // Verify independence - modifying original doesn't affect clone
        storage.remove::<TestLabel>();
        assert!(!storage.has::<TestLabel>());
        assert!(cloned.has::<TestLabel>());
    }

    /// Test component storage debug formatting
    #[test]
    fn test_component_storage_debug() {
        let mut storage = ComponentStorage::new();

        // Empty storage
        let debug_empty = format!("{:?}", storage);
        assert!(debug_empty.contains("ComponentStorage"));
        assert!(debug_empty.contains("components: []"));

        // Add components
        storage.add(TestLabel("test".to_string())).unwrap();
        storage.add(TestMetadata {
            key: "key".to_string(),
            value: 42,
        }).unwrap();

        let debug_full = format!("{:?}", storage);
        assert!(debug_full.contains("ComponentStorage"));
        assert!(debug_full.contains("TestLabel"));
        assert!(debug_full.contains("TestMetadata"));
    }

    /// Test thread safety of components
    #[test]
    fn test_component_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        // Components must be Send + Sync
        let label = TestLabel("test".to_string());
        let boxed: Box<dyn Component> = Box::new(label);
        let arc = Arc::new(boxed);

        // Spawn thread to verify Send
        let arc_clone = arc.clone();
        let handle = thread::spawn(move || {
            assert_eq!(arc_clone.type_name(), "TestLabel");
        });

        handle.join().unwrap();

        // Verify Sync by accessing from multiple threads
        let handles: Vec<_> = (0..3)
            .map(|_| {
                let arc_clone = arc.clone();
                thread::spawn(move || {
                    assert_eq!(arc_clone.type_name(), "TestLabel");
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    /// Test component downcasting
    #[test]
    fn test_component_downcasting() {
        let mut storage = ComponentStorage::new();

        // Add component
        let label = TestLabel("test".to_string());
        storage.add(label.clone()).unwrap();

        // Get as specific type
        let retrieved = storage.get::<TestLabel>().unwrap();
        assert_eq!(retrieved.0, "test");

        // Verify wrong type returns None
        assert!(storage.get::<TestMetadata>().is_none());
        assert!(storage.get::<TestTag>().is_none());
    }
}
