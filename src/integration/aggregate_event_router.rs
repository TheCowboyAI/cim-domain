// Copyright 2025 Cowboy AI, LLC.

use crate::{
    DomainError,
    events::DomainEvent,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Event handler trait for aggregate events
#[async_trait]
pub trait AggregateEventHandler: Send + Sync {
    /// Handle an event from another aggregate
    async fn handle_event(&self, event: &Box<dyn DomainEvent>) -> Result<(), DomainError>;
}

/// Routes events between aggregates to maintain consistency
pub struct AggregateEventRouter {
    routes: Arc<RwLock<HashMap<String, Vec<AggregateRoute>>>>,
    handlers: Arc<RwLock<HashMap<String, Box<dyn AggregateEventHandler>>>>,
}

#[derive(Clone)]
struct AggregateRoute {
    #[allow(dead_code)]
    source_aggregate: String,
    target_aggregate: String,
    event_pattern: String,
    transformation: Arc<dyn Fn(&Box<dyn DomainEvent>) -> Option<Box<dyn DomainEvent>> + Send + Sync>,
}

impl AggregateEventRouter {
    /// Create a new aggregate event router
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a route between aggregates
    pub async fn register_route<F>(
        &self,
        source_aggregate: &str,
        target_aggregate: &str,
        event_pattern: &str,
        transformation: F,
    ) -> Result<(), DomainError>
    where
        F: Fn(&Box<dyn DomainEvent>) -> Option<Box<dyn DomainEvent>> + Send + Sync + 'static,
    {
        let route = AggregateRoute {
            source_aggregate: source_aggregate.to_string(),
            target_aggregate: target_aggregate.to_string(),
            event_pattern: event_pattern.to_string(),
            transformation: Arc::new(transformation),
        };

        let mut routes = self.routes.write().await;
        routes
            .entry(source_aggregate.to_string())
            .or_default()
            .push(route);

        Ok(())
    }

    /// Register an event handler for a specific aggregate
    pub async fn register_handler(
        &self,
        aggregate_type: &str,
        handler: Box<dyn AggregateEventHandler>,
    ) -> Result<(), DomainError> {
        let mut handlers = self.handlers.write().await;
        handlers.insert(aggregate_type.to_string(), handler);
        Ok(())
    }

    /// Route an event from source aggregate to all registered targets
    pub async fn route_event(
        &self,
        source_aggregate: &str,
        event: &Box<dyn DomainEvent>,
    ) -> Result<Vec<Box<dyn DomainEvent>>, DomainError> {
        let routes = self.routes.read().await;
        let mut routed_events = Vec::new();

        if let Some(aggregate_routes) = routes.get(source_aggregate) {
            for route in aggregate_routes {
                if Self::matches_pattern(&route.event_pattern, event) {
                    if let Some(transformed_event) = (route.transformation)(event) {
                        // Apply the event to the target aggregate
                        if let Err(e) = self.apply_to_aggregate(&route.target_aggregate, &transformed_event).await {
                            eprintln!("Failed to apply event to {}: {:?}", route.target_aggregate, e);
                        }
                        routed_events.push(transformed_event);
                    }
                }
            }
        }

        Ok(routed_events)
    }

    /// Apply an event to a specific aggregate
    async fn apply_to_aggregate(
        &self,
        aggregate_type: &str,
        event: &Box<dyn DomainEvent>,
    ) -> Result<(), DomainError> {
        let handlers = self.handlers.read().await;
        
        if let Some(handler) = handlers.get(aggregate_type) {
            handler.handle_event(event).await?;
        }

        Ok(())
    }

    /// Check if an event matches a pattern
    fn matches_pattern(pattern: &str, event: &Box<dyn DomainEvent>) -> bool {
        let event_type = event.event_type();
        let subject = event.subject();
        
        // Support patterns like:
        // "*" - matches all events
        // "Person.*" - matches all Person events
        // "Person.Created" - matches specific event type
        // "*.Created" - matches all Created events
        
        if pattern == "*" {
            return true;
        }
        
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('.').collect();
            let subject_parts: Vec<&str> = subject.split('.').collect();
            
            // Handle patterns like "Person.*" that should match any Person event
            if parts.last() == Some(&"*") && parts.len() > 1 {
                // Check if the prefix matches
                let prefix_parts = &parts[..parts.len() - 1];
                if subject_parts.len() >= prefix_parts.len() {
                    return prefix_parts.iter().zip(subject_parts.iter())
                        .all(|(pattern_part, subject_part)| {
                            *pattern_part == "*" || *pattern_part == *subject_part
                        });
                }
                return false;
            }
            
            // Exact segment matching for patterns with wildcards in specific positions
            if parts.len() != subject_parts.len() {
                return false;
            }
            
            for (pattern_part, subject_part) in parts.iter().zip(subject_parts.iter()) {
                if *pattern_part != "*" && *pattern_part != *subject_part {
                    return false;
                }
            }
            
            true
        } else {
            // Check if pattern matches the subject or any prefix of it
            subject == pattern || subject.starts_with(&format!("{}.", pattern)) || pattern == event_type
        }
    }
}

/// Pre-configured routes for common aggregate interactions
impl AggregateEventRouter {
    /// Configure routes for workflow state transitions
    pub async fn configure_workflow_routes(&self) -> Result<(), DomainError> {
        // Example: When a workflow completes, notify dependent workflows
        self.register_route(
            "Workflow",
            "Workflow",
            "workflow.completed.*",
            |event| {
                // Check if this workflow completion should trigger dependent workflows
                if event.subject().contains("workflow.completed") {
                    // Extract workflow ID and check for dependencies
                    // For now, return None as we don't have the dependency graph
                    // TODO: Implement workflow dependency checking
                    None
                } else {
                    None
                }
            },
        ).await?;

        Ok(())
    }

    /// Configure all standard routes
    pub async fn configure_standard_routes(&self) -> Result<(), DomainError> {
        self.configure_workflow_routes().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    struct TestEvent {
        id: Uuid,
        event_type: String,
        aggregate_type: String,
    }

    impl DomainEvent for TestEvent {
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

    #[tokio::test]
    async fn test_event_routing() {
        let router = AggregateEventRouter::new();

        // Register a simple route
        router
            .register_route(
                "Person",
                "Organization",
                "Person.Created",
                |event| {
                    if event.subject().starts_with("Person.Created") {
                        Some(Box::new(TestEvent {
                            id: Uuid::new_v4(),
                            event_type: "MemberAdded".to_string(),
                            aggregate_type: "Organization".to_string(),
                        }) as Box<dyn DomainEvent>)
                    } else {
                        None
                    }
                },
            )
            .await
            .unwrap();

        // Create a test event
        let person_event = Box::new(TestEvent {
            id: Uuid::new_v4(),
            event_type: "Created".to_string(),
            aggregate_type: "Person".to_string(),
        }) as Box<dyn DomainEvent>;

        // Route the event
        let routed_events = router.route_event("Person", &person_event).await.unwrap();

        // Verify the event was transformed and routed
        assert_eq!(routed_events.len(), 1);
        assert!(routed_events[0].subject().contains("Organization"));
    }

    #[tokio::test]
    async fn test_pattern_matching() {
        let event = Box::new(TestEvent {
            id: Uuid::new_v4(),
            event_type: "Created".to_string(),
            aggregate_type: "Person".to_string(),
        }) as Box<dyn DomainEvent>;

        assert!(AggregateEventRouter::matches_pattern("*", &event));
        assert!(AggregateEventRouter::matches_pattern("Person.*", &event));
        assert!(AggregateEventRouter::matches_pattern("Person.Created.v1", &event));
        assert!(AggregateEventRouter::matches_pattern("*.Created.*", &event));
        assert!(!AggregateEventRouter::matches_pattern("Organization.*", &event));
    }
}