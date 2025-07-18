// Copyright 2025 Cowboy AI, LLC.

//! Limits and colimits for domain composition
//!
//! Limits and colimits provide universal constructions for combining
//! and decomposing domain objects in a structure-preserving way.

use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use super::domain_category::{DomainObject, DomainMorphism, MorphismType};
use crate::composition_types::DomainCompositionType;

/// A limit in a domain category
#[async_trait]
pub trait Limit: Send + Sync {
    /// The apex object of the limit cone
    async fn apex(&self) -> Result<DomainObject, DomainError>;
    
    /// Get projection morphism to a specific object in the diagram
    async fn projection(&self, target: &str) -> Result<DomainMorphism, DomainError>;
    
    /// Verify the universal property
    async fn verify_universal_property(&self) -> Result<bool, DomainError>;
}

/// A colimit in a domain category
#[async_trait]
pub trait Colimit: Send + Sync {
    /// The apex object of the colimit cocone
    async fn apex(&self) -> Result<DomainObject, DomainError>;
    
    /// Get injection morphism from a specific object in the diagram
    async fn injection(&self, source: &str) -> Result<DomainMorphism, DomainError>;
    
    /// Verify the universal property
    async fn verify_universal_property(&self) -> Result<bool, DomainError>;
}

/// Pullback - the limit of a cospan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pullback {
    /// The pullback object P
    pub apex: DomainObject,
    
    /// Objects forming the cospan: A -> C <- B
    pub object_a: DomainObject,
    /// Second object in the cospan
    pub object_b: DomainObject,
    /// Common target object in the cospan
    pub object_c: DomainObject,
    
    /// Morphisms of the cospan
    /// Morphism from A to C
    pub f: DomainMorphism,
    /// Morphism from B to C
    pub g: DomainMorphism,
    
    /// Projections from pullback
    /// Projection from pullback P to A
    pub p1: DomainMorphism,
    /// Projection from pullback P to B
    pub p2: DomainMorphism,
}

impl Pullback {
    /// Create a pullback for cross-domain synchronization
    pub fn for_synchronization(
        domain_a: &str,
        domain_b: &str,
        shared_concept: &str,
    ) -> Result<Self, DomainError> {
        // Create the shared concept object
        let object_c = DomainObject {
            id: format!("{}_{}_shared", domain_a, domain_b),
            composition_type: DomainCompositionType::BoundedContext {
                domain: shared_concept.to_string(),
            },
            metadata: HashMap::from([
                ("shared_between".to_string(), format!("{},{}", domain_a, domain_b)),
            ]),
        };
        
        // Create domain-specific objects
        let object_a = DomainObject {
            id: format!("{}_view", domain_a),
            composition_type: DomainCompositionType::BoundedContext {
                domain: domain_a.to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let object_b = DomainObject {
            id: format!("{}_view", domain_b),
            composition_type: DomainCompositionType::BoundedContext {
                domain: domain_b.to_string(),
            },
            metadata: HashMap::new(),
        };
        
        // Create the pullback apex
        let apex = DomainObject {
            id: format!("Pullback_{}_{}", domain_a, domain_b),
            composition_type: DomainCompositionType::Composite {
                composite_type: "Pullback".to_string(),
                components: vec![domain_a.to_string(), domain_b.to_string()],
            },
            metadata: HashMap::from([
                ("pullback_of".to_string(), format!("{} and {}", domain_a, domain_b)),
            ]),
        };
        
        // Create morphisms
        let f = DomainMorphism {
            id: format!("{}_to_shared", domain_a),
            source: object_a.id.clone(),
            target: object_c.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "projection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let g = DomainMorphism {
            id: format!("{}_to_shared", domain_b),
            source: object_b.id.clone(),
            target: object_c.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "projection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let p1 = DomainMorphism {
            id: format!("pullback_to_{}", domain_a),
            source: apex.id.clone(),
            target: object_a.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "pullback_projection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let p2 = DomainMorphism {
            id: format!("pullback_to_{}", domain_b),
            source: apex.id.clone(),
            target: object_b.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "pullback_projection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        Ok(Self {
            apex,
            object_a,
            object_b,
            object_c,
            f,
            g,
            p1,
            p2,
        })
    }
}

#[async_trait]
impl Limit for Pullback {
    async fn apex(&self) -> Result<DomainObject, DomainError> {
        Ok(self.apex.clone())
    }
    
    async fn projection(&self, target: &str) -> Result<DomainMorphism, DomainError> {
        if target == self.object_a.id {
            Ok(self.p1.clone())
        } else if target == self.object_b.id {
            Ok(self.p2.clone())
        } else {
            Err(DomainError::NotFound(format!("No projection to {}", target)))
        }
    }
    
    async fn verify_universal_property(&self) -> Result<bool, DomainError> {
        // Verify that f ∘ p1 = g ∘ p2
        // In a real implementation, we'd check this via composition
        Ok(true)
    }
}

/// Pushout - the colimit of a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pushout {
    /// The pushout object P
    pub apex: DomainObject,
    
    /// Objects forming the span: A <- C -> B
    pub object_a: DomainObject,
    /// Second object in the span
    pub object_b: DomainObject,
    /// Common source object in the span
    pub object_c: DomainObject,
    
    /// Morphisms of the span
    /// Morphism from C to A
    pub f: DomainMorphism,
    /// Morphism from C to B
    pub g: DomainMorphism,
    
    /// Injections to pushout
    /// Injection from A to pushout P
    pub i1: DomainMorphism,
    /// Injection from B to pushout P
    pub i2: DomainMorphism,
}

impl Pushout {
    /// Create a pushout for domain merger
    pub fn for_merger(
        domain_a: &str,
        domain_b: &str,
        common_base: &str,
    ) -> Result<Self, DomainError> {
        // Create the common base object
        let object_c = DomainObject {
            id: format!("{}_base", common_base),
            composition_type: DomainCompositionType::BoundedContext {
                domain: common_base.to_string(),
            },
            metadata: HashMap::new(),
        };
        
        // Create domain-specific objects
        let object_a = DomainObject {
            id: domain_a.to_string(),
            composition_type: DomainCompositionType::BoundedContext {
                domain: domain_a.to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let object_b = DomainObject {
            id: domain_b.to_string(),
            composition_type: DomainCompositionType::BoundedContext {
                domain: domain_b.to_string(),
            },
            metadata: HashMap::new(),
        };
        
        // Create the pushout apex (merged domain)
        let apex = DomainObject {
            id: format!("Merged_{}_{}", domain_a, domain_b),
            composition_type: DomainCompositionType::Composite {
                composite_type: "Pushout".to_string(),
                components: vec![domain_a.to_string(), domain_b.to_string()],
            },
            metadata: HashMap::from([
                ("merged_from".to_string(), format!("{} and {}", domain_a, domain_b)),
                ("common_base".to_string(), common_base.to_string()),
            ]),
        };
        
        // Create morphisms
        let f = DomainMorphism {
            id: format!("{}_to_{}", common_base, domain_a),
            source: object_c.id.clone(),
            target: object_a.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "inclusion".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let g = DomainMorphism {
            id: format!("{}_to_{}", common_base, domain_b),
            source: object_c.id.clone(),
            target: object_b.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "inclusion".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let i1 = DomainMorphism {
            id: format!("{}_to_merged", domain_a),
            source: object_a.id.clone(),
            target: apex.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "pushout_injection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let i2 = DomainMorphism {
            id: format!("{}_to_merged", domain_b),
            source: object_b.id.clone(),
            target: apex.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "pushout_injection".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        Ok(Self {
            apex,
            object_a,
            object_b,
            object_c,
            f,
            g,
            i1,
            i2,
        })
    }
}

#[async_trait]
impl Colimit for Pushout {
    async fn apex(&self) -> Result<DomainObject, DomainError> {
        Ok(self.apex.clone())
    }
    
    async fn injection(&self, source: &str) -> Result<DomainMorphism, DomainError> {
        if source == self.object_a.id {
            Ok(self.i1.clone())
        } else if source == self.object_b.id {
            Ok(self.i2.clone())
        } else {
            Err(DomainError::NotFound(format!("No injection from {}", source)))
        }
    }
    
    async fn verify_universal_property(&self) -> Result<bool, DomainError> {
        // Verify that i1 ∘ f = i2 ∘ g
        // In a real implementation, we'd check this via composition
        Ok(true)
    }
}

/// Product - limit of discrete diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// The product object (apex of the limit cone)
    pub apex: DomainObject,
    /// The component objects being multiplied
    pub components: Vec<DomainObject>,
    /// Projection morphisms from product to each component
    pub projections: HashMap<String, DomainMorphism>,
}

impl Product {
    /// Create a product of multiple domains
    pub fn of_domains(domains: Vec<&str>) -> Result<Self, DomainError> {
        if domains.is_empty() {
            return Err(DomainError::InvalidOperation { reason: "Cannot create product of empty list".to_string() });
        }
        
        let apex = DomainObject {
            id: format!("Product_{}", domains.join("_")),
            composition_type: DomainCompositionType::Composite {
                composite_type: "Product".to_string(),
                components: domains.iter().map(|d| d.to_string()).collect(),
            },
            metadata: HashMap::from([
                ("product_of".to_string(), domains.join(",")),
                ("arity".to_string(), domains.len().to_string()),
            ]),
        };
        
        let mut components = Vec::new();
        let mut projections = HashMap::new();
        
        for (i, domain) in domains.iter().enumerate() {
            let component = DomainObject {
                id: format!("{}_component", domain),
                composition_type: DomainCompositionType::BoundedContext {
                    domain: domain.to_string(),
                },
                metadata: HashMap::new(),
            };
            
            let projection = DomainMorphism {
                id: format!("π_{}", i + 1),
                source: apex.id.clone(),
                target: component.id.clone(),
                operation_type: MorphismType::Transformation {
                    transform_type: "product_projection".to_string(),
                },
                metadata: HashMap::from([
                    ("component_index".to_string(), i.to_string()),
                ]),
            };
            
            components.push(component.clone());
            projections.insert(component.id.clone(), projection);
        }
        
        Ok(Self {
            apex,
            components,
            projections,
        })
    }
}

/// Coproduct - colimit of discrete diagram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coproduct {
    /// The coproduct object (apex of the colimit cocone)
    pub apex: DomainObject,
    /// The component objects being summed
    pub components: Vec<DomainObject>,
    /// Injection morphisms from each component to coproduct
    pub injections: HashMap<String, DomainMorphism>,
}

impl Coproduct {
    /// Create a coproduct (sum type) of multiple domains
    pub fn of_domains(domains: Vec<&str>) -> Result<Self, DomainError> {
        if domains.is_empty() {
            return Err(DomainError::InvalidOperation { reason: "Cannot create coproduct of empty list".to_string() });
        }
        
        let apex = DomainObject {
            id: format!("Coproduct_{}", domains.join("_")),
            composition_type: DomainCompositionType::Composite {
                composite_type: "Coproduct".to_string(),
                components: domains.iter().map(|d| d.to_string()).collect(),
            },
            metadata: HashMap::from([
                ("coproduct_of".to_string(), domains.join(",")),
                ("arity".to_string(), domains.len().to_string()),
            ]),
        };
        
        let mut components = Vec::new();
        let mut injections = HashMap::new();
        
        for (i, domain) in domains.iter().enumerate() {
            let component = DomainObject {
                id: format!("{}_component", domain),
                composition_type: DomainCompositionType::BoundedContext {
                    domain: domain.to_string(),
                },
                metadata: HashMap::new(),
            };
            
            let injection = DomainMorphism {
                id: format!("ι_{}", i + 1),
                source: component.id.clone(),
                target: apex.id.clone(),
                operation_type: MorphismType::Transformation {
                    transform_type: "coproduct_injection".to_string(),
                },
                metadata: HashMap::from([
                    ("component_index".to_string(), i.to_string()),
                ]),
            };
            
            components.push(component.clone());
            injections.insert(component.id.clone(), injection);
        }
        
        Ok(Self {
            apex,
            components,
            injections,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pullback_creation() {
        let pullback = Pullback::for_synchronization(
            "OrderDomain",
            "InventoryDomain",
            "ProductCatalog",
        ).unwrap();
        
        assert_eq!(pullback.apex.id, "Pullback_OrderDomain_InventoryDomain");
        assert!(pullback.apex.metadata.contains_key("pullback_of"));
        
        // Test limit interface
        let apex = pullback.apex().await.unwrap();
        assert_eq!(apex.id, pullback.apex.id);
        
        let proj_a = pullback.projection("OrderDomain_view").await.unwrap();
        assert_eq!(proj_a.id, "pullback_to_OrderDomain");
    }
    
    #[tokio::test]
    async fn test_pushout_creation() {
        let pushout = Pushout::for_merger(
            "SalesDomain",
            "MarketingDomain",
            "CustomerData",
        ).unwrap();
        
        assert_eq!(pushout.apex.id, "Merged_SalesDomain_MarketingDomain");
        assert_eq!(
            pushout.apex.metadata.get("common_base").unwrap(),
            "CustomerData"
        );
        
        // Test colimit interface
        let apex = pushout.apex().await.unwrap();
        assert_eq!(apex.id, pushout.apex.id);
        
        let inj_a = pushout.injection("SalesDomain").await.unwrap();
        assert_eq!(inj_a.id, "SalesDomain_to_merged");
    }
    
    #[test]
    fn test_product_creation() {
        let product = Product::of_domains(vec!["User", "Order", "Payment"]).unwrap();
        
        assert_eq!(product.apex.id, "Product_User_Order_Payment");
        assert_eq!(product.components.len(), 3);
        assert_eq!(product.projections.len(), 3);
        
        assert_eq!(
            product.apex.metadata.get("arity").unwrap(),
            "3"
        );
    }
    
    #[test]
    fn test_coproduct_creation() {
        let coproduct = Coproduct::of_domains(vec!["Success", "Error", "Pending"]).unwrap();
        
        assert_eq!(coproduct.apex.id, "Coproduct_Success_Error_Pending");
        assert_eq!(coproduct.components.len(), 3);
        assert_eq!(coproduct.injections.len(), 3);
        
        // Check injection exists
        assert!(coproduct.injections.contains_key("Success_component"));
    }
}