/// Minimal common types for examples - to be copied into each example file
/// This avoids module/import conflicts while keeping examples self-contained

use serde::{Deserialize, Serialize};
use async_trait::async_trait;

// Core domain types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent {
    pub fn new(aggregate_id: String, payload: serde_json::Value) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            aggregate_id,
            event_type: "DomainEvent".into(),
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DomainError {
    ValidationError(String),
    Infrastructure(String),
    NotFound(String),
    Conflict(String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            DomainError::Infrastructure(msg) => write!(f, "Infrastructure error: {}", msg),
            DomainError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DomainError::Conflict(msg) => write!(f, "Conflict: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

impl From<serde_json::Error> for DomainError {
    fn from(err: serde_json::Error) -> Self {
        DomainError::Infrastructure(err.to_string())
    }
}

// Event store trait
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError>;
    async fn get_events(&self, aggregate_id: &str) -> Result<Vec<DomainEvent>, DomainError>;
}

// In-memory event store for examples
#[derive(Debug, Clone)]
pub struct InMemoryEventStore {
    events: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<DomainEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(&self, events: Vec<DomainEvent>) -> Result<(), DomainError> {
        let mut store = self.events.write().await;
        for event in events {
            let entry = store.entry(event.aggregate_id.clone()).or_insert_with(Vec::new);
            entry.push(event);
        }
        Ok(())
    }

    async fn get_events(&self, aggregate_id: &str) -> Result<Vec<DomainEvent>, DomainError> {
        let store = self.events.read().await;
        Ok(store.get(aggregate_id).cloned().unwrap_or_default())
    }
}

// Component types
pub type ComponentId = String;
pub type DomainContext = String;