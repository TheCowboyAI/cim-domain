//! Comprehensive tests for cim-component isomorphic mapping and NATS transport

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[cfg(test)]
mod isomorphic_mapping_tests {
    use super::*;
    
    // User Story: As a developer, I want to ensure every DDD component has a corresponding ECS representation
    #[test]
    fn test_component_can_be_mapped_to_ecs() {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct TestComponent {
            id: uuid::Uuid,
            value: String,
        }
        
        impl Component for TestComponent {
            fn as_any(&self) -> &dyn Any { self }
            fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
            fn type_name(&self) -> &'static str { "TestComponent" }
            fn to_json(&self) -> serde_json::Value {
                serde_json::to_value(self).unwrap_or_default()
            }
        }
        
        let component = TestComponent {
            id: uuid::Uuid::new_v4(),
            value: "test".to_string(),
        };
        
        // Should be able to convert to ECS-compatible format
        let ecs_data = component.to_ecs_data();
        assert!(ecs_data.is_ok());
        
        let data = ecs_data.unwrap();
        assert_eq!(data.component_type, "TestComponent");
        assert!(data.data.as_object().unwrap().contains_key("id"));
        assert!(data.data.as_object().unwrap().contains_key("value"));
    }
    
    // User Story: As a developer, I want to reconstruct DDD components from ECS data
    #[test]
    fn test_component_can_be_reconstructed_from_ecs() {
        let ecs_data = EcsComponentData {
            component_type: "TestComponent".to_string(),
            data: serde_json::json!({
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "value": "reconstructed"
            }),
        };
        
        // Should be able to reconstruct from ECS data
        let registry = ComponentRegistry::new();
        // Need to register the type first
        registry.register_with_name::<TestComponent>("TestComponent");
        let component = registry.reconstruct_component(&ecs_data);
        assert!(component.is_ok());
        
        let comp = component.unwrap();
        assert_eq!(comp.type_name(), "TestComponent");
    }
    
    // User Story: As a developer, I want components to maintain their identity across DDD/ECS boundary
    #[test]
    fn test_component_identity_preserved() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct IdentityComponent {
            entity_id: uuid::Uuid,
            version: u64,
        }
        
        impl Component for IdentityComponent {
            fn as_any(&self) -> &dyn Any { self }
            fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
            fn type_name(&self) -> &'static str { "IdentityComponent" }
            fn to_json(&self) -> serde_json::Value {
                serde_json::to_value(self).unwrap_or_default()
            }
        }
        
        let original = IdentityComponent {
            entity_id: uuid::Uuid::new_v4(),
            version: 42,
        };
        
        // Convert to ECS and back
        let ecs_data = original.to_ecs_data().unwrap();
        let registry = ComponentRegistry::new();
        registry.register_with_name::<IdentityComponent>("IdentityComponent");
        
        let reconstructed = registry.reconstruct_component(&ecs_data).unwrap();
        let typed = reconstructed.as_any().downcast_ref::<IdentityComponent>().unwrap();
        
        assert_eq!(&original, typed);
    }
}

#[cfg(test)]
mod nats_transport_tests {
    use super::*;
    
    // User Story: As a system architect, I want all component updates to be published via NATS
    #[tokio::test]
    async fn test_component_updates_published_to_nats() {
        let nats_client = Arc::new(MockNatsClient::new());
        let publisher = ComponentEventPublisher::new(nats_client.clone());
        
        let component = TestComponent {
            id: uuid::Uuid::new_v4(),
            value: "test".to_string(),
        };
        
        let entity_id = uuid::Uuid::new_v4();
        let component_data = component.to_ecs_data().unwrap();
        let event = ComponentEvent::Added {
            entity_id,
            component_data,
        };
        
        // Publish component event
        let result = publisher.publish(event).await;
        assert!(result.is_ok());
        
        // Verify NATS message was sent
        let messages = nats_client.get_published_messages().await;
        assert_eq!(messages.len(), 1);
        
        let msg = &messages[0];
        assert!(msg.subject.starts_with("cim.component."));
        assert!(msg.subject.contains("added"));
    }
    
    // User Story: As a developer, I want to subscribe to component events from other processes
    #[tokio::test]
    async fn test_component_event_subscription() {
        let nats_client = Arc::new(MockNatsClient::new());
        let mut subscriber = ComponentEventSubscriber::new(nats_client.clone());
        
        // Subscribe to component events
        let mut event_stream = subscriber.subscribe("cim.component.>").await.unwrap();
        
        // Simulate incoming NATS message
        let component_data = EcsComponentData {
            component_type: "TestComponent".to_string(),
            data: serde_json::json!({
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "value": "from_nats"
            }),
        };
        
        let event = ComponentEvent::Added {
            entity_id: uuid::Uuid::new_v4(),
            component_data,
        };
        
        nats_client.simulate_message("cim.component.test.added", &event).await;
        
        // Should receive the event
        let received = event_stream.next().await;
        assert!(received.is_some());
    }
    
    // User Story: As a system operator, I want component sync to handle network failures gracefully
    #[tokio::test]
    async fn test_component_sync_retry_on_failure() {
        let nats_client = Arc::new(MockNatsClient::new());
        nats_client.set_should_fail(true);
        
        let publisher = ComponentEventPublisher::new(nats_client.clone())
            .with_retry_policy(RetryPolicy::exponential_backoff(3, 100));
        
        let component_data = TestComponent::default().to_ecs_data().unwrap();
        let event = ComponentEvent::Added {
            entity_id: uuid::Uuid::new_v4(),
            component_data,
        };
        
        // Should retry on failure
        let _result = publisher.publish(event).await;
        
        // Verify retry attempts
        let attempts = nats_client.get_attempt_count();
        assert_eq!(attempts, 3);
    }
}

#[cfg(test)]
mod bidirectional_sync_tests {
    use super::*;
    
    // User Story: As a developer, I want DDD changes to automatically sync to ECS
    #[tokio::test]
    async fn test_ddd_to_ecs_sync() {
        let sync_manager = ComponentSyncManager::new();
        
        // Register a DDD component update
        let entity_id = uuid::Uuid::new_v4();
        let component = TestComponent {
            id: uuid::Uuid::new_v4(),
            value: "ddd_value".to_string(),
        };
        
        sync_manager.register_ddd_update(entity_id, Box::new(component)).await.unwrap();
        
        // Should create ECS update event
        let pending_ecs_updates = sync_manager.get_pending_ecs_updates().await;
        assert_eq!(pending_ecs_updates.len(), 1);
        
        let update = &pending_ecs_updates[0];
        assert_eq!(update.entity_id, entity_id);
        assert_eq!(update.component_type, "TestComponent");
    }
    
    // User Story: As a developer, I want ECS changes to automatically sync to DDD
    #[tokio::test]
    async fn test_ecs_to_ddd_sync() {
        let sync_manager = ComponentSyncManager::new();
        
        // Register an ECS component update
        let entity_id = uuid::Uuid::new_v4();
        let ecs_data = EcsComponentData {
            component_type: "TestComponent".to_string(),
            data: serde_json::json!({
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "value": "ecs_value"
            }),
        };
        
        sync_manager.register_ecs_update(entity_id, ecs_data).await.unwrap();
        
        // Should create DDD update event
        let pending_ddd_updates = sync_manager.get_pending_ddd_updates().await;
        assert_eq!(pending_ddd_updates.len(), 1);
        
        let update = &pending_ddd_updates[0];
        assert_eq!(update.entity_id, entity_id);
    }
    
    // User Story: As a system architect, I want to prevent sync loops between DDD and ECS
    #[tokio::test]
    async fn test_sync_loop_prevention() {
        let sync_manager = ComponentSyncManager::new();
        
        let entity_id = uuid::Uuid::new_v4();
        let component = TestComponent {
            id: uuid::Uuid::new_v4(),
            value: "test".to_string(),
        };
        
        // Register DDD update
        sync_manager.register_ddd_update(entity_id, Box::new(component.clone())).await.unwrap();
        
        // Process sync to ECS
        sync_manager.process_pending_syncs().await.unwrap();
        
        // The resulting ECS update should be marked to prevent loop
        let ecs_updates = sync_manager.get_processed_ecs_updates().await;
        assert!(ecs_updates[0].metadata.contains_key("sync_source"));
        assert_eq!(ecs_updates[0].metadata["sync_source"], "ddd");
        
        // When this comes back from ECS, it should not create another DDD update
        let ecs_data = EcsComponentData {
            component_type: "TestComponent".to_string(),
            data: serde_json::json!({}),
        };
        
        let result = sync_manager.register_ecs_update_with_metadata(
            entity_id,
            ecs_data,
            ecs_updates[0].metadata.clone()
        ).await;
        
        // Should be skipped due to loop prevention
        assert!(result.is_ok());
        let pending_ddd = sync_manager.get_pending_ddd_updates().await;
        assert_eq!(pending_ddd.len(), 0);
    }
}

#[cfg(test)]
mod component_registry_tests {
    use super::*;
    
    // User Story: As a developer, I want to register component types for automatic marshalling
    #[test]
    fn test_component_type_registration() {
        let registry = ComponentRegistry::new();
        
        // Register a component type with its constructor
        registry.register_type(
            "TestComponent",
            Box::new(|data: &serde_json::Value| {
                let id = uuid::Uuid::parse_str(data["id"].as_str()?).ok()?;
                let value = data["value"].as_str()?.to_string();
                
                Some(Box::new(TestComponent { id, value }) as Box<dyn Component>)
            })
        );
        
        assert!(registry.is_registered("TestComponent"));
        assert!(!registry.is_registered("UnknownComponent"));
    }
    
    // User Story: As a developer, I want compile-time safety for component mappings
    #[test]
    fn test_typed_component_registration() {
        let mut registry = TypedComponentRegistry::new();
        
        registry.register::<TestComponent>();
        registry.register::<AnotherComponent>();
        
        // Should be able to create components by type
        let data = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "value": "typed"
        });
        
        let component = registry.create::<TestComponent>(&data);
        assert!(component.is_some());
        
        let comp = component.unwrap();
        assert_eq!(comp.value, "typed");
    }
}

// Test helpers
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestComponent {
    id: uuid::Uuid,
    value: String,
}

impl Default for TestComponent {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            value: String::new(),
        }
    }
}

impl Component for TestComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
    fn type_name(&self) -> &'static str { "TestComponent" }
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnotherComponent {
    data: Vec<u8>,
}

impl Component for AnotherComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn Component> { Box::new(self.clone()) }
    fn type_name(&self) -> &'static str { "AnotherComponent" }
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

// Mock NATS client for testing
#[derive(Clone)]
struct MockNatsClient {
    messages: Arc<Mutex<Vec<NatsMessage>>>,
    should_fail: Arc<AtomicBool>,
    attempt_count: Arc<AtomicUsize>,
    subscribers: Arc<Mutex<Vec<(String, tokio::sync::mpsc::UnboundedSender<ComponentEvent>)>>>,
}

#[async_trait::async_trait]
impl NatsClient for MockNatsClient {
    async fn publish(&self, subject: &str, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.attempt_count.fetch_add(1, Ordering::SeqCst);
        
        if self.should_fail.load(Ordering::Relaxed) {
            return Err("Mock failure".into());
        }
        
        let msg = NatsMessage {
            subject: subject.to_string(),
            data,
        };
        self.messages.lock().await.push(msg);
        Ok(())
    }
    
    async fn subscribe(&self, pattern: &str) -> Result<Box<dyn Any + Send + Sync>, Box<dyn std::error::Error>> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.subscribers.lock().await.push((pattern.to_string(), tx));
        Ok(Box::new(rx))
    }
}

impl MockNatsClient {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            should_fail: Arc::new(AtomicBool::new(false)),
            attempt_count: Arc::new(AtomicUsize::new(0)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn set_should_fail(&self, fail: bool) {
        self.should_fail.store(fail, Ordering::Relaxed);
    }
    
    async fn get_published_messages(&self) -> Vec<NatsMessage> {
        self.messages.lock().await.clone()
    }
    
    fn get_attempt_count(&self) -> usize {
        self.attempt_count.load(Ordering::Relaxed)
    }
    
    async fn simulate_message<T: Serialize>(&self, subject: &str, data: &T) {
        let msg = NatsMessage {
            subject: subject.to_string(),
            data: serde_json::to_vec(data).unwrap(),
        };
        self.messages.lock().await.push(msg.clone());
        
        // Send to matching subscribers
        if let Ok(event) = serde_json::from_slice::<ComponentEvent>(&msg.data) {
            let subscribers = self.subscribers.lock().await;
            for (pattern, tx) in subscribers.iter() {
                if subject.starts_with(&pattern.replace(">", "")) {
                    let _ = tx.send(event.clone());
                }
            }
        }
    }
}

#[derive(Clone)]
struct NatsMessage {
    subject: String,
    data: Vec<u8>,
}