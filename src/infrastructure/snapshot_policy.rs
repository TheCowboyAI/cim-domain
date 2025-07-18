//! Automatic snapshot policies for performance optimization
//!
//! This module provides configurable policies for automatically creating
//! snapshots of aggregates to optimize event replay performance.

use crate::entity::AggregateRoot;
use crate::infrastructure::snapshot_store::{SnapshotStore, SnapshotError};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Policy configuration for automatic snapshots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPolicy {
    /// Create snapshot after this many events
    pub event_count_threshold: Option<u32>,
    /// Create snapshot after this time interval
    pub time_interval: Option<Duration>,
    /// Create snapshot after these specific event types
    pub after_events: Vec<String>,
    /// Number of snapshots to retain
    pub retention_count: u32,
    /// Whether this policy is active
    pub enabled: bool,
}

impl Default for SnapshotPolicy {
    fn default() -> Self {
        Self {
            event_count_threshold: Some(100),
            time_interval: Some(Duration::hours(1)),
            after_events: vec![],
            retention_count: 5,
            enabled: true,
        }
    }
}

/// Metrics tracking snapshot creation for an aggregate
#[derive(Debug, Clone)]
pub struct SnapshotMetrics {
    /// ID of the aggregate
    pub aggregate_id: String,
    /// Type name of the aggregate
    pub aggregate_type: String,
    /// Number of events since last snapshot
    pub events_since_snapshot: u32,
    /// When the last snapshot was created
    pub last_snapshot_at: Option<DateTime<Utc>>,
    /// Total number of snapshots created
    pub total_snapshots: u32,
}

/// Engine for managing snapshot policies and triggering snapshots
pub struct SnapshotPolicyEngine {
    policies: Arc<RwLock<std::collections::HashMap<String, SnapshotPolicy>>>,
    metrics: Arc<RwLock<std::collections::HashMap<String, SnapshotMetrics>>>,
    #[allow(dead_code)]
    snapshot_store: Arc<dyn SnapshotStore>,
}

impl SnapshotPolicyEngine {
    /// Create a new snapshot policy engine
    pub fn new(snapshot_store: Arc<dyn SnapshotStore>) -> Self {
        Self {
            policies: Arc::new(RwLock::new(std::collections::HashMap::new())),
            metrics: Arc::new(RwLock::new(std::collections::HashMap::new())),
            snapshot_store,
        }
    }

    /// Register a snapshot policy for an aggregate type
    pub async fn register_policy(&self, aggregate_type: String, policy: SnapshotPolicy) {
        let mut policies = self.policies.write().await;
        policies.insert(aggregate_type, policy);
    }

    /// Check if a snapshot should be created based on policies
    pub async fn should_snapshot(
        &self,
        aggregate_id: &str,
        aggregate_type: &str,
        event_type: &str,
        _event_number: u32,
    ) -> bool {
        let policies = self.policies.read().await;
        let policy = match policies.get(aggregate_type) {
            Some(p) if p.enabled => p,
            _ => return false,
        };

        let metrics = self.get_or_create_metrics(aggregate_id, aggregate_type).await;

        if let Some(threshold) = policy.event_count_threshold {
            if metrics.events_since_snapshot >= threshold {
                return true;
            }
        }

        if let Some(interval) = policy.time_interval {
            if let Some(last_snapshot) = metrics.last_snapshot_at {
                let elapsed = Utc::now() - last_snapshot;
                if elapsed > interval {
                    return true;
                }
            } else {
                return true;
            }
        }

        if policy.after_events.contains(&event_type.to_string()) {
            return true;
        }

        false
    }

    /// Record that a snapshot was created
    pub async fn record_snapshot(
        &self,
        aggregate_id: &str,
        aggregate_type: &str,
    ) -> Result<(), SnapshotError> {
        let mut metrics_map = self.metrics.write().await;
        let metrics = metrics_map
            .entry(aggregate_id.to_string())
            .or_insert_with(|| SnapshotMetrics {
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: aggregate_type.to_string(),
                events_since_snapshot: 0,
                last_snapshot_at: None,
                total_snapshots: 0,
            });

        metrics.events_since_snapshot = 0;
        metrics.last_snapshot_at = Some(Utc::now());
        metrics.total_snapshots += 1;

        self.enforce_retention_policy(aggregate_id, aggregate_type).await?;

        Ok(())
    }

    /// Record that an event was processed
    pub async fn record_event(&self, aggregate_id: &str, aggregate_type: &str) {
        let mut metrics_map = self.metrics.write().await;
        let metrics = metrics_map
            .entry(aggregate_id.to_string())
            .or_insert_with(|| SnapshotMetrics {
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: aggregate_type.to_string(),
                events_since_snapshot: 0,
                last_snapshot_at: None,
                total_snapshots: 0,
            });

        metrics.events_since_snapshot += 1;
    }

    async fn get_or_create_metrics(&self, aggregate_id: &str, aggregate_type: &str) -> SnapshotMetrics {
        let metrics_map = self.metrics.read().await;
        if let Some(metrics) = metrics_map.get(aggregate_id) {
            return metrics.clone();
        }
        drop(metrics_map);

        let mut metrics_map = self.metrics.write().await;
        metrics_map
            .entry(aggregate_id.to_string())
            .or_insert_with(|| SnapshotMetrics {
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: aggregate_type.to_string(),
                events_since_snapshot: 0,
                last_snapshot_at: None,
                total_snapshots: 0,
            })
            .clone()
    }

    async fn enforce_retention_policy(
        &self,
        _aggregate_id: &str,
        aggregate_type: &str,
    ) -> Result<(), SnapshotError> {
        let policies = self.policies.read().await;
        let _policy = match policies.get(aggregate_type) {
            Some(p) => p,
            None => return Ok(()),
        };

        // Note: Current SnapshotStore trait doesn't support listing or deleting snapshots
        // In production, you would need to extend the trait or use a different approach
        // For now, we rely on JetStream's built-in history retention
        
        Ok(())
    }

    /// Get metrics for an aggregate
    pub async fn get_metrics(&self, aggregate_id: &str) -> Option<SnapshotMetrics> {
        let metrics_map = self.metrics.read().await;
        metrics_map.get(aggregate_id).cloned()
    }
}

/// Service for automatically creating snapshots based on policies
pub struct AutoSnapshotService<A: AggregateRoot + serde::Serialize + Send + Sync> {
    policy_engine: Arc<SnapshotPolicyEngine>,
    snapshot_store: Arc<dyn SnapshotStore>,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: AggregateRoot + serde::Serialize + Send + Sync> AutoSnapshotService<A> {
    /// Create a new auto-snapshot service
    pub fn new(
        policy_engine: Arc<SnapshotPolicyEngine>,
        snapshot_store: Arc<dyn SnapshotStore>,
    ) -> Self {
        Self {
            policy_engine,
            snapshot_store,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Check if a snapshot should be created and create it if needed
    pub async fn maybe_snapshot(
        &self,
        aggregate: &A,
        event_type: &str,
        event_number: u32,
    ) -> Result<bool, SnapshotError> {
        // For now, we'll use a generated UUID as the aggregate ID
        // In a real implementation, you'd need to serialize the actual ID
        let aggregate_id = uuid::Uuid::new_v4().to_string();
        let aggregate_type = std::any::type_name::<A>();

        if self.policy_engine
            .should_snapshot(&aggregate_id, aggregate_type, event_type, event_number)
            .await
        {
            info!(
                aggregate_id = %aggregate_id,
                aggregate_type = %aggregate_type,
                version = aggregate.version(),
                "Creating automatic snapshot"
            );

            // Serialize the aggregate
            let data = serde_json::to_vec(aggregate)
                .map_err(|e| SnapshotError::SerializationError(e.to_string()))?;

            let snapshot = crate::infrastructure::snapshot_store::AggregateSnapshot {
                aggregate_id: aggregate_id.clone(),
                aggregate_type: aggregate_type.to_string(),
                version: aggregate.version(),
                data,
                created_at: chrono::Utc::now(),
            };

            self.snapshot_store
                .save_snapshot(&aggregate_id, snapshot)
                .await?;

            self.policy_engine
                .record_snapshot(&aggregate_id, aggregate_type)
                .await?;

            Ok(true)
        } else {
            self.policy_engine
                .record_event(&aggregate_id, aggregate_type)
                .await;
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::snapshot_store::InMemorySnapshotStore;

    #[tokio::test]
    async fn test_event_count_policy() {
        let snapshot_store = Arc::new(InMemorySnapshotStore::new());
        let engine = SnapshotPolicyEngine::new(snapshot_store);

        let policy = SnapshotPolicy {
            event_count_threshold: Some(5),
            time_interval: None,
            after_events: vec![],
            retention_count: 3,
            enabled: true,
        };

        engine.register_policy("TestAggregateRoot".to_string(), policy).await;

        for i in 1..=4 {
            engine.record_event("agg-1", "TestAggregateRoot").await;
            let should = engine
                .should_snapshot("agg-1", "TestAggregateRoot", "TestEvent", i)
                .await;
            assert!(!should);
        }

        engine.record_event("agg-1", "TestAggregateRoot").await;
        let should = engine
            .should_snapshot("agg-1", "TestAggregateRoot", "TestEvent", 5)
            .await;
        assert!(should);

        engine.record_snapshot("agg-1", "TestAggregateRoot").await.unwrap();

        let should = engine
            .should_snapshot("agg-1", "TestAggregateRoot", "TestEvent", 6)
            .await;
        assert!(!should);
    }

    #[tokio::test]
    async fn test_event_type_policy() {
        let snapshot_store = Arc::new(InMemorySnapshotStore::new());
        let engine = SnapshotPolicyEngine::new(snapshot_store);

        let policy = SnapshotPolicy {
            event_count_threshold: None,
            time_interval: None,
            after_events: vec!["ImportantEvent".to_string()],
            retention_count: 3,
            enabled: true,
        };

        engine.register_policy("TestAggregateRoot".to_string(), policy).await;

        let should = engine
            .should_snapshot("agg-1", "TestAggregateRoot", "RegularEvent", 1)
            .await;
        assert!(!should);

        let should = engine
            .should_snapshot("agg-1", "TestAggregateRoot", "ImportantEvent", 2)
            .await;
        assert!(should);
    }
}