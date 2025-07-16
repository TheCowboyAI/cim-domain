//! Domain invariants that must be maintained across boundaries
//!
//! Invariants represent business rules that must always be true,
//! regardless of which domain operations are performed.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::errors::DomainError;
use crate::composition::DomainComposition;
use crate::category::{DomainObject, DomainCategory};

/// A domain invariant that must be maintained
#[async_trait]
pub trait DomainInvariant: Send + Sync {
    /// Name of the invariant
    fn name(&self) -> &str;
    
    /// Description of what this invariant ensures
    fn description(&self) -> &str;
    
    /// Check if the invariant holds for a given composition
    async fn check(
        &self,
        composition: &DomainComposition,
    ) -> Result<InvariantCheckResult, DomainError>;
    
    /// Get the domains this invariant spans
    fn affected_domains(&self) -> Vec<String>;
}

/// Result of checking an invariant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCheckResult {
    /// Whether the invariant holds
    pub satisfied: bool,
    
    /// Violations found (if any)
    pub violations: Vec<InvariantViolation>,
    
    /// Timestamp of the check
    pub checked_at: DateTime<Utc>,
    
    /// Additional context
    pub context: HashMap<String, String>,
}

/// A violation of an invariant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantViolation {
    /// Which invariant was violated
    pub invariant_name: String,
    
    /// Where the violation occurred
    pub location: ViolationLocation,
    
    /// Description of the violation
    pub message: String,
    
    /// Severity of the violation
    pub severity: ViolationSeverity,
    
    /// Suggested remediation
    pub remediation: Option<String>,
}

/// Location where a violation occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationLocation {
    /// In a specific domain
    Domain { name: String },
    
    /// In a specific object
    Object { domain: String, object_id: String },
    
    /// In a morphism between objects
    Morphism { domain: String, morphism_id: String },
    
    /// In a cross-domain interaction
    CrossDomain { from: String, to: String },
}

/// Severity of an invariant violation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    /// Informational - not critical
    Info,
    
    /// Warning - should be addressed
    Warning,
    
    /// Error - must be fixed
    Error,
    
    /// Critical - system integrity at risk
    Critical,
}

/// Checker for multiple invariants
pub struct InvariantChecker {
    /// Registered invariants
    invariants: Vec<Box<dyn DomainInvariant>>,
    
    /// Check history
    history: Vec<InvariantCheckResult>,
}

impl InvariantChecker {
    /// Create a new invariant checker
    pub fn new() -> Self {
        Self {
            invariants: Vec::new(),
            history: Vec::new(),
        }
    }
    
    /// Register an invariant
    pub fn register(&mut self, invariant: Box<dyn DomainInvariant>) {
        self.invariants.push(invariant);
    }
    
    /// Check all invariants
    pub async fn check_all(
        &mut self,
        composition: &DomainComposition,
    ) -> Result<Vec<InvariantCheckResult>, DomainError> {
        let mut results = Vec::new();
        
        for invariant in &self.invariants {
            let result = invariant.check(composition).await?;
            self.history.push(result.clone());
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Get violations above a certain severity
    pub fn get_violations(
        &self,
        min_severity: ViolationSeverity,
    ) -> Vec<&InvariantViolation> {
        self.history.iter()
            .flat_map(|result| &result.violations)
            .filter(|v| v.severity >= min_severity)
            .collect()
    }
    
    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// Example: Referential integrity invariant
pub struct ReferentialIntegrityInvariant {
    name: String,
    source_domain: String,
    target_domain: String,
    reference_field: String,
}

impl ReferentialIntegrityInvariant {
    pub fn new(
        source_domain: String,
        target_domain: String,
        reference_field: String,
    ) -> Self {
        let name = format!(
            "ref_integrity_{}_{}_{}",
            source_domain, target_domain, reference_field
        );
        
        Self {
            name,
            source_domain,
            target_domain,
            reference_field,
        }
    }
}

#[async_trait]
impl DomainInvariant for ReferentialIntegrityInvariant {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "Ensures all references point to existing objects"
    }
    
    async fn check(
        &self,
        composition: &DomainComposition,
    ) -> Result<InvariantCheckResult, DomainError> {
        let mut violations = Vec::new();
        
        // Get source and target domains
        let source = composition.domains.get(&self.source_domain)
            .ok_or_else(|| DomainError::NotFound(
                format!("Source domain {} not found", self.source_domain)
            ))?;
        
        let target = composition.domains.get(&self.target_domain)
            .ok_or_else(|| DomainError::NotFound(
                format!("Target domain {} not found", self.target_domain)
            ))?;
        
        // Check all objects in source domain
        for (obj_id, obj) in &source.objects {
            if let Some(ref_id) = obj.metadata.get(&self.reference_field) {
                // Check if referenced object exists in target
                if !target.objects.contains_key(ref_id) {
                    violations.push(InvariantViolation {
                        invariant_name: self.name.clone(),
                        location: ViolationLocation::Object {
                            domain: self.source_domain.clone(),
                            object_id: obj_id.clone(),
                        },
                        message: format!(
                            "Object {} references non-existent {} in {}",
                            obj_id, ref_id, self.target_domain
                        ),
                        severity: ViolationSeverity::Error,
                        remediation: Some(format!(
                            "Create {} in {} or update reference",
                            ref_id, self.target_domain
                        )),
                    });
                }
            }
        }
        
        Ok(InvariantCheckResult {
            satisfied: violations.is_empty(),
            violations,
            checked_at: Utc::now(),
            context: HashMap::from([
                ("source_domain".to_string(), self.source_domain.clone()),
                ("target_domain".to_string(), self.target_domain.clone()),
                ("reference_field".to_string(), self.reference_field.clone()),
            ]),
        })
    }
    
    fn affected_domains(&self) -> Vec<String> {
        vec![self.source_domain.clone(), self.target_domain.clone()]
    }
}

/// Example: Consistency invariant for distributed aggregates
pub struct DistributedConsistencyInvariant {
    name: String,
    domains: Vec<String>,
    consistency_rule: String,
}

impl DistributedConsistencyInvariant {
    pub fn new(domains: Vec<String>, consistency_rule: String) -> Self {
        let name = format!("consistency_{}", domains.join("_"));
        Self {
            name,
            domains,
            consistency_rule,
        }
    }
}

#[async_trait]
impl DomainInvariant for DistributedConsistencyInvariant {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "Ensures consistency across distributed aggregates"
    }
    
    async fn check(
        &self,
        composition: &DomainComposition,
    ) -> Result<InvariantCheckResult, DomainError> {
        // In a real implementation, this would check specific consistency rules
        // For example: total inventory across warehouses equals sum of individual inventories
        
        Ok(InvariantCheckResult {
            satisfied: true,
            violations: vec![],
            checked_at: Utc::now(),
            context: HashMap::from([
                ("domains".to_string(), self.domains.join(",")),
                ("rule".to_string(), self.consistency_rule.clone()),
            ]),
        })
    }
    
    fn affected_domains(&self) -> Vec<String> {
        self.domains.clone()
    }
}

/// Example: Business constraint invariant
pub struct BusinessConstraintInvariant {
    name: String,
    constraint: Box<dyn Fn(&DomainComposition) -> bool + Send + Sync>,
    description: String,
    affected_domains: Vec<String>,
}

impl BusinessConstraintInvariant {
    pub fn new<F>(
        name: String,
        constraint: F,
        description: String,
        affected_domains: Vec<String>,
    ) -> Self
    where
        F: Fn(&DomainComposition) -> bool + Send + Sync + 'static,
    {
        Self {
            name,
            constraint: Box::new(constraint),
            description,
            affected_domains,
        }
    }
}

#[async_trait]
impl DomainInvariant for BusinessConstraintInvariant {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        &self.description
    }
    
    async fn check(
        &self,
        composition: &DomainComposition,
    ) -> Result<InvariantCheckResult, DomainError> {
        let satisfied = (self.constraint)(composition);
        
        let violations = if !satisfied {
            vec![InvariantViolation {
                invariant_name: self.name.clone(),
                location: ViolationLocation::CrossDomain {
                    from: self.affected_domains.first().cloned().unwrap_or_default(),
                    to: self.affected_domains.last().cloned().unwrap_or_default(),
                },
                message: format!("Business constraint '{}' violated", self.name),
                severity: ViolationSeverity::Error,
                remediation: None,
            }]
        } else {
            vec![]
        };
        
        Ok(InvariantCheckResult {
            satisfied,
            violations,
            checked_at: Utc::now(),
            context: HashMap::new(),
        })
    }
    
    fn affected_domains(&self) -> Vec<String> {
        self.affected_domains.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_referential_integrity() {
        let mut composition = DomainComposition::new("Test".to_string());
        
        // Create domains
        let mut order_domain = DomainCategory::new("OrderDomain".to_string());
        let mut customer_domain = DomainCategory::new("CustomerDomain".to_string());
        
        // Add customer
        customer_domain.add_object(DomainObject {
            id: "customer_123".to_string(),
            composition_type: crate::composition_types::DomainCompositionType::Entity {
                entity_type: "Customer".to_string(),
            },
            metadata: HashMap::new(),
        }).unwrap();
        
        // Add order with valid reference
        let mut order_metadata = HashMap::new();
        order_metadata.insert("customer_id".to_string(), "customer_123".to_string());
        
        order_domain.add_object(DomainObject {
            id: "order_456".to_string(),
            composition_type: crate::composition_types::DomainCompositionType::Entity {
                entity_type: "Order".to_string(),
            },
            metadata: order_metadata,
        }).unwrap();
        
        // Add order with invalid reference
        let mut invalid_metadata = HashMap::new();
        invalid_metadata.insert("customer_id".to_string(), "customer_999".to_string());
        
        order_domain.add_object(DomainObject {
            id: "order_789".to_string(),
            composition_type: crate::composition_types::DomainCompositionType::Entity {
                entity_type: "Order".to_string(),
            },
            metadata: invalid_metadata,
        }).unwrap();
        
        composition.add_domain(order_domain).unwrap();
        composition.add_domain(customer_domain).unwrap();
        
        // Check invariant
        let invariant = ReferentialIntegrityInvariant::new(
            "OrderDomain".to_string(),
            "CustomerDomain".to_string(),
            "customer_id".to_string(),
        );
        
        let result = invariant.check(&composition).await.unwrap();
        
        assert!(!result.satisfied);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].severity, ViolationSeverity::Error);
    }
    
    #[tokio::test]
    async fn test_invariant_checker() {
        let mut composition = DomainComposition::new("Test".to_string());
        composition.add_domain(DomainCategory::new("Domain1".to_string())).unwrap();
        composition.add_domain(DomainCategory::new("Domain2".to_string())).unwrap();
        
        let mut checker = InvariantChecker::new();
        
        // Add business constraint
        checker.register(Box::new(BusinessConstraintInvariant::new(
            "test_constraint".to_string(),
            |comp| comp.domains.len() <= 3,
            "No more than 3 domains allowed".to_string(),
            vec!["Domain1".to_string(), "Domain2".to_string()],
        )));
        
        let results = checker.check_all(&composition).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert!(results[0].satisfied);
        
        // Add another domain to violate constraint
        composition.add_domain(DomainCategory::new("Domain3".to_string())).unwrap();
        composition.add_domain(DomainCategory::new("Domain4".to_string())).unwrap();
        
        let results = checker.check_all(&composition).await.unwrap();
        assert!(!results[0].satisfied);
    }
}