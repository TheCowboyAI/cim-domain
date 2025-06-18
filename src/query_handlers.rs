//! Query handlers for CIM domain aggregates
//!
//! Query handlers process queries and return data from read models/projections.
//! They implement the read side of CQRS, providing optimized data access.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};

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
        self.filters.insert(key.into(), serde_json::to_value(value).unwrap());
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
    use super::*;

    // Workflow query tests have been moved to cim-domain-workflow

    // Location query tests have been moved to cim-domain-location
}
