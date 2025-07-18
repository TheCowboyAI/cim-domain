//! Semantic analysis for cross-domain understanding
//!
//! This module provides semantic analysis capabilities to understand
//! relationships and meanings across domain boundaries.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::errors::DomainError;
use crate::category::DomainObject;
use crate::composition::DomainComposition;
use tokio::sync::RwLock;

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f64], b: &[f64]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    (dot_product / (norm_a * norm_b)) as f32
}

/// Semantic analyzer for cross-domain understanding
pub struct SemanticAnalyzer {
    /// Concept embeddings
    embeddings: RwLock<HashMap<String, ConceptEmbedding>>,
    
    /// Concept relationships
    relationships: RwLock<Vec<ConceptRelationship>>,
    
    /// Domain ontologies
    ontologies: RwLock<HashMap<String, DomainOntology>>,
    
    /// Simple concepts for cross-domain search
    concepts: RwLock<Vec<crate::integration::cross_domain_search::Concept>>,
}

/// An embedding of a concept in semantic space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptEmbedding {
    /// Concept identifier
    pub concept_id: String,
    
    /// Domain this concept belongs to
    pub domain: String,
    
    /// Vector representation (normalized)
    pub vector: Vec<f64>,
    
    /// Metadata about the concept
    pub metadata: HashMap<String, String>,
}

/// A relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelationship {
    /// Source concept
    pub source: String,
    
    /// Target concept
    pub target: String,
    
    /// Type of relationship
    pub relationship_type: RelationshipType,
    
    /// Strength of relationship (0-1)
    pub strength: f64,
}

/// Types of semantic relationships
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipType {
    /// Is-a relationship (inheritance)
    IsA,
    
    /// Part-of relationship (composition)
    PartOf,
    
    /// Related-to (general association)
    RelatedTo,
    
    /// Same-as (equivalence)
    SameAs,
    
    /// Opposite-of (antonym)
    OppositeOf,
    
    /// Depends-on (dependency)
    DependsOn,
    
    /// Custom relationship
    Custom(String),
}

/// Domain ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainOntology {
    /// Domain name
    pub domain: String,
    
    /// Root concepts
    pub roots: Vec<String>,
    
    /// Concept hierarchy
    pub hierarchy: HashMap<String, Vec<String>>,
    
    /// Axioms
    pub axioms: Vec<OntologyAxiom>,
}

/// An axiom in the ontology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyAxiom {
    /// Axiom name
    pub name: String,
    
    /// Axiom type
    pub axiom_type: AxiomType,
    
    /// Concepts involved
    pub concepts: Vec<String>,
}

/// Types of ontology axioms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AxiomType {
    /// Disjoint concepts
    Disjoint,
    
    /// Covering (complete subclasses)
    Covering,
    
    /// Functional property
    Functional,
    
    /// Inverse property
    Inverse,
    
    /// Transitive property
    Transitive,
}

/// Semantic distance between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDistance {
    /// Euclidean distance in embedding space
    pub euclidean: f64,
    
    /// Cosine similarity (1 - cosine distance)
    pub cosine_similarity: f64,
    
    /// Path distance in ontology
    pub ontology_distance: Option<u32>,
    
    /// Combined semantic distance (0-1)
    pub combined: f64,
}

/// Alignment between concepts from different domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptAlignment {
    /// Source concept
    pub source_concept: String,
    
    /// Target concept
    pub target_concept: String,
    
    /// Alignment score (0-1)
    pub score: f64,
    
    /// Type of alignment
    pub alignment_type: AlignmentType,
    
    /// Evidence for the alignment
    pub evidence: Vec<AlignmentEvidence>,
}

/// Types of concept alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlignmentType {
    /// Exact match
    Exact,
    
    /// Subsumption (source is more specific)
    Subsumes,
    
    /// Generalization (source is more general)
    Generalizes,
    
    /// Partial overlap
    Partial,
    
    /// No alignment
    None,
}

/// Evidence for concept alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentEvidence {
    /// Type of evidence
    pub evidence_type: String,
    
    /// Confidence in this evidence (0-1)
    pub confidence: f64,
    
    /// Supporting data
    pub data: serde_json::Value,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        Self {
            embeddings: RwLock::new(HashMap::new()),
            relationships: RwLock::new(Vec::new()),
            ontologies: RwLock::new(HashMap::new()),
            concepts: RwLock::new(Vec::new()),
        }
    }
    
    /// Add a concept embedding
    pub async fn add_embedding(&self, embedding: ConceptEmbedding) {
        let mut embeddings = self.embeddings.write().await;
        embeddings.insert(embedding.concept_id.clone(), embedding);
    }
    
    /// Add a concept relationship
    pub async fn add_relationship(&self, relationship: ConceptRelationship) {
        let mut relationships = self.relationships.write().await;
        relationships.push(relationship);
    }
    
    /// Add a domain ontology
    pub async fn add_ontology(&self, ontology: DomainOntology) {
        let mut ontologies = self.ontologies.write().await;
        ontologies.insert(ontology.domain.clone(), ontology);
    }
    
    /// Add a simple concept for cross-domain search
    pub async fn add_concept(&self, concept: crate::integration::cross_domain_search::Concept) -> Result<(), DomainError> {
        let mut concepts = self.concepts.write().await;
        concepts.push(concept);
        Ok(())
    }
    
    /// Get all concepts
    pub async fn get_concepts(&self) -> Result<Vec<crate::integration::cross_domain_search::Concept>, DomainError> {
        let concepts = self.concepts.read().await;
        Ok(concepts.clone())
    }
    
    /// Find similar concepts based on vector similarity
    pub async fn find_similar(&self, concept_name: &str, min_similarity: f32) -> Result<Vec<(String, f32)>, DomainError> {
        let concepts = self.concepts.read().await;
        
        // Find the target concept
        let target = concepts.iter()
            .find(|c| c.name == concept_name)
            .ok_or_else(|| DomainError::NotFound(format!("Concept {} not found", concept_name)))?;
        
        let mut similar = Vec::new();
        
        for concept in concepts.iter() {
            if concept.name != concept_name {
                let similarity = cosine_similarity(&target.quality_dimensions, &concept.quality_dimensions);
                if similarity >= min_similarity {
                    similar.push((concept.name.clone(), similarity));
                }
            }
        }
        
        // Sort by similarity (highest first)
        similar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(similar)
    }
    
    /// Remove a concept
    pub async fn remove_concept(&self, concept_name: &str) -> Result<(), DomainError> {
        let mut concepts = self.concepts.write().await;
        concepts.retain(|c| c.name != concept_name);
        Ok(())
    }
    
    /// Create a concept from text (simplified version)
    pub async fn create_concept_from_text(&self, text: &str) -> Result<crate::integration::cross_domain_search::Concept, DomainError> {
        // In a real implementation, this would use an embedding model
        // For now, create a simple hash-based embedding
        let hash = text.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        
        let vector = vec![
            ((hash % 100) as f64) / 100.0,
            (((hash >> 8) % 100) as f64) / 100.0,
            (((hash >> 16) % 100) as f64) / 100.0,
            (((hash >> 24) % 100) as f64) / 100.0,
            ((text.len() as u32 % 100) as f64) / 100.0,
        ];
        
        Ok(crate::integration::cross_domain_search::Concept::new(text.to_string(), vector))
    }
    
    /// Calculate semantic distance between concepts
    pub async fn semantic_distance(
        &self,
        concept_a: &str,
        concept_b: &str,
    ) -> Result<SemanticDistance, DomainError> {
        let embeddings = self.embeddings.read().await;
        let embedding_a = embeddings.get(concept_a)
            .ok_or_else(|| DomainError::NotFound(
                format!("Concept {} not found", concept_a)
            ))?;
        
        let embedding_b = embeddings.get(concept_b)
            .ok_or_else(|| DomainError::NotFound(
                format!("Concept {} not found", concept_b)
            ))?;
        
        // Calculate Euclidean distance
        let euclidean = Self::euclidean_distance(&embedding_a.vector, &embedding_b.vector);
        
        // Calculate cosine similarity
        let cosine_similarity = Self::cosine_similarity(&embedding_a.vector, &embedding_b.vector);
        
        // Calculate ontology distance if both concepts are in same domain
        let ontology_distance = if embedding_a.domain == embedding_b.domain {
            self.ontology_distance(&embedding_a.domain, concept_a, concept_b)
        } else {
            None
        };
        
        // Combine distances (weighted average)
        let combined = if let Some(ont_dist) = ontology_distance {
            // Normalize ontology distance (assume max depth of 10)
            let norm_ont_dist = (ont_dist as f64) / 10.0;
            
            // Weighted combination
            0.3 * euclidean + 0.5 * (1.0 - cosine_similarity) + 0.2 * norm_ont_dist
        } else {
            0.4 * euclidean + 0.6 * (1.0 - cosine_similarity)
        };
        
        Ok(SemanticDistance {
            euclidean,
            cosine_similarity,
            ontology_distance,
            combined: combined.min(1.0),
        })
    }
    
    /// Find concept alignments between domains
    pub async fn find_alignments(
        &self,
        source_domain: &str,
        target_domain: &str,
        threshold: f64,
    ) -> Vec<ConceptAlignment> {
        let embeddings = self.embeddings.read().await;
        let mut alignments = Vec::new();
        
        // Get all concepts from each domain
        let source_concepts: Vec<_> = embeddings.values()
            .filter(|e| e.domain == source_domain)
            .collect();
        
        let target_concepts: Vec<_> = embeddings.values()
            .filter(|e| e.domain == target_domain)
            .collect();
        
        // Compare all pairs
        for source in &source_concepts {
            for target in &target_concepts {
                // Calculate similarity directly
                let euclidean = Self::euclidean_distance(&source.vector, &target.vector);
                let cosine_similarity = Self::cosine_similarity(&source.vector, &target.vector);
                
                // Use cosine similarity as the primary score
                let score = cosine_similarity;
                
                if score >= threshold {
                    let alignment_type = if score > 0.95 {
                        AlignmentType::Exact
                    } else if score > 0.8 {
                        AlignmentType::Partial
                    } else {
                        AlignmentType::None
                    };
                    
                    alignments.push(ConceptAlignment {
                        source_concept: source.concept_id.clone(),
                        target_concept: target.concept_id.clone(),
                        score,
                        alignment_type,
                        evidence: vec![
                            AlignmentEvidence {
                                evidence_type: "embedding_similarity".to_string(),
                                confidence: score,
                                data: serde_json::json!({
                                    "cosine_similarity": cosine_similarity,
                                    "euclidean": euclidean,
                                }),
                            },
                        ],
                    });
                }
            }
        }
        
        // Sort by score (highest first)
        alignments.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        alignments
    }
    
    /// Get related concepts
    pub async fn get_related_concepts(
        &self,
        concept: &str,
        max_distance: f64,
    ) -> Vec<(String, SemanticDistance)> {
        let embeddings = self.embeddings.read().await;
        let mut related = Vec::new();
        
        // Get the concept we're searching from
        let search_embedding = embeddings.get(concept);
        if search_embedding.is_none() {
            return related;
        }
        let search_embedding = search_embedding.unwrap();
        
        for (other_concept, other_embedding) in embeddings.iter() {
            if other_concept != concept {
                let distance = SemanticDistance {
                    euclidean: Self::euclidean_distance(&search_embedding.vector, &other_embedding.vector),
                    cosine_similarity: Self::cosine_similarity(&search_embedding.vector, &other_embedding.vector),
                    ontology_distance: None,
                    combined: 0.5, // Placeholder
                };
                if distance.combined <= max_distance {
                    related.push((other_concept.clone(), distance));
                }
            }
        }
        
        // Sort by distance (closest first)
        related.sort_by(|a, b| a.1.combined.partial_cmp(&b.1.combined).unwrap());
        related
    }
    
    /// Euclidean distance between vectors
    fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return 1.0; // Max distance for mismatched dimensions
        }
        
        let sum_sq: f64 = a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum();
        
        sum_sq.sqrt() / (a.len() as f64).sqrt() // Normalize by dimension
    }
    
    /// Cosine similarity between vectors
    fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() {
            return 0.0; // No similarity for mismatched dimensions
        }
        
        let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            (dot_product / (norm_a * norm_b)).max(-1.0).min(1.0)
        }
    }
    
    /// Calculate path distance in ontology
    fn ontology_distance(
        &self,
        _domain: &str,
        concept_a: &str,
        concept_b: &str,
    ) -> Option<u32> {
        // In a real implementation, would use graph algorithms
        // to find shortest path in ontology hierarchy
        
        // For now, return a placeholder
        if concept_a == concept_b {
            Some(0)
        } else {
            Some(2) // Assume 2 hops for different concepts
        }
    }
}

/// Example: Create embeddings from domain objects
impl SemanticAnalyzer {
    /// Create embeddings for all objects in a composition
    pub async fn embed_composition(&self, composition: &DomainComposition) -> Result<(), DomainError> {
        for (domain_name, domain) in &composition.domains {
            for (obj_id, obj) in &domain.objects {
                // Create a simple embedding based on object properties
                // In a real implementation, would use proper embedding model
                let vector = self.create_embedding_vector(obj);
                
                let embedding = ConceptEmbedding {
                    concept_id: format!("{}:{}", domain_name, obj_id),
                    domain: domain_name.clone(),
                    vector,
                    metadata: obj.metadata.clone(),
                };
                
                self.add_embedding(embedding).await;
            }
        }
        
        Ok(())
    }
    
    /// Create a simple embedding vector for an object
    fn create_embedding_vector(&self, obj: &DomainObject) -> Vec<f64> {
        // Simple 5D embedding based on object properties
        // In real implementation, would use learned embeddings
        
        let mut vector = vec![0.0; 5];
        
        // Dimension 0: Entity type indicator
        vector[0] = match &obj.composition_type {
            crate::composition_types::DomainCompositionType::Entity { .. } => 1.0,
            crate::composition_types::DomainCompositionType::Aggregate { .. } => 0.8,
            crate::composition_types::DomainCompositionType::ValueObject { .. } => 0.3,
            _ => 0.5,
        };
        
        // Dimension 1: Metadata richness
        vector[1] = (obj.metadata.len() as f64 / 10.0).min(1.0);
        
        // Dimension 2-4: Hash-based pseudo-random values
        let hash = obj.id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        vector[2] = ((hash % 100) as f64) / 100.0;
        vector[3] = (((hash >> 8) % 100) as f64) / 100.0;
        vector[4] = (((hash >> 16) % 100) as f64) / 100.0;
        
        // Normalize vector
        let norm: f64 = vector.iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }
        
        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_semantic_distance() {
        let mut analyzer = SemanticAnalyzer::new();
        
        // Add test embeddings
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "concept_a".to_string(),
            domain: "domain1".to_string(),
            vector: vec![1.0, 0.0, 0.0, 0.0, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "concept_b".to_string(),
            domain: "domain1".to_string(),
            vector: vec![0.8, 0.6, 0.0, 0.0, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "concept_c".to_string(),
            domain: "domain2".to_string(),
            vector: vec![0.0, 0.0, 1.0, 0.0, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        // Test distances
        let dist_ab = analyzer.semantic_distance("concept_a", "concept_b").await.unwrap();
        let dist_ac = analyzer.semantic_distance("concept_a", "concept_c").await.unwrap();
        
        // A and B should be closer than A and C
        assert!(dist_ab.combined < dist_ac.combined);
        assert!(dist_ab.cosine_similarity > 0.5);
        assert!(dist_ac.cosine_similarity < 0.1);
    }
    
    #[tokio::test]
    async fn test_concept_alignment() {
        let mut analyzer = SemanticAnalyzer::new();
        
        // Add embeddings from two domains
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "order_1".to_string(),
            domain: "sales".to_string(),
            vector: vec![0.8, 0.6, 0.0, 0.0, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "purchase_order".to_string(),
            domain: "procurement".to_string(),
            vector: vec![0.75, 0.65, 0.1, 0.0, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        analyzer.add_embedding(ConceptEmbedding {
            concept_id: "invoice".to_string(),
            domain: "finance".to_string(),
            vector: vec![0.0, 0.0, 0.8, 0.6, 0.0],
            metadata: HashMap::new(),
        }).await;
        
        // Find alignments
        let alignments = analyzer.find_alignments("sales", "procurement", 0.8).await;
        
        assert!(!alignments.is_empty());
        assert_eq!(alignments[0].source_concept, "order_1");
        assert_eq!(alignments[0].target_concept, "purchase_order");
        assert!(alignments[0].score > 0.8);
    }
    
    #[test]
    fn test_vector_operations() {
        let a = vec![3.0, 4.0];
        let b = vec![0.0, 0.0];
        
        // Euclidean distance
        let dist = SemanticAnalyzer::euclidean_distance(&a, &b);
        assert!((dist - 5.0 / 2.0_f64.sqrt()).abs() < 0.001); // Normalized
        
        // Cosine similarity
        let sim = SemanticAnalyzer::cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001); // Same vector = 1.0
        
        let c = vec![4.0, 3.0];
        let sim_ac = SemanticAnalyzer::cosine_similarity(&a, &c);
        assert!(sim_ac > 0.9); // Very similar
    }
}