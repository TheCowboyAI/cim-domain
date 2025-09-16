// Copyright 2025 Cowboy AI, LLC.

//! Domain composition using functors and natural transformations
//!
//! This module implements the composition of multiple domains into
//! coherent larger structures using category theory principles.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::category::limits::{Coproduct, Product, Pullback, Pushout};
use crate::category::DomainCategory;
use crate::errors::DomainError;
use crate::events::DomainEvent;
// Integration removed - infrastructure concern
// use crate::integration::domain_bridge::SerializedCommand;

/// A composition of multiple domains
#[derive(Debug, Serialize, Deserialize)]
pub struct DomainComposition {
    /// Unique identifier
    pub id: Uuid,

    /// Name of the composition
    pub name: String,

    /// Participating domains
    pub domains: HashMap<String, DomainCategory>,

    /// Functors between domains (stored as JSON for serialization)
    #[serde(skip)]
    pub functors: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,

    /// Natural transformations between functors (stored as JSON for serialization)
    #[serde(skip)]
    pub transformations: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,

    /// Shared objects (limits/colimits)
    pub shared_structures: HashMap<String, SharedStructure>,

    /// Composition metadata
    pub metadata: HashMap<String, String>,
}

/// Shared structures in domain composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharedStructure {
    /// Pullback for synchronization
    Pullback(Pullback),

    /// Pushout for merger
    Pushout(Pushout),

    /// Product for parallel composition
    Product(Product),

    /// Coproduct for choice
    Coproduct(Coproduct),

    /// Custom limit/colimit
    Custom {
        /// Type name of the custom structure
        structure_type: String,
        /// Additional data for the custom structure
        data: HashMap<String, serde_json::Value>,
    },
}

/// Strategy for composing domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompositionStrategy {
    /// Sequential composition (Kleisli)
    Sequential {
        /// Order of domain execution
        order: Vec<String>,
    },

    /// Parallel composition (Product)
    Parallel {
        /// Domains to run in parallel
        domains: Vec<String>,
    },

    /// Synchronized composition (Pullback)
    Synchronized {
        /// Domains to synchronize
        domains: Vec<String>,
        /// Shared concept
        shared_concept: String,
    },

    /// Merged composition (Pushout)
    Merged {
        /// Domains to merge
        domains: Vec<String>,
        /// Common base
        common_base: String,
    },

    /// Choice composition (Coproduct)
    Choice {
        /// Available domain choices
        options: Vec<String>,
    },
}

impl DomainComposition {
    /// Create a new domain composition
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            domains: HashMap::new(),
            functors: HashMap::new(),
            transformations: HashMap::new(),
            shared_structures: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a domain to the composition
    pub fn add_domain(&mut self, domain: DomainCategory) -> Result<(), DomainError> {
        if self.domains.contains_key(&domain.name) {
            return Err(DomainError::AlreadyExists(format!(
                "Domain {} already in composition",
                domain.name
            )));
        }

        self.domains.insert(domain.name.clone(), domain);
        Ok(())
    }

    /// Create a synchronized composition using pullback
    pub fn synchronize_domains(
        &mut self,
        domain_a: &str,
        domain_b: &str,
        shared_concept: &str,
    ) -> Result<String, DomainError> {
        // Verify domains exist
        if !self.domains.contains_key(domain_a) {
            return Err(DomainError::NotFound(format!(
                "Domain {domain_a} not found"
            )));
        }
        if !self.domains.contains_key(domain_b) {
            return Err(DomainError::NotFound(format!(
                "Domain {domain_b} not found"
            )));
        }

        // Create pullback
        let pullback = Pullback::for_synchronization(domain_a, domain_b, shared_concept)?;
        let structure_id = format!("sync_{domain_a}_{domain_b}");

        self.shared_structures
            .insert(structure_id.clone(), SharedStructure::Pullback(pullback));

        Ok(structure_id)
    }

    /// Create a merged composition using pushout
    pub fn merge_domains(
        &mut self,
        domain_a: &str,
        domain_b: &str,
        common_base: &str,
    ) -> Result<String, DomainError> {
        // Verify domains exist
        if !self.domains.contains_key(domain_a) {
            return Err(DomainError::NotFound(format!(
                "Domain {domain_a} not found"
            )));
        }
        if !self.domains.contains_key(domain_b) {
            return Err(DomainError::NotFound(format!(
                "Domain {domain_b} not found"
            )));
        }

        // Create pushout
        let pushout = Pushout::for_merger(domain_a, domain_b, common_base)?;
        let structure_id = format!("merge_{domain_a}_{domain_b}");

        self.shared_structures
            .insert(structure_id.clone(), SharedStructure::Pushout(pushout));

        Ok(structure_id)
    }

    /// Create a parallel composition using product
    pub fn parallel_composition(&mut self, domains: Vec<&str>) -> Result<String, DomainError> {
        // Verify all domains exist
        for domain in &domains {
            if !self.domains.contains_key(*domain) {
                return Err(DomainError::NotFound(format!("Domain {domain} not found")));
            }
        }

        // Create product
        let product = Product::of_domains(domains.clone())?;
        let structure_id = format!("parallel_{}", domains.join("_"));

        self.shared_structures
            .insert(structure_id.clone(), SharedStructure::Product(product));

        Ok(structure_id)
    }

    /// Create a choice composition using coproduct
    pub fn choice_composition(&mut self, options: Vec<&str>) -> Result<String, DomainError> {
        // Verify all domains exist
        for domain in &options {
            if !self.domains.contains_key(*domain) {
                return Err(DomainError::NotFound(format!("Domain {domain} not found")));
            }
        }

        // Create coproduct
        let coproduct = Coproduct::of_domains(options.clone())?;
        let structure_id = format!("choice_{}", options.join("_"));

        self.shared_structures
            .insert(structure_id.clone(), SharedStructure::Coproduct(coproduct));

        Ok(structure_id)
    }

    /// Apply a composition strategy
    pub async fn apply_strategy(
        &mut self,
        strategy: CompositionStrategy,
    ) -> Result<String, DomainError> {
        match strategy {
            CompositionStrategy::Sequential { order } => {
                // Create a sequential composition
                // This would chain domains using Kleisli composition
                let structure_id = format!("seq_{}", order.join("_"));
                Ok(structure_id)
            }

            CompositionStrategy::Parallel { domains } => {
                self.parallel_composition(domains.iter().map(|s| s.as_str()).collect())
            }

            CompositionStrategy::Synchronized {
                domains,
                shared_concept,
            } => {
                if domains.len() != 2 {
                    return Err(DomainError::InvalidOperation {
                        reason: "Synchronized composition requires exactly 2 domains".to_string(),
                    });
                }
                self.synchronize_domains(&domains[0], &domains[1], &shared_concept)
            }

            CompositionStrategy::Merged {
                domains,
                common_base,
            } => {
                if domains.len() != 2 {
                    return Err(DomainError::InvalidOperation {
                        reason: "Merged composition requires exactly 2 domains".to_string(),
                    });
                }
                self.merge_domains(&domains[0], &domains[1], &common_base)
            }

            CompositionStrategy::Choice { options } => {
                self.choice_composition(options.iter().map(|s| s.as_str()).collect())
            }
        }
    }

    /// Route a command to the appropriate domain
    // Command routing removed - infrastructure concern
    // Commands should be handled at the application/infrastructure layer
    pub async fn route_command(
        &self,
        _command: serde_json::Value, // Placeholder - routing belongs in infrastructure
    ) -> Result<Vec<Box<dyn DomainEvent>>, DomainError> {
        // In a real implementation, this would:
        // 1. Determine target domain from command metadata
        // 2. Apply any necessary functors/transformations
        // 3. Execute in target domain
        // 4. Transform results back

        Err(DomainError::NotImplemented("Command routing".to_string()))
    }

    /// Execute a cross-domain query
    pub async fn cross_domain_query(
        &self,
        _query_type: &str,
        _domains: Vec<&str>,
    ) -> Result<HashMap<String, serde_json::Value>, DomainError> {
        // In a real implementation, this would:
        // 1. Decompose query into domain-specific parts
        // 2. Execute in parallel across domains
        // 3. Combine results using appropriate limit/colimit

        Err(DomainError::NotImplemented(
            "Cross-domain query".to_string(),
        ))
    }
}

// FP-style helpers (persistent updates)
/// Create a new domain composition (alias for `DomainComposition::new`).
pub fn composition(name: impl Into<String>) -> DomainComposition {
    DomainComposition::new(name.into())
}

/// Return a new composition with the domain added.
pub fn with_domain(
    mut c: DomainComposition,
    domain: DomainCategory,
) -> Result<DomainComposition, DomainError> {
    c.add_domain(domain)?;
    Ok(c)
}

/// Return a new composition with metadata set.
pub fn with_metadata(
    mut c: DomainComposition,
    key: impl Into<String>,
    value: impl Into<String>,
) -> DomainComposition {
    c.metadata.insert(key.into(), value.into());
    c
}

/// Return a new composition plus the id of the created pullback.
pub fn with_sync(
    mut c: DomainComposition,
    a: &str,
    b: &str,
    shared: &str,
) -> Result<(DomainComposition, String), DomainError> {
    let id = c.synchronize_domains(a, b, shared)?;
    Ok((c, id))
}

/// Return a new composition plus the id of the created product.
pub fn with_parallel(
    mut c: DomainComposition,
    domains: Vec<&str>,
) -> Result<(DomainComposition, String), DomainError> {
    let id = c.parallel_composition(domains)?;
    Ok((c, id))
}

/// Return a new composition plus the id of the created pushout.
pub fn with_merge(
    mut c: DomainComposition,
    a: &str,
    b: &str,
    base: &str,
) -> Result<(DomainComposition, String), DomainError> {
    let id = c.merge_domains(a, b, base)?;
    Ok((c, id))
}

/// Return a new composition plus the id of the created coproduct.
pub fn with_choice(
    mut c: DomainComposition,
    options: Vec<&str>,
) -> Result<(DomainComposition, String), DomainError> {
    let id = c.choice_composition(options)?;
    Ok((c, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composition_fp_helpers() {
        let domain_a = DomainCategory::new("DomainA".to_string());
        let domain_b = DomainCategory::new("DomainB".to_string());
        let domain_c = DomainCategory::new("DomainC".to_string());

        let mut comp = composition("CompositionX");
        comp = with_domain(comp, domain_a).unwrap();
        comp = with_domain(comp, domain_b).unwrap();
        comp = with_domain(comp, domain_c).unwrap();
        comp = with_metadata(comp, "version", "1.0");
        let (comp, _sync_id) = with_sync(comp, "DomainA", "DomainB", "SharedConcept").unwrap();
        let (comp, _par_id) = with_parallel(comp, vec!["DomainA", "DomainC"]).unwrap();

        assert_eq!(comp.domains.len(), 3);
        assert_eq!(comp.shared_structures.len(), 2);
        assert_eq!(comp.metadata.get("version").unwrap(), "1.0");
    }

    #[tokio::test]
    async fn test_composition_strategies() {
        let mut composition = DomainComposition::new("Test".to_string());

        composition
            .add_domain(DomainCategory::new("A".to_string()))
            .unwrap();
        composition
            .add_domain(DomainCategory::new("B".to_string()))
            .unwrap();
        composition
            .add_domain(DomainCategory::new("C".to_string()))
            .unwrap();

        // Test parallel strategy
        let parallel_id = composition
            .apply_strategy(CompositionStrategy::Parallel {
                domains: vec!["A".to_string(), "B".to_string()],
            })
            .await
            .unwrap();

        assert!(composition.shared_structures.contains_key(&parallel_id));

        // Test synchronized strategy
        let sync_id = composition
            .apply_strategy(CompositionStrategy::Synchronized {
                domains: vec!["B".to_string(), "C".to_string()],
                shared_concept: "SharedData".to_string(),
            })
            .await
            .unwrap();

        assert!(composition.shared_structures.contains_key(&sync_id));

        // Test choice strategy
        let choice_id = composition
            .apply_strategy(CompositionStrategy::Choice {
                options: vec!["A".to_string(), "B".to_string(), "C".to_string()],
            })
            .await
            .unwrap();

        assert!(composition.shared_structures.contains_key(&choice_id));
    }
}
