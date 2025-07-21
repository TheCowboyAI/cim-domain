// Copyright 2025 Cowboy AI, LLC.

//! Infrastructure Layer 1.2: Event Store Tests for cim-domain
//!
//! User Story: As a domain aggregate, I need to persist events with CID chains for integrity
//!
//! Test Requirements:
//! - Verify aggregate event persistence with CID calculation
//! - Verify CID chain integrity for aggregate event streams
//! - Verify aggregate replay from event store
//! - Verify snapshot creation for aggregates
//!
//! Event Sequence:
//! 1. AggregateEventStoreInitialized
//! 2. AggregateEventPersisted { aggregate_id, event_id, cid, previous_cid }
//! 3. AggregateCIDChainValidated { aggregate_id, start_cid, end_cid, length }
//! 4. AggregateReplayed { aggregate_id, event_count, final_version }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Initialize Aggregate Store]
//!     B --> C[AggregateEventStoreInitialized]
//!     C --> D[Persist Aggregate Event]
//!     D --> E[Calculate CID]
//!     E --> F[AggregateEventPersisted]
//!     F --> G[Validate Aggregate Chain]
//!     G --> H[AggregateCIDChainValidated]
//!     H --> I[Replay Aggregate]
//!     I --> J[AggregateReplayed]
//!     J --> K[Test Success]
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mock CID type for domain testing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainCid(String);

impl DomainCid {
    pub fn from_aggregate_event(
        data: &[u8],
        previous: Option<&DomainCid>,
        aggregate_id: &str,
    ) -> Self {
        // Domain-specific CID calculation including aggregate_id
        let mut hash_data = aggregate_id.as_bytes().to_vec();
        hash_data.extend_from_slice(data);
        if let Some(prev) = previous {
            hash_data.extend_from_slice(prev.0.as_bytes());
        }

        let hash = hash_data
            .iter()
            .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));

        DomainCid(format!("domain_cid_{:016x}", hash))
    }
}

/// Domain aggregate event for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub event_type: String,
    pub sequence: u64,
    pub version: u64,
    pub payload: serde_json::Value,
}

/// Aggregate snapshot for testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateSnapshot {
    pub aggregate_id: String,
    pub version: u64,
    pub state: serde_json::Value,
    pub created_at: u64,
}

/// Domain event store for aggregate testing
pub struct DomainEventStore {
    events: Vec<(AggregateEvent, DomainCid, Option<DomainCid>)>,
    snapshots: HashMap<String, AggregateSnapshot>,
    aggregate_streams: HashMap<String, Vec<usize>>, // aggregate_id -> event indices
}

impl DomainEventStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            snapshots: HashMap::new(),
            aggregate_streams: HashMap::new(),
        }
    }

    pub fn persist_aggregate_event(
        &mut self,
        event: AggregateEvent,
        previous_cid: Option<DomainCid>,
    ) -> Result<DomainCid, String> {
        // Validate version sequence
        if let Some(indices) = self.aggregate_streams.get(&event.aggregate_id) {
            if !indices.is_empty() {
                let last_event = &self.events[indices[indices.len() - 1]].0;
                if event.version != last_event.version + 1 {
                    return Err(format!(
                        "Version mismatch: expected {}, got {}",
                        last_event.version + 1,
                        event.version
                    ));
                }
            }
        }

        // Serialize event for CID calculation
        let event_bytes =
            serde_json::to_vec(&event).map_err(|e| format!("Serialization error: {e}"))?;

        let cid = DomainCid::from_aggregate_event(
            &event_bytes,
            previous_cid.as_ref(),
            &event.aggregate_id,
        );

        let event_index = self.events.len();
        self.events.push((event.clone(), cid.clone(), previous_cid));

        // Update aggregate stream index
        self.aggregate_streams
            .entry(event.aggregate_id.clone())
            .or_insert_with(Vec::new)
            .push(event_index);

        Ok(cid)
    }

    pub fn validate_aggregate_chain(
        &self,
        aggregate_id: &str,
    ) -> Result<(DomainCid, DomainCid, usize), String> {
        let indices = self
            .aggregate_streams
            .get(aggregate_id)
            .ok_or_else(|| format!("No events for aggregate {aggregate_id}"))?;

        if indices.is_empty() {
            return Err("No events in aggregate stream".to_string());
        }

        // Validate chain for this aggregate
        for i in 1..indices.len() {
            let (_, _, prev_cid) = &self.events[indices[i]];
            let (_, expected_prev_cid, _) = &self.events[indices[i - 1]];

            if prev_cid.as_ref() != Some(expected_prev_cid) {
                return Err(format!(
                    "Chain broken at position {i} for aggregate {aggregate_id}"
                ));
            }
        }

        // Validate version sequence
        for i in 1..indices.len() {
            let current_event = &self.events[indices[i]].0;
            let previous_event = &self.events[indices[i - 1]].0;

            if current_event.version != previous_event.version + 1 {
                return Err(format!(
                    "Version sequence broken at position {}: expected {}, got {}",
                    i,
                    previous_event.version + 1,
                    current_event.version
                ));
            }
        }

        let start_cid = self.events[indices[0]].1.clone();
        let end_cid = self.events[indices[indices.len() - 1]].1.clone();

        Ok((start_cid, end_cid, indices.len()))
    }

    pub fn replay_aggregate(&self, aggregate_id: &str) -> Vec<AggregateEvent> {
        self.aggregate_streams
            .get(aggregate_id)
            .map(|indices| indices.iter().map(|&i| self.events[i].0.clone()).collect())
            .unwrap_or_default()
    }

    pub fn create_aggregate_snapshot(
        &mut self,
        aggregate_id: &str,
        state: serde_json::Value,
    ) -> Result<(), String> {
        let indices = self
            .aggregate_streams
            .get(aggregate_id)
            .ok_or_else(|| format!("No events for aggregate {aggregate_id}"))?;

        if indices.is_empty() {
            return Err("Cannot snapshot aggregate with no events".to_string());
        }

        let last_event = &self.events[indices[indices.len() - 1]].0;

        let snapshot = AggregateSnapshot {
            aggregate_id: aggregate_id.to_string(),
            version: last_event.version,
            state,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.snapshots.insert(aggregate_id.to_string(), snapshot);
        Ok(())
    }

    pub fn get_aggregate_snapshot(&self, aggregate_id: &str) -> Option<&AggregateSnapshot> {
        self.snapshots.get(aggregate_id)
    }

    pub fn get_latest_cid(&self, aggregate_id: &str) -> Option<DomainCid> {
        self.aggregate_streams
            .get(aggregate_id)
            .and_then(|indices| indices.last())
            .map(|&i| self.events[i].1.clone())
    }

    pub fn get_aggregate_version(&self, aggregate_id: &str) -> Option<u64> {
        self.aggregate_streams
            .get(aggregate_id)
            .and_then(|indices| indices.last())
            .map(|&i| self.events[i].0.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_domain_event_store_initialization() {
        // Arrange & Act
        let store = DomainEventStore::new();

        // Assert
        assert_eq!(store.events.len(), 0);
        assert_eq!(store.snapshots.len(), 0);
        assert_eq!(store.aggregate_streams.len(), 0);
    }

    #[test]
    fn test_aggregate_event_persistence() {
        // Arrange
        let mut store = DomainEventStore::new();
        let event = AggregateEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: "user_123".to_string(),
            aggregate_type: "User".to_string(),
            event_type: "UserCreated".to_string(),
            sequence: 1,
            version: 1,
            payload: json!({
                "name": "John Doe",
                "email": "john@example.com"
            }),
        };

        // Act
        let cid = store.persist_aggregate_event(event.clone(), None).unwrap();

        // Assert
        assert!(cid.0.starts_with("domain_cid_"));
        assert_eq!(store.events.len(), 1);
        assert_eq!(store.aggregate_streams.get("user_123").unwrap().len(), 1);

        let (stored_event, stored_cid, prev_cid) = &store.events[0];
        assert_eq!(stored_event.event_id, "evt_1");
        assert_eq!(stored_event.aggregate_id, "user_123");
        assert_eq!(stored_event.version, 1);
        assert_eq!(stored_cid, &cid);
        assert_eq!(prev_cid, &None);
    }

    #[test]
    fn test_aggregate_cid_chain_integrity() {
        // Arrange
        let mut store = DomainEventStore::new();
        let aggregate_id = "order_456";

        // Create a chain of aggregate events
        let events = vec![
            AggregateEvent {
                event_id: "evt_1".to_string(),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: "Order".to_string(),
                event_type: "OrderCreated".to_string(),
                sequence: 1,
                version: 1,
                payload: json!({ "total": 100.0 }),
            },
            AggregateEvent {
                event_id: "evt_2".to_string(),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: "Order".to_string(),
                event_type: "OrderItemAdded".to_string(),
                sequence: 2,
                version: 2,
                payload: json!({ "item_id": "item_1", "quantity": 2 }),
            },
            AggregateEvent {
                event_id: "evt_3".to_string(),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: "Order".to_string(),
                event_type: "OrderConfirmed".to_string(),
                sequence: 3,
                version: 3,
                payload: json!({ "confirmed_at": "2025-01-22T10:00:00Z" }),
            },
        ];

        // Act
        let mut previous_cid = None;
        let mut cids = Vec::new();

        for event in events {
            let cid = store
                .persist_aggregate_event(event, previous_cid.clone())
                .unwrap();
            cids.push(cid.clone());
            previous_cid = Some(cid);
        }

        // Validate chain
        let (start_cid, end_cid, length) = store.validate_aggregate_chain(aggregate_id).unwrap();

        // Assert
        assert_eq!(start_cid, cids[0]);
        assert_eq!(end_cid, cids[2]);
        assert_eq!(length, 3);
    }

    #[test]
    fn test_aggregate_replay() {
        // Arrange
        let mut store = DomainEventStore::new();
        let aggregate_id = "product_789";

        // Add events for aggregate
        for i in 1..=3 {
            let event = AggregateEvent {
                event_id: format!("evt_{i}"),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: "Product".to_string(),
                event_type: format!("Event{i}"),
                sequence: i as u64,
                version: i as u64,
                payload: json!({ "data": i }),
            };
            store
                .persist_aggregate_event(event, store.get_latest_cid(aggregate_id))
                .ok();
        }

        // Act
        let replayed = store.replay_aggregate(aggregate_id);

        // Assert
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].version, 1);
        assert_eq!(replayed[1].version, 2);
        assert_eq!(replayed[2].version, 3);

        // Verify events are in order
        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(event.sequence, (i + 1) as u64);
            assert_eq!(event.version, (i + 1) as u64);
        }
    }

    #[test]
    fn test_aggregate_snapshot_creation() {
        // Arrange
        let mut store = DomainEventStore::new();
        let aggregate_id = "account_111";

        // Add some events
        for i in 1..=5 {
            let event = AggregateEvent {
                event_id: format!("evt_{i}"),
                aggregate_id: aggregate_id.to_string(),
                aggregate_type: "Account".to_string(),
                event_type: "BalanceUpdated".to_string(),
                sequence: i as u64,
                version: i as u64,
                payload: json!({ "balance": i * 100 }),
            };
            store
                .persist_aggregate_event(event, store.get_latest_cid(aggregate_id))
                .ok();
        }

        let snapshot_state = json!({
            "balance": 500,
            "transactions": 5,
            "status": "active"
        });

        // Act
        store
            .create_aggregate_snapshot(aggregate_id, snapshot_state.clone())
            .unwrap();
        let snapshot = store.get_aggregate_snapshot(aggregate_id).unwrap();

        // Assert
        assert_eq!(snapshot.aggregate_id, aggregate_id);
        assert_eq!(snapshot.version, 5);
        assert_eq!(snapshot.state, snapshot_state);
        assert!(snapshot.created_at > 0);
    }

    #[test]
    fn test_version_sequence_validation() {
        // Arrange
        let mut store = DomainEventStore::new();
        let aggregate_id = "entity_222";

        // Create first event
        let event1 = AggregateEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: "Entity".to_string(),
            event_type: "Created".to_string(),
            sequence: 1,
            version: 1,
            payload: json!({}),
        };

        store.persist_aggregate_event(event1, None).unwrap();

        // Try to add event with wrong version
        let event_wrong_version = AggregateEvent {
            event_id: "evt_2".to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: "Entity".to_string(),
            event_type: "Updated".to_string(),
            sequence: 2,
            version: 3, // Should be 2
            payload: json!({}),
        };

        // Act
        let result =
            store.persist_aggregate_event(event_wrong_version, store.get_latest_cid(aggregate_id));

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Version mismatch"));
    }

    #[test]
    fn test_multiple_aggregates_isolation() {
        // Arrange
        let mut store = DomainEventStore::new();

        // Add events for different aggregates
        let agg1_event = AggregateEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: "agg_1".to_string(),
            aggregate_type: "TypeA".to_string(),
            event_type: "Created".to_string(),
            sequence: 1,
            version: 1,
            payload: json!({"name": "Aggregate 1"}),
        };

        let agg2_event = AggregateEvent {
            event_id: "evt_2".to_string(),
            aggregate_id: "agg_2".to_string(),
            aggregate_type: "TypeB".to_string(),
            event_type: "Created".to_string(),
            sequence: 1,
            version: 1,
            payload: json!({"name": "Aggregate 2"}),
        };

        store.persist_aggregate_event(agg1_event, None).unwrap();
        store.persist_aggregate_event(agg2_event, None).unwrap();

        // Act
        let agg1_events = store.replay_aggregate("agg_1");
        let agg2_events = store.replay_aggregate("agg_2");

        // Assert
        assert_eq!(agg1_events.len(), 1);
        assert_eq!(agg2_events.len(), 1);
        assert_eq!(agg1_events[0].aggregate_id, "agg_1");
        assert_eq!(agg2_events[0].aggregate_id, "agg_2");

        // Verify separate version tracking
        assert_eq!(store.get_aggregate_version("agg_1"), Some(1));
        assert_eq!(store.get_aggregate_version("agg_2"), Some(1));
    }
}
