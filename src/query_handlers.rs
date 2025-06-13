//! Query handlers for CIM domain aggregates
//!
//! Query handlers process queries and return data from read models/projections.
//! They implement the read side of CQRS, providing optimized data access.

use crate::{
    cqrs::{Query, QueryHandler as CqrsQueryHandler, QueryEnvelope, QueryAcknowledgment, QueryStatus, QueryId},
    errors::DomainResult,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

/// Query result type
pub type QueryResult<T> = Result<T, String>;

/// Event publisher trait for publishing query results
pub trait EventPublisher: Send + Sync {
    /// Publish query results as events
    fn publish_query_result(&self, query_id: QueryId, result: serde_json::Value) -> DomainResult<()>;
}

/// Mock event publisher for testing
#[derive(Clone)]
pub struct MockEventPublisher;

impl EventPublisher for MockEventPublisher {
    fn publish_query_result(&self, _query_id: QueryId, _result: serde_json::Value) -> DomainResult<()> {
        Ok(())
    }
}

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

// Location Queries and Views

/// Location view for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationView {
    /// Location's unique identifier
    pub location_id: Uuid,
    /// Name of the location
    pub name: String,
    /// Type of location (Physical, Virtual, etc.)
    pub location_type: String,
    /// Physical address if applicable
    pub address: Option<String>,
    /// Geographic coordinates (latitude, longitude)
    pub coordinates: Option<(f64, f64)>,
    /// Name of the parent location if any
    pub parent_location: Option<String>,
}

/// Query to find locations by type
#[derive(Debug, Clone)]
pub struct FindLocationsByType {
    /// The type of location to search for
    pub location_type: String,
    /// Maximum number of results to return
    pub limit: Option<usize>,
}

impl Query for FindLocationsByType {}

/// Handler for location queries
pub struct LocationQueryHandler<R: ReadModelStorage<LocationView>> {
    read_model: R,
    event_publisher: Arc<dyn EventPublisher>,
}

impl<R: ReadModelStorage<LocationView>> LocationQueryHandler<R> {
    /// Create a new location query handler
    pub fn new(read_model: R, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { read_model, event_publisher }
    }
}

impl<R: ReadModelStorage<LocationView>> DirectQueryHandler<FindLocationsByType, Vec<LocationView>> for LocationQueryHandler<R> {
    fn handle(&self, query: FindLocationsByType) -> QueryResult<Vec<LocationView>> {
        let criteria = QueryCriteria::new()
            .with_filter("location_type", query.location_type)
            .with_limit(query.limit.unwrap_or(100));

        Ok(self.read_model.query(&criteria))
    }
}

impl<R: ReadModelStorage<LocationView>> CqrsQueryHandler<FindLocationsByType> for LocationQueryHandler<R> {
    fn handle(&self, envelope: QueryEnvelope<FindLocationsByType>) -> QueryAcknowledgment {
        match DirectQueryHandler::<FindLocationsByType, Vec<LocationView>>::handle(self, envelope.query) {
            Ok(result) => {
                let result_json = serde_json::to_value(&result).unwrap();
                if let Err(e) = self.event_publisher.publish_query_result(envelope.id, result_json) {
                    return QueryAcknowledgment {
                        query_id: envelope.id,
                        correlation_id: envelope.correlation_id,
                        status: QueryStatus::Rejected,
                        reason: Some(format!("Failed to publish result: {}", e)),
                    };
                }

                QueryAcknowledgment {
                    query_id: envelope.id,
                    correlation_id: envelope.correlation_id,
                    status: QueryStatus::Accepted,
                    reason: None,
                }
            }
            Err(e) => QueryAcknowledgment {
                query_id: envelope.id,
                correlation_id: envelope.correlation_id,
                status: QueryStatus::Rejected,
                reason: Some(e),
            },
        }
    }
}





// Workflow Queries and Views have been moved to cim-domain-workflow

#[cfg(test)]
mod tests {
    use super::*;





    // Workflow query tests have been moved to cim-domain-workflow

    #[test]
    fn test_location_type_query() {
        // Setup
        let read_model = InMemoryReadModel::<LocationView>::new();
        let handler = LocationQueryHandler::new(read_model.clone(), Arc::new(MockEventPublisher));

        // Insert test data
        let location_view = LocationView {
            location_id: Uuid::new_v4(),
            name: "Main Office".to_string(),
            location_type: "Physical".to_string(),
            address: Some("123 Main St".to_string()),
            coordinates: Some((40.7128, -74.0060)),
            parent_location: None,
        };

        read_model.insert(location_view.location_id.to_string(), location_view);

        // Test query
        let query = FindLocationsByType {
            location_type: "Physical".to_string(),
            limit: Some(10),
        };

        let result = DirectQueryHandler::<FindLocationsByType, Vec<LocationView>>::handle(&handler, query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Main Office");
    }
}
