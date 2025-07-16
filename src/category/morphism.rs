//! Morphism abstractions for domain operations
//!
//! Morphisms represent structure-preserving transformations between domain objects.
//! They capture the essence of domain operations while maintaining category laws.

use std::marker::PhantomData;
use async_trait::async_trait;

use crate::errors::DomainError;
use crate::events::DomainEvent;
use crate::commands::DomainCommand;

/// A morphism between domain objects
#[async_trait]
pub trait Morphism: Send + Sync {
    /// Source domain object type
    type Source;
    
    /// Target domain object type
    type Target;
    
    /// Apply the morphism to transform source to target
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError>;
    
    /// Get a human-readable description
    fn description(&self) -> String;
}

/// Composition of two morphisms
pub struct MorphismComposition<F, G, A, B, C> 
where
    F: Morphism<Source = A, Target = B>,
    G: Morphism<Source = B, Target = C>,
    A: Send + Sync,
    B: Send + Sync,
    C: Send + Sync,
{
    first: F,
    second: G,
    _phantom: PhantomData<(A, B, C)>,
}

impl<F, G, A, B, C> MorphismComposition<F, G, A, B, C>
where
    F: Morphism<Source = A, Target = B>,
    G: Morphism<Source = B, Target = C>,
    A: Send + Sync,
    B: Send + Sync,
    C: Send + Sync,
{
    /// Create a new composition
    pub fn new(first: F, second: G) -> Self {
        Self {
            first,
            second,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<F, G, A, B, C> Morphism for MorphismComposition<F, G, A, B, C>
where
    F: Morphism<Source = A, Target = B> + Send + Sync,
    G: Morphism<Source = B, Target = C> + Send + Sync,
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
{
    type Source = A;
    type Target = C;
    
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
        let intermediate = self.first.apply(source).await?;
        self.second.apply(intermediate).await
    }
    
    fn description(&self) -> String {
        format!("{} ∘ {}", self.second.description(), self.first.description())
    }
}

/// Identity morphism
pub struct MorphismIdentity<T> {
    _phantom: PhantomData<T>,
}

impl<T> MorphismIdentity<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for MorphismIdentity<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<T> Morphism for MorphismIdentity<T>
where
    T: Send + Sync + 'static,
{
    type Source = T;
    type Target = T;
    
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
        Ok(source)
    }
    
    fn description(&self) -> String {
        "identity".to_string()
    }
}

/// Command metadata for morphisms
pub struct CommandMetadata {
    pub command_type: String,
    pub aggregate_id: String,
}

/// Command morphism - transforms state via commands
pub struct CommandMorphism<S, T> {
    command_metadata: CommandMetadata,
    transformer: Box<dyn Fn(S, Vec<Box<dyn DomainEvent>>) -> Result<T, DomainError> + Send + Sync>,
}

impl<S, T> CommandMorphism<S, T> {
    pub fn new(
        command_metadata: CommandMetadata,
        transformer: Box<dyn Fn(S, Vec<Box<dyn DomainEvent>>) -> Result<T, DomainError> + Send + Sync>,
    ) -> Self {
        Self {
            command_metadata,
            transformer,
        }
    }
    
    pub fn from_command<C: DomainCommand>(
        command: &C,
        transformer: Box<dyn Fn(S, Vec<Box<dyn DomainEvent>>) -> Result<T, DomainError> + Send + Sync>,
    ) -> Self {
        Self {
            command_metadata: CommandMetadata {
                command_type: command.command_type().to_string(),
                aggregate_id: command.aggregate_id(),
            },
            transformer,
        }
    }
}

#[async_trait]
impl<S, T> Morphism for CommandMorphism<S, T>
where
    S: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Source = S;
    type Target = T;
    
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
        // In a real implementation, this would:
        // 1. Execute the command
        // 2. Collect resulting events
        // 3. Apply the transformer
        
        // For now, we'll simulate with empty events
        let events = Vec::new();
        (self.transformer)(source, events)
    }
    
    fn description(&self) -> String {
        format!("Command[{}]", self.command_metadata.aggregate_id)
    }
}

/// Event morphism - transforms state via events
pub struct EventMorphism<S, T> {
    event_type: String,
    transformer: Box<dyn Fn(S, Box<dyn DomainEvent>) -> Result<T, DomainError> + Send + Sync>,
}

impl<S, T> EventMorphism<S, T> {
    pub fn new(
        event_type: String,
        transformer: Box<dyn Fn(S, Box<dyn DomainEvent>) -> Result<T, DomainError> + Send + Sync>,
    ) -> Self {
        Self {
            event_type,
            transformer,
        }
    }
}

#[async_trait]
impl<S, T> Morphism for EventMorphism<S, T>
where
    S: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Source = S;
    type Target = T;
    
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
        // In a real implementation, this would apply the event
        // For now, return an error indicating not implemented
        Err(DomainError::NotImplemented("Event morphism application".to_string()))
    }
    
    fn description(&self) -> String {
        format!("Event[{}]", self.event_type)
    }
}

/// Query morphism - extracts information without changing state
pub struct QueryMorphism<S, T> {
    query_type: String,
    extractor: Box<dyn Fn(&S) -> Result<T, DomainError> + Send + Sync>,
}

impl<S, T> QueryMorphism<S, T> {
    pub fn new(
        query_type: String,
        extractor: Box<dyn Fn(&S) -> Result<T, DomainError> + Send + Sync>,
    ) -> Self {
        Self {
            query_type,
            extractor,
        }
    }
}

#[async_trait]
impl<S, T> Morphism for QueryMorphism<S, T>
where
    S: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Source = S;
    type Target = T;
    
    async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
        (self.extractor)(&source)
    }
    
    fn description(&self) -> String {
        format!("Query[{}]", self.query_type)
    }
}

/// Isomorphism - bidirectional morphism
pub struct Isomorphism<A, B> {
    forward: Box<dyn Morphism<Source = A, Target = B>>,
    backward: Box<dyn Morphism<Source = B, Target = A>>,
}

impl<A, B> Isomorphism<A, B> {
    pub fn new(
        forward: Box<dyn Morphism<Source = A, Target = B>>,
        backward: Box<dyn Morphism<Source = B, Target = A>>,
    ) -> Self {
        Self {
            forward,
            backward,
        }
    }
    
    pub fn inverse(self) -> Isomorphism<B, A> {
        Isomorphism {
            forward: self.backward,
            backward: self.forward,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_identity_morphism() {
        let id = MorphismIdentity::<String>::new();
        let input = "test".to_string();
        let result = id.apply(input.clone()).await.unwrap();
        assert_eq!(result, input);
    }
    
    #[tokio::test]
    async fn test_morphism_composition() {
        // Create two simple morphisms
        struct AddOne;
        struct MultiplyTwo;
        
        #[async_trait]
        impl Morphism for AddOne {
            type Source = i32;
            type Target = i32;
            
            async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
                Ok(source + 1)
            }
            
            fn description(&self) -> String {
                "add_one".to_string()
            }
        }
        
        #[async_trait]
        impl Morphism for MultiplyTwo {
            type Source = i32;
            type Target = i32;
            
            async fn apply(&self, source: Self::Source) -> Result<Self::Target, DomainError> {
                Ok(source * 2)
            }
            
            fn description(&self) -> String {
                "multiply_two".to_string()
            }
        }
        
        let add = AddOne;
        let mul = MultiplyTwo;
        
        let composition = MorphismComposition::new(add, mul);
        
        // (5 + 1) * 2 = 12
        let result = composition.apply(5).await.unwrap();
        assert_eq!(result, 12);
        
        assert_eq!(composition.description(), "multiply_two ∘ add_one");
    }
    
    #[tokio::test]
    async fn test_query_morphism() {
        struct User {
            name: String,
            age: u32,
        }
        
        let get_name = QueryMorphism::new(
            "get_name".to_string(),
            Box::new(|user: &User| Ok(user.name.clone())),
        );
        
        let user = User {
            name: "Alice".to_string(),
            age: 30,
        };
        
        let name = get_name.apply(user).await.unwrap();
        assert_eq!(name, "Alice");
    }
}