//! Cross-domain semantic search using category theory bridges
//!
//! This module implements semantic search across multiple domains by leveraging
//! functors and natural transformations to map concepts between domains.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::DomainError;
use crate::category::{DomainCategory, DomainFunctor};
use crate::domain::semantic_analyzer::SemanticAnalyzer;
use super::event_bridge::EventBridge;

/// A simplified concept representation for cross-domain search
#[derive(Debug, Clone)]
pub struct Concept {
    pub name: String,
    pub quality_dimensions: Vec<f64>,
}

impl Concept {
    pub fn new(name: String, quality_dimensions: Vec<f64>) -> Self {
        Self { name, quality_dimensions }
    }
}

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

/// Cross-domain search engine that uses category theory for concept mapping
pub struct CrossDomainSearchEngine {
    /// Semantic analyzers for each domain
    analyzers: Arc<RwLock<HashMap<String, Arc<SemanticAnalyzer>>>>,
    
    /// Domain categories
    domains: Arc<RwLock<HashMap<String, DomainCategory>>>,
    
    /// Functors between domains for concept mapping
    functors: Arc<RwLock<HashMap<(String, String), Box<dyn std::any::Any + Send + Sync>>>>,
    
    /// Event bridge for cross-domain communication
    event_bridge: Arc<EventBridge>,
    
    /// Search configuration
    config: SearchConfig,
}

/// Configuration for cross-domain search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default number of results per domain
    pub results_per_domain: usize,
    
    /// Minimum similarity threshold
    pub min_similarity: f32,
    
    /// Whether to follow domain relationships
    pub follow_relationships: bool,
    
    /// Maximum depth for relationship traversal
    pub max_depth: usize,
    
    /// Whether to aggregate similar results
    pub aggregate_results: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            results_per_domain: 10,
            min_similarity: 0.7,
            follow_relationships: true,
            max_depth: 3,
            aggregate_results: true,
        }
    }
}

/// A cross-domain search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainQuery {
    /// The search text
    pub query: String,
    
    /// Starting domain (optional)
    pub start_domain: Option<String>,
    
    /// Domains to search (empty = all)
    pub target_domains: Vec<String>,
    
    /// Concept vector (if pre-computed)
    pub concept_vector: Option<Vec<f64>>,
    
    /// Search configuration overrides
    pub config_overrides: Option<SearchConfig>,
}

/// A search result from multiple domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainResult {
    /// Results grouped by domain
    pub domain_results: HashMap<String, Vec<DomainSearchResult>>,
    
    /// Cross-domain relationships found
    pub relationships: Vec<DomainRelationship>,
    
    /// Aggregated concepts
    pub aggregated_concepts: Vec<AggregatedConcept>,
    
    /// Search metadata
    pub metadata: SearchMetadata,
}

/// A search result within a specific domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSearchResult {
    /// The domain this result belongs to
    pub domain: String,
    
    /// The concept ID
    pub concept_id: String,
    
    /// The concept name
    pub concept_name: String,
    
    /// Similarity score
    pub similarity: f32,
    
    /// Concept metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Related concepts in other domains
    pub cross_domain_links: Vec<CrossDomainLink>,
}

/// A link between concepts in different domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainLink {
    /// Source domain
    pub source_domain: String,
    
    /// Source concept
    pub source_concept: String,
    
    /// Target domain
    pub target_domain: String,
    
    /// Target concept
    pub target_concept: String,
    
    /// Link type (functor name)
    pub link_type: String,
    
    /// Link strength
    pub strength: f32,
}

/// A relationship between domains discovered during search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainRelationship {
    /// First domain
    pub domain_a: String,
    
    /// Second domain
    pub domain_b: String,
    
    /// Relationship type
    pub relationship_type: String,
    
    /// Shared concepts
    pub shared_concepts: Vec<String>,
    
    /// Relationship strength
    pub strength: f32,
}

/// An aggregated concept across multiple domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedConcept {
    /// Canonical name
    pub name: String,
    
    /// Domains where this concept appears
    pub domains: Vec<String>,
    
    /// Average similarity across domains
    pub avg_similarity: f32,
    
    /// Concept variations by domain
    pub variations: HashMap<String, String>,
}

/// Metadata about the search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// The search query
    pub query: CrossDomainQuery,
    
    /// Total results found
    pub total_results: usize,
    
    /// Domains searched
    pub domains_searched: Vec<String>,
    
    /// Search duration in milliseconds
    pub duration_ms: u64,
    
    /// Whether results were truncated
    pub truncated: bool,
    
    /// Search depth reached
    pub max_depth_reached: usize,
}

impl CrossDomainSearchEngine {
    /// Create a new cross-domain search engine
    pub fn new(event_bridge: Arc<EventBridge>, config: SearchConfig) -> Self {
        Self {
            analyzers: Arc::new(RwLock::new(HashMap::new())),
            domains: Arc::new(RwLock::new(HashMap::new())),
            functors: Arc::new(RwLock::new(HashMap::new())),
            event_bridge,
            config,
        }
    }
    
    /// Register a domain with its semantic analyzer
    pub async fn register_domain(
        &self,
        domain: DomainCategory,
        analyzer: Arc<SemanticAnalyzer>,
    ) -> Result<(), DomainError> {
        let domain_name = domain.name.clone();
        
        let mut domains = self.domains.write().await;
        let mut analyzers = self.analyzers.write().await;
        
        if domains.contains_key(&domain_name) {
            return Err(DomainError::AlreadyExists(
                format!("Domain {} already registered", domain_name)
            ));
        }
        
        domains.insert(domain_name.clone(), domain);
        analyzers.insert(domain_name, analyzer);
        
        Ok(())
    }
    
    /// Register a functor between domains
    pub async fn register_functor<F>(
        &self,
        source: String,
        target: String,
        functor: F,
    ) -> Result<(), DomainError>
    where
        F: DomainFunctor + 'static,
    {
        let mut functors = self.functors.write().await;
        let key = (source, target);
        
        if functors.contains_key(&key) {
            return Err(DomainError::AlreadyExists(
                format!("Functor from {} to {} already exists", key.0, key.1)
            ));
        }
        
        functors.insert(key, Box::new(functor));
        Ok(())
    }
    
    /// Perform a cross-domain search
    pub async fn search(&self, query: CrossDomainQuery) -> Result<CrossDomainResult, DomainError> {
        let start_time = std::time::Instant::now();
        let config = query.config_overrides.as_ref().unwrap_or(&self.config);
        
        // Get domains to search
        let domains_to_search = self.get_domains_to_search(&query).await?;
        
        // Compute query concept vector if not provided
        let concept_vector = match query.concept_vector.clone() {
            Some(v) => v,
            None => self.compute_query_vector(&query.query, query.start_domain.as_deref()).await?,
        };
        
        // Search each domain
        let mut domain_results = HashMap::new();
        let mut all_relationships = Vec::new();
        
        for domain in &domains_to_search {
            let results = self.search_domain(
                domain,
                &concept_vector,
                config,
            ).await?;
            
            // Find cross-domain links
            if config.follow_relationships {
                for result in &results {
                    let links = self.find_cross_domain_links(
                        domain,
                        &result.concept_name,
                        config.max_depth,
                    ).await?;
                    
                    // Add to relationships
                    for link in &links {
                        all_relationships.push(DomainRelationship {
                            domain_a: link.source_domain.clone(),
                            domain_b: link.target_domain.clone(),
                            relationship_type: link.link_type.clone(),
                            shared_concepts: vec![link.source_concept.clone(), link.target_concept.clone()],
                            strength: link.strength,
                        });
                    }
                }
            }
            
            domain_results.insert(domain.clone(), results);
        }
        
        // Aggregate similar concepts across domains
        let aggregated_concepts = if config.aggregate_results {
            self.aggregate_concepts(&domain_results).await?
        } else {
            Vec::new()
        };
        
        // Build result
        let total_results = domain_results.values().map(|v| v.len()).sum();
        let truncated = domain_results.values().any(|results| results.len() >= config.results_per_domain);
        
        let result = CrossDomainResult {
            domain_results,
            relationships: self.deduplicate_relationships(all_relationships),
            aggregated_concepts,
            metadata: SearchMetadata {
                query: query.clone(),
                total_results,
                domains_searched: domains_to_search,
                duration_ms: start_time.elapsed().as_millis() as u64,
                truncated,
                max_depth_reached: config.max_depth,
            },
        };
        
        Ok(result)
    }
    
    /// Get domains to search based on query
    async fn get_domains_to_search(&self, query: &CrossDomainQuery) -> Result<Vec<String>, DomainError> {
        let domains = self.domains.read().await;
        
        if query.target_domains.is_empty() {
            // Search all domains
            Ok(domains.keys().cloned().collect())
        } else {
            // Validate requested domains exist
            for domain in &query.target_domains {
                if !domains.contains_key(domain) {
                    return Err(DomainError::NotFound(
                        format!("Domain {} not found", domain)
                    ));
                }
            }
            Ok(query.target_domains.clone())
        }
    }
    
    /// Compute concept vector for query
    async fn compute_query_vector(
        &self,
        query: &str,
        start_domain: Option<&str>,
    ) -> Result<Vec<f64>, DomainError> {
        // If start domain specified, use its analyzer
        if let Some(domain) = start_domain {
            let analyzers = self.analyzers.read().await;
            if let Some(analyzer) = analyzers.get(domain) {
                // Create temporary concept
                // Use analyzer's conceptual space to create query embedding
                let concept = analyzer.create_concept_from_text(query).await
                    .unwrap_or_else(|_| Concept::new(query.to_string(), vec![0.5; 5]));
                return Ok(concept.quality_dimensions);
            }
        }
        
        // Otherwise, try to find any analyzer to generate embeddings
        let analyzers = self.analyzers.read().await;
        if let Some((_, analyzer)) = analyzers.iter().next() {
            let concept = analyzer.create_concept_from_text(query).await
                .unwrap_or_else(|_| Concept::new(query.to_string(), vec![0.5; 5]));
            Ok(concept.quality_dimensions)
        } else {
            // Fallback to default vector
            Ok(vec![0.5; 5])
        }
    }
    
    /// Search within a single domain
    async fn search_domain(
        &self,
        domain: &str,
        concept_vector: &[f64],
        config: &SearchConfig,
    ) -> Result<Vec<DomainSearchResult>, DomainError> {
        let analyzers = self.analyzers.read().await;
        
        let analyzer = analyzers.get(domain)
            .ok_or_else(|| DomainError::NotFound(format!("Analyzer for {} not found", domain)))?;
        
        // Create temporary concept for search
        let search_concept = Concept::new("_search_".to_string(), concept_vector.to_vec());
        analyzer.add_concept(search_concept.clone()).await?;
        
        // Find similar concepts
        let similar = analyzer.find_similar("_search_", config.min_similarity).await?;
        
        // Remove temporary concept
        analyzer.remove_concept("_search_").await?;
        
        // Convert to domain results
        let mut results = Vec::new();
        for (concept_name, similarity) in similar.into_iter().take(config.results_per_domain) {
            results.push(DomainSearchResult {
                domain: domain.to_string(),
                concept_id: Uuid::new_v4().to_string(),
                concept_name: concept_name.clone(),
                similarity,
                metadata: HashMap::new(),
                cross_domain_links: Vec::new(),
            });
        }
        
        Ok(results)
    }
    
    /// Find cross-domain links for a concept
    async fn find_cross_domain_links(
        &self,
        source_domain: &str,
        concept: &str,
        max_depth: usize,
    ) -> Result<Vec<CrossDomainLink>, DomainError> {
        let mut links = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        // Start with the source concept
        queue.push_back((source_domain.to_string(), concept.to_string(), 0));
        visited.insert((source_domain.to_string(), concept.to_string()));
        
        let functors = self.functors.read().await;
        let analyzers = self.analyzers.read().await;
        
        while let Some((current_domain, current_concept, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            
            // Look for functors from current domain to other domains
            for ((from_domain, to_domain), _functor) in functors.iter() {
                if from_domain == &current_domain {
                    // Check if target domain has similar concepts
                    if let Some(target_analyzer) = analyzers.get(to_domain) {
                        // Get concept from source domain
                        if let Some(source_analyzer) = analyzers.get(&current_domain) {
                            if let Ok(source_concepts) = source_analyzer.get_concepts().await {
                                if let Some(source_concept) = source_concepts.iter()
                                    .find(|c| c.name == current_concept) {
                                    
                                    // Find similar concepts in target domain
                                    if let Ok(similar) = target_analyzer.find_similar(&source_concept.name, 0.6).await {
                                        for (target_concept_name, similarity) in similar {
                                            let link = CrossDomainLink {
                                                source_domain: current_domain.clone(),
                                                source_concept: current_concept.clone(),
                                                target_domain: to_domain.clone(),
                                                target_concept: target_concept_name.clone(),
                                                link_type: format!("{}→{}", from_domain, to_domain),
                                                strength: similarity,
                                            };
                                            links.push(link);
                                            
                                            // Add to queue for further exploration
                                            let key = (to_domain.clone(), target_concept_name.clone());
                                            if !visited.contains(&key) {
                                                visited.insert(key.clone());
                                                queue.push_back((to_domain.clone(), target_concept_name, depth + 1));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(links)
    }
    
    /// Aggregate similar concepts across domains
    async fn aggregate_concepts(
        &self,
        domain_results: &HashMap<String, Vec<DomainSearchResult>>,
    ) -> Result<Vec<AggregatedConcept>, DomainError> {
        // Group concepts by similarity
        let mut concept_groups: HashMap<String, AggregatedConcept> = HashMap::new();
        let analyzers = self.analyzers.read().await;
        
        // First pass: collect all concepts with their vectors
        let mut all_concepts: Vec<(String, String, f32, Vec<f64>)> = Vec::new();
        
        for (domain, results) in domain_results {
            if let Some(analyzer) = analyzers.get(domain) {
                for result in results {
                    // Get the concept's vector from the analyzer
                    if let Ok(concepts) = analyzer.get_concepts().await {
                        if let Some(concept) = concepts.iter().find(|c| c.name == result.concept_name) {
                            all_concepts.push((
                                domain.clone(),
                                result.concept_name.clone(),
                                result.similarity,
                                concept.quality_dimensions.clone(),
                            ));
                        }
                    }
                }
            }
        }
        
        // Second pass: group similar concepts using vector similarity
        let similarity_threshold = 0.85; // High threshold for grouping
        
        for (domain, concept_name, similarity, vector) in all_concepts {
            let mut best_group: Option<String> = None;
            let mut best_similarity = 0.0;
            
            // Find the best matching group
            for (group_key, group) in concept_groups.iter() {
                // Compare with the first concept in the group to determine similarity
                if let Some((first_domain, _)) = group.domains.first().zip(group.variations.iter().next()) {
                    if let Some(analyzer) = analyzers.get(first_domain) {
                        if let Ok(concepts) = analyzer.get_concepts().await {
                            if let Some(group_concept) = concepts.iter()
                                .find(|c| group.variations.get(first_domain)
                                    .map_or(false, |name| &c.name == name)) {
                                
                                // Calculate cosine similarity
                                let sim = cosine_similarity(&vector, &group_concept.quality_dimensions);
                                if sim > similarity_threshold && sim > best_similarity {
                                    best_group = Some(group_key.clone());
                                    best_similarity = sim;
                                }
                            }
                        }
                    }
                }
            }
            
            if let Some(group_key) = best_group {
                // Add to existing group
                concept_groups.entry(group_key)
                    .and_modify(|agg| {
                        if !agg.domains.contains(&domain) {
                            agg.domains.push(domain.clone());
                        }
                        agg.variations.insert(domain.clone(), concept_name.clone());
                        // Update average similarity
                        let n = agg.domains.len() as f32;
                        agg.avg_similarity = ((n - 1.0) * agg.avg_similarity + similarity) / n;
                    });
            } else {
                // Create new group
                let key = format!("{}_{}", concept_name.to_lowercase().replace(' ', "_"), concept_groups.len());
                concept_groups.insert(key, AggregatedConcept {
                    name: concept_name.clone(),
                    domains: vec![domain.clone()],
                    avg_similarity: similarity,
                    variations: HashMap::from([(domain.clone(), concept_name)]),
                });
            }
        }
        
        // Sort by average similarity and number of domains
        let mut aggregated: Vec<AggregatedConcept> = concept_groups.into_values().collect();
        aggregated.sort_by(|a, b| {
            let score_a = a.avg_similarity * a.domains.len() as f32;
            let score_b = b.avg_similarity * b.domains.len() as f32;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(aggregated)
    }
    
    /// Deduplicate domain relationships
    fn deduplicate_relationships(&self, relationships: Vec<DomainRelationship>) -> Vec<DomainRelationship> {
        use std::collections::HashMap;
        
        // Group relationships by domain pair (normalized)
        let mut grouped: HashMap<(String, String), Vec<DomainRelationship>> = HashMap::new();
        
        for rel in relationships {
            // Normalize domain pair (alphabetical order)
            let key = if rel.domain_a < rel.domain_b {
                (rel.domain_a.clone(), rel.domain_b.clone())
            } else {
                (rel.domain_b.clone(), rel.domain_a.clone())
            };
            
            grouped.entry(key).or_default().push(rel);
        }
        
        // Merge relationships for each domain pair
        let mut deduplicated = Vec::new();
        
        for ((domain_a, domain_b), rels) in grouped {
            // Collect all shared concepts and relationship types
            let mut all_concepts = std::collections::HashSet::new();
            let mut all_types = std::collections::HashSet::new();
            let mut total_strength = 0.0;
            
            for rel in &rels {
                all_concepts.extend(rel.shared_concepts.iter().cloned());
                all_types.insert(rel.relationship_type.clone());
                total_strength += rel.strength;
            }
            
            // Create merged relationship
            let merged = DomainRelationship {
                domain_a,
                domain_b,
                relationship_type: if all_types.len() == 1 {
                    all_types.into_iter().next().unwrap()
                } else {
                    format!("multi[{}]", all_types.into_iter().collect::<Vec<_>>().join(","))
                },
                shared_concepts: all_concepts.into_iter().collect(),
                strength: total_strength / rels.len() as f32,
            };
            
            deduplicated.push(merged);
        }
        
        // Sort by strength
        deduplicated.sort_by(|a, b| b.strength.partial_cmp(&a.strength).unwrap_or(std::cmp::Ordering::Equal));
        
        deduplicated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cross_domain_search() {
        let event_bridge = Arc::new(EventBridge::new(Default::default()));
        let engine = CrossDomainSearchEngine::new(event_bridge, SearchConfig::default());
        
        // Register test domains
        let sales_domain = DomainCategory::new("Sales".to_string());
        let sales_analyzer = Arc::new(SemanticAnalyzer::new());
        engine.register_domain(sales_domain, sales_analyzer.clone()).await.unwrap();
        
        let billing_domain = DomainCategory::new("Billing".to_string());
        let billing_analyzer = Arc::new(SemanticAnalyzer::new());
        engine.register_domain(billing_domain, billing_analyzer.clone()).await.unwrap();
        
        // Add test concepts
        sales_analyzer.add_concept(Concept::new(
            "Order".to_string(),
            vec![0.9, 0.8, 0.7, 0.6, 0.5],
        )).await.unwrap();
        
        billing_analyzer.add_concept(Concept::new(
            "Invoice".to_string(),
            vec![0.85, 0.75, 0.65, 0.55, 0.45],
        )).await.unwrap();
        
        // Perform search
        let query = CrossDomainQuery {
            query: "Order".to_string(),
            start_domain: Some("Sales".to_string()),
            target_domains: vec![],
            concept_vector: Some(vec![0.9, 0.8, 0.7, 0.6, 0.5]),
            config_overrides: None,
        };
        
        let results = engine.search(query).await.unwrap();
        
        assert_eq!(results.domain_results.len(), 2);
        assert!(results.domain_results.contains_key("Sales"));
        assert!(results.domain_results.contains_key("Billing"));
    }
}