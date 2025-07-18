// Copyright 2025 Cowboy AI, LLC.

//! Read model projections for CIM domain
//!
//! Projections are optimized read models that are updated by handling domain events.
//! They provide efficient queries without needing to replay all events.

// graph_summary has been moved to cim-domain-graph
// node_list has been moved to cim-domain-graph
// workflow_status has been moved to cim-domain-workflow

// GraphSummaryProjection is now in cim-domain-graph
// NodeListProjection is now in cim-domain-graph
// WorkflowStatusProjection is now in cim-domain-workflow

use crate::domain_events::DomainEventEnum;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Trait for all projections
#[async_trait]
pub trait Projection: Send + Sync {
    /// Handle a domain event to update the projection
    async fn handle_event(&mut self, event: DomainEventEnum) -> Result<(), String>;

    /// Get the current checkpoint (last processed event sequence)
    async fn get_checkpoint(&self) -> Option<EventSequence>;

    /// Save the checkpoint after processing events
    async fn save_checkpoint(&mut self, sequence: EventSequence) -> Result<(), String>;

    /// Clear the projection (for rebuilding)
    async fn clear(&mut self) -> Result<(), String>;
}

/// Event sequence number for checkpointing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventSequence(pub u64);

impl EventSequence {
    /// Create a new event sequence with the given value
    pub fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// Increment the sequence number by one
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    /// Get the current sequence value
    pub fn value(&self) -> u64 {
        self.0
    }
}
