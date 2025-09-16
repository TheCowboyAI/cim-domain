// Copyright 2025 Cowboy AI, LLC.

//! Query handlers for CIM domain aggregates
//!
//! Query handlers process queries and return data from read models/projections.
//! They implement the read side of CQRS, providing optimized data access.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Query result type - represents the outcome of a query operation
pub type QueryResult<T> = Result<T, String>;

/// Query handler trait that returns data directly (for internal use)
pub trait DirectQueryHandler<Q, R> {
    /// Handle the query and return the result
    fn handle(&self, query: Q) -> QueryResult<R>;
}

/// Read model storage trait
pub trait ReadModelStorage<T>: Send + Sync {
    /// Get an item by ID
    fn get(&self, id: &str) -> Option<T>;

    /// Query items by criteria
    fn query(&self, criteria: &QueryCriteria) -> Vec<T>;

    /// Get all items
    fn all(&self) -> Vec<T>;
}

/// Query criteria for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCriteria {
    /// Filter conditions as key-value pairs
    pub filters: HashMap<String, serde_json::Value>,
    /// Maximum number of results to return
    pub limit: Option<usize>,
    /// Number of results to skip
    pub offset: Option<usize>,
    /// Field to order results by
    pub order_by: Option<String>,
}

impl Default for QueryCriteria {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryCriteria {
    /// Create a new empty query criteria
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
            limit: None,
            offset: None,
            order_by: None,
        }
    }

    /// Add a filter condition
    pub fn with_filter(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.filters
            .insert(key.into(), serde_json::to_value(value).unwrap());
        self
    }

    /// Set the result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// In-memory read model storage
#[derive(Clone)]
pub struct InMemoryReadModel<T: Clone> {
    storage: Arc<RwLock<HashMap<String, T>>>,
}

impl<T: Clone> Default for InMemoryReadModel<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> InMemoryReadModel<T> {
    /// Create a new in-memory read model
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert an item into the read model
    pub fn insert(&self, id: String, item: T) {
        self.storage.write().unwrap().insert(id, item);
    }
}

impl<T: Clone + Send + Sync> ReadModelStorage<T> for InMemoryReadModel<T> {
    fn get(&self, id: &str) -> Option<T> {
        self.storage.read().unwrap().get(id).cloned()
    }

    fn query(&self, criteria: &QueryCriteria) -> Vec<T> {
        let storage = self.storage.read().unwrap();
        let mut results: Vec<T> = storage.values().cloned().collect();

        // Apply limit
        if let Some(limit) = criteria.limit {
            results.truncate(limit);
        }

        results
    }

    fn all(&self) -> Vec<T> {
        self.storage.read().unwrap().values().cloned().collect()
    }
}

// Location Queries and Views have been moved to cim-domain-location

// Workflow Queries and Views have been moved to cim-domain-workflow

#[cfg(test)]
mod tests {
    // Read-path tests (pure, no IO)
    use super::*;
    use crate::cqrs::{self, AggregateTransactionId, Query as CqrsQuery, QueryEnvelope, QueryHandler as CqrsQueryHandler, QueryResponse};
    use uuid::Uuid;

    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    struct Item { id: String, v: i32 }

    #[test]
    fn in_memory_read_model_basic_ops() {
        let rm: InMemoryReadModel<Item> = InMemoryReadModel::new();
        rm.insert("a".into(), Item { id: "a".into(), v: 1 });
        rm.insert("b".into(), Item { id: "b".into(), v: 2 });
        assert_eq!(rm.get("a").unwrap().v, 1);
        let res = rm.query(&QueryCriteria::new().with_limit(1));
        assert_eq!(res.len(), 1);
        assert_eq!(rm.all().len(), 2);
    }

    #[derive(Clone, Debug)]
    struct GetTop { n: usize }
    impl CqrsQuery for GetTop {}

    struct ItemsHandler { rm: InMemoryReadModel<Item> }
    impl ItemsHandler { fn new(rm: InMemoryReadModel<Item>) -> Self { Self { rm } } }

    impl CqrsQueryHandler<GetTop> for ItemsHandler {
        fn handle(&self, envelope: QueryEnvelope<GetTop>) -> QueryResponse {
            let mut items = self.rm.all();
            items.sort_by_key(|i| i.id.clone());
            items.truncate(envelope.query.n);
            let result = serde_json::to_value(items).unwrap();
            QueryResponse { query_id: *envelope.id.as_uuid(), correlation_id: envelope.identity.correlation_id, result }
        }
    }

    #[test]
    fn query_path_responds_with_data() {
        // Arrange read model
        let rm: InMemoryReadModel<Item> = InMemoryReadModel::new();
        rm.insert("b".into(), Item { id: "b".into(), v: 2 });
        rm.insert("a".into(), Item { id: "a".into(), v: 1 });

        // Build query envelope
        let q = GetTop { n: 1 };
        let tx = AggregateTransactionId(Uuid::new_v4());
        let env = QueryEnvelope::new_in_tx(q, "tester".into(), tx);

        // Handle and assert
        let handler = ItemsHandler::new(rm);
        let resp = CqrsQueryHandler::handle(&handler, env);
        let arr = resp.result.as_array().expect("array result");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"].as_str(), Some("a"));
    }
}
