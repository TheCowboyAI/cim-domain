//! Query support for persistence layer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Query options for filtering and pagination
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryOptions {
    /// Filter conditions
    pub filters: HashMap<String, serde_json::Value>,
    /// Sort field and direction
    pub sort_by: Option<(String, SortDirection)>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Number of results to skip
    pub offset: Option<usize>,
    /// Include only specific fields
    pub projection: Option<Vec<String>>,
    /// Time range filter
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    /// Sort in ascending order
    Ascending,
    /// Sort in descending order
    Descending,
}

/// Query result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// The actual results
    pub items: Vec<T>,
    /// Total count (before pagination)
    pub total_count: usize,
    /// Whether there are more results
    pub has_more: bool,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl<T> QueryResult<T> {
    /// Create a new query result
    pub fn new(items: Vec<T>, total_count: usize, execution_time_ms: u64) -> Self {
        let has_more = items.len() < total_count;
        Self {
            items,
            total_count,
            has_more,
            execution_time_ms,
        }
    }
    
    /// Map the items to a different type
    pub fn map<U, F>(self, f: F) -> QueryResult<U>
    where
        F: FnMut(T) -> U,
    {
        QueryResult {
            items: self.items.into_iter().map(f).collect(),
            total_count: self.total_count,
            has_more: self.has_more,
            execution_time_ms: self.execution_time_ms,
        }
    }
}

/// Builder for query options
pub struct QueryBuilder {
    options: QueryOptions,
}

impl QueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            options: QueryOptions::default(),
        }
    }
    
    /// Add a filter condition
    pub fn filter(mut self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.options.filters.insert(field.into(), value);
        self
    }
    
    /// Add sorting
    pub fn sort_by(mut self, field: impl Into<String>, direction: SortDirection) -> Self {
        self.options.sort_by = Some((field.into(), direction));
        self
    }
    
    /// Set the limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.options.limit = Some(limit);
        self
    }
    
    /// Set the offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.options.offset = Some(offset);
        self
    }
    
    /// Set the projection fields
    pub fn project(mut self, fields: Vec<String>) -> Self {
        self.options.projection = Some(fields);
        self
    }
    
    /// Set time range filter
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.options.time_range = Some((start, end));
        self
    }
    
    /// Build the query options
    pub fn build(self) -> QueryOptions {
        self.options
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Pagination helper
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pagination {
    /// Current page (1-based)
    pub page: usize,
    /// Items per page
    pub per_page: usize,
    /// Total pages
    pub total_pages: usize,
    /// Total items
    pub total_items: usize,
}

impl Pagination {
    /// Create pagination from limit/offset
    pub fn from_query(limit: usize, offset: usize, total_items: usize) -> Self {
        let per_page = limit.max(1);
        let page = (offset / per_page) + 1;
        let total_pages = (total_items + per_page - 1) / per_page;
        
        Self {
            page,
            per_page,
            total_pages,
            total_items,
        }
    }
    
    /// Convert to limit/offset
    pub fn to_limit_offset(&self) -> (usize, usize) {
        let limit = self.per_page;
        let offset = (self.page - 1) * self.per_page;
        (limit, offset)
    }
    
    /// Check if there's a next page
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }
    
    /// Check if there's a previous page
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_query_builder() {
        let options = QueryBuilder::new()
            .filter("status", serde_json::json!("active"))
            .filter("type", serde_json::json!("premium"))
            .sort_by("created_at", SortDirection::Descending)
            .limit(10)
            .offset(20)
            .build();
        
        assert_eq!(options.filters.len(), 2);
        assert_eq!(options.limit, Some(10));
        assert_eq!(options.offset, Some(20));
        assert!(options.sort_by.is_some());
    }
    
    #[test]
    fn test_query_result_map() {
        let result = QueryResult::new(
            vec![1, 2, 3],
            10,
            50,
        );
        
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6]);
        assert_eq!(mapped.total_count, 10);
        assert!(mapped.has_more);
    }
    
    #[test]
    fn test_pagination() {
        let pagination = Pagination::from_query(10, 20, 100);
        assert_eq!(pagination.page, 3);
        assert_eq!(pagination.per_page, 10);
        assert_eq!(pagination.total_pages, 10);
        assert!(pagination.has_next());
        assert!(pagination.has_prev());
        
        let (limit, offset) = pagination.to_limit_offset();
        assert_eq!(limit, 10);
        assert_eq!(offset, 20);
    }
}