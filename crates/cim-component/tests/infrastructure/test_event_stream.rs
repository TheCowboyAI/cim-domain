//! Infrastructure Layer 1.2: Event Store Tests
//! 
//! User Story: As a domain, I need to persist events with CID chains for integrity
//!
//! Test Requirements:
//! - Verify event persistence with CID calculation
//! - Verify CID chain integrity
//! - Verify event replay from store
//! - Verify snapshot creation and restoration
//!
//! Event Sequence:
//! 1. EventStoreInitialized
//! 2. EventPersisted { event_id, cid, previous_cid }
//! 3. CIDChainValidated { start_cid, end_cid, length }
//! 4. EventsReplayed { count, aggregate_id }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Initialize Event Store]
//!     B --> C[EventStoreInitialized]
//!     C --> D[Persist Event]
//!     D --> E[Calculate CID]
//!     E --> F[EventPersisted]
//!     F --> G[Validate Chain]
//!     G --> H[CIDChainValidated]
//!     H --> I[Replay Events]
//!     I --> J[EventsReplayed]
//!     J --> K[Test Success]
//! ```

use std::collections::HashMap;

/// Mock CID type for testing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MockCid(String);

impl MockCid {
    pub fn from_data(data: &[u8]) -> Self {
        // Simple mock hash calculation
        let hash = data.iter().fold(0u64, |acc, &b| acc.wrapping_add(b as u64));
        MockCid(format!("cid_{:016x}", hash))
    }
}

/// Mock event for testing
#[derive(Debug, Clone)]
pub struct MockEvent {
    pub event_id: String,
    pub aggregate_id: String,
    pub sequence: u64,
    pub data: Vec<u8>,
}

/// Mock event store for testing
pub struct MockEventStore {
    events: Vec<(MockEvent, MockCid, Option<MockCid>)>,
    snapshots: HashMap<String, Vec<u8>>,
}

impl MockEventStore {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            snapshots: HashMap::new(),
        }
    }

    pub fn persist_event(
        &mut self,
        event: MockEvent,
        previous_cid: Option<MockCid>,
    ) -> Result<MockCid, String> {
        // Calculate CID from event data and previous CID
        let mut cid_data = event.data.clone();
        if let Some(prev) = &previous_cid {
            cid_data.extend_from_slice(prev.0.as_bytes());
        }
        
        let cid = MockCid::from_data(&cid_data);
        self.events.push((event, cid.clone(), previous_cid));
        
        Ok(cid)
    }

    pub fn validate_chain(&self) -> Result<(MockCid, MockCid, usize), String> {
        if self.events.is_empty() {
            return Err("No events in store".to_string());
        }

        // Validate each event's CID chain
        for i in 1..self.events.len() {
            let (_, _, prev_cid) = &self.events[i];
            let (_, expected_prev_cid, _) = &self.events[i - 1];
            
            if prev_cid.as_ref() != Some(expected_prev_cid) {
                return Err(format!("Chain broken at position {i}"));
            }
        }

        let start_cid = self.events.first().unwrap().1.clone();
        let end_cid = self.events.last().unwrap().1.clone();
        
        Ok((start_cid, end_cid, self.events.len()))
    }

    pub fn replay_events(&self, aggregate_id: &str) -> Vec<MockEvent> {
        self.events
            .iter()
            .filter(|(e, _, _)| e.aggregate_id == aggregate_id)
            .map(|(e, _, _)| e.clone())
            .collect()
    }

    pub fn create_snapshot(&mut self, aggregate_id: &str, data: Vec<u8>) -> Result<(), String> {
        self.snapshots.insert(aggregate_id.to_string(), data);
        Ok(())
    }

    pub fn restore_snapshot(&self, aggregate_id: &str) -> Option<Vec<u8>> {
        self.snapshots.get(aggregate_id).cloned()
    }
}

/// Event types for event store testing
#[derive(Debug, Clone, PartialEq)]
pub enum EventStoreEvent {
    EventStoreInitialized,
    EventPersisted { event_id: String, cid: MockCid, previous_cid: Option<MockCid> },
    CIDChainValidated { start_cid: MockCid, end_cid: MockCid, length: usize },
    EventsReplayed { count: usize, aggregate_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::test_nats_connection::EventStreamValidator;

    #[test]
    fn test_event_store_initialization() {
        // Arrange
        let _validator = EventStreamValidator::new();
        
        // Act
        let store = MockEventStore::new();
        
        // Assert
        assert_eq!(store.events.len(), 0);
        assert_eq!(store.snapshots.len(), 0);
    }

    #[test]
    fn test_event_persistence_with_cid() {
        // Arrange
        let mut store = MockEventStore::new();
        let event = MockEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 1,
            data: vec![1, 2, 3, 4],
        };

        // Act
        let cid = store.persist_event(event.clone(), None).unwrap();

        // Assert
        assert!(!cid.0.is_empty());
        assert_eq!(store.events.len(), 1);
        
        let (stored_event, stored_cid, prev_cid) = &store.events[0];
        assert_eq!(stored_event.event_id, "evt_1");
        assert_eq!(stored_cid, &cid);
        assert_eq!(prev_cid, &None);
    }

    #[test]
    fn test_cid_chain_integrity() {
        // Arrange
        let mut store = MockEventStore::new();
        
        // Create a chain of events
        let event1 = MockEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 1,
            data: vec![1, 2, 3],
        };
        
        let event2 = MockEvent {
            event_id: "evt_2".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 2,
            data: vec![4, 5, 6],
        };
        
        let event3 = MockEvent {
            event_id: "evt_3".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 3,
            data: vec![7, 8, 9],
        };

        // Act
        let cid1 = store.persist_event(event1, None).unwrap();
        let cid2 = store.persist_event(event2, Some(cid1.clone())).unwrap();
        let cid3 = store.persist_event(event3, Some(cid2.clone())).unwrap();

        // Validate chain
        let (start_cid, end_cid, length) = store.validate_chain().unwrap();

        // Assert
        assert_eq!(start_cid, cid1);
        assert_eq!(end_cid, cid3);
        assert_eq!(length, 3);
    }

    #[test]
    fn test_event_replay() {
        // Arrange
        let mut store = MockEventStore::new();
        
        // Add events for different aggregates
        for i in 1..=3 {
            let event = MockEvent {
                event_id: format!("evt_{i}"),
                aggregate_id: "agg_1".to_string(),
                sequence: i as u64,
                data: vec![i as u8],
            };
            store.persist_event(event, None).ok();
        }
        
        // Add event for different aggregate
        let other_event = MockEvent {
            event_id: "evt_other".to_string(),
            aggregate_id: "agg_2".to_string(),
            sequence: 1,
            data: vec![99],
        };
        store.persist_event(other_event, None).ok();

        // Act
        let replayed = store.replay_events("agg_1");

        // Assert
        assert_eq!(replayed.len(), 3);
        assert_eq!(replayed[0].event_id, "evt_1");
        assert_eq!(replayed[1].event_id, "evt_2");
        assert_eq!(replayed[2].event_id, "evt_3");
    }

    #[test]
    fn test_snapshot_creation_and_restoration() {
        // Arrange
        let mut store = MockEventStore::new();
        let aggregate_id = "agg_1";
        let snapshot_data = vec![10, 20, 30, 40, 50];

        // Act
        store.create_snapshot(aggregate_id, snapshot_data.clone()).unwrap();
        let restored = store.restore_snapshot(aggregate_id);

        // Assert
        assert_eq!(restored, Some(snapshot_data));
    }

    #[test]
    fn test_broken_chain_detection() {
        // Arrange
        let mut store = MockEventStore::new();
        
        // Manually break the chain by inserting events with wrong previous CIDs
        let event1 = MockEvent {
            event_id: "evt_1".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 1,
            data: vec![1],
        };
        
        let cid1 = MockCid::from_data(&event1.data);
        store.events.push((event1, cid1.clone(), None));
        
        // Add second event with wrong previous CID
        let event2 = MockEvent {
            event_id: "evt_2".to_string(),
            aggregate_id: "agg_1".to_string(),
            sequence: 2,
            data: vec![2],
        };
        
        let wrong_cid = MockCid("wrong_cid".to_string());
        let cid2 = MockCid::from_data(&event2.data);
        store.events.push((event2, cid2, Some(wrong_cid)));

        // Act
        let result = store.validate_chain();

        // Assert
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Chain broken at position 1");
    }
} 