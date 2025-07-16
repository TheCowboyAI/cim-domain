//! Bridge between cross-domain search and agent domain semantic search
//!
//! This module provides adapters to connect the category theory-based
//! cross-domain search with the agent domain's vector-based semantic search.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;
use crate::events::DomainEvent;
use super::cross_domain_search::{
    CrossDomainSearchEngine, CrossDomainQuery, CrossDomainResult,
    DomainSearchResult,
};

/// Event emitted when a semantic search is performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchPerformed {
    /// Search ID
    pub search_id: Uuid,
    
    /// The query that was performed
    pub query: String,
    
    /// Domains that were searched
    pub domains_searched: Vec<String>,
    
    /// Number of results found
    pub total_results: usize,
    
    /// Search duration in milliseconds
    pub duration_ms: u64,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent for SemanticSearchPerformed {
    fn event_type(&self) -> &'static str {
        "SemanticSearchPerformed"
    }
    
    fn subject(&self) -> String {
        format!("search.semantic.performed.{}", self.search_id)
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.search_id
    }
}

/// Adapter to use agent domain's embedding service with cross-domain search
pub struct EmbeddingServiceAdapter<E> {
    embedding_service: Arc<E>,
    dimension: usize,
}

impl<E> EmbeddingServiceAdapter<E> {
    pub fn new(embedding_service: Arc<E>, dimension: usize) -> Self {
        Self {
            embedding_service,
            dimension,
        }
    }
}

/// Request to index content across domains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossDomainIndexRequest {
    /// The domain this content belongs to
    pub domain: String,
    
    /// The content to index
    pub content: String,
    
    /// Concept name
    pub concept_name: String,
    
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Bridge between semantic search implementations
#[async_trait]
pub trait SemanticSearchBridge: Send + Sync {
    /// Convert agent domain search results to cross-domain format
    async fn convert_to_cross_domain(
        &self,
        agent_results: Vec<serde_json::Value>,
        domain: &str,
    ) -> Result<Vec<DomainSearchResult>, DomainError>;
    
    /// Convert cross-domain results to agent domain format
    async fn convert_from_cross_domain(
        &self,
        cross_domain_results: CrossDomainResult,
    ) -> Result<serde_json::Value, DomainError>;
    
    /// Index content for cross-domain search
    async fn index_for_cross_domain(
        &self,
        request: CrossDomainIndexRequest,
    ) -> Result<Uuid, DomainError>;
}

/// Default implementation of semantic search bridge
pub struct DefaultSemanticSearchBridge {
    search_engine: Arc<CrossDomainSearchEngine>,
}

impl DefaultSemanticSearchBridge {
    pub fn new(search_engine: Arc<CrossDomainSearchEngine>) -> Self {
        Self { search_engine }
    }
}

#[async_trait]
impl SemanticSearchBridge for DefaultSemanticSearchBridge {
    async fn convert_to_cross_domain(
        &self,
        agent_results: Vec<serde_json::Value>,
        domain: &str,
    ) -> Result<Vec<DomainSearchResult>, DomainError> {
        let mut results = Vec::new();
        
        for agent_result in agent_results {
            // Extract fields from agent result
            let concept_name = agent_result.get("source_id")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            
            let similarity = agent_result.get("similarity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            
            let metadata = agent_result.get("metadata")
                .and_then(|v| v.as_object())
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            
            results.push(DomainSearchResult {
                domain: domain.to_string(),
                concept_id: Uuid::new_v4().to_string(),
                concept_name,
                similarity,
                metadata,
                cross_domain_links: Vec::new(),
            });
        }
        
        Ok(results)
    }
    
    async fn convert_from_cross_domain(
        &self,
        cross_domain_results: CrossDomainResult,
    ) -> Result<serde_json::Value, DomainError> {
        // Convert to a format compatible with agent domain
        let mut results = Vec::new();
        
        for (domain, domain_results) in cross_domain_results.domain_results {
            for result in domain_results {
                let agent_result = serde_json::json!({
                    "source_id": result.concept_name,
                    "source_type": domain,
                    "similarity": result.similarity,
                    "metadata": result.metadata,
                    "embedding_id": result.concept_id,
                });
                results.push(agent_result);
            }
        }
        
        Ok(serde_json::json!({
            "results": results,
            "total": cross_domain_results.metadata.total_results,
            "duration_ms": cross_domain_results.metadata.duration_ms,
            "domains_searched": cross_domain_results.metadata.domains_searched,
        }))
    }
    
    async fn index_for_cross_domain(
        &self,
        request: CrossDomainIndexRequest,
    ) -> Result<Uuid, DomainError> {
        // TODO: Implement actual indexing
        // This would:
        // 1. Use embedding service to generate vector
        // 2. Add concept to the appropriate domain's semantic analyzer
        // 3. Update any indices
        
        Ok(Uuid::new_v4())
    }
}

/// Query builder for cross-domain searches
pub struct CrossDomainQueryBuilder {
    query: String,
    start_domain: Option<String>,
    target_domains: Vec<String>,
    limit: Option<usize>,
    min_similarity: Option<f32>,
}

impl CrossDomainQueryBuilder {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            start_domain: None,
            target_domains: Vec::new(),
            limit: None,
            min_similarity: None,
        }
    }
    
    pub fn from_domain(mut self, domain: impl Into<String>) -> Self {
        self.start_domain = Some(domain.into());
        self
    }
    
    pub fn in_domains(mut self, domains: Vec<String>) -> Self {
        self.target_domains = domains;
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    
    pub fn with_min_similarity(mut self, threshold: f32) -> Self {
        self.min_similarity = Some(threshold);
        self
    }
    
    pub fn build(self) -> CrossDomainQuery {
        let mut config = crate::integration::cross_domain_search::SearchConfig::default();
        
        if let Some(limit) = self.limit {
            config.results_per_domain = limit;
        }
        
        if let Some(threshold) = self.min_similarity {
            config.min_similarity = threshold;
        }
        
        CrossDomainQuery {
            query: self.query,
            start_domain: self.start_domain,
            target_domains: self.target_domains,
            concept_vector: None,
            config_overrides: Some(config),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::EventBridge;
    
    #[tokio::test]
    async fn test_query_builder() {
        let query = CrossDomainQueryBuilder::new("test query")
            .from_domain("Sales")
            .in_domains(vec!["Billing".to_string(), "Shipping".to_string()])
            .with_limit(20)
            .with_min_similarity(0.8)
            .build();
        
        assert_eq!(query.query, "test query");
        assert_eq!(query.start_domain, Some("Sales".to_string()));
        assert_eq!(query.target_domains.len(), 2);
        
        let config = query.config_overrides.unwrap();
        assert_eq!(config.results_per_domain, 20);
        assert_eq!(config.min_similarity, 0.8);
    }
    
    #[tokio::test]
    async fn test_result_conversion() {
        let event_bridge = Arc::new(EventBridge::new(Default::default()));
        let search_engine = Arc::new(CrossDomainSearchEngine::new(
            event_bridge,
            Default::default(),
        ));
        
        let bridge = DefaultSemanticSearchBridge::new(search_engine);
        
        // Test converting from agent format
        let agent_results = vec![
            serde_json::json!({
                "source_id": "Order123",
                "similarity": 0.95,
                "metadata": {
                    "created_at": "2024-01-01",
                    "status": "pending"
                }
            }),
        ];
        
        let cross_domain_results = bridge.convert_to_cross_domain(
            agent_results,
            "Sales"
        ).await.unwrap();
        
        assert_eq!(cross_domain_results.len(), 1);
        assert_eq!(cross_domain_results[0].concept_name, "Order123");
        assert_eq!(cross_domain_results[0].similarity, 0.95);
        assert_eq!(cross_domain_results[0].domain, "Sales");
    }
}