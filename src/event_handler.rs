// Copyright 2025 Cowboy AI, LLC.

//! Generic event handler trait for domain events

use async_trait::async_trait;

/// Trait for handling specific domain events
#[async_trait]
pub trait EventHandler<E> {
    /// Error type for this handler
    type Error;

    /// Handle a domain event
    async fn handle(&self, event: E) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    
    // Test event type
    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent {
        id: String,
        value: i32,
    }
    
    // Test error type
    #[derive(Debug, PartialEq)]
    enum TestError {
        ProcessingError(String),
        ValidationError(String),
    }
    
    // Simple event handler that counts events
    struct CountingEventHandler {
        count: Arc<Mutex<usize>>,
    }
    
    impl CountingEventHandler {
        fn new() -> Self {
            Self {
                count: Arc::new(Mutex::new(0)),
            }
        }
        
        fn get_count(&self) -> usize {
            *self.count.lock().unwrap()
        }
    }
    
    #[async_trait]
    impl EventHandler<TestEvent> for CountingEventHandler {
        type Error = TestError;
        
        async fn handle(&self, _event: TestEvent) -> Result<(), Self::Error> {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            Ok(())
        }
    }
    
    // Event handler that validates events
    struct ValidatingEventHandler {
        min_value: i32,
    }
    
    #[async_trait]
    impl EventHandler<TestEvent> for ValidatingEventHandler {
        type Error = TestError;
        
        async fn handle(&self, event: TestEvent) -> Result<(), Self::Error> {
            if event.value < self.min_value {
                Err(TestError::ValidationError(
                    format!("Value {} is less than minimum {}", event.value, self.min_value)
                ))
            } else {
                Ok(())
            }
        }
    }
    
    // Event handler that processes and transforms events
    struct ProcessingEventHandler {
        processed_events: Arc<Mutex<Vec<TestEvent>>>,
    }
    
    impl ProcessingEventHandler {
        fn new() -> Self {
            Self {
                processed_events: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        fn get_processed_events(&self) -> Vec<TestEvent> {
            self.processed_events.lock().unwrap().clone()
        }
    }
    
    #[async_trait]
    impl EventHandler<TestEvent> for ProcessingEventHandler {
        type Error = TestError;
        
        async fn handle(&self, mut event: TestEvent) -> Result<(), Self::Error> {
            // Simulate processing by modifying the event
            event.value *= 2;
            
            let mut events = self.processed_events.lock().unwrap();
            events.push(event);
            Ok(())
        }
    }
    
    // Composite handler that delegates to multiple handlers
    struct CompositeEventHandler {
        handlers: Vec<Box<dyn EventHandler<TestEvent, Error = TestError> + Send + Sync>>,
    }
    
    #[async_trait]
    impl EventHandler<TestEvent> for CompositeEventHandler {
        type Error = TestError;
        
        async fn handle(&self, event: TestEvent) -> Result<(), Self::Error> {
            for handler in &self.handlers {
                handler.handle(event.clone()).await?;
            }
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_counting_event_handler() {
        let handler = CountingEventHandler::new();
        
        assert_eq!(handler.get_count(), 0);
        
        let event1 = TestEvent {
            id: "1".to_string(),
            value: 10,
        };
        
        handler.handle(event1).await.unwrap();
        assert_eq!(handler.get_count(), 1);
        
        let event2 = TestEvent {
            id: "2".to_string(),
            value: 20,
        };
        
        handler.handle(event2).await.unwrap();
        assert_eq!(handler.get_count(), 2);
    }
    
    #[tokio::test]
    async fn test_validating_event_handler() {
        let handler = ValidatingEventHandler { min_value: 10 };
        
        let valid_event = TestEvent {
            id: "valid".to_string(),
            value: 15,
        };
        
        assert!(handler.handle(valid_event).await.is_ok());
        
        let invalid_event = TestEvent {
            id: "invalid".to_string(),
            value: 5,
        };
        
        let result = handler.handle(invalid_event).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            TestError::ValidationError(msg) => {
                assert!(msg.contains("less than minimum"));
            }
            _ => panic!("Expected validation error"),
        }
    }
    
    #[tokio::test]
    async fn test_processing_event_handler() {
        let handler = ProcessingEventHandler::new();
        
        let event = TestEvent {
            id: "process".to_string(),
            value: 5,
        };
        
        handler.handle(event.clone()).await.unwrap();
        
        let processed = handler.get_processed_events();
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].id, "process");
        assert_eq!(processed[0].value, 10); // Should be doubled
    }
    
    #[tokio::test]
    async fn test_composite_event_handler() {
        let counting_handler = CountingEventHandler::new();
        let processing_handler = ProcessingEventHandler::new();
        
        // Store references to check results later
        let count_ref = counting_handler.count.clone();
        let events_ref = processing_handler.processed_events.clone();
        
        let composite = CompositeEventHandler {
            handlers: vec![
                Box::new(counting_handler),
                Box::new(processing_handler),
            ],
        };
        
        let event = TestEvent {
            id: "composite".to_string(),
            value: 7,
        };
        
        composite.handle(event).await.unwrap();
        
        assert_eq!(*count_ref.lock().unwrap(), 1);
        assert_eq!(events_ref.lock().unwrap().len(), 1);
        assert_eq!(events_ref.lock().unwrap()[0].value, 14);
    }
    
    #[tokio::test]
    async fn test_error_propagation() {
        let validating_handler = ValidatingEventHandler { min_value: 10 };
        let counting_handler = CountingEventHandler::new();
        let count_ref = counting_handler.count.clone();
        
        let composite = CompositeEventHandler {
            handlers: vec![
                Box::new(validating_handler),
                Box::new(counting_handler),
            ],
        };
        
        let invalid_event = TestEvent {
            id: "error".to_string(),
            value: 3,
        };
        
        // Should fail on validation and not reach counting handler
        let result = composite.handle(invalid_event).await;
        assert!(result.is_err());
        assert_eq!(*count_ref.lock().unwrap(), 0);
    }
    
    // Test that the trait can be used with different event types
    #[derive(Debug)]
    struct StringEvent(String);
    
    struct StringEventHandler;
    
    #[async_trait]
    impl EventHandler<StringEvent> for StringEventHandler {
        type Error = String;
        
        async fn handle(&self, event: StringEvent) -> Result<(), Self::Error> {
            if event.0.is_empty() {
                Err("Empty string not allowed".to_string())
            } else {
                Ok(())
            }
        }
    }
    
    #[tokio::test]
    async fn test_different_event_type() {
        let handler = StringEventHandler;
        
        assert!(handler.handle(StringEvent("hello".to_string())).await.is_ok());
        assert!(handler.handle(StringEvent("".to_string())).await.is_err());
    }
    
    // Test async behavior
    struct AsyncEventHandler {
        delay_ms: u64,
    }
    
    #[async_trait]
    impl EventHandler<TestEvent> for AsyncEventHandler {
        type Error = TestError;
        
        async fn handle(&self, _event: TestEvent) -> Result<(), Self::Error> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_async_handler() {
        let handler = AsyncEventHandler { delay_ms: 10 };
        let event = TestEvent {
            id: "async".to_string(),
            value: 42,
        };
        
        let start = tokio::time::Instant::now();
        handler.handle(event).await.unwrap();
        let elapsed = start.elapsed();
        
        assert!(elapsed.as_millis() >= 10);
    }
}
