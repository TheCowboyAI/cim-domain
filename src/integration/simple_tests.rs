//! Simple integration tests that demonstrate core functionality

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    #[tokio::test]
    async fn test_dependency_injection_basics() {
        // Create a container
        let mut builder = ContainerBuilder::new();
        
        // Register a simple singleton
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();
        
        builder.register_singleton::<Arc<Mutex<i32>>>(move || {
            counter_clone.clone()
        });
        
        let container = builder.build();
        
        // Resolve the singleton multiple times
        let instance1 = container.resolve::<Arc<Mutex<i32>>>().unwrap().await.unwrap();
        let instance2 = container.resolve::<Arc<Mutex<i32>>>().unwrap().await.unwrap();
        
        // Verify they're the same instance
        {
            let mut val1 = instance1.lock().await;
            *val1 = 42;
        }
        
        {
            let val2 = instance2.lock().await;
            assert_eq!(*val2, 42);
        }
    }
    
    #[tokio::test]
    async fn test_service_registry_basic() {
        let registry = ServiceRegistry::new();
        let container = ContainerBuilder::new().build();
        
        // Track how many times the factory is called
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();
        
        // Register a singleton service
        registry.register(
            ServiceDescriptor::new::<i32>(
                ServiceLifetime::Singleton,
                Box::new(move |_| {
                    let count = call_count_clone.clone();
                    Box::pin(async move {
                        let mut c = count.lock().await;
                        *c += 1;
                        Ok(Arc::new(42))
                    })
                }),
            )
        ).await.unwrap();
        
        // Resolve multiple times
        let _val1 = registry.resolve::<i32>(&container).await.unwrap();
        let _val2 = registry.resolve::<i32>(&container).await.unwrap();
        
        // Factory should only be called once for singleton
        let final_count = *call_count.lock().await;
        assert_eq!(final_count, 1);
    }
    
    #[tokio::test]
    async fn test_event_bridge_basic() {
        let bridge = EventBridge::new(Default::default());
        
        // Set up a simple router
        let mut router = EventRouter::new();
        router.add_rule(
            "test.*".to_string(),
            vec!["output.stream".to_string()],
            None,
        );
        
        bridge.set_router(router).await;
        
        // Track received events
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_clone = received.clone();
        
        // Subscribe to output stream
        bridge.subscribe(
            "output.stream".to_string(),
            Box::new(move |event| {
                let r = received_clone.clone();
                Box::pin(async move {
                    let mut events = r.lock().await;
                    events.push(event.subject());
                    Ok(())
                })
            }),
        ).await;
        
        // Publish a test event
        #[derive(Debug, Clone)]
        struct TestEvent;
        
        impl crate::events::DomainEvent for TestEvent {
            fn subject(&self) -> String {
                "test.event.v1".to_string()
            }
            
            fn aggregate_id(&self) -> uuid::Uuid {
                uuid::Uuid::new_v4()
            }
            
            fn event_type(&self) -> &'static str {
                "TestEvent"
            }
        }
        
        bridge.publish(Box::new(TestEvent)).await.unwrap();
        
        // Give async processing time to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Verify event was routed
        let events = received.lock().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], "test.event.v1");
    }
    
    #[tokio::test]
    async fn test_domain_bridge_translation() {
        // Create a simple translator
        let mut translator = PropertyBasedTranslator::new();
        translator.add_command_mapping(
            "CreateUser".to_string(),
            "CreateEmployee".to_string(),
            vec![
                ("username".to_string(), "employee_id".to_string()),
                ("email".to_string(), "work_email".to_string()),
            ],
        );
        
        // Create a bridge
        let mut bridge = DomainBridge::new("Users".to_string(), "HR".to_string());
        bridge.set_translator(Box::new(translator));
        
        // Test command translation
        let command = SerializedCommand {
            command_type: "CreateUser".to_string(),
            aggregate_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::json!({
                "username": "john_doe",
                "email": "john@example.com",
                "department": "Engineering"
            }),
        };
        
        let context = TranslationContext::new()
            .with_source_data("domain".to_string(), serde_json::json!("Users"))
            .with_target_data("domain".to_string(), serde_json::json!("HR"));
        
        // Send command through bridge (without adapter, just translation)
        let translated = bridge.send_command(command, &context).await;
        
        // Should fail because no adapter is configured
        assert!(translated.is_err());
        
        // But we can test the translator directly
        let translator = bridge.translator.as_ref().unwrap();
        let command = SerializedCommand {
            command_type: "CreateUser".to_string(),
            aggregate_id: uuid::Uuid::new_v4().to_string(),
            payload: serde_json::json!({
                "username": "john_doe",
                "email": "john@example.com"
            }),
        };
        
        let translated = translator.translate_command(command, &context).await.unwrap();
        
        assert_eq!(translated.command_type, "CreateEmployee");
        assert_eq!(translated.payload["employee_id"], "john_doe");
        assert_eq!(translated.payload["work_email"], "john@example.com");
    }
    
    #[tokio::test]
    async fn test_bridge_registry() {
        let registry = BridgeRegistry::new();
        
        // Register bridges
        let bridge1 = DomainBridge::new("Domain1".to_string(), "Domain2".to_string());
        let bridge2 = DomainBridge::new("Domain2".to_string(), "Domain3".to_string());
        
        registry.register(bridge1).await.unwrap();
        registry.register(bridge2).await.unwrap();
        
        // Find bridges
        let from_domain1 = registry.find_from_source("Domain1").await;
        assert_eq!(from_domain1.len(), 1);
        assert_eq!(from_domain1[0].1, "Domain2");
        
        let to_domain3 = registry.find_to_target("Domain3").await;
        assert_eq!(to_domain3.len(), 1);
        assert_eq!(to_domain3[0].0, "Domain2");
        
        // Try to register duplicate
        let duplicate = DomainBridge::new("Domain1".to_string(), "Domain2".to_string());
        let result = registry.register(duplicate).await;
        assert!(result.is_err());
    }
}