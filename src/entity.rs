//! Entity types with identity and lifecycle

use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::time::SystemTime;
use uuid::Uuid;

/// A generic entity with a typed ID
///
/// Entities are domain objects with identity that persists across time.
/// They have a lifecycle with creation and update timestamps.
///
/// # Examples
///
/// ```rust
/// use cim_domain::{Entity, EntityId};
/// 
/// // Define a domain entity type
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// struct Customer;
/// 
/// // Create a new customer entity
/// let customer = Entity::<Customer>::new();
/// assert_eq!(customer.created_at, customer.updated_at);
/// 
/// // Create with a specific ID
/// let id = EntityId::<Customer>::new();
/// let customer = Entity::with_id(id);
/// assert_eq!(customer.id, id);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Entity<T> {
    /// The unique identifier for this entity
    pub id: EntityId<T>,
    /// When this entity was created
    pub created_at: SystemTime,
    /// When this entity was last updated
    pub updated_at: SystemTime,
}

impl<T> Entity<T> {
    /// Create a new entity with a generated ID
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            id: EntityId::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create an entity with a specific ID
    pub fn with_id(id: EntityId<T>) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the entity's timestamp
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cim_domain::Entity;
    /// use std::thread;
    /// use std::time::Duration;
    /// 
    /// struct Task;
    /// 
    /// let mut task = Entity::<Task>::new();
    /// let original_updated = task.updated_at;
    /// 
    /// // Wait a bit to ensure time difference
    /// thread::sleep(Duration::from_millis(10));
    /// 
    /// task.touch();
    /// assert!(task.updated_at > original_updated);
    /// ```
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now();
    }
}

impl<T> Default for Entity<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A typed entity ID using phantom types for type safety
///
/// These IDs are globally unique and persistent. The phantom type
/// parameter ensures that IDs for different entity types cannot be
/// mixed up at compile time.
///
/// # Examples
///
/// ```rust
/// use cim_domain::EntityId;
/// 
/// struct User;
/// struct Product;
/// 
/// let user_id = EntityId::<User>::new();
/// let product_id = EntityId::<Product>::new();
/// 
/// // These are different types - won't compile if mixed up:
/// // let _: EntityId<User> = product_id; // ERROR!
/// 
/// // But you can explicitly cast if needed (use carefully):
/// let casted: EntityId<Product> = user_id.cast();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId<T> {
    id: Uuid,
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

impl<T> EntityId<T> {
    /// Create a new random entity ID
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            _phantom: PhantomData,
        }
    }

    /// Create an entity ID from a UUID
    pub fn from_uuid(id: Uuid) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> &Uuid {
        &self.id
    }

    /// Convert to a different entity ID type (use with caution)
    pub fn cast<U>(self) -> EntityId<U> {
        EntityId {
            id: self.id,
            _phantom: PhantomData,
        }
    }
}

impl<T> fmt::Display for EntityId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T> Default for EntityId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<EntityId<T>> for Uuid {
    fn from(id: EntityId<T>) -> Self {
        id.id
    }
}

impl<T> From<&EntityId<T>> for Uuid {
    fn from(id: &EntityId<T>) -> Self {
        id.id
    }
}

/// Marker trait for aggregate roots
///
/// Aggregate roots are the entry points for modifying aggregates.
/// All changes to entities within an aggregate must go through the root.
///
/// # Examples
///
/// ```rust
/// use cim_domain::{AggregateRoot, EntityId};
/// 
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// struct Order;
/// 
/// struct OrderAggregate {
///     id: EntityId<Order>,
///     version: u64,
///     items: Vec<OrderItem>,
/// }
/// 
/// struct OrderItem {
///     product_id: String,
///     quantity: u32,
/// }
/// 
/// impl AggregateRoot for OrderAggregate {
///     type Id = EntityId<Order>;
///     
///     fn id(&self) -> Self::Id {
///         self.id
///     }
///     
///     fn version(&self) -> u64 {
///         self.version
///     }
///     
///     fn increment_version(&mut self) {
///         self.version += 1;
///     }
/// }
/// 
/// let mut order = OrderAggregate {
///     id: EntityId::new(),
///     version: 0,
///     items: vec![],
/// };
/// 
/// // All modifications go through the aggregate root
/// order.items.push(OrderItem {
///     product_id: "PROD-123".to_string(),
///     quantity: 2,
/// });
/// order.increment_version();
/// assert_eq!(order.version(), 1);
/// ```
pub trait AggregateRoot: Sized {
    /// The type of ID for this aggregate
    type Id: Copy + Eq + Send + Sync;

    /// Get the aggregate's ID
    fn id(&self) -> Self::Id;

    /// Get the aggregate's version for optimistic concurrency
    fn version(&self) -> u64;

    /// Increment the version
    fn increment_version(&mut self);
}

/// Trait for domain entities with identity
pub trait DomainEntity: Sized + Send + Sync {
    /// The marker type for this entity
    type IdType;
    
    /// Get the entity's ID
    fn id(&self) -> EntityId<Self::IdType>;
}

// Marker types for entity IDs
/// Marker for graph entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphMarker;

/// Marker for aggregate entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateMarker;

/// Marker for bounded context entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoundedContextMarker;

/// Marker for entity references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityMarker;

/// Marker for value object containers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValueObjectMarker;

/// Marker for service entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceMarker;

/// Marker for event entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventMarker;

/// Marker for command entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandMarker;

/// Marker for query entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryMarker;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    /// Test entity creation with generated ID
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Entity::new] -->|Generates| B[UUID]
    ///     A -->|Sets| C[created_at]
    ///     A -->|Sets| D[updated_at]
    ///     C -->|Equals| D
    /// ```
    #[test]
    fn test_entity_new() {
        let entity: Entity<GraphMarker> = Entity::new();

        // ID should be generated
        assert!(!entity.id.as_uuid().is_nil());

        // Timestamps should be set and equal
        assert_eq!(entity.created_at, entity.updated_at);

        // Timestamps should be recent (within last second)
        let now = SystemTime::now();
        let duration = now.duration_since(entity.created_at).unwrap();
        assert!(duration.as_secs() < 1);
    }

    /// Test entity creation with specific ID
    #[test]
    fn test_entity_with_id() {
        let id = EntityId::<GraphMarker>::new();
        let entity = Entity::with_id(id);

        assert_eq!(entity.id, id);
        assert_eq!(entity.created_at, entity.updated_at);
    }

    /// Test entity touch updates timestamp
    ///
    /// ```mermaid
    /// graph TD
    ///     A[Original Entity] -->|touch()| B[Updated Entity]
    ///     A -->|created_at| C[Unchanged]
    ///     A -->|updated_at| D[Changed]
    ///     A -->|id| E[Unchanged]
    /// ```
    #[test]
    fn test_entity_touch() {
        let mut entity: Entity<GraphMarker> = Entity::new();
        let original_created = entity.created_at;
        let original_updated = entity.updated_at;
        let original_id = entity.id;

        // Sleep briefly to ensure time difference
        thread::sleep(Duration::from_millis(10));

        entity.touch();

        // ID and created_at should not change
        assert_eq!(entity.id, original_id);
        assert_eq!(entity.created_at, original_created);

        // updated_at should change
        assert_ne!(entity.updated_at, original_updated);
        assert!(entity.updated_at > original_updated);
    }

    /// Test entity default implementation
    #[test]
    fn test_entity_default() {
        let entity1: Entity<GraphMarker> = Entity::default();
        let entity2: Entity<GraphMarker> = Entity::new();

        // Both should have unique IDs
        assert_ne!(entity1.id, entity2.id);

        // Both should have recent timestamps
        let now = SystemTime::now();
        assert!(now.duration_since(entity1.created_at).unwrap().as_secs() < 1);
        assert!(now.duration_since(entity2.created_at).unwrap().as_secs() < 1);
    }

    /// Test EntityId creation and uniqueness
    ///
    /// ```mermaid
    /// graph LR
    ///     A[EntityId::new] -->|UUID v4| B[Unique ID]
    ///     C[EntityId::new] -->|UUID v4| D[Different ID]
    ///     B -->|Not Equal| D
    /// ```
    #[test]
    fn test_entity_id_new() {
        let id1 = EntityId::<GraphMarker>::new();
        let id2 = EntityId::<GraphMarker>::new();

        // IDs should be unique
        assert_ne!(id1, id2);

        // IDs should not be nil
        assert!(!id1.as_uuid().is_nil());
        assert!(!id2.as_uuid().is_nil());
    }

    /// Test EntityId from UUID
    #[test]
    fn test_entity_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EntityId::<GraphMarker>::from_uuid(uuid);

        assert_eq!(id.as_uuid(), &uuid);
    }

    /// Test EntityId display formatting
    #[test]
    fn test_entity_id_display() {
        let uuid = Uuid::new_v4();
        let id = EntityId::<GraphMarker>::from_uuid(uuid);

        assert_eq!(format!("{id}"), format!("{uuid}"));
    }

    /// Test EntityId type safety with phantom types
    ///
    /// ```mermaid
    /// graph TD
    ///     A[EntityId<GraphMarker>] -->|cast| B[EntityId<AggregateMarker>]
    ///     A -->|Same UUID| B
    ///     A -->|Different Type| B
    /// ```
    #[test]
    fn test_entity_id_type_safety() {
        let graph_id = EntityId::<GraphMarker>::new();
        let aggregate_id: EntityId<AggregateMarker> = graph_id.cast();

        // Same underlying UUID
        assert_eq!(graph_id.as_uuid(), aggregate_id.as_uuid());

        // But different types at compile time
        // This would not compile:
        // let _: EntityId<GraphMarker> = aggregate_id;
    }

    /// Test EntityId serialization/deserialization
    #[test]
    fn test_entity_id_serde() {
        let original = EntityId::<GraphMarker>::new();

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: EntityId<GraphMarker> = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    /// Test Entity serialization/deserialization
    #[test]
    fn test_entity_serde() {
        let original = Entity::<GraphMarker>::new();

        // Serialize
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize
        let deserialized: Entity<GraphMarker> = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    /// Test aggregate root implementation
    struct TestAggregate {
        id: EntityId<AggregateMarker>,
        version: u64,
        _data: String,
    }

    impl AggregateRoot for TestAggregate {
        type Id = EntityId<AggregateMarker>;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn version(&self) -> u64 {
            self.version
        }

        fn increment_version(&mut self) {
            self.version += 1;
        }
    }

    /// Test AggregateRoot trait implementation
    ///
    /// ```mermaid
    /// graph LR
    ///     A[Aggregate v1] -->|increment_version| B[Aggregate v2]
    ///     B -->|increment_version| C[Aggregate v3]
    ///     A -->|Same ID| B
    ///     B -->|Same ID| C
    /// ```
    #[test]
    fn test_aggregate_root() {
        let mut aggregate = TestAggregate {
            id: EntityId::new(),
            version: 1,
            _data: "test".to_string(),
        };

        let original_id = aggregate.id();

        // Test initial state
        assert_eq!(aggregate.version(), 1);

        // Test version increment
        aggregate.increment_version();
        assert_eq!(aggregate.version(), 2);

        // ID should not change
        assert_eq!(aggregate.id(), original_id);

        // Test multiple increments
        aggregate.increment_version();
        aggregate.increment_version();
        assert_eq!(aggregate.version(), 4);
    }

    /// Test all marker types are distinct
    #[test]
    fn test_marker_types() {
        // Create IDs with different markers
        let graph_id = EntityId::<GraphMarker>::new();
        let aggregate_id = EntityId::<AggregateMarker>::new();
        let context_id = EntityId::<BoundedContextMarker>::new();
        let entity_id = EntityId::<EntityMarker>::new();
        let value_id = EntityId::<ValueObjectMarker>::new();
        let service_id = EntityId::<ServiceMarker>::new();
        let event_id = EntityId::<EventMarker>::new();
        let command_id = EntityId::<CommandMarker>::new();
        let query_id = EntityId::<QueryMarker>::new();

        // All should have unique UUIDs
        let uuids = vec![
            graph_id.as_uuid(),
            aggregate_id.as_uuid(),
            context_id.as_uuid(),
            entity_id.as_uuid(),
            value_id.as_uuid(),
            service_id.as_uuid(),
            event_id.as_uuid(),
            command_id.as_uuid(),
            query_id.as_uuid(),
        ];

        // Check all UUIDs are unique
        for i in 0..uuids.len() {
            for j in (i + 1)..uuids.len() {
                assert_ne!(uuids[i], uuids[j]);
            }
        }
    }

    /// Test Entity equality and hashing
    #[test]
    fn test_entity_equality() {
        // Create entity and clone it to ensure same timestamps
        let entity1 = Entity::<GraphMarker>::new();
        let entity2 = entity1.clone();

        // Cloned entities should be equal
        assert_eq!(entity1, entity2);

        // Different entities should not be equal
        let entity3 = Entity::<GraphMarker>::new();
        assert_ne!(entity1, entity3);

        // Entities with same ID but different timestamps are not equal
        let id = EntityId::<GraphMarker>::new();
        let entity4 = Entity::with_id(id);
        thread::sleep(Duration::from_millis(1));
        let entity5 = Entity::with_id(id);
        assert_ne!(entity4, entity5); // Different timestamps
    }

    /// Test EntityId as hash map key
    #[test]
    fn test_entity_id_as_key() {
        use std::collections::HashMap;

        let mut map = HashMap::new();
        let id1 = EntityId::<GraphMarker>::new();
        let id2 = EntityId::<GraphMarker>::new();

        map.insert(id1, "value1");
        map.insert(id2, "value2");

        assert_eq!(map.get(&id1), Some(&"value1"));
        assert_eq!(map.get(&id2), Some(&"value2"));
        assert_eq!(map.len(), 2);
    }
}
