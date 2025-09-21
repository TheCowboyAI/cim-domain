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

use crate::DomainEvent;
use async_trait::async_trait;

/// Trait for all projections (read models)
///
/// Projections are domain concepts that define how to build
/// optimized read models from events. The actual storage and
/// checkpointing are infrastructure concerns.
#[async_trait]
pub trait Projection: Send + Sync {
    /// Handle a domain event to update the projection
    async fn handle_event(&mut self, event: &dyn DomainEvent) -> Result<(), String>;

    /// Clear the projection (for rebuilding)
    async fn clear(&mut self) -> Result<(), String>;
}

// Checkpointing and event sequencing removed - infrastructure concerns
// These belong in the infrastructure layer that implements projections

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainEvent;
    use uuid::Uuid;

    #[derive(Debug, Default)]
    struct CounterProjection(usize);

    #[async_trait]
    impl Projection for CounterProjection {
        async fn handle_event(&mut self, _event: &dyn DomainEvent) -> Result<(), String> {
            self.0 += 1;
            Ok(())
        }
        async fn clear(&mut self) -> Result<(), String> {
            self.0 = 0;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct E(Uuid);
    impl DomainEvent for E {
        fn aggregate_id(&self) -> Uuid {
            self.0
        }
        fn event_type(&self) -> &'static str {
            "E"
        }
    }

    #[tokio::test]
    async fn projection_handles_and_clears() {
        let mut p = CounterProjection::default();
        let event = E(Uuid::new_v4());
        p.handle_event(&event).await.unwrap();
        p.handle_event(&event).await.unwrap();
        assert_eq!(p.0, 2);
        p.clear().await.unwrap();
        assert_eq!(p.0, 0);
    }
}
