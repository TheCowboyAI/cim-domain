// Copyright 2025 Cowboy AI, LLC.

//! Bridge between cross-domain search and agent domain semantic search
//!
//! This module provides adapters to connect the category theory-based
//! cross-domain search with the agent domain's vector-based semantic search.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::cross_domain_search::{
    CrossDomainQuery, CrossDomainResult, CrossDomainSearchEngine, DomainSearchResult,
};
use crate::errors::DomainError;
use crate::events::DomainEvent;

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
    _embedding_service: Arc<E>,
    _dimension: usize,
}

impl<E> EmbeddingServiceAdapter<E> {
    /// Create a new embedding service adapter
    ///
    /// # Arguments
    /// * `embedding_service` - The underlying embedding service
    /// * `dimension` - Dimensionality of the embeddings
    pub fn new(embedding_service: Arc<E>, dimension: usize) -> Self {
        Self {
            _embedding_service: embedding_service,
            _dimension: dimension,
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
    _search_engine: Arc<CrossDomainSearchEngine>,
}

impl DefaultSemanticSearchBridge {
    /// Create a new default semantic search bridge
    ///
    /// # Arguments
    /// * `search_engine` - The cross-domain search engine to use
    pub fn new(search_engine: Arc<CrossDomainSearchEngine>) -> Self {
        Self {
            _search_engine: search_engine,
        }
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
            let concept_name = agent_result
                .get("source_id")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let similarity = agent_result
                .get("similarity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;

            let metadata = agent_result
                .get("metadata")
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
        _request: CrossDomainIndexRequest,
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
    /// Create a new cross-domain query builder
    ///
    /// # Arguments
    /// * `query` - The search query text
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            start_domain: None,
            target_domains: Vec::new(),
            limit: None,
            min_similarity: None,
        }
    }

    /// Specify the starting domain for the search
    ///
    /// # Arguments
    /// * `domain` - The domain to start searching from
    pub fn from_domain(mut self, domain: impl Into<String>) -> Self {
        self.start_domain = Some(domain.into());
        self
    }

    /// Specify target domains to search within
    ///
    /// # Arguments
    /// * `domains` - List of domains to search
    pub fn in_domains(mut self, domains: Vec<String>) -> Self {
        self.target_domains = domains;
        self
    }

    /// Set the maximum number of results per domain
    ///
    /// # Arguments
    /// * `limit` - Maximum results to return per domain
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the minimum similarity threshold
    ///
    /// # Arguments
    /// * `threshold` - Minimum similarity score (0.0 to 1.0)
    pub fn with_min_similarity(mut self, threshold: f32) -> Self {
        self.min_similarity = Some(threshold);
        self
    }

    /// Build the final cross-domain query
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
        let agent_results = vec![serde_json::json!({
            "source_id": "Order123",
            "similarity": 0.95,
            "metadata": {
                "created_at": "2024-01-01",
                "status": "pending"
            }
        })];

        let cross_domain_results = bridge
            .convert_to_cross_domain(agent_results, "Sales")
            .await
            .unwrap();

        assert_eq!(cross_domain_results.len(), 1);
        assert_eq!(cross_domain_results[0].concept_name, "Order123");
        assert_eq!(cross_domain_results[0].similarity, 0.95);
        assert_eq!(cross_domain_results[0].domain, "Sales");
    }

    #[test]
    fn test_semantic_search_performed_event() {
        let event = SemanticSearchPerformed {
            search_id: Uuid::new_v4(),
            query: "test query".to_string(),
            domains_searched: vec!["Sales".to_string(), "Billing".to_string()],
            total_results: 15,
            duration_ms: 150,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(event.event_type(), "SemanticSearchPerformed");
        assert!(event.subject().starts_with("search.semantic.performed."));
        assert_eq!(event.aggregate_id(), event.search_id);
    }

    #[tokio::test]
    async fn test_convert_from_cross_domain() {
        let event_bridge = Arc::new(EventBridge::new(Default::default()));
        let search_engine = Arc::new(CrossDomainSearchEngine::new(
            event_bridge,
            Default::default(),
        ));

        let bridge = DefaultSemanticSearchBridge::new(search_engine);

        // Create mock cross-domain results
        let mut domain_results = std::collections::HashMap::new();
        domain_results.insert(
            "Sales".to_string(),
            vec![DomainSearchResult {
                domain: "Sales".to_string(),
                concept_id: Uuid::new_v4().to_string(),
                concept_name: "Order123".to_string(),
                similarity: 0.95,
                metadata: std::collections::HashMap::new(),
                cross_domain_links: Vec::new(),
            }],
        );
        domain_results.insert(
            "Billing".to_string(),
            vec![DomainSearchResult {
                domain: "Billing".to_string(),
                concept_id: Uuid::new_v4().to_string(),
                concept_name: "Invoice456".to_string(),
                similarity: 0.88,
                metadata: std::collections::HashMap::new(),
                cross_domain_links: Vec::new(),
            }],
        );

        let cross_domain_result = CrossDomainResult {
            domain_results,
            relationships: Vec::new(),
            aggregated_concepts: Vec::new(),
            metadata: crate::integration::cross_domain_search::SearchMetadata {
                query: CrossDomainQuery {
                    query: "test".to_string(),
                    start_domain: None,
                    target_domains: vec![],
                    concept_vector: None,
                    config_overrides: None,
                },
                total_results: 2,
                domains_searched: vec!["Sales".to_string(), "Billing".to_string()],
                duration_ms: 100,
                truncated: false,
                max_depth_reached: 1,
            },
        };

        let agent_format = bridge
            .convert_from_cross_domain(cross_domain_result)
            .await
            .unwrap();

        assert!(agent_format.is_object());
        assert_eq!(agent_format["total"], 2);
        assert_eq!(agent_format["duration_ms"], 100);

        let results = agent_format["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);

        // Check first result
        let first = &results[0];
        assert!(
            first["source_id"].as_str().unwrap() == "Order123"
                || first["source_id"].as_str().unwrap() == "Invoice456"
        );
    }

    #[tokio::test]
    async fn test_convert_with_missing_fields() {
        let event_bridge = Arc::new(EventBridge::new(Default::default()));
        let search_engine = Arc::new(CrossDomainSearchEngine::new(
            event_bridge,
            Default::default(),
        ));

        let bridge = DefaultSemanticSearchBridge::new(search_engine);

        // Test with missing fields in agent results
        let agent_results = vec![
            serde_json::json!({
                // Missing source_id
                "similarity": 0.75,
            }),
            serde_json::json!({
                "source_id": "Test",
                // Missing similarity
            }),
        ];

        let cross_domain_results = bridge
            .convert_to_cross_domain(agent_results, "TestDomain")
            .await
            .unwrap();

        assert_eq!(cross_domain_results.len(), 2);
        assert_eq!(cross_domain_results[0].concept_name, "Unknown");
        assert_eq!(cross_domain_results[0].similarity, 0.75);
        assert_eq!(cross_domain_results[1].concept_name, "Test");
        assert_eq!(cross_domain_results[1].similarity, 0.0);
    }

    #[test]
    fn test_cross_domain_index_request() {
        let request = CrossDomainIndexRequest {
            domain: "Sales".to_string(),
            content: "New order from customer ABC".to_string(),
            concept_name: "Order789".to_string(),
            metadata: serde_json::json!({
                "customer": "ABC",
                "amount": 100.50
            }),
        };

        assert_eq!(request.domain, "Sales");
        assert_eq!(request.concept_name, "Order789");
        assert!(request.metadata["customer"].as_str().unwrap() == "ABC");
    }

    #[tokio::test]
    async fn test_index_for_cross_domain() {
        let event_bridge = Arc::new(EventBridge::new(Default::default()));
        let search_engine = Arc::new(CrossDomainSearchEngine::new(
            event_bridge,
            Default::default(),
        ));

        let bridge = DefaultSemanticSearchBridge::new(search_engine);

        let request = CrossDomainIndexRequest {
            domain: "TestDomain".to_string(),
            content: "Test content".to_string(),
            concept_name: "TestConcept".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = bridge.index_for_cross_domain(request).await.unwrap();

        // Should return a valid UUID
        assert!(!result.to_string().is_empty());
    }

    #[test]
    fn test_embedding_service_adapter() {
        struct MockEmbedding;

        let mock_service = Arc::new(MockEmbedding);
        let adapter = EmbeddingServiceAdapter::new(mock_service, 512);

        assert_eq!(adapter._dimension, 512);
    }

    #[test]
    fn test_query_builder_default() {
        let query = CrossDomainQueryBuilder::new("search text").build();

        assert_eq!(query.query, "search text");
        assert!(query.start_domain.is_none());
        assert!(query.target_domains.is_empty());
        assert!(query.concept_vector.is_none());

        let config = query.config_overrides.unwrap();
        assert_eq!(config.results_per_domain, 10); // Default
        assert_eq!(config.min_similarity, 0.7); // Default
    }

    #[test]
    fn test_query_builder_chaining() {
        let query = CrossDomainQueryBuilder::new("complex search")
            .from_domain("StartDomain")
            .in_domains(vec!["A".to_string(), "B".to_string(), "C".to_string()])
            .with_limit(50)
            .with_min_similarity(0.9)
            .build();

        assert_eq!(query.query, "complex search");
        assert_eq!(query.start_domain.unwrap(), "StartDomain");
        assert_eq!(query.target_domains.len(), 3);

        let config = query.config_overrides.unwrap();
        assert_eq!(config.results_per_domain, 50);
        assert_eq!(config.min_similarity, 0.9);
    }

    #[test]
    fn test_semantic_search_event_serialization() {
        let event = SemanticSearchPerformed {
            search_id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            query: "test".to_string(),
            domains_searched: vec!["D1".to_string()],
            total_results: 5,
            duration_ms: 50,
            timestamp: chrono::Utc::now(),
        };

        // Test serialization
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(json.contains("test"));

        // Test deserialization
        let deserialized: SemanticSearchPerformed = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.search_id, event.search_id);
        assert_eq!(deserialized.query, event.query);
        assert_eq!(deserialized.total_results, 5);
    }
}
