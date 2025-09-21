// Copyright (c) 2025 - Cowboy AI, LLC.

//! Domain predicates for cross-domain reasoning
//!
//! Predicates that can be evaluated across domain boundaries to
//! implement business logic and constraints.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::composition::DomainComposition;
use crate::errors::DomainError;

/// A predicate that can be evaluated in a domain context
#[async_trait]
pub trait DomainPredicate: Send + Sync {
    /// Evaluate the predicate
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError>;

    /// Get a description of this predicate
    fn description(&self) -> String;

    /// Combine with another predicate via AND
    fn and(self: Box<Self>, other: Box<dyn DomainPredicate>) -> Box<dyn DomainPredicate>
    where
        Self: 'static + Sized,
    {
        Box::new(AndPredicate {
            left: self,
            right: other,
        })
    }

    /// Combine with another predicate via OR
    fn or(self: Box<Self>, other: Box<dyn DomainPredicate>) -> Box<dyn DomainPredicate>
    where
        Self: 'static + Sized,
    {
        Box::new(OrPredicate {
            left: self,
            right: other,
        })
    }

    /// Negate this predicate
    fn not(self: Box<Self>) -> Box<dyn DomainPredicate>
    where
        Self: 'static + Sized,
    {
        Box::new(NotPredicate { inner: self })
    }

    /// Create an implication
    fn implies(self: Box<Self>, consequent: Box<dyn DomainPredicate>) -> Box<dyn DomainPredicate>
    where
        Self: 'static + Sized,
    {
        Box::new(ImplicationPredicate {
            antecedent: self,
            consequent,
        })
    }
}

/// Context for predicate evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateContext {
    /// Target domain
    pub domain: Option<String>,

    /// Target object
    pub object_id: Option<String>,

    /// Additional parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

impl Default for PredicateContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PredicateContext {
    /// Create a new predicate context
    pub fn new() -> Self {
        Self {
            domain: None,
            object_id: None,
            parameters: HashMap::new(),
        }
    }

    /// Set the domain for this context
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Set the object ID for this context
    pub fn with_object(mut self, object_id: String) -> Self {
        self.object_id = Some(object_id);
        self
    }

    /// Add a parameter to the context
    pub fn with_parameter(mut self, key: String, value: serde_json::Value) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

/// Result of evaluating a predicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateResult {
    /// The truth value
    pub value: bool,

    /// Confidence in the result (0-100)
    pub confidence: u8,

    /// Explanation of how the result was derived
    pub explanation: String,

    /// Evidence supporting the result
    pub evidence: Vec<Evidence>,
}

/// Evidence supporting a predicate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,

    /// Source of the evidence
    pub source: String,

    /// The evidence data
    pub data: serde_json::Value,

    /// Weight of this evidence (0-100)
    pub weight: u8,
}

/// Types of evidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Direct observation
    Observation,

    /// Derived from other facts
    Inference,

    /// Historical data
    Historical,

    /// External source
    External,

    /// Default assumption
    Default,
}

/// Evaluator for domain predicates
pub struct PredicateEvaluator {
    /// Registered predicates
    predicates: HashMap<String, Box<dyn DomainPredicate>>,

    /// Evaluation cache
    cache: HashMap<String, PredicateResult>,
}

impl Default for PredicateEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl PredicateEvaluator {
    /// Create a new predicate evaluator
    pub fn new() -> Self {
        Self {
            predicates: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a named predicate
    pub fn register(
        &mut self,
        name: String,
        predicate: Box<dyn DomainPredicate>,
    ) -> Result<(), DomainError> {
        if self.predicates.contains_key(&name) {
            return Err(DomainError::AlreadyExists(format!(
                "Predicate {name} already registered"
            )));
        }
        self.predicates.insert(name, predicate);
        Ok(())
    }

    /// Evaluate a named predicate
    pub async fn evaluate(
        &mut self,
        name: &str,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        // Check cache
        let cache_key = format!("{}:{:?}:{:?}", name, context.domain, context.object_id);
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        // Get predicate
        let predicate = self
            .predicates
            .get(name)
            .ok_or_else(|| DomainError::NotFound(format!("Predicate {name} not found")))?;

        // Evaluate
        let result = predicate.evaluate(composition, context).await?;

        // Cache result
        self.cache.insert(cache_key, result.clone());

        Ok(result)
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

// Logical connectives

struct AndPredicate {
    left: Box<dyn DomainPredicate>,
    right: Box<dyn DomainPredicate>,
}

#[async_trait]
impl DomainPredicate for AndPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        let left_result = self.left.evaluate(composition, context).await?;
        if !left_result.value {
            return Ok(PredicateResult {
                value: false,
                confidence: left_result.confidence,
                explanation: format!("AND: left side false - {}", left_result.explanation),
                evidence: left_result.evidence,
            });
        }

        let right_result = self.right.evaluate(composition, context).await?;

        Ok(PredicateResult {
            value: right_result.value,
            confidence: left_result.confidence.min(right_result.confidence),
            explanation: format!(
                "AND: {} AND {}",
                left_result.explanation, right_result.explanation
            ),
            evidence: [left_result.evidence, right_result.evidence].concat(),
        })
    }

    fn description(&self) -> String {
        format!(
            "({} AND {})",
            self.left.description(),
            self.right.description()
        )
    }
}

struct OrPredicate {
    left: Box<dyn DomainPredicate>,
    right: Box<dyn DomainPredicate>,
}

#[async_trait]
impl DomainPredicate for OrPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        let left_result = self.left.evaluate(composition, context).await?;
        if left_result.value {
            return Ok(PredicateResult {
                value: true,
                confidence: left_result.confidence,
                explanation: format!("OR: left side true - {}", left_result.explanation),
                evidence: left_result.evidence,
            });
        }

        let right_result = self.right.evaluate(composition, context).await?;

        Ok(PredicateResult {
            value: right_result.value,
            confidence: left_result.confidence.max(right_result.confidence),
            explanation: format!(
                "OR: {} OR {}",
                left_result.explanation, right_result.explanation
            ),
            evidence: [left_result.evidence, right_result.evidence].concat(),
        })
    }

    fn description(&self) -> String {
        format!(
            "({} OR {})",
            self.left.description(),
            self.right.description()
        )
    }
}

struct NotPredicate {
    inner: Box<dyn DomainPredicate>,
}

#[async_trait]
impl DomainPredicate for NotPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        let inner_result = self.inner.evaluate(composition, context).await?;

        Ok(PredicateResult {
            value: !inner_result.value,
            confidence: inner_result.confidence,
            explanation: format!("NOT: {}", inner_result.explanation),
            evidence: inner_result.evidence,
        })
    }

    fn description(&self) -> String {
        format!("NOT {}", self.inner.description())
    }
}

struct ImplicationPredicate {
    antecedent: Box<dyn DomainPredicate>,
    consequent: Box<dyn DomainPredicate>,
}

#[async_trait]
impl DomainPredicate for ImplicationPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        let antecedent_result = self.antecedent.evaluate(composition, context).await?;

        // If antecedent is false, implication is true
        if !antecedent_result.value {
            return Ok(PredicateResult {
                value: true,
                confidence: antecedent_result.confidence,
                explanation: format!(
                    "IMPLIES: antecedent false, so implication is true - {}",
                    antecedent_result.explanation
                ),
                evidence: antecedent_result.evidence,
            });
        }

        // Antecedent is true, so check consequent
        let consequent_result = self.consequent.evaluate(composition, context).await?;

        Ok(PredicateResult {
            value: consequent_result.value,
            confidence: antecedent_result
                .confidence
                .min(consequent_result.confidence),
            explanation: format!(
                "IMPLIES: {} => {}",
                antecedent_result.explanation, consequent_result.explanation
            ),
            evidence: [antecedent_result.evidence, consequent_result.evidence].concat(),
        })
    }

    fn description(&self) -> String {
        format!(
            "({} => {})",
            self.antecedent.description(),
            self.consequent.description()
        )
    }
}

// Example predicates

/// Predicate that checks if an object exists in a domain
pub struct ExistsPredicate {
    /// Name of the domain to check
    domain_name: String,
    /// Type of object to look for
    object_type: String,
}

impl ExistsPredicate {
    /// Create a new exists predicate
    pub fn new(domain_name: String, object_type: String) -> Self {
        Self {
            domain_name,
            object_type,
        }
    }
}

#[async_trait]
impl DomainPredicate for ExistsPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        let domain = composition.domains.get(&self.domain_name).ok_or_else(|| {
            DomainError::NotFound(format!("Domain {} not found", self.domain_name))
        })?;

        let exists = domain.objects.values().any(|obj| {
            if let Some(obj_id) = &context.object_id {
                obj.id == *obj_id
            } else {
                // Check type
                match &obj.composition_type {
                    crate::composition_types::DomainCompositionType::Entity { entity_type } => {
                        entity_type == &self.object_type
                    }
                    _ => false,
                }
            }
        });

        Ok(PredicateResult {
            value: exists,
            confidence: 100,
            explanation: format!(
                "{} {} in {}",
                if exists { "Found" } else { "Not found" },
                self.object_type,
                self.domain_name
            ),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::Observation,
                source: self.domain_name.clone(),
                data: serde_json::json!({
                    "object_type": self.object_type,
                    "exists": exists,
                }),
                weight: 100,
            }],
        })
    }

    fn description(&self) -> String {
        format!("EXISTS {} IN {}", self.object_type, self.domain_name)
    }
}

/// Predicate that checks relationships between domains
pub struct RelationshipPredicate {
    /// Source domain of the relationship
    source_domain: String,
    /// Target domain of the relationship
    target_domain: String,
    /// Type of relationship to check
    relationship_type: String,
}

impl RelationshipPredicate {
    /// Create a new relationship predicate
    pub fn new(source_domain: String, target_domain: String, relationship_type: String) -> Self {
        Self {
            source_domain,
            target_domain,
            relationship_type,
        }
    }
}

#[async_trait]
impl DomainPredicate for RelationshipPredicate {
    async fn evaluate(
        &self,
        composition: &DomainComposition,
        _context: &PredicateContext,
    ) -> Result<PredicateResult, DomainError> {
        // Check if there are morphisms between the domains
        let source = composition
            .domains
            .get(&self.source_domain)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Source domain {} not found", self.source_domain))
            })?;

        let target = composition
            .domains
            .get(&self.target_domain)
            .ok_or_else(|| {
                DomainError::NotFound(format!("Target domain {} not found", self.target_domain))
            })?;

        // Look for cross-domain morphisms
        let has_relationship = source.morphisms.values().any(|morph| {
            // Check if morphism targets something in the target domain
            target.objects.contains_key(&morph.target)
        });

        Ok(PredicateResult {
            value: has_relationship,
            confidence: 95,
            explanation: format!(
                "{} relationship {} between {} and {}",
                if has_relationship { "Found" } else { "No" },
                self.relationship_type,
                self.source_domain,
                self.target_domain
            ),
            evidence: vec![Evidence {
                evidence_type: EvidenceType::Inference,
                source: format!("{}→{}", self.source_domain, self.target_domain),
                data: serde_json::json!({
                    "relationship_type": self.relationship_type,
                    "exists": has_relationship,
                }),
                weight: 95,
            }],
        })
    }

    fn description(&self) -> String {
        format!(
            "RELATIONSHIP {} FROM {} TO {}",
            self.relationship_type, self.source_domain, self.target_domain
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::DomainCategory;

    #[tokio::test]
    async fn test_exists_predicate() {
        let mut composition = DomainComposition::new("Test".to_string());
        let mut domain = DomainCategory::new("TestDomain".to_string());

        domain
            .add_object(crate::category::DomainObject {
                id: "order_123".to_string(),
                composition_type: crate::composition_types::DomainCompositionType::Entity {
                    entity_type: "Order".to_string(),
                },
                metadata: HashMap::new(),
            })
            .unwrap();

        composition.add_domain(domain).unwrap();

        let predicate = ExistsPredicate::new("TestDomain".to_string(), "Order".to_string());
        let context = PredicateContext::new();

        let result = predicate.evaluate(&composition, &context).await.unwrap();
        assert!(result.value);
        assert_eq!(result.confidence, 100);
    }

    #[tokio::test]
    async fn test_logical_connectives() {
        let composition = DomainComposition::new("Test".to_string());
        let context = PredicateContext::new();

        // Create test predicates that always return specific values
        struct TruePredicate;
        struct FalsePredicate;

        #[async_trait]
        impl DomainPredicate for TruePredicate {
            async fn evaluate(
                &self,
                _composition: &DomainComposition,
                _context: &PredicateContext,
            ) -> Result<PredicateResult, DomainError> {
                Ok(PredicateResult {
                    value: true,
                    confidence: 100,
                    explanation: "Always true".to_string(),
                    evidence: vec![],
                })
            }

            fn description(&self) -> String {
                "TRUE".to_string()
            }
        }

        #[async_trait]
        impl DomainPredicate for FalsePredicate {
            async fn evaluate(
                &self,
                _composition: &DomainComposition,
                _context: &PredicateContext,
            ) -> Result<PredicateResult, DomainError> {
                Ok(PredicateResult {
                    value: false,
                    confidence: 100,
                    explanation: "Always false".to_string(),
                    evidence: vec![],
                })
            }

            fn description(&self) -> String {
                "FALSE".to_string()
            }
        }

        // Test AND
        let and_pred = Box::new(TruePredicate).and(Box::new(TruePredicate));
        let result = and_pred.evaluate(&composition, &context).await.unwrap();
        assert!(result.value);

        let and_pred = Box::new(TruePredicate).and(Box::new(FalsePredicate));
        let result = and_pred.evaluate(&composition, &context).await.unwrap();
        assert!(!result.value);

        // Test OR
        let or_pred = Box::new(TruePredicate).or(Box::new(FalsePredicate));
        let result = or_pred.evaluate(&composition, &context).await.unwrap();
        assert!(result.value);

        // Test NOT
        let not_pred = Box::new(FalsePredicate).not();
        let result = not_pred.evaluate(&composition, &context).await.unwrap();
        assert!(result.value);

        // Test IMPLIES
        let implies_pred = Box::new(TruePredicate).implies(Box::new(TruePredicate));
        let result = implies_pred.evaluate(&composition, &context).await.unwrap();
        assert!(result.value);

        let implies_pred = Box::new(FalsePredicate).implies(Box::new(FalsePredicate));
        let result = implies_pred.evaluate(&composition, &context).await.unwrap();
        assert!(result.value); // False => anything is true
    }
}
