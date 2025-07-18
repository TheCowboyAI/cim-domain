//! Subject-based routing for persistence layer

use crate::DomainError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use cim_subject::{Subject, Pattern, Permissions, PermissionRule};

/// Route pattern for subject-based routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePattern {
    /// Pattern name
    pub name: String,
    /// Subject pattern
    pub pattern: String,
    /// Handler name
    pub handler: String,
    /// Priority (higher = higher priority)
    pub priority: i32,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Routing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// First match wins
    FirstMatch,
    /// All matches executed
    AllMatches,
    /// Highest priority wins
    PriorityBased,
    /// Round-robin among matches
    RoundRobin,
}

/// Route handler trait
#[async_trait]
pub trait RouteHandler: Send + Sync {
    /// Handle a routed message
    async fn handle(
        &self,
        subject: &Subject,
        payload: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<u8>, DomainError>;
    
    /// Get handler name
    fn name(&self) -> &str;
    
    /// Check if handler can process subject
    fn can_handle(&self, subject: &Subject) -> bool;
}

/// Subject router for persistence operations
pub struct SubjectRouter {
    /// Routing strategy
    strategy: RoutingStrategy,
    /// Route patterns
    routes: Arc<RwLock<Vec<RoutePattern>>>,
    /// Registered handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn RouteHandler>>>>,
    /// Permissions for routing
    permissions: Permissions,
    /// Round-robin counters
    rr_counters: Arc<RwLock<HashMap<String, usize>>>,
}

impl SubjectRouter {
    /// Create a new subject router
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            strategy,
            routes: Arc::new(RwLock::new(Vec::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            permissions: Permissions::new(cim_subject::permissions::Policy::Allow),
            rr_counters: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a route pattern
    pub async fn add_route(&self, route: RoutePattern) -> Result<(), DomainError> {
        // Validate pattern
        Pattern::new(&route.pattern)
            .map_err(|e| DomainError::InvalidOperation {
                reason: format!("Invalid pattern: {}", e),
            })?;
        
        let mut routes = self.routes.write().await;
        routes.push(route);
        
        // Sort by priority if using priority-based routing
        if self.strategy == RoutingStrategy::PriorityBased {
            routes.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        
        Ok(())
    }
    
    /// Register a handler
    pub async fn register_handler(
        &self,
        name: String,
        handler: Box<dyn RouteHandler>,
    ) -> Result<(), DomainError> {
        let mut handlers = self.handlers.write().await;
        handlers.insert(name, handler);
        Ok(())
    }
    
    /// Add permission rule
    pub fn add_permission(&mut self, rule: PermissionRule) {
        self.permissions.add_rule(rule);
    }
    
    /// Route a subject to appropriate handlers
    pub async fn route(
        &self,
        subject: &Subject,
        payload: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Vec<Vec<u8>>, DomainError> {
        // Check permissions
        if !self.permissions.is_allowed(subject, cim_subject::permissions::Operation::Subscribe) {
            return Err(DomainError::ValidationError(
                "Permission denied for subject".to_string()
            ));
        }
        
        // Find matching routes
        let routes = self.routes.read().await;
        let matching_routes: Vec<&RoutePattern> = routes
            .iter()
            .filter(|route| {
                Pattern::new(&route.pattern)
                    .map(|p| p.matches(subject))
                    .unwrap_or(false)
            })
            .collect();
        
        if matching_routes.is_empty() {
            return Err(DomainError::NotFound(
                format!("No route found for subject: {}", subject)
            ));
        }
        
        // Apply routing strategy
        let selected_routes: Vec<&RoutePattern> = match self.strategy {
            RoutingStrategy::FirstMatch => vec![matching_routes[0]],
            RoutingStrategy::AllMatches => matching_routes,
            RoutingStrategy::PriorityBased => {
                // Already sorted by priority
                vec![matching_routes[0]]
            }
            RoutingStrategy::RoundRobin => {
                // Get round-robin counter
                let mut counters = self.rr_counters.write().await;
                let counter = counters
                    .entry(subject.to_string())
                    .or_insert(0);
                
                let selected = matching_routes[*counter % matching_routes.len()];
                *counter += 1;
                
                vec![selected]
            }
        };
        
        // Execute handlers
        let handlers = self.handlers.read().await;
        let mut results = Vec::new();
        
        for route in selected_routes {
            if let Some(handler) = handlers.get(&route.handler) {
                if handler.can_handle(subject) {
                    let result = handler.handle(
                        subject,
                        payload.clone(),
                        metadata.clone(),
                    ).await?;
                    results.push(result);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get all routes matching a pattern
    pub async fn find_routes(&self, pattern: &Pattern) -> Vec<RoutePattern> {
        let routes = self.routes.read().await;
        routes
            .iter()
            .filter(|route| {
                Subject::new(&route.pattern)
                    .map(|s| pattern.matches(&s))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
    
    /// Remove a route by name
    pub async fn remove_route(&self, name: &str) -> Result<(), DomainError> {
        let mut routes = self.routes.write().await;
        let original_len = routes.len();
        routes.retain(|r| r.name != name);
        
        if routes.len() == original_len {
            Err(DomainError::NotFound(format!("Route not found: {}", name)))
        } else {
            Ok(())
        }
    }
    
    /// Get routing statistics
    pub async fn get_stats(&self) -> RouterStats {
        let routes = self.routes.read().await;
        let handlers = self.handlers.read().await;
        let counters = self.rr_counters.read().await;
        
        RouterStats {
            total_routes: routes.len(),
            total_handlers: handlers.len(),
            strategy: self.strategy,
            route_hits: counters.clone(),
        }
    }
}

/// Router statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStats {
    /// Total number of routes
    pub total_routes: usize,
    /// Total number of handlers
    pub total_handlers: usize,
    /// Current routing strategy
    pub strategy: RoutingStrategy,
    /// Route hit counters (for round-robin)
    pub route_hits: HashMap<String, usize>,
}

/// Example handler implementation for aggregate persistence
pub struct AggregateHandler {
    name: String,
    aggregate_type: String,
}

impl AggregateHandler {
    pub fn new(name: String, aggregate_type: String) -> Self {
        Self { name, aggregate_type }
    }
}

#[async_trait]
impl RouteHandler for AggregateHandler {
    async fn handle(
        &self,
        subject: &Subject,
        payload: Vec<u8>,
        _metadata: HashMap<String, String>,
    ) -> Result<Vec<u8>, DomainError> {
        // Handle aggregate persistence based on subject
        // This is a simplified example
        Ok(payload)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn can_handle(&self, subject: &Subject) -> bool {
        subject.to_string().contains(&self.aggregate_type)
    }
}