// Copyright (c) 2025 - Cowboy AI, LLC.

//! Entity as MONAD - The bridge between DDD and ECS
//!
//! This module implements Entity as a proper monad M where M(A) wraps type A
//! with identity and components. This is the fundamental abstraction that
//! bridges Domain-Driven Design (DDD) with Entity-Component-System (ECS).
//!
//! # Mathematical Foundation
//!
//! Entity forms a monad with:
//! - `pure` (return): Lifts a value into the monad
//! - `bind` (>>=): Chains monadic computations
//! - `map`: Functor operation (derived from bind and pure)
//!
//! # Monad Laws
//!
//! 1. Left Identity: `pure a >>= f ≡ f a`
//! 2. Right Identity: `m >>= pure ≡ m`
//! 3. Associativity: `(m >>= f) >>= g ≡ m >>= (λx. f x >>= g)`

use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::entity::EntityId;

/// Entity is the MONAD M where M(A) wraps type A with identity and components
///
/// This is our fundamental abstraction that bridges DDD concepts with ECS patterns.
/// Every domain object is wrapped in this monad, providing:
/// - Type-safe identity through phantom types
/// - Component storage for ECS patterns
/// - Monadic composition for pure functional transformations
#[derive(Clone, Debug)]
pub struct Entity<A> {
    /// Type-safe entity ID with phantom type
    pub id: EntityId<A>,
    /// Components storage with type erasure
    pub components: Components<A>,
}

/// Components storage with type erasure
#[derive(Clone, Debug)]
pub struct Components<A> {
    /// The actual data, type-erased for flexibility
    pub(crate) data: Arc<dyn Any + Send + Sync>,
    /// Phantom type for compile-time type safety
    _phantom: PhantomData<A>,
}

impl<A: 'static + Send + Sync> Entity<A> {
    /// return/pure: Lift a value into the monad
    ///
    /// This is the monadic return operation that wraps a plain value
    /// into the Entity monad context.
    ///
    /// # Example
    /// ```rust
    /// use cim_domain::fp_monad::Entity;
    ///
    /// let value = 42;
    /// let entity = Entity::pure(value);
    /// ```
    pub fn pure(value: A) -> Entity<A> {
        Entity {
            id: EntityId::new(),
            components: Components {
                data: Arc::new(value),
                _phantom: PhantomData,
            },
        }
    }

    /// Create an entity with a specific ID
    pub fn with_id(id: EntityId<A>, value: A) -> Entity<A> {
        Entity {
            id,
            components: Components {
                data: Arc::new(value),
                _phantom: PhantomData,
            },
        }
    }

    /// bind/flatMap: M(A) -> (A -> M(B)) -> M(B)
    ///
    /// The monadic bind operation that allows chaining computations
    /// that return Entity values.
    ///
    /// # Example
    /// ```rust
    /// use cim_domain::fp_monad::Entity;
    ///
    /// let entity = Entity::pure(10);
    /// let result = entity.bind(|x| Entity::pure(x * 2));
    /// ```
    pub fn bind<B, F>(self, f: F) -> Entity<B>
    where
        F: FnOnce(A) -> Entity<B>,
        A: Clone,
        B: Send + Sync + 'static,
    {
        let value = self
            .components
            .data
            .downcast_ref::<A>()
            .expect("Type mismatch in Entity monad")
            .clone();
        f(value)
    }

    /// map: Functor operation
    ///
    /// Transform the value inside the monad without changing the monadic context.
    /// This is derived from bind and pure.
    ///
    /// # Example
    /// ```rust
    /// use cim_domain::fp_monad::Entity;
    ///
    /// let entity = Entity::pure(5);
    /// let doubled = entity.map(|x| x * 2);
    /// ```
    pub fn map<B, F>(self, f: F) -> Entity<B>
    where
        F: FnOnce(A) -> B,
        A: Clone,
        B: Send + Sync + 'static,
    {
        self.bind(|a| Entity::pure(f(a)))
    }

    /// Extract the value (use carefully at module boundaries)
    ///
    /// BREAKING FP: Entity extraction at module boundaries
    /// REASON: Need to bridge monadic and non-monadic code at system boundaries
    pub fn extract(self) -> A
    where
        A: Clone,
    {
        self.components
            .data
            .downcast_ref::<A>()
            .expect("Type mismatch in Entity extraction")
            .clone()
    }
}

/// Helper to run an Entity computation and extract the result
///
/// BREAKING FP: Entity extraction at module boundaries
/// REASON: Need to bridge monadic and non-monadic code at system boundaries
pub fn run_entity<A: Clone + Send + Sync + 'static>(entity: Entity<A>) -> A {
    entity.extract()
}

/// Kleisli arrow: A → Entity<B>
///
/// Functions that return monadic values, used for composing
/// monadic computations.
pub trait KleisliArrow<A, B>: Fn(A) -> Entity<B>
where
    B: Send + Sync + 'static,
{
}

impl<A, B, F> KleisliArrow<A, B> for F
where
    F: Fn(A) -> Entity<B>,
    B: Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monad_left_identity() {
        // Left Identity: pure a >>= f ≡ f a
        let a = 42;
        let f = |x: i32| Entity::pure(x * 2);

        let left = Entity::pure(a).bind(f);
        let right = f(a);

        assert_eq!(left.extract(), right.extract());
    }

    #[test]
    fn test_monad_right_identity() {
        // Right Identity: m >>= pure ≡ m
        let m = Entity::pure(42);
        let m_value = m.clone().extract();

        let result = m.bind(Entity::pure);

        assert_eq!(result.extract(), m_value);
    }

    #[test]
    fn test_monad_associativity() {
        // Associativity: (m >>= f) >>= g ≡ m >>= (λx. f x >>= g)
        let m = Entity::pure(10);
        let f = |x: i32| Entity::pure(x * 2);
        let g = |x: i32| Entity::pure(x + 5);

        let left = m.clone().bind(f).bind(g);
        let right = m.bind(|x| f(x).bind(g));

        assert_eq!(left.extract(), right.extract());
    }

    #[test]
    fn test_functor_map() {
        let entity = Entity::pure(10);
        let doubled = entity.map(|x| x * 2);

        assert_eq!(doubled.extract(), 20);
    }

    #[test]
    fn test_entity_with_id() {
        let id = EntityId::new();
        let entity = Entity::with_id(id, "test");

        assert_eq!(entity.id, id);
        assert_eq!(entity.extract(), "test");
    }
}
