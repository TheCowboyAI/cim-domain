//! Example demonstrating the complete integration layer
//!
//! This example shows how to:
//! - Set up cross-aggregate event routing
//! - Configure domain bridges
//! - Use saga orchestration for complex workflows
//! - Implement dependency injection

use cim_domain::{
    DomainError,
    integration::*,
    infrastructure::saga::*,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Example domain event
#[derive(Debug, Clone)]
struct ExampleEvent {
    id: Uuid,
    event_type: String,
    aggregate_type: String,
    payload: serde_json::Value,
}

// Trait for domain events
trait DomainEvent: Send + Sync {
    fn subject(&self) -> String;
    fn aggregate_id(&self) -> Uuid;
    fn event_type(&self) -> &'static str;
}

impl DomainEvent for ExampleEvent {
    fn subject(&self) -> String {
        format!("{}.{}.v1", self.aggregate_type, self.event_type)
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.id
    }
    
    fn event_type(&self) -> &'static str {
        Box::leak(self.event_type.clone().into_boxed_str())
    }
}

/// Example command for saga execution
#[derive(Debug)]
struct ExampleSagaCommand {
    command_type: String,
    aggregate_id: String,
}

impl SagaCommand for ExampleSagaCommand {
    fn command_type(&self) -> &str {
        &self.command_type
    }
    
    fn aggregate_id(&self) -> &str {
        &self.aggregate_id
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Example command bus implementation
struct ExampleCommandBus {
    handlers: Arc<RwLock<Vec<String>>>,
}

#[async_trait]
impl CommandBus for ExampleCommandBus {
    async fn send(&self, command: Box<dyn SagaCommand>) -> Result<(), String> {
        let mut handlers = self.handlers.write().await;
        handlers.push(format!("Executed: {} for {}", command.command_type(), command.aggregate_id()));
        println!("Command executed: {} for aggregate {}", command.command_type(), command.aggregate_id());
        Ok(())
    }
}

/// Example event handler that logs events
struct LoggingEventHandler {
    name: String,
}

#[async_trait]
impl AggregateEventHandler for LoggingEventHandler {
    async fn handle_event(&self, event: &Box<dyn DomainEvent>) -> Result<(), DomainError> {
        println!("[{}] Received event: {} for aggregate {}", 
            self.name, 
            event.event_type(), 
            event.aggregate_id()
        );
        Ok(())
    }
}

/// Employee onboarding saga definition
struct EmployeeOnboardingSaga;

#[async_trait]
impl SagaDefinition for EmployeeOnboardingSaga {
    fn saga_type(&self) -> &str {
        "EmployeeOnboarding"
    }
    
    async fn create_saga(&self, context: serde_json::Value) -> Result<Saga, SagaError> {
        let person_id = context.get("person_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SagaError::Serialization("Missing person_id".to_string()))?;
            
        let org_id = context.get("organization_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SagaError::Serialization("Missing organization_id".to_string()))?;
        
        Ok(Saga {
            id: Uuid::new_v4(),
            name: "EmployeeOnboarding".to_string(),
            steps: vec![
                SagaStep {
                    id: "create_employee_profile".to_string(),
                    domain: "Person".to_string(),
                    command_type: "AddEmploymentComponent".to_string(),
                    depends_on: vec![],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
                SagaStep {
                    id: "add_to_organization".to_string(),
                    domain: "Organization".to_string(),
                    command_type: "AddMember".to_string(),
                    depends_on: vec!["create_employee_profile".to_string()],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
                SagaStep {
                    id: "create_agent_account".to_string(),
                    domain: "Agent".to_string(),
                    command_type: "CreateAgent".to_string(),
                    depends_on: vec!["add_to_organization".to_string()],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
                SagaStep {
                    id: "apply_default_policies".to_string(),
                    domain: "Policy".to_string(),
                    command_type: "ApplyDefaultPolicies".to_string(),
                    depends_on: vec!["create_agent_account".to_string()],
                    retry_policy: RetryPolicy::default(),
                    timeout_ms: 30000,
                },
            ],
            state: SagaState::Pending,
            compensations: std::collections::HashMap::new(),
            context: context.as_object()
                .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
            metadata: std::collections::HashMap::new(),
        })
    }
    
    async fn event_to_input(
        &self,
        _saga: &Saga,
        event: &dyn DomainEvent,
    ) -> Option<SagaTransitionInput> {
        match event.event_type() {
            "ComponentAdded" => Some(SagaTransitionInput::StepCompleted {
                step_id: "create_employee_profile".to_string(),
                result: serde_json::json!({"success": true}),
            }),
            "MemberAdded" => Some(SagaTransitionInput::StepCompleted {
                step_id: "add_to_organization".to_string(),
                result: serde_json::json!({"success": true}),
            }),
            "AgentCreated" => Some(SagaTransitionInput::StepCompleted {
                step_id: "create_agent_account".to_string(),
                result: serde_json::json!({"success": true}),
            }),
            "PolicyApplied" => Some(SagaTransitionInput::StepCompleted {
                step_id: "apply_default_policies".to_string(),
                result: serde_json::json!({"success": true}),
            }),
            _ => None,
        }
    }
    
    async fn on_completed(&self, saga: &Saga) -> Result<(), SagaError> {
        println!("Employee onboarding completed for saga: {}", saga.id);
        Ok(())
    }
    
    async fn on_failed(&self, saga: &Saga, error: &str) -> Result<(), SagaError> {
        println!("Employee onboarding failed for saga {}: {}", saga.id, error);
        Ok(())
    }
}

/// Process policy that starts employee onboarding when a person is created
#[derive(Debug)]
struct NewEmployeePolicy;

#[async_trait]
impl ProcessPolicy for NewEmployeePolicy {
    async fn should_start(
        &self,
        event: &dyn DomainEvent,
    ) -> Option<(String, serde_json::Value)> {
        if event.event_type() == "PersonCreated" && event.subject().starts_with("Person") {
            println!("New person created with ID: {}", event.aggregate_id());
            
            // In a real scenario, we'd check additional conditions
            // For demo, we'll onboard everyone
            Some((
                "EmployeeOnboarding".to_string(),
                serde_json::json!({
                    "person_id": event.aggregate_id().to_string(),
                    "organization_id": Uuid::new_v4().to_string(),
                }),
            ))
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CIM Domain Integration Example");
    println!("==============================\n");

    // 1. Set up dependency injection
    println!("1. Setting up dependency injection...");
    let mut container_builder = ContainerBuilder::new();
    
    // Register services
    container_builder.register_singleton::<Arc<ExampleCommandBus>>(|| {
        Arc::new(ExampleCommandBus {
            handlers: Arc::new(RwLock::new(Vec::new())),
        })
    });
    
    let container = container_builder.build();
    println!("   ✓ Container configured\n");

    // 2. Set up aggregate event router
    println!("2. Configuring aggregate event routing...");
    let event_router = AggregateEventRouter::new();
    
    // Register handlers
    event_router.register_handler(
        "Person",
        Box::new(LoggingEventHandler { name: "PersonHandler".to_string() }),
    ).await?;
    
    event_router.register_handler(
        "Organization", 
        Box::new(LoggingEventHandler { name: "OrgHandler".to_string() }),
    ).await?;
    
    // Configure standard routes
    event_router.configure_standard_routes().await?;
    
    // Add custom route: When person is created, notify organization
    event_router.register_route(
        "Person",
        "Organization",
        "Person.PersonCreated.*",
        |event| {
            if event.subject().starts_with("Person.PersonCreated") {
                Some(Box::new(ExampleEvent {
                    id: Uuid::new_v4(),
                    event_type: "MemberAdded".to_string(),
                    aggregate_type: "Organization".to_string(),
                    payload: serde_json::json!({
                        "person_id": event.aggregate_id(),
                        "role": "Employee",
                    }),
                }) as Box<dyn DomainEvent>)
            } else {
                None
            }
        },
    ).await?;
    println!("   ✓ Event routing configured\n");

    // 3. Set up saga orchestration
    println!("3. Setting up saga orchestration...");
    let command_bus = container.resolve::<Arc<ExampleCommandBus>>()?.await?;
    let saga_coordinator = Arc::new(SagaCoordinator::new(command_bus as Arc<dyn CommandBus>));
    
    // Register saga definitions
    saga_coordinator.register_saga(Arc::new(EmployeeOnboardingSaga)).await;
    
    // Set up process manager
    let process_manager = ProcessManager::new(saga_coordinator.clone());
    process_manager.register_policy(Box::new(NewEmployeePolicy)).await;
    println!("   ✓ Saga orchestration configured\n");

    // 4. Set up domain bridges
    println!("4. Configuring domain bridges...");
    let bridge_registry = BridgeRegistry::new();
    
    // Example: Bridge between Person and HR domains
    let mut person_hr_bridge = DomainBridge::new("Person".to_string(), "HR".to_string());
    
    // Configure translator with property mappings
    let mut translator = PropertyBasedTranslator::new();
    translator.add_command_mapping(
        "CreatePerson".to_string(),
        "CreateEmployee".to_string(),
        vec![
            ("name".to_string(), "employee_name".to_string()),
            ("email".to_string(), "work_email".to_string()),
        ],
    );
    person_hr_bridge.set_translator(Box::new(translator));
    
    bridge_registry.register_bridge(person_hr_bridge).await?;
    println!("   ✓ Domain bridges configured\n");

    // 5. Demonstrate the integration
    println!("5. Demonstrating integration flow...\n");
    
    // Create a person event (triggers the whole flow)
    let person_event = Box::new(ExampleEvent {
        id: Uuid::new_v4(),
        event_type: "PersonCreated".to_string(),
        aggregate_type: "Person".to_string(),
        payload: serde_json::json!({
            "name": "Jane Smith",
            "email": "jane.smith@example.com",
        }),
    }) as Box<dyn DomainEvent>;
    
    println!("   → Creating new person: Jane Smith");
    
    // Route the event through aggregate router
    let routed_events = event_router.route_event("Person", &person_event).await?;
    println!("   → Routed {} events through aggregate router", routed_events.len());
    
    // Process through saga orchestration
    process_manager.handle_event(&*person_event, None).await?;
    println!("   → Started employee onboarding saga");
    
    // 6. Show service registry in action
    println!("\n6. Demonstrating service registry...");
    let service_registry = ServiceRegistry::new();
    
    // Register a service
    service_registry.register(
        ServiceDescriptor::new::<LoggingEventHandler>(
            ServiceLifetime::Singleton,
            Box::new(|_| Ok(Arc::new(LoggingEventHandler { 
                name: "GlobalLogger".to_string() 
            }))),
        )
    ).await?;
    
    // Resolve the service
    let logger = service_registry.resolve::<LoggingEventHandler>(&container).await?;
    logger.handle_event(&person_event).await?;
    println!("   ✓ Service registry demonstrated\n");

    // 7. Demonstrate cross-domain search
    println!("7. Demonstrating cross-domain search...");
    let search_engine = CrossDomainSearchEngine::new();
    
    // Register a domain searcher
    struct ExampleSearcher;
    
    #[async_trait]
    impl DomainSearcher for ExampleSearcher {
        fn domain_name(&self) -> &str {
            "Person"
        }
        
        async fn search(&self, query: &str) -> Result<Vec<SearchResult>, DomainError> {
            if query.contains("Jane") {
                Ok(vec![SearchResult {
                    domain: "Person".to_string(),
                    entity_id: Uuid::new_v4().to_string(),
                    entity_type: "Person".to_string(),
                    relevance_score: 0.95,
                    matched_fields: vec!["name".to_string()],
                    metadata: std::collections::HashMap::new(),
                }])
            } else {
                Ok(vec![])
            }
        }
        
        async fn get_relationships(
            &self,
            _entity_id: &str,
        ) -> Result<Vec<(String, String, String)>, DomainError> {
            Ok(vec![])
        }
    }
    
    search_engine.register_domain_searcher(Box::new(ExampleSearcher)).await;
    
    // Perform a search
    let results = search_engine.search("Jane Smith").await?;
    println!("   → Search returned {} results", results.len());
    if !results.is_empty() {
        println!("   → Found: {} in {} domain with score {:.2}", 
            results[0].entity_type, 
            results[0].domain,
            results[0].relevance_score
        );
    }
    
    // 8. Demonstrate event bridge
    println!("\n8. Demonstrating event bridge...");
    let event_bridge = EventBridge::new();
    
    // Configure routing
    let mut router = EventRouter::new();
    router.add_rule(
        "Person.*".to_string(),
        vec!["hr.events".to_string(), "audit.log".to_string()],
        None,
    );
    event_bridge.set_router(router).await;
    
    // Subscribe to events
    event_bridge.subscribe(
        "hr.events".to_string(),
        Box::new(|event| {
            Box::pin(async move {
                println!("   → HR system received: {}", event.event_type());
                Ok(())
            })
        }),
    ).await;
    
    // Publish an event
    event_bridge.publish(person_event.clone()).await?;
    println!("   ✓ Event bridge demonstrated\n");
    
    println!("✅ Integration example completed successfully!");
    println!("\nThe integration layer provides:");
    println!("  • Cross-aggregate event routing");
    println!("  • Domain saga orchestration");
    println!("  • Dependency injection");
    println!("  • Service lifecycle management");
    println!("  • Domain bridges for translation");
    println!("  • Cross-domain search");
    println!("  • Event-driven integration");
    
    Ok(())
}