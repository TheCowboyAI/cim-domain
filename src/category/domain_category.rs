//! Domain categories - representing domains as categories
//!
//! Each domain in CIM is modeled as a category where:
//! - Objects are domain concepts (entities, value objects, aggregates)
//! - Morphisms are domain operations (commands, events, queries)
//! - Composition is operation chaining
//! - Identity is the no-op operation

use std::collections::HashMap;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::composition_types::DomainCompositionType;
use crate::errors::DomainError;

/// Represents a domain as a category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCategory {
    /// Unique identifier for this domain category
    pub id: Uuid,
    
    /// Name of the domain (e.g., "Graph", "Agent", "Workflow")
    pub name: String,
    
    /// Objects in this category (domain concepts)
    pub objects: HashMap<String, DomainObject>,
    
    /// Morphisms in this category (domain operations)
    pub morphisms: HashMap<String, DomainMorphism>,
    
    /// Composition table for morphisms
    pub composition_table: HashMap<(String, String), String>,
    
    /// Identity morphisms for each object
    pub identities: HashMap<String, String>,
}

/// An object in a domain category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainObject {
    /// Unique identifier within the domain
    pub id: String,
    
    /// Type of domain concept
    pub composition_type: DomainCompositionType,
    
    /// Metadata about the object
    pub metadata: HashMap<String, String>,
}

/// A morphism in a domain category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMorphism {
    /// Unique identifier within the domain
    pub id: String,
    
    /// Source object
    pub source: String,
    
    /// Target object
    pub target: String,
    
    /// Type of operation (Command, Event, Query)
    pub operation_type: MorphismType,
    
    /// Metadata about the morphism
    pub metadata: HashMap<String, String>,
}

/// Types of morphisms in domain categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MorphismType {
    /// Command - request to change state
    Command { 
        /// The specific type/name of the command
        command_type: String 
    },
    
    /// Event - notification of state change
    Event { 
        /// The specific type/name of the event
        event_type: String 
    },
    
    /// Query - request for information
    Query { 
        /// The specific type/name of the query
        query_type: String 
    },
    
    /// Transformation - pure data transformation
    Transformation { 
        /// The specific type/name of the transformation
        transform_type: String 
    },
    
    /// Policy - business rule application
    Policy { 
        /// The specific type/name of the policy
        policy_type: String 
    },
}

impl DomainCategory {
    /// Create a new domain category
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            objects: HashMap::new(),
            morphisms: HashMap::new(),
            composition_table: HashMap::new(),
            identities: HashMap::new(),
        }
    }
    
    /// Add an object to the category
    pub fn add_object(&mut self, object: DomainObject) -> Result<(), DomainError> {
        if self.objects.contains_key(&object.id) {
            return Err(DomainError::AlreadyExists(format!("Object {} already exists", object.id)));
        }
        
        // Create identity morphism for this object
        let identity_id = format!("id_{}", object.id);
        let identity = DomainMorphism {
            id: identity_id.clone(),
            source: object.id.clone(),
            target: object.id.clone(),
            operation_type: MorphismType::Transformation {
                transform_type: "identity".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        self.objects.insert(object.id.clone(), object.clone());
        self.morphisms.insert(identity_id.clone(), identity);
        self.identities.insert(object.id, identity_id);
        
        Ok(())
    }
    
    /// Add a morphism to the category
    pub fn add_morphism(&mut self, morphism: DomainMorphism) -> Result<(), DomainError> {
        // Validate source and target exist
        if !self.objects.contains_key(&morphism.source) {
            return Err(DomainError::NotFound(format!("Source object {} not found", morphism.source)));
        }
        if !self.objects.contains_key(&morphism.target) {
            return Err(DomainError::NotFound(format!("Target object {} not found", morphism.target)));
        }
        if self.morphisms.contains_key(&morphism.id) {
            return Err(DomainError::AlreadyExists(format!("Morphism {} already exists", morphism.id)));
        }
        
        self.morphisms.insert(morphism.id.clone(), morphism);
        Ok(())
    }
    
    /// Define composition of two morphisms
    pub fn define_composition(
        &mut self,
        first: &str,
        second: &str,
        result: &str,
    ) -> Result<(), DomainError> {
        // Validate morphisms exist
        let first_morph = self.morphisms.get(first)
            .ok_or_else(|| DomainError::NotFound(format!("First morphism {} not found", first)))?;
        let second_morph = self.morphisms.get(second)
            .ok_or_else(|| DomainError::NotFound(format!("Second morphism {} not found", second)))?;
        let result_morph = self.morphisms.get(result)
            .ok_or_else(|| DomainError::NotFound(format!("Result morphism {} not found", result)))?;
        
        // Validate composition is valid (first.target == second.source)
        if first_morph.target != second_morph.source {
            return Err(DomainError::InvalidOperation {
                reason: format!("Cannot compose: {} target != {} source", first, second)
            });
        }
        
        // Validate result has correct source and target
        if result_morph.source != first_morph.source || result_morph.target != second_morph.target {
            return Err(DomainError::InvalidOperation {
                reason: format!("Result morphism has incorrect source/target")
            });
        }
        
        self.composition_table.insert((first.to_string(), second.to_string()), result.to_string());
        Ok(())
    }
    
    /// Compose two morphisms
    pub fn compose(&self, first: &str, second: &str) -> Result<String, DomainError> {
        // Check if composition is defined
        if let Some(result) = self.composition_table.get(&(first.to_string(), second.to_string())) {
            return Ok(result.clone());
        }
        
        // Check if we can use identity laws
        let first_morph = self.morphisms.get(first)
            .ok_or_else(|| DomainError::NotFound(format!("First morphism {} not found", first)))?;
        let second_morph = self.morphisms.get(second)
            .ok_or_else(|| DomainError::NotFound(format!("Second morphism {} not found", second)))?;
        
        // Identity law: id ∘ f = f
        if let Some(id) = self.identities.get(&first_morph.source) {
            if first == id {
                return Ok(second.to_string());
            }
        }
        
        // Identity law: f ∘ id = f
        if let Some(id) = self.identities.get(&second_morph.target) {
            if second == id {
                return Ok(first.to_string());
            }
        }
        
        Err(DomainError::InvalidOperation {
            reason: format!("Composition of {} and {} not defined", first, second)
        })
    }
    
    /// Get all objects of a specific type
    pub fn objects_of_type(&self, comp_type: &DomainCompositionType) -> Vec<&DomainObject> {
        self.objects.values()
            .filter(|obj| &obj.composition_type == comp_type)
            .collect()
    }
    
    /// Get all morphisms of a specific type
    pub fn morphisms_of_type(&self, morph_type: &MorphismType) -> Vec<&DomainMorphism> {
        self.morphisms.values()
            .filter(|morph| std::mem::discriminant(&morph.operation_type) == std::mem::discriminant(morph_type))
            .collect()
    }
    
    /// Get all morphisms from a source object
    pub fn morphisms_from(&self, source: &str) -> Vec<&DomainMorphism> {
        self.morphisms.values()
            .filter(|morph| morph.source == source)
            .collect()
    }
    
    /// Get all morphisms to a target object
    pub fn morphisms_to(&self, target: &str) -> Vec<&DomainMorphism> {
        self.morphisms.values()
            .filter(|morph| morph.target == target)
            .collect()
    }
    
    /// Verify category laws
    pub fn verify_laws(&self) -> Result<(), DomainError> {
        // Verify identity laws
        for (obj_id, identity_id) in &self.identities {
            let identity = self.morphisms.get(identity_id)
                .ok_or_else(|| DomainError::InvalidOperation { reason: format!("Identity {} not found", identity_id) })?;
            
            // Identity should have same source and target
            if identity.source != *obj_id || identity.target != *obj_id {
                return Err(DomainError::InvalidOperation {
                    reason: format!("Identity {} has wrong source/target", identity_id)
                });
            }
        }
        
        // Verify associativity of composition
        // For all composable triples (f, g, h), verify (f ∘ g) ∘ h = f ∘ (g ∘ h)
        for (f_id, f) in &self.morphisms {
            for (g_id, g) in &self.morphisms {
                if f.target != g.source {
                    continue;
                }
                
                for (h_id, h) in &self.morphisms {
                    if g.target != h.source {
                        continue;
                    }
                    
                    // Try to compose (f ∘ g) ∘ h
                    if let Ok(fg) = self.compose(f_id, g_id) {
                        if let Ok(fg_h) = self.compose(&fg, h_id) {
                            // Try to compose f ∘ (g ∘ h)
                            if let Ok(gh) = self.compose(g_id, h_id) {
                                if let Ok(f_gh) = self.compose(f_id, &gh) {
                                    // Verify they are equal
                                    if fg_h != f_gh {
                                        return Err(DomainError::InvalidOperation {
                                            reason: format!("Associativity violated: ({} ∘ {}) ∘ {} != {} ∘ ({} ∘ {})",
                                                f_id, g_id, h_id, f_id, g_id, h_id)
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_domain_category_creation() {
        let mut category = DomainCategory::new("TestDomain".to_string());
        
        // Add an aggregate object
        let aggregate = DomainObject {
            id: "Order".to_string(),
            composition_type: DomainCompositionType::Aggregate {
                aggregate_type: "Order".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        assert!(category.add_object(aggregate).is_ok());
        assert_eq!(category.objects.len(), 1);
        assert_eq!(category.morphisms.len(), 1); // Identity morphism
        assert_eq!(category.identities.len(), 1);
    }
    
    #[test]
    fn test_morphism_addition() {
        let mut category = DomainCategory::new("TestDomain".to_string());
        
        // Add objects
        let order = DomainObject {
            id: "Order".to_string(),
            composition_type: DomainCompositionType::Aggregate {
                aggregate_type: "Order".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let order_placed = DomainObject {
            id: "OrderPlaced".to_string(),
            composition_type: DomainCompositionType::Event {
                event_type: "OrderPlaced".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        category.add_object(order.clone()).unwrap();
        category.add_object(order_placed).unwrap();
        
        // Add morphism
        let place_order = DomainMorphism {
            id: "PlaceOrder".to_string(),
            source: "Order".to_string(),
            target: "OrderPlaced".to_string(),
            operation_type: MorphismType::Command {
                command_type: "PlaceOrder".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        assert!(category.add_morphism(place_order).is_ok());
        assert_eq!(category.morphisms.len(), 3); // 2 identities + 1 command
    }
    
    #[test]
    fn test_composition() {
        let mut category = DomainCategory::new("TestDomain".to_string());
        
        // Create a simple composition scenario: A -> B -> C
        let a = DomainObject {
            id: "A".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "A".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let b = DomainObject {
            id: "B".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "B".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let c = DomainObject {
            id: "C".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "C".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        category.add_object(a).unwrap();
        category.add_object(b).unwrap();
        category.add_object(c).unwrap();
        
        // Add morphisms f: A -> B and g: B -> C
        let f = DomainMorphism {
            id: "f".to_string(),
            source: "A".to_string(),
            target: "B".to_string(),
            operation_type: MorphismType::Transformation {
                transform_type: "f".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        let g = DomainMorphism {
            id: "g".to_string(),
            source: "B".to_string(),
            target: "C".to_string(),
            operation_type: MorphismType::Transformation {
                transform_type: "g".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        // Add composition h: A -> C
        let h = DomainMorphism {
            id: "h".to_string(),
            source: "A".to_string(),
            target: "C".to_string(),
            operation_type: MorphismType::Transformation {
                transform_type: "g∘f".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        category.add_morphism(f).unwrap();
        category.add_morphism(g).unwrap();
        category.add_morphism(h).unwrap();
        
        // Define composition
        category.define_composition("f", "g", "h").unwrap();
        
        // Test composition
        assert_eq!(category.compose("f", "g").unwrap(), "h");
    }
    
    #[test]
    fn test_identity_laws() {
        let mut category = DomainCategory::new("TestDomain".to_string());
        
        let obj = DomainObject {
            id: "X".to_string(),
            composition_type: DomainCompositionType::Entity {
                entity_type: "X".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        category.add_object(obj).unwrap();
        
        let f = DomainMorphism {
            id: "f".to_string(),
            source: "X".to_string(),
            target: "X".to_string(),
            operation_type: MorphismType::Transformation {
                transform_type: "f".to_string(),
            },
            metadata: HashMap::new(),
        };
        
        category.add_morphism(f).unwrap();
        
        // Test identity laws
        let id_x = category.identities.get("X").unwrap();
        assert_eq!(category.compose(id_x, "f").unwrap(), "f");
        assert_eq!(category.compose("f", id_x).unwrap(), "f");
    }
}