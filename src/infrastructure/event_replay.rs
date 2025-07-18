// Copyright 2025 Cowboy AI, LLC.

//! Event replay service for rebuilding aggregates and projections

use crate::infrastructure::{EventStore, EventStoreError, StoredEvent};
use crate::domain_events::DomainEventEnum;
use crate::events::DomainEvent;
use async_trait::async_trait;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;
use tokio::sync::RwLock;
use serde_json;

/// Errors that can occur during event replay
#[derive(Debug, Error)]
pub enum ReplayError {
    /// Error from the underlying event store
    #[error("Event store error: {0}")]
    EventStoreError(#[from] EventStoreError),

    /// General replay failure with description
    #[error("Replay failed: {0}")]
    ReplayFailed(String),

    /// Aggregate not found in event store
    #[error("Aggregate not found: {0}")]
    AggregateNotFound(String),

    /// Error occurred in event handler during processing
    #[error("Event handler error: {0}")]
    EventHandlerError(String),

    /// Failed to deserialize event data
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

/// Statistics collected during event replay
#[derive(Debug, Clone)]
pub struct ReplayStats {
    /// Total number of events processed
    pub events_processed: u64,
    /// Number of aggregates rebuilt from events
    pub aggregates_rebuilt: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Total duration of replay in milliseconds
    pub duration_ms: u64,
    /// Average events processed per second
    pub events_per_second: f64,
}

/// Options for controlling replay behavior
#[derive(Debug, Clone)]
pub struct ReplayOptions {
    /// Maximum events to process (None = all)
    pub max_events: Option<u64>,

    /// Batch size for processing
    pub batch_size: usize,

    /// Whether to continue on errors
    pub continue_on_error: bool,

    /// Filter by aggregate types
    pub aggregate_types: Option<Vec<String>>,

    /// Filter by event types
    pub event_types: Option<Vec<String>>,

    /// Start from specific sequence
    pub from_sequence: Option<u64>,
}

impl Default for ReplayOptions {
    fn default() -> Self {
        Self {
            max_events: None,
            batch_size: 100,
            continue_on_error: false,
            aggregate_types: None,
            event_types: None,
            from_sequence: None,
        }
    }
}

/// Trait for handling events during replay
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle a single event
    async fn handle_event(&mut self, event: &StoredEvent) -> Result<(), ReplayError>;

    /// Called when replay starts
    async fn on_replay_start(&mut self) -> Result<(), ReplayError> {
        Ok(())
    }

    /// Called when replay completes
    async fn on_replay_complete(&mut self, _stats: &ReplayStats) -> Result<(), ReplayError> {
        // Default implementation does nothing
        Ok(())
    }
}

/// Handler that rebuilds aggregates from events
pub struct AggregateRebuilder {
    /// Map of aggregate ID to version
    pub aggregate_versions: Arc<RwLock<HashMap<String, u64>>>,

    /// Custom event processors by aggregate type
    pub processors: HashMap<String, Box<dyn AggregateEventProcessor>>,
}

/// Generic aggregate processor that tracks events by type
pub struct GenericAggregateProcessor {
    _aggregate_type: String,
    aggregates: HashMap<String, Vec<DomainEventEnum>>,
}

impl GenericAggregateProcessor {
    /// Create a new generic processor for an aggregate type
    pub fn new(aggregate_type: &str) -> Self {
        Self {
            _aggregate_type: aggregate_type.to_string(),
            aggregates: HashMap::new(),
        }
    }
}

#[async_trait]
impl AggregateEventProcessor for GenericAggregateProcessor {
    async fn process_event(&mut self, event: &DomainEventEnum) -> Result<(), ReplayError> {
        // Extract aggregate ID from event
        let aggregate_id = match event {
            DomainEventEnum::WorkflowStarted(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowTransitionExecuted(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowTransitioned(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowCompleted(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowSuspended(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowResumed(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowCancelled(e) => e.workflow_id.to_string(),
            DomainEventEnum::WorkflowFailed(e) => e.workflow_id.to_string(),
        };
        
        // Store event for the aggregate
        self.aggregates
            .entry(aggregate_id)
            .or_default()
            .push(event.clone());
        
        Ok(())
    }

    async fn get_aggregate(&self, aggregate_id: &str) -> Option<Box<dyn std::any::Any + Send>> {
        self.aggregates.get(aggregate_id)
            .map(|events| Box::new(events.clone()) as Box<dyn std::any::Any + Send>)
    }
}

/// Trait for processing events for specific aggregate types
#[async_trait]
pub trait AggregateEventProcessor: Send + Sync {
    /// Process an event for this aggregate type
    async fn process_event(&mut self, event: &DomainEventEnum) -> Result<(), ReplayError>;

    /// Get the current state of an aggregate
    async fn get_aggregate(&self, aggregate_id: &str) -> Option<Box<dyn std::any::Any + Send>>;
}

#[async_trait]
impl EventHandler for AggregateRebuilder {
    async fn handle_event(&mut self, stored_event: &StoredEvent) -> Result<(), ReplayError> {
        // Get the event directly - it's already deserialized
        let event = &stored_event.event;

        // Update version tracking
        let mut versions = self.aggregate_versions.write().await;
        versions.insert(stored_event.aggregate_id.clone(), stored_event.sequence);
        drop(versions);

        // Find processor for this aggregate type
        let aggregate_type = stored_event.aggregate_type.clone();
        if let Some(processor) = self.processors.get_mut(&aggregate_type) {
            processor.process_event(event).await?;
        }

        Ok(())
    }

    async fn on_replay_complete(&mut self, stats: &ReplayStats) -> Result<(), ReplayError> {
        // Log replay completion statistics
        let versions = self.aggregate_versions.read().await;
        let aggregates_count = versions.len() as u64;

        // In production, this would log to monitoring system
        if stats.errors > 0 {
            eprintln!("Replay completed with errors: {} events processed, {} errors, {} aggregates rebuilt in {}ms ({:.2} events/sec)", 
                stats.events_processed, stats.errors, aggregates_count, stats.duration_ms, stats.events_per_second);
        } else {
            println!("Replay completed successfully: {} events processed, {} aggregates rebuilt in {}ms ({:.2} events/sec)", 
                stats.events_processed, aggregates_count, stats.duration_ms, stats.events_per_second);
        }

        Ok(())
    }
}

/// Handler that builds projections from events
pub struct ProjectionBuilder {
    /// Projection handlers by name
    pub projections: HashMap<String, Box<dyn ProjectionHandler>>,

    /// Track last processed sequence per projection
    pub checkpoints: Arc<RwLock<HashMap<String, u64>>>,
}

/// Trait for projection handlers
#[async_trait]
pub trait ProjectionHandler: Send + Sync {
    /// Handle an event for this projection
    async fn handle_event(&mut self, event: &DomainEventEnum, sequence: u64) -> Result<(), ReplayError>;

    /// Get the name of this projection
    fn name(&self) -> &str;

    /// Reset the projection state
    async fn reset(&mut self) -> Result<(), ReplayError>;
}

#[async_trait]
impl EventHandler for ProjectionBuilder {
    async fn handle_event(&mut self, stored_event: &StoredEvent) -> Result<(), ReplayError> {
        // Get the event directly - it's already deserialized
        let event = &stored_event.event;

        // Process event in all projections
        for (name, projection) in self.projections.iter_mut() {
            projection.handle_event(event, stored_event.sequence).await?;

            // Update checkpoint
            let mut checkpoints = self.checkpoints.write().await;
            checkpoints.insert(name.to_string(), stored_event.sequence);
        }

        Ok(())
    }

    async fn on_replay_start(&mut self) -> Result<(), ReplayError> {
        // Reset all projections
        for projection in self.projections.values_mut() {
            projection.reset().await?;
        }
        Ok(())
    }
}

/// Service for replaying events from the event store
pub struct EventReplayService {
    event_store: Arc<dyn EventStore>,
}

impl EventReplayService {
    /// Create a new event replay service with the given event store
    pub fn new(event_store: Arc<dyn EventStore>) -> Self {
        Self { event_store }
    }

    /// Replay events with a custom handler
    pub async fn replay_with_handler(
        &self,
        handler: &mut dyn EventHandler,
        options: ReplayOptions,
    ) -> Result<ReplayStats, ReplayError> {
        let start_time = Instant::now();
        let mut stats = ReplayStats {
            events_processed: 0,
            aggregates_rebuilt: 0,
            errors: 0,
            duration_ms: 0,
            events_per_second: 0.0,
        };

        // Notify handler of replay start
        handler.on_replay_start().await?;

        // Create event stream
        let mut event_stream = self.event_store
            .stream_all_events(options.from_sequence)
            .await
            .map_err(ReplayError::EventStoreError)?;

        let mut batch = Vec::with_capacity(options.batch_size);
        let mut total_processed = 0u64;

        // Process events in batches
        while let Some(result) = event_stream.next().await {
            match result {
                Ok(event) => {
                    // Apply filters
                    if let Some(ref types) = options.aggregate_types {
                        if !types.contains(&event.aggregate_type) {
                            continue;
                        }
                    }

                    if let Some(ref types) = options.event_types {
                        if !types.contains(&event.event.event_type().to_string()) {
                            continue;
                        }
                    }

                    batch.push(event);

                    // Process batch when full
                    if batch.len() >= options.batch_size {
                        self.process_batch(&mut batch, handler, &mut stats, options.continue_on_error).await?;
                        total_processed += batch.len() as u64;

                        // Check max events limit
                        if let Some(max) = options.max_events {
                            if total_processed >= max {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    if !options.continue_on_error {
                        return Err(ReplayError::EventStoreError(e));
                    }
                }
            }
        }

        // Process remaining events
        if !batch.is_empty() {
            self.process_batch(&mut batch, handler, &mut stats, options.continue_on_error).await?;
        }

        // Calculate final stats
        stats.duration_ms = start_time.elapsed().as_millis() as u64;
        stats.events_per_second = if stats.duration_ms > 0 {
            (stats.events_processed as f64 * 1000.0) / stats.duration_ms as f64
        } else {
            0.0
        };

        // Notify handler of completion
        handler.on_replay_complete(&stats).await?;

        Ok(stats)
    }

    /// Process a batch of events
    async fn process_batch(
        &self,
        batch: &mut Vec<StoredEvent>,
        handler: &mut dyn EventHandler,
        stats: &mut ReplayStats,
        continue_on_error: bool,
    ) -> Result<(), ReplayError> {
        for event in batch.drain(..) {
            match handler.handle_event(&event).await {
                Ok(()) => {
                    stats.events_processed += 1;
                }
                Err(e) => {
                    stats.errors += 1;
                    if !continue_on_error {
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Replay all events to rebuild aggregates
    pub async fn replay_all_aggregates(
        &self,
        options: ReplayOptions,
    ) -> Result<ReplayStats, ReplayError> {
        let aggregate_versions = Arc::new(RwLock::new(HashMap::new()));
        let mut processors: HashMap<String, Box<dyn AggregateEventProcessor>> = HashMap::new();

        // Register aggregate processors for each domain
        processors.insert("Person".to_string(), Box::new(GenericAggregateProcessor::new("Person")));
        processors.insert("Organization".to_string(), Box::new(GenericAggregateProcessor::new("Organization")));
        processors.insert("Document".to_string(), Box::new(GenericAggregateProcessor::new("Document")));
        processors.insert("Graph".to_string(), Box::new(GenericAggregateProcessor::new("Graph")));
        processors.insert("Workflow".to_string(), Box::new(GenericAggregateProcessor::new("Workflow")));
        processors.insert("Agent".to_string(), Box::new(GenericAggregateProcessor::new("Agent")));
        processors.insert("Dialog".to_string(), Box::new(GenericAggregateProcessor::new("Dialog")));
        processors.insert("Location".to_string(), Box::new(GenericAggregateProcessor::new("Location")));
        processors.insert("ConceptualSpace".to_string(), Box::new(GenericAggregateProcessor::new("ConceptualSpace")));
        processors.insert("Policy".to_string(), Box::new(GenericAggregateProcessor::new("Policy")));
        processors.insert("NixConfiguration".to_string(), Box::new(GenericAggregateProcessor::new("NixConfiguration")));
        
        let mut rebuilder = AggregateRebuilder {
            aggregate_versions,
            processors,
        };

        self.replay_with_handler(&mut rebuilder, options).await
    }

    /// Replay events to rebuild a specific aggregate
    pub async fn replay_aggregate(
        &self,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, ReplayError> {
        let events = self.event_store
            .get_events(aggregate_id, None)
            .await
            .map_err(ReplayError::EventStoreError)?;

        if events.is_empty() {
            return Err(ReplayError::AggregateNotFound(aggregate_id.to_string()));
        }

        Ok(events)
    }

    /// Get replay progress for a projection
    pub async fn get_projection_checkpoint(
        &self,
        projection_name: &str,
    ) -> Result<Option<u64>, ReplayError> {
        // Query the checkpoint from the event store metadata
        // This would typically be stored in a separate checkpoint stream
        let _checkpoint_key = format!("checkpoint.{projection_name}");

        // For now, we'll use a simple in-memory approach
        // In production, this would query NATS KV store or similar
        Ok(None)
    }

    /// Save projection checkpoint
    pub async fn save_projection_checkpoint(
        &self,
        projection_name: &str,
        sequence: u64,
    ) -> Result<(), ReplayError> {
        // Save the checkpoint to persistent storage
        let _checkpoint_key = format!("checkpoint.{projection_name}");
        let _checkpoint_data = serde_json::json!({
            "projection": projection_name,
            "sequence": sequence,
            "timestamp": chrono::Utc::now(),
        });

        // In production, this would save to NATS KV store or similar
        // For now, we just validate the inputs
        if projection_name.is_empty() {
            return Err(ReplayError::ReplayFailed("Projection name cannot be empty".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::tests::MockEventStore;
    use crate::infrastructure::EventMetadata;
    use crate::domain_events::WorkflowStarted;
    use crate::identifiers::{WorkflowId, GraphId};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestHandler {
        events_handled: AtomicU64,
    }

    #[async_trait]
    impl EventHandler for TestHandler {
        async fn handle_event(&mut self, _event: &StoredEvent) -> Result<(), ReplayError> {
            self.events_handled.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_replay_with_handler() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventReplayService::new(event_store.clone());

        // Add some test events
        let workflow_id = WorkflowId::new();
        let event = DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id,
            definition_id: GraphId::new(),
            initial_state: "Start".to_string(),
            started_at: chrono::Utc::now(),
        });

        event_store.append_events(
            &workflow_id.to_string(),
            "Workflow",
            vec![event],
            None,
            EventMetadata::default(),
        ).await.unwrap();

        // Replay with test handler
        let mut handler = TestHandler {
            events_handled: AtomicU64::new(0),
        };

        let stats = service.replay_with_handler(
            &mut handler,
            ReplayOptions::default(),
        ).await.unwrap();

        assert_eq!(stats.events_processed, 1);
        assert_eq!(handler.events_handled.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_replay_with_filters() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventReplayService::new(event_store.clone());

        // Add events of different types
        let workflow_id = WorkflowId::new();
        let workflow_event = DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id,
            definition_id: GraphId::new(),
            initial_state: "Start".to_string(),
            started_at: chrono::Utc::now(),
        });

        event_store.append_events(
            &workflow_id.to_string(),
            "Workflow",
            vec![workflow_event],
            None,
            EventMetadata::default(),
        ).await.unwrap();

        // Replay with event type filter
        let mut handler = TestHandler {
            events_handled: AtomicU64::new(0),
        };

        let options = ReplayOptions {
            event_types: Some(vec!["WorkflowStarted".to_string()]),
            ..Default::default()
        };

        let stats = service.replay_with_handler(
            &mut handler,
            options,
        ).await.unwrap();

        assert_eq!(stats.events_processed, 1);
    }

    #[tokio::test]
    async fn test_replay_aggregate() {
        let event_store = Arc::new(MockEventStore::new());
        let service = EventReplayService::new(event_store.clone());

        let workflow_id = WorkflowId::new();
        let event = DomainEventEnum::WorkflowStarted(WorkflowStarted {
            workflow_id,
            definition_id: GraphId::new(),
            initial_state: "Start".to_string(),
            started_at: chrono::Utc::now(),
        });

        event_store.append_events(
            &workflow_id.to_string(),
            "Workflow",
            vec![event],
            None,
            EventMetadata::default(),
        ).await.unwrap();

        // Replay specific aggregate
        let events = service.replay_aggregate(&workflow_id.to_string()).await.unwrap();
        assert_eq!(events.len(), 1);
    }
}
