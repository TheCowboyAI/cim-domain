// Copyright 2025 Cowboy AI, LLC.

//! Functors for mapping between domain categories
//!
//! Functors preserve the structure of domain categories while allowing
//! transformations between different domains. They are the foundation
//! for cross-domain communication in CIM.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use super::domain_category::{DomainCategory, DomainMorphism, DomainObject};
use crate::errors::DomainError;

// Type aliases for complex function types
type ObjectValidator = Arc<dyn Fn(&DomainObject) -> bool + Send + Sync>;
type ObjectTransformer = Arc<dyn Fn(DomainObject) -> DomainObject + Send + Sync>;

/// A functor between domain categories
#[async_trait]
pub trait DomainFunctor: Send + Sync {
    /// Source category
    type Source: Send + Sync;

    /// Target category
    type Target: Send + Sync;

    /// Map an object from source to target category
    async fn map_object(&self, obj: DomainObject) -> Result<DomainObject, DomainError>;

    /// Map a morphism from source to target category
    async fn map_morphism(&self, morph: DomainMorphism) -> Result<DomainMorphism, DomainError>;

    /// Get the source category name
    fn source_category(&self) -> String;

    /// Get the target category name
    fn target_category(&self) -> String;
}

/// Identity functor - maps a category to itself
pub struct FunctorIdentity<C> {
    category_name: String,
    _phantom: PhantomData<C>,
}

impl<C> FunctorIdentity<C> {
    /// Create a new identity functor for a category
    ///
    /// # Arguments
    /// * `category_name` - Name of the category this functor operates on
    pub fn new(category_name: String) -> Self {
        Self {
            category_name,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<C> DomainFunctor for FunctorIdentity<C>
where
    C: Send + Sync + 'static,
{
    type Source = C;
    type Target = C;

    async fn map_object(&self, obj: DomainObject) -> Result<DomainObject, DomainError> {
        Ok(obj)
    }

    async fn map_morphism(&self, morph: DomainMorphism) -> Result<DomainMorphism, DomainError> {
        Ok(morph)
    }

    fn source_category(&self) -> String {
        self.category_name.clone()
    }

    fn target_category(&self) -> String {
        self.category_name.clone()
    }
}

/// Composition of functors
pub struct FunctorComposition<F, G>
where
    F: DomainFunctor,
    G: DomainFunctor,
{
    first: F,
    second: G,
}

impl<F, G> FunctorComposition<F, G>
where
    F: DomainFunctor,
    G: DomainFunctor,
{
    /// Create a new composed functor F ∘ G
    ///
    /// # Arguments
    /// * `first` - The first functor to apply
    /// * `second` - The second functor to apply (after first)
    pub fn new(first: F, second: G) -> Self {
        Self { first, second }
    }
}

#[async_trait]
impl<F, G> DomainFunctor for FunctorComposition<F, G>
where
    F: DomainFunctor + Send + Sync,
    G: DomainFunctor + Send + Sync,
    F::Target: Send + Sync + 'static,
{
    type Source = F::Source;
    type Target = G::Target;

    async fn map_object(&self, obj: DomainObject) -> Result<DomainObject, DomainError> {
        let intermediate = self.first.map_object(obj).await?;
        self.second.map_object(intermediate).await
    }

    async fn map_morphism(&self, morph: DomainMorphism) -> Result<DomainMorphism, DomainError> {
        let intermediate = self.first.map_morphism(morph).await?;
        self.second.map_morphism(intermediate).await
    }

    fn source_category(&self) -> String {
        self.first.source_category()
    }

    fn target_category(&self) -> String {
        self.second.target_category()
    }
}

/// Context mapping functor - maps between bounded contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMappingFunctor {
    source_context: String,
    target_context: String,
    object_mappings: HashMap<String, String>,
    morphism_mappings: HashMap<String, String>,
    type_transformations: HashMap<String, String>,
}

impl ContextMappingFunctor {
    /// Create a new context mapping functor
    ///
    /// # Arguments
    /// * `source_context` - The source bounded context name
    /// * `target_context` - The target bounded context name
    pub fn new(source_context: String, target_context: String) -> Self {
        Self {
            source_context,
            target_context,
            object_mappings: HashMap::new(),
            morphism_mappings: HashMap::new(),
            type_transformations: HashMap::new(),
        }
    }

    /// Add an object mapping
    pub fn add_object_mapping(&mut self, source_id: String, target_id: String) {
        self.object_mappings.insert(source_id, target_id);
    }

    /// Add a morphism mapping
    pub fn add_morphism_mapping(&mut self, source_id: String, target_id: String) {
        self.morphism_mappings.insert(source_id, target_id);
    }

    /// Add a type transformation
    pub fn add_type_transformation(&mut self, source_type: String, target_type: String) {
        self.type_transformations.insert(source_type, target_type);
    }
}

#[async_trait]
impl DomainFunctor for ContextMappingFunctor {
    type Source = DomainCategory;
    type Target = DomainCategory;

    async fn map_object(&self, mut obj: DomainObject) -> Result<DomainObject, DomainError> {
        // Map object ID if mapping exists
        if let Some(target_id) = self.object_mappings.get(&obj.id) {
            obj.id = target_id.clone();
        }

        // Transform metadata to indicate context mapping
        obj.metadata
            .insert("mapped_from".to_string(), self.source_context.clone());
        obj.metadata
            .insert("mapped_to".to_string(), self.target_context.clone());

        Ok(obj)
    }

    async fn map_morphism(&self, mut morph: DomainMorphism) -> Result<DomainMorphism, DomainError> {
        // Map morphism ID if mapping exists
        if let Some(target_id) = self.morphism_mappings.get(&morph.id) {
            morph.id = target_id.clone();
        }

        // Map source and target objects
        if let Some(target_source) = self.object_mappings.get(&morph.source) {
            morph.source = target_source.clone();
        }
        if let Some(target_target) = self.object_mappings.get(&morph.target) {
            morph.target = target_target.clone();
        }

        // Transform metadata
        morph
            .metadata
            .insert("mapped_from".to_string(), self.source_context.clone());
        morph
            .metadata
            .insert("mapped_to".to_string(), self.target_context.clone());

        Ok(morph)
    }

    fn source_category(&self) -> String {
        self.source_context.clone()
    }

    fn target_category(&self) -> String {
        self.target_context.clone()
    }
}

/// Anti-corruption layer functor - protects domain integrity
pub struct AntiCorruptionFunctor {
    source_domain: String,
    target_domain: String,
    validators: Vec<ObjectValidator>,
    transformers: HashMap<String, ObjectTransformer>,
}

impl AntiCorruptionFunctor {
    /// Create a new anti-corruption layer functor
    ///
    /// # Arguments
    /// * `source_domain` - The source domain name
    /// * `target_domain` - The target domain name (protected)
    pub fn new(source_domain: String, target_domain: String) -> Self {
        Self {
            source_domain,
            target_domain,
            validators: Vec::new(),
            transformers: HashMap::new(),
        }
    }

    /// Add a validation rule
    pub fn add_validator<F>(&mut self, validator: F)
    where
        F: Fn(&DomainObject) -> bool + Send + Sync + 'static,
    {
        self.validators.push(Arc::new(validator));
    }

    /// Add a transformation for a specific object type
    pub fn add_transformer<F>(&mut self, object_type: String, transformer: F)
    where
        F: Fn(DomainObject) -> DomainObject + Send + Sync + 'static,
    {
        self.transformers.insert(object_type, Arc::new(transformer));
    }
}

#[async_trait]
impl DomainFunctor for AntiCorruptionFunctor {
    type Source = DomainCategory;
    type Target = DomainCategory;

    async fn map_object(&self, obj: DomainObject) -> Result<DomainObject, DomainError> {
        // Validate object
        for validator in &self.validators {
            if !validator(&obj) {
                return Err(DomainError::InvalidOperation {
                    reason: format!("Object {} failed anti-corruption validation", obj.id),
                });
            }
        }

        // Apply transformation if available
        let base_type = obj.composition_type.base_type_name();
        if let Some(transformer) = self.transformers.get(base_type) {
            Ok(transformer(obj))
        } else {
            Ok(obj)
        }
    }

    async fn map_morphism(&self, morph: DomainMorphism) -> Result<DomainMorphism, DomainError> {
        // For now, pass through morphisms
        // In a real implementation, we'd validate and transform these too
        Ok(morph)
    }

    fn source_category(&self) -> String {
        self.source_domain.clone()
    }

    fn target_category(&self) -> String {
        self.target_domain.clone()
    }
}

/// Forgetful functor - forgets some structure
pub struct ForgetfulFunctor {
    source_domain: String,
    target_domain: String,
    properties_to_forget: Vec<String>,
}

impl ForgetfulFunctor {
    /// Create a new forgetful functor
    ///
    /// # Arguments
    /// * `source_domain` - The source domain with full structure
    /// * `target_domain` - The target domain with reduced structure
    pub fn new(source_domain: String, target_domain: String) -> Self {
        Self {
            source_domain,
            target_domain,
            properties_to_forget: Vec::new(),
        }
    }

    /// Add a property to forget when mapping objects
    ///
    /// # Arguments
    /// * `property` - Name of the property to remove
    pub fn forget_property(&mut self, property: String) {
        self.properties_to_forget.push(property);
    }
}

#[async_trait]
impl DomainFunctor for ForgetfulFunctor {
    type Source = DomainCategory;
    type Target = DomainCategory;

    async fn map_object(&self, mut obj: DomainObject) -> Result<DomainObject, DomainError> {
        // Remove specified properties from metadata
        for prop in &self.properties_to_forget {
            obj.metadata.remove(prop);
        }

        Ok(obj)
    }

    async fn map_morphism(&self, mut morph: DomainMorphism) -> Result<DomainMorphism, DomainError> {
        // Remove specified properties from metadata
        for prop in &self.properties_to_forget {
            morph.metadata.remove(prop);
        }

        Ok(morph)
    }

    fn source_category(&self) -> String {
        self.source_domain.clone()
    }

    fn target_category(&self) -> String {
        self.target_domain.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition_types::DomainCompositionType;

    #[tokio::test]
    async fn test_identity_functor() {
        let functor = FunctorIdentity::<DomainCategory>::new("TestDomain".to_string());

        let obj = DomainObject {
            id: "test_obj".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "TestEntity".to_string(),
            },
            metadata: HashMap::new(),
        };

        let mapped = functor.map_object(obj.clone()).await.unwrap();
        assert_eq!(mapped.id, obj.id);
    }

    #[tokio::test]
    async fn test_context_mapping_functor() {
        let mut functor = ContextMappingFunctor::new("Sales".to_string(), "Billing".to_string());

        functor.add_object_mapping("Order".to_string(), "Invoice".to_string());

        let obj = DomainObject {
            id: "Order".to_string(),
            composition_type: DomainCompositionType::Aggregate {
                aggregate_type: "Order".to_string(),
            },
            metadata: HashMap::new(),
        };

        let mapped = functor.map_object(obj).await.unwrap();
        assert_eq!(mapped.id, "Invoice");
        assert_eq!(mapped.metadata.get("mapped_from").unwrap(), "Sales");
        assert_eq!(mapped.metadata.get("mapped_to").unwrap(), "Billing");
    }

    #[tokio::test]
    async fn test_anti_corruption_functor() {
        let mut functor =
            AntiCorruptionFunctor::new("External".to_string(), "Internal".to_string());

        // Add validator that rejects empty IDs
        functor.add_validator(|obj| !obj.id.is_empty());

        let valid_obj = DomainObject {
            id: "valid".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "TestEntity".to_string(),
            },
            metadata: HashMap::new(),
        };

        let invalid_obj = DomainObject {
            id: "".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "TestEntity".to_string(),
            },
            metadata: HashMap::new(),
        };

        assert!(functor.map_object(valid_obj).await.is_ok());
        assert!(functor.map_object(invalid_obj).await.is_err());
    }

    #[tokio::test]
    async fn test_forgetful_functor() {
        let mut functor = ForgetfulFunctor::new("Detailed".to_string(), "Simple".to_string());

        functor.forget_property("internal_id".to_string());
        functor.forget_property("timestamp".to_string());

        let mut metadata = HashMap::new();
        metadata.insert("name".to_string(), "Test".to_string());
        metadata.insert("internal_id".to_string(), "12345".to_string());
        metadata.insert("timestamp".to_string(), "2024-01-01".to_string());

        let obj = DomainObject {
            id: "test".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "TestEntity".to_string(),
            },
            metadata,
        };

        let mapped = functor.map_object(obj).await.unwrap();
        assert!(mapped.metadata.contains_key("name"));
        assert!(!mapped.metadata.contains_key("internal_id"));
        assert!(!mapped.metadata.contains_key("timestamp"));
    }
}
