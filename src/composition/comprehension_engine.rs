// Copyright 2025 Cowboy AI, LLC.

//! Comprehension engine for creating sub-aggregates from predicates
//!
//! This module implements the comprehension principles from topos theory,
//! allowing the creation of sub-aggregates that satisfy specific predicates.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::category::DomainObject;
use crate::composition_types::DomainCompositionType;
use crate::errors::DomainError;

/// Engine for creating sub-aggregates via comprehension
pub struct ComprehensionEngine {
    /// Registered predicates
    predicates: HashMap<String, Box<dyn Predicate>>,

    /// Comprehension cache
    cache: HashMap<String, SubAggregate>,
}

/// A predicate for comprehension
#[async_trait]
pub trait Predicate: Send + Sync {
    /// Evaluate the predicate on an object
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError>;

    /// Get a description of this predicate
    fn description(&self) -> String;

    /// Combine with another predicate via AND
    fn and(self: Box<Self>, other: Box<dyn Predicate>) -> Box<dyn Predicate>
    where
        Self: 'static + Sized,
    {
        Box::new(AndPredicate {
            left: self,
            right: other,
        })
    }

    /// Combine with another predicate via OR
    fn or(self: Box<Self>, other: Box<dyn Predicate>) -> Box<dyn Predicate>
    where
        Self: 'static + Sized,
    {
        Box::new(OrPredicate {
            left: self,
            right: other,
        })
    }

    /// Negate this predicate
    fn not(self: Box<Self>) -> Box<dyn Predicate>
    where
        Self: 'static + Sized,
    {
        Box::new(NotPredicate { inner: self })
    }
}

/// A sub-aggregate created via comprehension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAggregate {
    /// Unique identifier
    pub id: Uuid,

    /// Parent aggregate ID
    pub parent_id: String,

    /// Predicate that defines this sub-aggregate
    pub predicate_name: String,

    /// Objects that satisfy the predicate
    pub members: Vec<DomainObject>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// AND combination of predicates
struct AndPredicate {
    left: Box<dyn Predicate>,
    right: Box<dyn Predicate>,
}

#[async_trait]
impl Predicate for AndPredicate {
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        let left_result = self.left.evaluate(object).await?;
        if !left_result {
            return Ok(false); // Short-circuit
        }
        self.right.evaluate(object).await
    }

    fn description(&self) -> String {
        format!(
            "({} AND {})",
            self.left.description(),
            self.right.description()
        )
    }
}

/// OR combination of predicates
struct OrPredicate {
    left: Box<dyn Predicate>,
    right: Box<dyn Predicate>,
}

#[async_trait]
impl Predicate for OrPredicate {
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        let left_result = self.left.evaluate(object).await?;
        if left_result {
            return Ok(true); // Short-circuit
        }
        self.right.evaluate(object).await
    }

    fn description(&self) -> String {
        format!(
            "({} OR {})",
            self.left.description(),
            self.right.description()
        )
    }
}

/// NOT predicate
struct NotPredicate {
    inner: Box<dyn Predicate>,
}

#[async_trait]
impl Predicate for NotPredicate {
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        let result = self.inner.evaluate(object).await?;
        Ok(!result)
    }

    fn description(&self) -> String {
        format!("NOT {}", self.inner.description())
    }
}

/// Property-based predicate
pub struct PropertyPredicate {
    property_name: String,
    expected_value: String,
}

impl PropertyPredicate {
    /// Create a new property predicate
    ///
    /// # Arguments
    /// * `property_name` - Name of the property to check
    /// * `expected_value` - Expected value of the property
    pub fn new(property_name: String, expected_value: String) -> Self {
        Self {
            property_name,
            expected_value,
        }
    }
}

#[async_trait]
impl Predicate for PropertyPredicate {
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        if let Some(value) = object.metadata.get(&self.property_name) {
            Ok(value == &self.expected_value)
        } else {
            Ok(false)
        }
    }

    fn description(&self) -> String {
        format!("{} = {}", self.property_name, self.expected_value)
    }
}

/// Type-based predicate
pub struct TypePredicate {
    expected_type: DomainCompositionType,
}

impl TypePredicate {
    /// Create a new type predicate
    ///
    /// # Arguments
    /// * `expected_type` - The expected domain composition type
    pub fn new(expected_type: DomainCompositionType) -> Self {
        Self { expected_type }
    }
}

#[async_trait]
impl Predicate for TypePredicate {
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        Ok(object.composition_type == self.expected_type)
    }

    fn description(&self) -> String {
        format!("type = {}", self.expected_type.display_name())
    }
}

/// Lambda predicate for custom logic
pub struct LambdaPredicate<F>
where
    F: Fn(&DomainObject) -> bool + Send + Sync,
{
    function: F,
    description: String,
}

impl<F> LambdaPredicate<F>
where
    F: Fn(&DomainObject) -> bool + Send + Sync,
{
    /// Create a new lambda predicate with a custom function
    ///
    /// # Arguments
    /// * `function` - The predicate function
    /// * `description` - Human-readable description of the predicate
    pub fn new(function: F, description: String) -> Self {
        Self {
            function,
            description,
        }
    }
}

#[async_trait]
impl<F> Predicate for LambdaPredicate<F>
where
    F: Fn(&DomainObject) -> bool + Send + Sync,
{
    async fn evaluate(&self, object: &DomainObject) -> Result<bool, DomainError> {
        Ok((self.function)(object))
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

impl Default for ComprehensionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ComprehensionEngine {
    /// Create a new comprehension engine
    pub fn new() -> Self {
        Self {
            predicates: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a predicate
    pub fn register_predicate(
        &mut self,
        name: String,
        predicate: Box<dyn Predicate>,
    ) -> Result<(), DomainError> {
        if self.predicates.contains_key(&name) {
            return Err(DomainError::AlreadyExists(format!(
                "Predicate {name} already registered"
            )));
        }
        self.predicates.insert(name, predicate);
        Ok(())
    }

    /// Create a sub-aggregate via comprehension
    pub async fn comprehend(
        &mut self,
        parent_id: String,
        predicate_name: String,
        candidates: Vec<DomainObject>,
    ) -> Result<SubAggregate, DomainError> {
        let predicate = self.predicates.get(&predicate_name).ok_or_else(|| {
            DomainError::NotFound(format!("Predicate {predicate_name} not found"))
        })?;

        // Filter candidates that satisfy the predicate
        let mut members = Vec::new();
        for candidate in candidates {
            if predicate.evaluate(&candidate).await? {
                members.push(candidate);
            }
        }

        let sub_aggregate = SubAggregate {
            id: Uuid::new_v4(),
            parent_id: parent_id.clone(),
            predicate_name: predicate_name.clone(),
            members,
            metadata: HashMap::from([
                ("created_at".to_string(), chrono::Utc::now().to_rfc3339()),
                ("predicate_desc".to_string(), predicate.description()),
            ]),
        };

        // Cache the result
        let cache_key = format!("{parent_id}:{predicate_name}");
        self.cache.insert(cache_key, sub_aggregate.clone());

        Ok(sub_aggregate)
    }

    /// Get a cached sub-aggregate
    pub fn get_cached(&self, parent_id: &str, predicate_name: &str) -> Option<&SubAggregate> {
        let cache_key = format!("{parent_id}:{predicate_name}");
        self.cache.get(&cache_key)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Create common business predicates
    pub fn with_business_predicates(mut self) -> Self {
        // Active entities
        self.register_predicate(
            "active".to_string(),
            Box::new(PropertyPredicate::new(
                "status".to_string(),
                "active".to_string(),
            )),
        )
        .unwrap();

        // High-value predicate
        self.register_predicate(
            "high_value".to_string(),
            Box::new(LambdaPredicate::new(
                |obj| {
                    obj.metadata
                        .get("value")
                        .and_then(|v| v.parse::<f64>().ok())
                        .map(|v| v > 1000.0)
                        .unwrap_or(false)
                },
                "value > 1000".to_string(),
            )),
        )
        .unwrap();

        // Aggregate type predicate
        self.register_predicate(
            "is_aggregate".to_string(),
            Box::new(LambdaPredicate::new(
                |obj| {
                    matches!(
                        obj.composition_type,
                        DomainCompositionType::Aggregate { .. }
                    )
                },
                "type is Aggregate".to_string(),
            )),
        )
        .unwrap();

        self
    }
}

/// Example: Order filtering via comprehension
pub struct OrderComprehension {
    engine: ComprehensionEngine,
}

impl Default for OrderComprehension {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderComprehension {
    /// Create a new order comprehension engine with business predicates
    pub fn new() -> Self {
        let engine = ComprehensionEngine::new().with_business_predicates();
        Self { engine }
    }

    /// Get high-value active orders
    pub async fn high_value_active_orders(
        &mut self,
        orders: Vec<DomainObject>,
    ) -> Result<SubAggregate, DomainError> {
        // Register combined predicate
        let high_value = Box::new(PropertyPredicate::new(
            "value".to_string(),
            "high".to_string(),
        ));
        let active = Box::new(PropertyPredicate::new(
            "status".to_string(),
            "active".to_string(),
        ));
        let combined = high_value.and(active);

        self.engine
            .register_predicate("high_value_active".to_string(), combined)?;

        self.engine
            .comprehend(
                "OrderAggregate".to_string(),
                "high_value_active".to_string(),
                orders,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_property_predicate() {
        let predicate = PropertyPredicate::new("status".to_string(), "active".to_string());

        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), "active".to_string());

        let object = DomainObject {
            id: "test".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "Test".to_string(),
            },
            metadata,
        };

        assert!(predicate.evaluate(&object).await.unwrap());
    }

    #[tokio::test]
    async fn test_predicate_combination() {
        let status_active = Box::new(PropertyPredicate::new(
            "status".to_string(),
            "active".to_string(),
        ));

        let type_aggregate = Box::new(TypePredicate::new(DomainCompositionType::Aggregate {
            aggregate_type: "Order".to_string(),
        }));

        let combined = status_active.and(type_aggregate);

        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), "active".to_string());

        let object = DomainObject {
            id: "test".to_string(),
            composition_type: DomainCompositionType::Aggregate {
                aggregate_type: "Order".to_string(),
            },
            metadata,
        };

        assert!(combined.evaluate(&object).await.unwrap());
    }

    #[tokio::test]
    async fn test_comprehension_engine() {
        let mut engine = ComprehensionEngine::new();

        engine
            .register_predicate(
                "active".to_string(),
                Box::new(PropertyPredicate::new(
                    "status".to_string(),
                    "active".to_string(),
                )),
            )
            .unwrap();

        let mut objects = vec![];
        for i in 0..5 {
            let mut metadata = HashMap::new();
            metadata.insert(
                "status".to_string(),
                if i % 2 == 0 { "active" } else { "inactive" }.to_string(),
            );

            objects.push(DomainObject {
                id: format!("obj_{}", i),
                composition_type: DomainCompositionType::Entity {
                    entity_type: "Test".to_string(),
                },
                metadata,
            });
        }

        let sub_aggregate = engine
            .comprehend("parent".to_string(), "active".to_string(), objects)
            .await
            .unwrap();

        assert_eq!(sub_aggregate.members.len(), 3); // 0, 2, 4 are active
    }

    #[tokio::test]
    async fn test_not_predicate() {
        let active = Box::new(PropertyPredicate::new(
            "status".to_string(),
            "active".to_string(),
        ));

        let not_active = active.not();

        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), "inactive".to_string());

        let object = DomainObject {
            id: "test".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "Test".to_string(),
            },
            metadata,
        };

        assert!(not_active.evaluate(&object).await.unwrap());
    }
}
