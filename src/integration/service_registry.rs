//! Service registry for domain services
//!
//! This module provides a registry for managing domain services
//! with different lifetimes and discovery mechanisms.

use std::collections::HashMap;
use std::sync::Arc;
use std::any::{Any, TypeId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::errors::DomainError;
use super::dependency_injection::DependencyContainer;

/// Service lifetime management
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceLifetime {
    /// New instance for each request
    Transient,
    
    /// New instance for each scope
    Scoped,
    
    /// Single instance for application lifetime
    Singleton,
}

/// Service descriptor
#[derive(Clone)]
pub struct ServiceDescriptor {
    /// Service type ID
    pub service_type: TypeId,
    
    /// Service type name
    pub service_name: String,
    
    /// Implementation type ID
    pub implementation_type: TypeId,
    
    /// Implementation type name
    pub implementation_name: String,
    
    /// Service lifetime
    pub lifetime: ServiceLifetime,
    
    /// Factory function
    pub factory: Arc<dyn Fn(&DependencyContainer) -> Result<Box<dyn Any + Send + Sync>, DomainError> + Send + Sync>,
    
    /// Service metadata
    pub metadata: HashMap<String, String>,
}

impl ServiceDescriptor {
    /// Create a new service descriptor
    pub fn new<T>(
        lifetime: ServiceLifetime,
        factory: Box<dyn Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync>,
    ) -> Self
    where
        T: 'static + Send + Sync,
    {
        ServiceDescriptor {
            service_type: TypeId::of::<T>(),
            service_name: std::any::type_name::<T>().to_string(),
            implementation_type: TypeId::of::<T>(),
            implementation_name: std::any::type_name::<T>().to_string(),
            lifetime,
            factory: Arc::new(move |container| {
                let instance = factory(container)?;
                Ok(Box::new(instance) as Box<dyn Any + Send + Sync>)
            }),
            metadata: HashMap::new(),
        }
    }
}

/// Service registry
pub struct ServiceRegistry {
    /// Registered services
    services: Arc<RwLock<HashMap<TypeId, ServiceDescriptor>>>,
    
    /// Service instances (for singletons)
    instances: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
    
    /// Service discovery
    discovery: Arc<RwLock<ServiceDiscovery>>,
}

/// Service discovery mechanism
pub struct ServiceDiscovery {
    /// Service tags
    tags: HashMap<TypeId, Vec<String>>,
    
    /// Tag to services mapping
    services_by_tag: HashMap<String, Vec<TypeId>>,
    
    /// Service endpoints (for remote services)
    endpoints: HashMap<TypeId, ServiceEndpoint>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service URL
    pub url: String,
    
    /// Protocol (http, grpc, nats, etc.)
    pub protocol: String,
    
    /// Authentication method
    pub auth_method: Option<String>,
    
    /// Health check endpoint
    pub health_check: Option<String>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            instances: Arc::new(RwLock::new(HashMap::new())),
            discovery: Arc::new(RwLock::new(ServiceDiscovery {
                tags: HashMap::new(),
                services_by_tag: HashMap::new(),
                endpoints: HashMap::new(),
            })),
        }
    }
    
    /// Register a service
    pub async fn register<TService, TImpl, F>(
        &self,
        lifetime: ServiceLifetime,
        factory: F,
    ) -> Result<(), DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        let service_type = TypeId::of::<TService>();
        let implementation_type = TypeId::of::<TImpl>();
        
        let descriptor = ServiceDescriptor {
            service_type,
            service_name: std::any::type_name::<TService>().to_string(),
            implementation_type,
            implementation_name: std::any::type_name::<TImpl>().to_string(),
            lifetime,
            factory: Arc::new(move |container| {
                let instance = factory(container)?;
                Ok(instance as Box<dyn Any + Send + Sync>)
            }),
            metadata: HashMap::new(),
        };
        
        let mut services = self.services.write().await;
        if services.contains_key(&service_type) {
            return Err(DomainError::AlreadyExists(
                format!("Service {} already registered", descriptor.service_name)
            ));
        }
        
        services.insert(service_type, descriptor);
        Ok(())
    }
    
    /// Register a singleton service
    pub async fn register_singleton<TService, TImpl, F>(
        &self,
        factory: F,
    ) -> Result<(), DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.register::<TService, TImpl, F>(ServiceLifetime::Singleton, factory).await
    }
    
    /// Register a transient service
    pub async fn register_transient<TService, TImpl, F>(
        &self,
        factory: F,
    ) -> Result<(), DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.register::<TService, TImpl, F>(ServiceLifetime::Transient, factory).await
    }
    
    /// Register a scoped service
    pub async fn register_scoped<TService, TImpl, F>(
        &self,
        factory: F,
    ) -> Result<(), DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.register::<TService, TImpl, F>(ServiceLifetime::Scoped, factory).await
    }
    
    /// Resolve a service
    pub async fn resolve<T: 'static + Send + Sync>(
        &self,
        container: &DependencyContainer,
    ) -> Result<Arc<T>, DomainError> {
        let service_type = TypeId::of::<T>();
        
        // Get service descriptor
        let services = self.services.read().await;
        let descriptor = services.get(&service_type)
            .ok_or_else(|| DomainError::NotFound(
                format!("Service {} not registered", std::any::type_name::<T>())
            ))?
            .clone();
        drop(services);
        
        // Handle based on lifetime
        match descriptor.lifetime {
            ServiceLifetime::Singleton => {
                // Check if instance exists
                let instances = self.instances.read().await;
                if let Some(instance) = instances.get(&service_type) {
                    // Try to downcast the stored instance
                    if let Some(arc_any) = instance.downcast_ref::<Arc<T>>() {
                        return Ok(arc_any.clone());
                    }
                }
                drop(instances);
                
                // Create new instance
                let boxed_instance = (descriptor.factory)(container)?;
                
                // Try to downcast to Arc<T>
                if let Ok(arc_t) = boxed_instance.downcast::<Arc<T>>() {
                    let instance = *arc_t;
                    
                    // Store the instance for future use
                    let mut instances = self.instances.write().await;
                    instances.insert(service_type, Box::new(instance.clone()) as Box<dyn Any + Send + Sync>);
                    
                    Ok(instance)
                } else {
                    Err(DomainError::InvalidOperation {
                        reason: "Failed to downcast service instance".to_string()
                    })
                }
            }
            
            ServiceLifetime::Transient | ServiceLifetime::Scoped => {
                // Always create new instance
                let boxed_instance = (descriptor.factory)(container)?;
                
                // Try to downcast to Arc<T>
                if let Ok(arc_t) = boxed_instance.downcast::<Arc<T>>() {
                    Ok(*arc_t)
                } else {
                    Err(DomainError::InvalidOperation {
                        reason: "Failed to downcast service instance".to_string()
                    })
                }
            }
        }
    }
    
    /// Add tags to a service
    pub async fn add_tags<T: ?Sized + 'static>(
        &self,
        tags: Vec<String>,
    ) -> Result<(), DomainError> {
        let service_type = TypeId::of::<T>();
        let mut discovery = self.discovery.write().await;
        
        // Add tags to service
        discovery.tags.insert(service_type, tags.clone());
        
        // Update reverse mapping
        for tag in tags {
            discovery.services_by_tag
                .entry(tag)
                .or_insert_with(Vec::new)
                .push(service_type);
        }
        
        Ok(())
    }
    
    /// Find services by tag
    pub async fn find_by_tag(&self, tag: &str) -> Vec<TypeId> {
        let discovery = self.discovery.read().await;
        discovery.services_by_tag
            .get(tag)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Register a service endpoint
    pub async fn register_endpoint<T: ?Sized + 'static>(
        &self,
        endpoint: ServiceEndpoint,
    ) -> Result<(), DomainError> {
        let service_type = TypeId::of::<T>();
        let mut discovery = self.discovery.write().await;
        discovery.endpoints.insert(service_type, endpoint);
        Ok(())
    }
    
    /// Get service endpoint
    pub async fn get_endpoint<T: ?Sized + 'static>(&self) -> Option<ServiceEndpoint> {
        let service_type = TypeId::of::<T>();
        let discovery = self.discovery.read().await;
        discovery.endpoints.get(&service_type).cloned()
    }
    
    /// Get all registered services
    pub async fn list_services(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        services.values()
            .map(|desc| ServiceInfo {
                service_name: desc.service_name.clone(),
                implementation_name: desc.implementation_name.clone(),
                lifetime: desc.lifetime,
                metadata: desc.metadata.clone(),
            })
            .collect()
    }
}

/// Information about a registered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Name of the service interface
    pub service_name: String,
    /// Name of the implementation
    pub implementation_name: String,
    /// Service lifetime strategy
    pub lifetime: ServiceLifetime,
    /// Additional service metadata
    pub metadata: HashMap<String, String>,
}

/// Service collection builder
pub struct ServiceCollectionBuilder {
    registry: ServiceRegistry,
}

impl ServiceCollectionBuilder {
    /// Create a new service collection builder
    pub fn new() -> Self {
        Self {
            registry: ServiceRegistry::new(),
        }
    }
    
    /// Add a singleton service
    ///
    /// # Arguments
    /// * `factory` - Factory function to create the service
    pub async fn add_singleton<TService, TImpl, F>(
        self,
        factory: F,
    ) -> Result<Self, DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.registry.register_singleton::<TService, TImpl, F>(factory).await?;
        Ok(self)
    }
    
    /// Add a transient service
    ///
    /// # Arguments
    /// * `factory` - Factory function to create the service
    pub async fn add_transient<TService, TImpl, F>(
        self,
        factory: F,
    ) -> Result<Self, DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.registry.register_transient::<TService, TImpl, F>(factory).await?;
        Ok(self)
    }
    
    /// Add a scoped service
    ///
    /// # Arguments
    /// * `factory` - Factory function to create the service
    pub async fn add_scoped<TService, TImpl, F>(
        self,
        factory: F,
    ) -> Result<Self, DomainError>
    where
        TService: ?Sized + 'static,
        TImpl: 'static + Send + Sync,
        F: Fn(&DependencyContainer) -> Result<Box<TImpl>, DomainError> + Send + Sync + 'static,
    {
        self.registry.register_scoped::<TService, TImpl, F>(factory).await?;
        Ok(self)
    }
    
    /// Add tags to a service
    ///
    /// # Arguments
    /// * `tags` - Tags to add to the service
    pub async fn with_tags<T: ?Sized + 'static>(
        self,
        tags: Vec<String>,
    ) -> Result<Self, DomainError> {
        self.registry.add_tags::<T>(tags).await?;
        Ok(self)
    }
    
    /// Build the configured service registry
    pub fn build(self) -> ServiceRegistry {
        self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    trait TestService: Send + Sync {
        fn get_value(&self) -> &str;
    }
    
    struct TestServiceImpl {
        value: String,
    }
    
    impl TestService for TestServiceImpl {
        fn get_value(&self) -> &str {
            &self.value
        }
    }
    
    #[tokio::test]
    async fn test_service_registration() {
        let registry = ServiceRegistry::new();
        let container = DependencyContainer::new();
        
        // Register singleton
        registry.register_singleton::<TestServiceImpl, TestServiceImpl, _>(|_| {
            Ok(Box::new(TestServiceImpl {
                value: "singleton".to_string(),
            }))
        }).await.unwrap();
        
        // Resolve service
        let service1 = registry.resolve::<TestServiceImpl>(&container).await.unwrap();
        let service2 = registry.resolve::<TestServiceImpl>(&container).await.unwrap();
        
        assert_eq!(service1.get_value(), "singleton");
        assert_eq!(service2.get_value(), "singleton");
    }
    
    #[tokio::test]
    async fn test_transient_lifetime() {
        let registry = ServiceRegistry::new();
        let container = DependencyContainer::new();
        
        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // Register transient
        registry.register_transient::<TestServiceImpl, TestServiceImpl, _>(move |_| {
            let id = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Box::new(TestServiceImpl {
                value: format!("instance_{}", id),
            }))
        }).await.unwrap();
        
        // Each resolve creates new instance
        let service1 = registry.resolve::<TestServiceImpl>(&container).await.unwrap();
        let service2 = registry.resolve::<TestServiceImpl>(&container).await.unwrap();
        
        assert_eq!(service1.get_value(), "instance_0");
        assert_eq!(service2.get_value(), "instance_1");
    }
    
    #[tokio::test]
    async fn test_service_discovery() {
        let registry = ServiceRegistry::new();
        
        // Register service
        registry.register_singleton::<dyn TestService, TestServiceImpl, _>(|_| {
            Ok(Box::new(TestServiceImpl {
                value: "tagged".to_string(),
            }))
        }).await.unwrap();
        
        // Add tags
        registry.add_tags::<dyn TestService>(vec![
            "domain:test".to_string(),
            "type:singleton".to_string(),
        ]).await.unwrap();
        
        // Find by tag
        let services = registry.find_by_tag("domain:test").await;
        assert_eq!(services.len(), 1);
        assert_eq!(services[0], TypeId::of::<dyn TestService>());
    }
    
    #[tokio::test]
    async fn test_service_endpoints() {
        let registry = ServiceRegistry::new();
        
        // Register service
        registry.register_singleton::<dyn TestService, TestServiceImpl, _>(|_| {
            Ok(Box::new(TestServiceImpl {
                value: "remote".to_string(),
            }))
        }).await.unwrap();
        
        // Register endpoint
        let endpoint = ServiceEndpoint {
            url: "http://localhost:8080/test-service".to_string(),
            protocol: "http".to_string(),
            auth_method: Some("bearer".to_string()),
            health_check: Some("/health".to_string()),
        };
        
        registry.register_endpoint::<dyn TestService>(endpoint.clone()).await.unwrap();
        
        // Get endpoint
        let retrieved = registry.get_endpoint::<dyn TestService>().await.unwrap();
        assert_eq!(retrieved.url, endpoint.url);
        assert_eq!(retrieved.protocol, "http");
    }
    
    #[tokio::test]
    async fn test_service_collection_builder() {
        let registry = ServiceCollectionBuilder::new()
            .add_singleton::<dyn TestService, TestServiceImpl, _>(|_| {
                Ok(Box::new(TestServiceImpl {
                    value: "built".to_string(),
                }))
            }).await.unwrap()
            .with_tags::<dyn TestService>(vec!["built-service".to_string()]).await.unwrap()
            .build();
        
        let services = registry.find_by_tag("built-service").await;
        assert_eq!(services.len(), 1);
        
        let service_list = registry.list_services().await;
        assert!(!service_list.is_empty());
    }
}