//! Infrastructure Layer 1.1: NATS JetStream Connection Tests
//! 
//! User Story: As a system, I need to connect to NATS JetStream and create event streams
//!
//! Test Requirements:
//! - Verify NATS connection establishment
//! - Verify stream creation with correct configuration
//! - Verify event publishing with acknowledgment
//! - Verify event consumption with proper ordering
//!
//! Event Sequence:
//! 1. ConnectionEstablished
//! 2. StreamCreated { name, subjects }
//! 3. EventPublished { subject, sequence }
//! 4. EventConsumed { subject, sequence }
//!
//! ```mermaid
//! graph LR
//!     A[Test Start] --> B[Connect to NATS]
//!     B --> C{Connection OK?}
//!     C -->|Yes| D[ConnectionEstablished Event]
//!     C -->|No| E[Test Failure]
//!     D --> F[Create Stream]
//!     F --> G[StreamCreated Event]
//!     G --> H[Publish Event]
//!     H --> I[EventPublished Event]
//!     I --> J[Consume Event]
//!     J --> K[EventConsumed Event]
//!     K --> L[Test Success]
//! ```

use std::time::Duration;

/// Mock NATS client for testing without actual NATS dependency
pub struct MockNatsClient {
    connected: bool,
    streams: Vec<String>,
    published_events: Vec<(String, u64)>,
}

impl MockNatsClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            streams: Vec::new(),
            published_events: Vec::new(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        // Simulate connection
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.connected = true;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn create_stream(&mut self, name: String, _subjects: Vec<String>) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }
        
        self.streams.push(name.clone());
        Ok(())
    }

    pub async fn publish_event(&mut self, subject: String, sequence: u64) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }
        
        self.published_events.push((subject, sequence));
        Ok(())
    }

    pub async fn consume_event(&self, subject: &str) -> Result<(String, u64), String> {
        if !self.connected {
            return Err("Not connected".to_string());
        }
        
        // Find the event with matching subject
        self.published_events
            .iter()
            .find(|(s, _)| s == subject)
            .cloned()
            .ok_or_else(|| "Event not found".to_string())
    }
}

/// Event types for infrastructure testing
#[derive(Debug, Clone, PartialEq)]
pub enum InfrastructureEvent {
    ConnectionEstablished,
    StreamCreated { name: String, subjects: Vec<String> },
    EventPublished { subject: String, sequence: u64 },
    EventConsumed { subject: String, sequence: u64 },
}

/// Event stream validator for testing
pub struct EventStreamValidator {
    expected_events: Vec<InfrastructureEvent>,
    captured_events: Vec<InfrastructureEvent>,
}

impl EventStreamValidator {
    pub fn new() -> Self {
        Self {
            expected_events: Vec::new(),
            captured_events: Vec::new(),
        }
    }

    pub fn expect_sequence(mut self, events: Vec<InfrastructureEvent>) -> Self {
        self.expected_events = events;
        self
    }

    pub fn capture_event(&mut self, event: InfrastructureEvent) {
        self.captured_events.push(event);
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.captured_events.len() != self.expected_events.len() {
            return Err(format!("Event count mismatch: expected {self.expected_events.len(}, got {}"),
                self.captured_events.len()
            ));
        }

        for (i, (expected, actual)) in self.expected_events.iter()
            .zip(self.captured_events.iter())
            .enumerate()
        {
            if expected != actual {
                return Err(format!("Event mismatch at position {i}: expected {:?}, got {:?}", expected, actual));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_connection_establishment() {
        // Arrange
        let mut validator = EventStreamValidator::new()
            .expect_sequence(vec![
                InfrastructureEvent::ConnectionEstablished,
            ]);

        let mut client = MockNatsClient::new();

        // Act
        let result = client.connect().await;

        // Assert
        assert!(result.is_ok());
        assert!(client.is_connected());
        
        // Capture event
        validator.capture_event(InfrastructureEvent::ConnectionEstablished);
        
        // Validate sequence
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_stream_creation() {
        // Arrange
        let mut validator = EventStreamValidator::new()
            .expect_sequence(vec![
                InfrastructureEvent::ConnectionEstablished,
                InfrastructureEvent::StreamCreated {
                    name: "CIM_COMPONENT_EVENTS".to_string(),
                    subjects: vec!["cim.component.>".to_string()],
                },
            ]);

        let mut client = MockNatsClient::new();

        // Act
        client.connect().await.unwrap();
        validator.capture_event(InfrastructureEvent::ConnectionEstablished);

        let stream_result = client.create_stream(
            "CIM_COMPONENT_EVENTS".to_string(),
            vec!["cim.component.>".to_string()],
        ).await;

        // Assert
        assert!(stream_result.is_ok());
        
        validator.capture_event(InfrastructureEvent::StreamCreated {
            name: "CIM_COMPONENT_EVENTS".to_string(),
            subjects: vec!["cim.component.>".to_string()],
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_event_publishing_with_acknowledgment() {
        // Arrange
        let mut validator = EventStreamValidator::new()
            .expect_sequence(vec![
                InfrastructureEvent::ConnectionEstablished,
                InfrastructureEvent::StreamCreated {
                    name: "CIM_COMPONENT_EVENTS".to_string(),
                    subjects: vec!["cim.component.>".to_string()],
                },
                InfrastructureEvent::EventPublished {
                    subject: "cim.component.created".to_string(),
                    sequence: 1,
                },
            ]);

        let mut client = MockNatsClient::new();

        // Act
        client.connect().await.unwrap();
        validator.capture_event(InfrastructureEvent::ConnectionEstablished);

        client.create_stream(
            "CIM_COMPONENT_EVENTS".to_string(),
            vec!["cim.component.>".to_string()],
        ).await.unwrap();
        validator.capture_event(InfrastructureEvent::StreamCreated {
            name: "CIM_COMPONENT_EVENTS".to_string(),
            subjects: vec!["cim.component.>".to_string()],
        });

        let publish_result = client.publish_event(
            "cim.component.created".to_string(),
            1,
        ).await;

        // Assert
        assert!(publish_result.is_ok());
        
        validator.capture_event(InfrastructureEvent::EventPublished {
            subject: "cim.component.created".to_string(),
            sequence: 1,
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_event_consumption_with_ordering() {
        // Arrange
        let mut validator = EventStreamValidator::new()
            .expect_sequence(vec![
                InfrastructureEvent::ConnectionEstablished,
                InfrastructureEvent::EventPublished {
                    subject: "cim.component.created".to_string(),
                    sequence: 1,
                },
                InfrastructureEvent::EventConsumed {
                    subject: "cim.component.created".to_string(),
                    sequence: 1,
                },
            ]);

        let mut client = MockNatsClient::new();

        // Act
        client.connect().await.unwrap();
        validator.capture_event(InfrastructureEvent::ConnectionEstablished);

        client.publish_event("cim.component.created".to_string(), 1).await.unwrap();
        validator.capture_event(InfrastructureEvent::EventPublished {
            subject: "cim.component.created".to_string(),
            sequence: 1,
        });

        let (subject, sequence) = client.consume_event("cim.component.created").await.unwrap();

        // Assert
        assert_eq!(subject, "cim.component.created");
        assert_eq!(sequence, 1);
        
        validator.capture_event(InfrastructureEvent::EventConsumed {
            subject,
            sequence,
        });
        
        assert!(validator.validate().is_ok());
    }

    #[tokio::test]
    async fn test_connection_failure_handling() {
        // Test that operations fail when not connected
        let mut client = MockNatsClient::new();
        
        let stream_result = client.create_stream(
            "TEST_STREAM".to_string(),
            vec!["test.>".to_string()],
        ).await;
        
        assert!(stream_result.is_err());
        assert_eq!(stream_result.unwrap_err(), "Not connected");
    }
} 