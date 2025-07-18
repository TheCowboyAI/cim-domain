// Copyright 2025 Cowboy AI, LLC.

//! Example demonstrating integration patterns
//!
//! This example shows:
//! - Domain bridges for cross-domain communication
//! - Event routing between aggregates
//! - Service registry and dependency injection
//! - Basic integration patterns

use cim_domain::{
    // Integration components
    integration::{
        DomainBridge, BridgeRegistry,
        AggregateEventRouter, AggregateEventHandler,
        ServiceRegistry, ServiceLifetime,
        DependencyContainer,
    },
    
    // Domain types
    DomainEvent, DomainError,
};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use uuid::Uuid;

/// Example domain event: UserCreated
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserCreated {
    user_id: Uuid,
    username: String,
    email: String,
}

impl DomainEvent for UserCreated {
    fn subject(&self) -> String {
        format!("user.created.{}", self.user_id)
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.user_id
    }
    
    fn event_type(&self) -> &'static str {
        "UserCreated"
    }
}

/// Example domain event: EmployeeAdded
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmployeeAdded {
    employee_id: Uuid,
    name: String,
    email: String,
    department: String,
}

impl DomainEvent for EmployeeAdded {
    fn subject(&self) -> String {
        format!("employee.added.{}", self.employee_id)
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.employee_id
    }
    
    fn event_type(&self) -> &'static str {
        "EmployeeAdded"
    }
}

/// Example event handler for User aggregate
struct UserEventHandler;

#[async_trait]
impl AggregateEventHandler for UserEventHandler {
    async fn handle_event(&self, event: &Box<dyn DomainEvent>) -> Result<(), DomainError> {
        println!("   [User] Handling {} event for aggregate {}", 
            event.event_type(), 
            event.aggregate_id()
        );
        Ok(())
    }
}

/// Example event handler for HR aggregate
struct HREventHandler;

#[async_trait]
impl AggregateEventHandler for HREventHandler {
    async fn handle_event(&self, event: &Box<dyn DomainEvent>) -> Result<(), DomainError> {
        println!("   [HR] Handling {} event for aggregate {}", 
            event.event_type(), 
            event.aggregate_id()
        );
        Ok(())
    }
}

/// Example service: EmailService
struct EmailService {
    name: String,
}

impl EmailService {
    fn new() -> Self {
        Self {
            name: "EmailService".to_string(),
        }
    }
    
    fn send_email(&self, to: &str, subject: &str, body: &str) {
        println!("   [{}] Sending email to: {}", self.name, to);
        println!("     Subject: {}", subject);
        println!("     Body: {}", body);
    }
}

/// Example service: NotificationService
struct NotificationService {
    email_service: Arc<EmailService>,
}

impl NotificationService {
    fn new(email_service: Arc<EmailService>) -> Self {
        Self { email_service }
    }
    
    fn notify_user_created(&self, username: &str, email: &str) {
        self.email_service.send_email(
            email,
            "Welcome!",
            &format!("Welcome to our platform, {}!", username),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Integration Example");
    println!("==================\n");
    
    // Example 1: Domain Bridges
    println!("1. Domain Bridges...");
    
    let bridge_registry = BridgeRegistry::new();
    
    // Create bridges between domains
    let user_hr_bridge = DomainBridge::new("User".to_string(), "HR".to_string());
    bridge_registry.register(user_hr_bridge).await?;
    
    let hr_it_bridge = DomainBridge::new("HR".to_string(), "IT".to_string());
    bridge_registry.register(hr_it_bridge).await?;
    
    println!("   ✓ Registered User → HR bridge");
    println!("   ✓ Registered HR → IT bridge");
    
    // Get bridge
    let bridge = bridge_registry.get_bridge("User", "HR").await?;
    println!("   ✓ Found bridge: {} → {}", bridge.source_domain, bridge.target_domain);
    
    // Example 2: Aggregate Event Router
    println!("\n2. Aggregate Event Router...");
    
    let event_router = AggregateEventRouter::new();
    
    // Register handlers
    event_router.register_handler("User", Box::new(UserEventHandler)).await?;
    event_router.register_handler("HR", Box::new(HREventHandler)).await?;
    
    println!("   ✓ Registered event handlers");
    
    // Configure route: User events → HR
    event_router.register_route(
        "User",
        "HR",
        "User.Created.*",
        |event| {
            // Transform UserCreated to EmployeeAdded
            if event.event_type() == "UserCreated" {
                // In real system, would deserialize and transform
                println!("     → Transforming UserCreated to EmployeeAdded");
                Some(Box::new(EmployeeAdded {
                    employee_id: event.aggregate_id(),
                    name: "Transformed User".to_string(),
                    email: "user@example.com".to_string(),
                    department: "Engineering".to_string(),
                }) as Box<dyn DomainEvent>)
            } else {
                None
            }
        },
    ).await?;
    
    println!("   ✓ Configured User.Created → HR route");
    
    // Route an event
    let user_created = Box::new(UserCreated {
        user_id: Uuid::new_v4(),
        username: "john_doe".to_string(),
        email: "john@example.com".to_string(),
    }) as Box<dyn DomainEvent>;
    
    println!("\n   Routing UserCreated event...");
    let routed_events = event_router.route_event("User", &user_created).await?;
    println!("   ✓ Event routed to {} aggregates", routed_events.len());
    
    // Example 3: Service Registry
    println!("\n3. Service Registry...");
    
    let service_registry = ServiceRegistry::new();
    
    // Register services
    service_registry.register::<EmailService, EmailService, _>(
        ServiceLifetime::Singleton,
        |_| Ok(Box::new(EmailService::new())),
    ).await?;
    
    println!("   ✓ Registered EmailService as singleton");
    
    // Service registry demonstrates singleton pattern
    println!("   ✓ EmailService registered with singleton lifetime");
    
    // Example 4: Dependency Injection
    println!("\n4. Dependency Injection...");
    
    let container = DependencyContainer::new();
    
    // Register EmailService
    container.register_instance(Arc::new(EmailService::new())).await?;
    println!("   ✓ Registered EmailService");
    
    // Resolve service
    let email_service = container.resolve::<Arc<EmailService>>().await?;
    println!("   ✓ Resolved EmailService");
    
    // Use the service
    email_service.send_email(
        "jane@example.com",
        "Welcome!",
        "Welcome to our platform, Jane!"
    );
    
    // Example 5: Pattern Matching Routes
    println!("\n5. Pattern Matching Routes...");
    
    // Configure wildcard routes
    event_router.register_route(
        "*",           // Any source
        "Audit",       // To audit aggregate
        "*.*.v2",      // Any v2 events
        |event| {
            println!("     → Routing v2 event to Audit: {}", event.event_type());
            None // For demo, don't actually transform
        },
    ).await?;
    
    println!("   ✓ Configured wildcard route: *.*.v2 → Audit");
    
    // Configure standard routes
    event_router.configure_standard_routes().await?;
    println!("   ✓ Configured standard cross-aggregate routes");
    
    // Example 6: Bridge Metadata
    println!("\n6. Bridge Configuration...");
    
    // Create bridge with metadata
    let mut config_bridge = DomainBridge::new("Config".to_string(), "Runtime".to_string());
    config_bridge.metadata.insert("version".to_string(), "2.0".to_string());
    config_bridge.metadata.insert("protocol".to_string(), "async".to_string());
    
    bridge_registry.register(config_bridge).await?;
    
    let config_runtime_bridge = bridge_registry.get_bridge("Config", "Runtime").await?;
    println!("   ✓ Bridge {} → {} metadata:", config_runtime_bridge.source_domain, config_runtime_bridge.target_domain);
    for (key, value) in &config_runtime_bridge.metadata {
        println!("     - {}: {}", key, value);
    }
    
    println!("\n✅ Example completed successfully!");
    println!("\nThis demonstrates:");
    println!("  • Domain bridges for anti-corruption layers");
    println!("  • Event routing with transformation");
    println!("  • Service registry with tagging");
    println!("  • Dependency injection with dependencies");
    println!("  • Pattern-based event routing");
    println!("  • Bridge metadata configuration");
    
    println!("\nKey Integration Patterns:");
    println!("  • Bridge: Isolate domain boundaries");
    println!("  • Router: Transform and route events");
    println!("  • Registry: Dynamic service discovery");
    println!("  • Container: Manage dependencies");
    println!("  • Patterns: Flexible event routing");
    
    Ok(())
} 