//! Dependency injection for domain instantiation
//!
//! This module provides dependency injection capabilities for
//! instantiating domains with their required dependencies.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::errors::DomainError;
use crate::category::DomainCategory;

/// Trait for types that can be injected
pub trait Injectable: Any + Send + Sync {
    /// Get the type ID for this injectable
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl<T: Any + Send + Sync> Injectable for T {}

/// Service provider for creating instances
#[async_trait]
pub trait ServiceProvider: Send + Sync {
    /// The type this provider creates
    type Service: Injectable;
    
    /// Create a new instance of the service
    async fn provide(&self, container: &DependencyContainer) -> Result<Arc<Self::Service>, DomainError>;
}

/// Factory function service provider
pub struct FactoryProvider<T> {
    factory: Box<dyn Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync>,
}

impl<T> FactoryProvider<T> {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        Self {
            factory: Box::new(factory),
        }
    }
}

#[async_trait]
impl<T: Injectable + 'static> ServiceProvider for FactoryProvider<T> {
    type Service = T;
    
    async fn provide(&self, container: &DependencyContainer) -> Result<Arc<Self::Service>, DomainError> {
        (self.factory)(container)
    }
}

/// Singleton service provider
pub struct SingletonProvider<T> {
    instance: Arc<RwLock<Option<Arc<T>>>>,
    factory: Box<dyn Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync>,
}

impl<T> SingletonProvider<T> {
    pub fn new<F>(factory: F) -> Self
    where
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        Self {
            instance: Arc::new(RwLock::new(None)),
            factory: Box::new(factory),
        }
    }
}

#[async_trait]
impl<T: Injectable + 'static> ServiceProvider for SingletonProvider<T> {
    type Service = T;
    
    async fn provide(&self, container: &DependencyContainer) -> Result<Arc<Self::Service>, DomainError> {
        let read_lock = self.instance.read().await;
        if let Some(instance) = read_lock.as_ref() {
            return Ok(instance.clone());
        }
        drop(read_lock);
        
        let mut write_lock = self.instance.write().await;
        if let Some(instance) = write_lock.as_ref() {
            return Ok(instance.clone());
        }
        
        let new_instance = (self.factory)(container)?;
        *write_lock = Some(new_instance.clone());
        Ok(new_instance)
    }
}

/// Dependency injection container
pub struct DependencyContainer {
    /// Registered services
    services: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
    
    /// Service providers
    providers: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl DependencyContainer {
    /// Create a new container
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a service instance
    pub async fn register_instance<T: Injectable + 'static>(&self, service: T) -> Result<(), DomainError> {
        let mut services = self.services.write().await;
        let type_id = TypeId::of::<T>();
        
        if services.contains_key(&type_id) {
            return Err(DomainError::AlreadyExists(
                format!("Service of type {} already registered", std::any::type_name::<T>())
            ));
        }
        
        services.insert(type_id, Box::new(Arc::new(service)));
        Ok(())
    }
    
    /// Register a service provider
    pub async fn register_provider<P>(&self, provider: P) -> Result<(), DomainError>
    where
        P: ServiceProvider + 'static,
        P::Service: 'static,
    {
        let mut providers = self.providers.write().await;
        let type_id = TypeId::of::<P::Service>();
        
        if providers.contains_key(&type_id) {
            return Err(DomainError::AlreadyExists(
                format!("Provider for type {} already registered", std::any::type_name::<P::Service>())
            ));
        }
        
        providers.insert(type_id, Box::new(provider));
        Ok(())
    }
    
    /// Register a factory function
    pub async fn register_factory<T, F>(&self, factory: F) -> Result<(), DomainError>
    where
        T: Injectable + 'static,
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        self.register_provider(FactoryProvider::new(factory)).await
    }
    
    /// Register an async factory function
    pub async fn register_async_factory<T, F, Fut>(&self, factory: F) -> Result<(), DomainError>
    where
        T: Injectable + 'static,
        F: Fn(DependencyContainer) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Arc<T>, DomainError>> + Send + 'static,
    {
        // For now, async factories need special handling
        // This would require a different provider implementation
        Err(DomainError::NotImplemented("Async factories not yet supported".to_string()))
    }
    
    /// Register a singleton
    pub async fn register_singleton<T, F>(&self, factory: F) -> Result<(), DomainError>
    where
        T: Injectable + 'static,
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        self.register_provider(SingletonProvider::new(factory)).await
    }
    
    /// Resolve a service
    pub async fn resolve<T: Injectable + 'static>(&self) -> Result<Arc<T>, DomainError> {
        let type_id = TypeId::of::<T>();
        
        // Check if we have an instance
        {
            let services = self.services.read().await;
            if let Some(service) = services.get(&type_id) {
                if let Some(arc_any) = service.downcast_ref::<Arc<T>>() {
                    return Ok(arc_any.clone());
                }
            }
        }
        
        // Check if we have a provider
        {
            let providers = self.providers.read().await;
            if let Some(provider_any) = providers.get(&type_id) {
                // Clone the provider reference to avoid holding the lock
                let provider_any = provider_any.as_ref();
                
                // Try to downcast to different provider types
                if let Some(provider) = provider_any.downcast_ref::<FactoryProvider<T>>() {
                    return provider.provide(self).await;
                }
                if let Some(provider) = provider_any.downcast_ref::<SingletonProvider<T>>() {
                    return provider.provide(self).await;
                }
            }
        }
        
        Err(DomainError::NotFound(
            format!("Service of type {} not registered", std::any::type_name::<T>())
        ))
    }
    
    /// Create a scoped container
    pub fn create_scope(&self) -> DependencyContainer {
        // Scoped container shares providers but has its own services
        DependencyContainer {
            services: Arc::new(RwLock::new(HashMap::new())),
            providers: self.providers.clone(),
        }
    }
}

/// Extension trait for domain categories
#[async_trait]
pub trait DomainCategoryExt {
    /// Create with dependency injection
    async fn create_with_di(
        name: String,
        container: &DependencyContainer,
    ) -> Result<DomainCategory, DomainError>;
}

#[async_trait]
impl DomainCategoryExt for DomainCategory {
    async fn create_with_di(
        name: String,
        container: &DependencyContainer,
    ) -> Result<DomainCategory, DomainError> {
        // In a real implementation, would inject required services
        // For now, just create a basic category
        Ok(DomainCategory::new(name))
    }
}

/// Builder for configuring dependency injection
pub struct ContainerBuilder {
    container: DependencyContainer,
}

impl ContainerBuilder {
    pub fn new() -> Self {
        Self {
            container: DependencyContainer::new(),
        }
    }
    
    pub async fn add_instance<T: Injectable + 'static>(self, service: T) -> Result<Self, DomainError> {
        self.container.register_instance(service).await?;
        Ok(self)
    }
    
    pub async fn add_factory<T, F>(self, factory: F) -> Result<Self, DomainError>
    where
        T: Injectable + 'static,
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        self.container.register_factory(factory).await?;
        Ok(self)
    }
    
    pub async fn add_singleton<T, F>(self, factory: F) -> Result<Self, DomainError>
    where
        T: Injectable + 'static,
        F: Fn(&DependencyContainer) -> Result<Arc<T>, DomainError> + Send + Sync + 'static,
    {
        self.container.register_singleton(factory).await?;
        Ok(self)
    }
    
    pub fn build(self) -> DependencyContainer {
        self.container
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct TestService {
        value: String,
    }
    
    #[derive(Debug)]
    struct DependentService {
        test_service: Arc<TestService>,
        id: u32,
    }
    
    #[tokio::test]
    async fn test_register_and_resolve_instance() {
        let container = DependencyContainer::new();
        
        let service = TestService {
            value: "test".to_string(),
        };
        
        container.register_instance(service).await.unwrap();
        
        let resolved = container.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "test");
    }
    
    #[tokio::test]
    async fn test_factory_provider() {
        let container = DependencyContainer::new();
        
        container.register_factory(|_| {
            Ok(Arc::new(TestService {
                value: "factory".to_string(),
            }))
        }).await.unwrap();
        
        let resolved1 = container.resolve::<TestService>().await.unwrap();
        let resolved2 = container.resolve::<TestService>().await.unwrap();
        
        assert_eq!(resolved1.value, "factory");
        assert_eq!(resolved2.value, "factory");
        // Factory creates new instances
        assert!(!Arc::ptr_eq(&resolved1, &resolved2));
    }
    
    #[tokio::test]
    async fn test_singleton_provider() {
        let container = DependencyContainer::new();
        
        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        container.register_singleton(move |_| {
            let id = counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Arc::new(DependentService {
                test_service: Arc::new(TestService {
                    value: "singleton".to_string(),
                }),
                id,
            }))
        }).await.unwrap();
        
        let resolved1 = container.resolve::<DependentService>().await.unwrap();
        let resolved2 = container.resolve::<DependentService>().await.unwrap();
        
        // Singleton returns same instance
        assert!(Arc::ptr_eq(&resolved1, &resolved2));
        assert_eq!(resolved1.id, 0); // Only created once
    }
    
    #[tokio::test]
    async fn test_dependency_resolution() {
        let container = DependencyContainer::new();
        
        // Register TestService
        container.register_singleton(|_| {
            Ok(Arc::new(TestService {
                value: "injected".to_string(),
            }))
        }).await.unwrap();
        
        // Register DependentService that depends on TestService
        // Note: This is a simplified test - in real usage, we'd use async factories
        let test_service_for_dependent = container.resolve::<TestService>().await.unwrap();
        container.register_instance(DependentService {
            test_service: test_service_for_dependent.clone(),
            id: 123,
        }).await.unwrap();
        
        let dependent = container.resolve::<DependentService>().await.unwrap();
        assert_eq!(dependent.test_service.value, "injected");
        assert_eq!(dependent.id, 123);
    }
    
    #[tokio::test]
    async fn test_scoped_container() {
        let root_container = DependencyContainer::new();
        
        // Register singleton in root
        root_container.register_singleton(|_| {
            Ok(Arc::new(TestService {
                value: "root".to_string(),
            }))
        }).await.unwrap();
        
        // Create scope
        let scoped_container = root_container.create_scope();
        
        // Resolve in scope - should get root's singleton
        let resolved = scoped_container.resolve::<TestService>().await.unwrap();
        assert_eq!(resolved.value, "root");
    }
    
    #[tokio::test]
    async fn test_container_builder() {
        let container = ContainerBuilder::new()
            .add_instance(TestService {
                value: "builder".to_string(),
            }).await.unwrap()
            .build();
        
        // Register dependent service after building
        let test_service = container.resolve::<TestService>().await.unwrap();
        container.register_instance(DependentService {
            test_service: test_service.clone(),
            id: 999,
        }).await.unwrap();
        
        let service = container.resolve::<TestService>().await.unwrap();
        assert_eq!(service.value, "builder");
        
        let dependent = container.resolve::<DependentService>().await.unwrap();
        assert_eq!(dependent.id, 999);
    }
}