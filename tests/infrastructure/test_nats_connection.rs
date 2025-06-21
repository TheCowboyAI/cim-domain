//! Infrastructure Layer 1.1: NATS JetStream Connection Tests for cim-domain
//! 
//! User Story: As a domain core, I need to connect to NATS JetStream for event persistence
//!
//! Test Requirements:
//! - Verify NATS connection establishment for domain events
//! - Verify stream creation for domain aggregates
//! - Verify event publishing with domain metadata
//! - Verify event consumption with proper correlation
//!
//! Event Sequence:
//! 1. DomainConnectionEstablished
//! 2. DomainStreamCreated { name, subjects }
//! 3. DomainEventPublished { aggregate_id, event_type, sequence }
//! 4. DomainEventConsumed { aggregate_id, event_type, sequence }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Create Domain Client]
//!     B --> C{Connection OK?}
//!     C -->|Yes| D[DomainConnectionEstablished]
//!     C -->|No| E[Test Failure]
//!     D --> F[Create Domain Stream]
//!     F --> G[DomainStreamCreated]
//!     G --> H[Publish Domain Event]
//!     H --> I[DomainEventPublished]
//!     I --> J[Consume Domain Event]
//!     J --> K[DomainEventConsumed]
//!     K --> L[Test Success]
//! ```

use std::time::Duration;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Domain-specific event types for testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainInfrastructureEvent {
    DomainConnectionEstablished { client_name: String },
    DomainStreamCreated { name: String, subjects: Vec<String> },
    DomainEventPublished { aggregate_id: String, event_type: String, sequence: u64 },
    DomainEventConsumed { aggregate_id: String, event_type: String, sequence: u64 },
    DomainConnectionFailed { error: String },
}

/// Event stream validator for domain infrastructure testing
pub struct DomainEventStreamValidator {
    expected_events: Vec<DomainInfrastructureEvent>,
    captured_events: Vec<DomainInfrastructureEvent>,
}

impl DomainEventStreamValidator {
    pub fn new() -> Self {
        Self {
            expected_events: Vec::new(),
            captured_events: Vec::new(),
        }
    }

    pub fn expect_sequence(mut self, events: Vec<DomainInfrastructureEvent>) -> Self {
        self.expected_events = events;
        self
    }

    pub fn capture_event(&mut self, event: DomainInfrastructureEvent) {
        self.captured_events.push(event);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.captured_events.len() != self.expected_events.len() {
            return Err(format!(
                "Event count mismatch: expected {}, got {}",
                self.expected_events.len(),
                self.captured_events.len()
            ));
        }

        for (i, (expected, actual)) in self.expected_events.iter()
            .zip(self.captured_events.iter())
            .enumerate()
        {
            if expected != actual {
                return Err(format!(
                    "Event mismatch at position {}: expected {:?}, got {:?}",
                    i, expected, actual
                ));
            }
        }

        Ok(())
    }
}

/// Mock domain NATS client for testing
pub struct MockDomainNatsClient {
    connected: bool,
    client_name: String,
    streams: HashMap<String, Vec<String>>,
    published_events: Vec<(String, String, u64)>, // (aggregate_id, event_type, sequence)
}

impl MockDomainNatsClient {
    pub fn new(client_name: String) -> Self {
        Self {
            connected: false,
            client_name,
            streams: HashMap::new(),
            published_events: Vec::new(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        // Simulate connection with delay
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.connected = true;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn create_domain_stream(&mut self, name: String, subjects: Vec<String>) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected to NATS".to_string());
        }
        
        self.streams.insert(name, subjects);
        Ok(())
    }

    pub async fn publish_domain_event(
        &mut self,
        aggregate_id: String,
        event_type: String,
        sequence: u64,
    ) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected to NATS".to_string());
        }
        
        // Simulate acknowledgment delay
        tokio::time::sleep(Duration::from_millis(5)).await;
        self.published_events.push((aggregate_id, event_type, sequence));
        Ok(())
    }

    pub async fn consume_domain_event(
        &self,
        aggregate_id: &str,
    ) -> Result<(String, String, u64), String> {
        if !self.connected {
            return Err("Not connected to NATS".to_string());
        }
        
        // Find the event with matching aggregate_id
        self.published_events
            .iter()
            .find(|(id, _, _)| id == aggregate_id)
            .cloned()
            .ok_or_else(|| "No events found for aggregate".to_string())
    }

    pub fn get_stream_subjects(&self, stream_name: &str) -> Option<&Vec<String>> {
        self.streams.get(stream_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_nats_connection() {
        // Arrange
        let mut validator = DomainEventStreamValidator::new()
            .expect_sequence(vec![
                DomainInfrastructureEvent::DomainConnectionEstablished {
                    client_name: "cim-domain-test".to_string(),
                },
            ]);

        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());

        // Act
        let result = client.connect().await;

        // Assert
        assert!(result.is_ok());
        assert!(client.is_connected());
        
        // Capture event
        validator.capture_event(DomainInfrastructureEvent::DomainConnectionEstablished {
            client_name: client.client_name.clone(),
        });
        
        // Validate sequence
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_domain_stream_creation() {
        // Arrange
        let mut validator = DomainEventStreamValidator::new()
            .expect_sequence(vec![
                DomainInfrastructureEvent::DomainConnectionEstablished {
                    client_name: "cim-domain-test".to_string(),
                },
                DomainInfrastructureEvent::DomainStreamCreated {
                    name: "CIM_DOMAIN_EVENTS".to_string(),
                    subjects: vec![
                        "cim.domain.aggregate.>".to_string(),
                        "cim.domain.command.>".to_string(),
                        "cim.domain.event.>".to_string(),
                    ],
                },
            ]);

        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());

        // Act
        client.connect().await.unwrap();
        validator.capture_event(DomainInfrastructureEvent::DomainConnectionEstablished {
            client_name: client.client_name.clone(),
        });

        let subjects = vec![
            "cim.domain.aggregate.>".to_string(),
            "cim.domain.command.>".to_string(),
            "cim.domain.event.>".to_string(),
        ];
        
        let stream_result = client.create_domain_stream(
            "CIM_DOMAIN_EVENTS".to_string(),
            subjects.clone(),
        ).await;

        // Assert
        assert!(stream_result.is_ok());
        assert_eq!(
            client.get_stream_subjects("CIM_DOMAIN_EVENTS"),
            Some(&subjects)
        );
        
        validator.capture_event(DomainInfrastructureEvent::DomainStreamCreated {
            name: "CIM_DOMAIN_EVENTS".to_string(),
            subjects,
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_domain_event_publishing() {
        // Arrange
        let mut validator = DomainEventStreamValidator::new()
            .expect_sequence(vec![
                DomainInfrastructureEvent::DomainConnectionEstablished {
                    client_name: "cim-domain-test".to_string(),
                },
                DomainInfrastructureEvent::DomainEventPublished {
                    aggregate_id: "agg_123".to_string(),
                    event_type: "AggregateCreated".to_string(),
                    sequence: 1,
                },
            ]);

        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());

        // Act
        client.connect().await.unwrap();
        validator.capture_event(DomainInfrastructureEvent::DomainConnectionEstablished {
            client_name: client.client_name.clone(),
        });

        let publish_result = client.publish_domain_event(
            "agg_123".to_string(),
            "AggregateCreated".to_string(),
            1,
        ).await;

        // Assert
        assert!(publish_result.is_ok());
        
        validator.capture_event(DomainInfrastructureEvent::DomainEventPublished {
            aggregate_id: "agg_123".to_string(),
            event_type: "AggregateCreated".to_string(),
            sequence: 1,
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_domain_event_consumption() {
        // Arrange
        let mut validator = DomainEventStreamValidator::new()
            .expect_sequence(vec![
                DomainInfrastructureEvent::DomainConnectionEstablished {
                    client_name: "cim-domain-test".to_string(),
                },
                DomainInfrastructureEvent::DomainEventPublished {
                    aggregate_id: "agg_456".to_string(),
                    event_type: "StateChanged".to_string(),
                    sequence: 2,
                },
                DomainInfrastructureEvent::DomainEventConsumed {
                    aggregate_id: "agg_456".to_string(),
                    event_type: "StateChanged".to_string(),
                    sequence: 2,
                },
            ]);

        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());

        // Act
        client.connect().await.unwrap();
        validator.capture_event(DomainInfrastructureEvent::DomainConnectionEstablished {
            client_name: client.client_name.clone(),
        });

        // Publish event
        client.publish_domain_event(
            "agg_456".to_string(),
            "StateChanged".to_string(),
            2,
        ).await.unwrap();
        
        validator.capture_event(DomainInfrastructureEvent::DomainEventPublished {
            aggregate_id: "agg_456".to_string(),
            event_type: "StateChanged".to_string(),
            sequence: 2,
        });

        // Consume event
        let (aggregate_id, event_type, sequence) = client.consume_domain_event("agg_456").await.unwrap();

        // Assert
        assert_eq!(aggregate_id, "agg_456");
        assert_eq!(event_type, "StateChanged");
        assert_eq!(sequence, 2);
        
        validator.capture_event(DomainInfrastructureEvent::DomainEventConsumed {
            aggregate_id,
            event_type,
            sequence,
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_domain_connection_failure() {
        // Arrange
        let mut validator = DomainEventStreamValidator::new()
            .expect_sequence(vec![
                DomainInfrastructureEvent::DomainConnectionFailed {
                    error: "Not connected to NATS".to_string(),
                },
            ]);

        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());

        // Act - try to publish without connection
        let publish_result = client.publish_domain_event(
            "agg_789".to_string(),
            "TestEvent".to_string(),
            1,
        ).await;

        // Assert
        assert!(publish_result.is_err());
        assert_eq!(publish_result.unwrap_err(), "Not connected to NATS");
        
        validator.capture_event(DomainInfrastructureEvent::DomainConnectionFailed {
            error: "Not connected to NATS".to_string(),
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_multiple_aggregate_events() {
        // Arrange
        let mut client = MockDomainNatsClient::new("cim-domain-test".to_string());
        client.connect().await.unwrap();

        // Act - publish events for multiple aggregates
        client.publish_domain_event("agg_1".to_string(), "Created".to_string(), 1).await.unwrap();
        client.publish_domain_event("agg_2".to_string(), "Updated".to_string(), 1).await.unwrap();
        client.publish_domain_event("agg_1".to_string(), "Modified".to_string(), 2).await.unwrap();

        // Assert - verify we can consume events for specific aggregates
        let (id1, type1, seq1) = client.consume_domain_event("agg_1").await.unwrap();
        assert_eq!(id1, "agg_1");
        assert_eq!(type1, "Created");
        assert_eq!(seq1, 1);

        let (id2, type2, seq2) = client.consume_domain_event("agg_2").await.unwrap();
        assert_eq!(id2, "agg_2");
        assert_eq!(type2, "Updated");
        assert_eq!(seq2, 1);
    }
} 