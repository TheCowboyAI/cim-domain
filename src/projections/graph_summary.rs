//! Graph summary projection
//!
//! Provides a summary view of graphs including node/edge counts and metadata.

use super::{EventSequence, Projection};
use crate::{
    domain_events::{DomainEventEnum, GraphCreated, NodeAdded, NodeRemoved, EdgeAdded, EdgeRemoved},
    identifiers::GraphId,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary information about a graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSummary {
    /// Unique identifier of the graph
    pub graph_id: GraphId,
    /// Human-readable name of the graph
    pub name: String,
    /// Description of the graph's purpose
    pub description: String,
    /// Current number of nodes in the graph
    pub node_count: usize,
    /// Current number of edges in the graph
    pub edge_count: usize,
    /// When the graph was created
    pub created_at: DateTime<Utc>,
    /// When the graph was last modified
    pub last_modified: DateTime<Utc>,
    /// Additional metadata about the graph
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Projection that maintains graph summaries
#[derive(Debug, Clone)]
pub struct GraphSummaryProjection {
    summaries: HashMap<GraphId, GraphSummary>,
    checkpoint: Option<EventSequence>,
}

impl GraphSummaryProjection {
    /// Create a new graph summary projection
    pub fn new() -> Self {
        Self {
            summaries: HashMap::new(),
            checkpoint: None,
        }
    }

    /// Get a graph summary by ID
    pub fn get_summary(&self, graph_id: &GraphId) -> Option<&GraphSummary> {
        self.summaries.get(graph_id)
    }

    /// Get all graph summaries
    pub fn get_all_summaries(&self) -> Vec<&GraphSummary> {
        self.summaries.values().collect()
    }

    /// Get summaries with pagination
    pub fn get_summaries_paginated(&self, offset: usize, limit: usize) -> Vec<&GraphSummary> {
        self.summaries
            .values()
            .skip(offset)
            .take(limit)
            .collect()
    }

    /// Get total number of graphs
    pub fn total_graphs(&self) -> usize {
        self.summaries.len()
    }
}

#[async_trait]
impl Projection for GraphSummaryProjection {
    async fn handle_event(&mut self, event: DomainEventEnum) -> Result<(), String> {
        match event {
            DomainEventEnum::GraphCreated(GraphCreated {
                graph_id,
                name,
                description,
                metadata,
                created_at,
            }) => {
                let summary = GraphSummary {
                    graph_id,
                    name,
                    description,
                    node_count: 0,
                    edge_count: 0,
                    created_at,
                    last_modified: created_at,
                    metadata,
                };
                self.summaries.insert(graph_id, summary);
            }

            DomainEventEnum::NodeAdded(NodeAdded { graph_id, .. }) => {
                if let Some(summary) = self.summaries.get_mut(&graph_id) {
                    summary.node_count += 1;
                    summary.last_modified = Utc::now();
                }
            }

            DomainEventEnum::NodeRemoved(NodeRemoved { graph_id, .. }) => {
                if let Some(summary) = self.summaries.get_mut(&graph_id) {
                    summary.node_count = summary.node_count.saturating_sub(1);
                    summary.last_modified = Utc::now();
                }
            }

            DomainEventEnum::EdgeAdded(EdgeAdded { graph_id, .. }) => {
                if let Some(summary) = self.summaries.get_mut(&graph_id) {
                    summary.edge_count += 1;
                    summary.last_modified = Utc::now();
                }
            }

            DomainEventEnum::EdgeRemoved(EdgeRemoved { graph_id, .. }) => {
                if let Some(summary) = self.summaries.get_mut(&graph_id) {
                    summary.edge_count = summary.edge_count.saturating_sub(1);
                    summary.last_modified = Utc::now();
                }
            }

            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }

    async fn get_checkpoint(&self) -> Option<EventSequence> {
        self.checkpoint
    }

    async fn save_checkpoint(&mut self, sequence: EventSequence) -> Result<(), String> {
        self.checkpoint = Some(sequence);
        Ok(())
    }

    async fn clear(&mut self) -> Result<(), String> {
        self.summaries.clear();
        self.checkpoint = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::NodeId;

    #[tokio::test]
    async fn test_graph_summary_projection() {
        let mut projection = GraphSummaryProjection::new();
        let graph_id = GraphId::new();

        // Create graph
        let create_event = DomainEventEnum::GraphCreated(GraphCreated {
            graph_id,
            name: "Test Graph".to_string(),
            description: "A test graph".to_string(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        });

        projection.handle_event(create_event).await.unwrap();

        // Verify graph was created
        let summary = projection.get_summary(&graph_id).unwrap();
        assert_eq!(summary.name, "Test Graph");
        assert_eq!(summary.node_count, 0);
        assert_eq!(summary.edge_count, 0);

        // Add a node
        let node_event = DomainEventEnum::NodeAdded(NodeAdded {
            graph_id,
            node_id: NodeId::new(),
            node_type: "TestNode".to_string(),
            metadata: HashMap::new(),
        });

        projection.handle_event(node_event).await.unwrap();

        // Verify node count increased
        let summary = projection.get_summary(&graph_id).unwrap();
        assert_eq!(summary.node_count, 1);
    }

    #[tokio::test]
    async fn test_checkpoint_handling() {
        let mut projection = GraphSummaryProjection::new();

        // Initially no checkpoint
        assert!(projection.get_checkpoint().await.is_none());

        // Save checkpoint
        let seq = EventSequence::new(42);
        projection.save_checkpoint(seq).await.unwrap();

        // Verify checkpoint saved
        assert_eq!(projection.get_checkpoint().await, Some(seq));

        // Clear projection
        projection.clear().await.unwrap();
        assert!(projection.get_checkpoint().await.is_none());
        assert_eq!(projection.total_graphs(), 0);
    }
}
