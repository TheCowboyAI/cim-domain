//! Instrumented repository wrapper that adds metrics collection

use crate::{
    DomainEntity,
    DomainError,
    entity::EntityId,
    persistence::{
        SimpleRepository, SimpleAggregateMetadata,
        metrics::{PersistenceMetrics, MetricsInstrumented, MetricsTimer},
    },
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Repository wrapper that adds metrics instrumentation
#[derive(Clone)]
pub struct InstrumentedRepository<T, R>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de>,
    T::IdType: Send + Sync,
    R: SimpleRepository<T>,
{
    inner: R,
    metrics: PersistenceMetrics,
    _phantom: PhantomData<T>,
}

impl<T, R> InstrumentedRepository<T, R>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de>,
    T::IdType: Send + Sync,
    R: SimpleRepository<T>,
{
    /// Create a new instrumented repository
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            metrics: PersistenceMetrics::new(),
            _phantom: PhantomData,
        }
    }
    
    /// Create with existing metrics collector
    pub fn with_metrics(inner: R, metrics: PersistenceMetrics) -> Self {
        Self {
            inner,
            metrics,
            _phantom: PhantomData,
        }
    }
    
    /// Get the metrics collector
    pub fn metrics(&self) -> &PersistenceMetrics {
        &self.metrics
    }
    
    /// Get the inner repository
    pub fn inner(&self) -> &R {
        &self.inner
    }
}

#[async_trait]
impl<T, R> SimpleRepository<T> for InstrumentedRepository<T, R>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de> + Send + Sync,
    T::IdType: Send + Sync,
    R: SimpleRepository<T> + Send + Sync,
{
    async fn save(&self, aggregate: &T) -> Result<SimpleAggregateMetadata, DomainError> {
        let timer = MetricsTimer::new(&self.metrics, "repository.save");
        
        match self.inner.save(aggregate).await {
            Ok(metadata) => {
                timer.record().await;
                self.metrics.increment("repository.save.success").await;
                Ok(metadata)
            }
            Err(e) => {
                timer.record_error().await;
                self.metrics.increment("repository.save.error").await;
                Err(e)
            }
        }
    }
    
    async fn load(&self, id: &EntityId<T::IdType>) -> Result<Option<T>, DomainError> {
        let timer = MetricsTimer::new(&self.metrics, "repository.load");
        
        match self.inner.load(id).await {
            Ok(Some(aggregate)) => {
                timer.record().await;
                self.metrics.increment("repository.load.hit").await;
                Ok(Some(aggregate))
            }
            Ok(None) => {
                timer.record().await;
                self.metrics.increment("repository.load.miss").await;
                Ok(None)
            }
            Err(e) => {
                timer.record_error().await;
                self.metrics.increment("repository.load.error").await;
                Err(e)
            }
        }
    }
    
    async fn exists(&self, id: &EntityId<T::IdType>) -> Result<bool, DomainError> {
        let timer = MetricsTimer::new(&self.metrics, "repository.exists");
        
        match self.inner.exists(id).await {
            Ok(exists) => {
                timer.record().await;
                if exists {
                    self.metrics.increment("repository.exists.true").await;
                } else {
                    self.metrics.increment("repository.exists.false").await;
                }
                Ok(exists)
            }
            Err(e) => {
                timer.record_error().await;
                self.metrics.increment("repository.exists.error").await;
                Err(e)
            }
        }
    }
}

impl<T, R> MetricsInstrumented for InstrumentedRepository<T, R>
where
    T: DomainEntity + Serialize + for<'de> Deserialize<'de>,
    T::IdType: Send + Sync,
    R: SimpleRepository<T>,
{
    fn metrics(&self) -> &PersistenceMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::NatsSimpleRepository;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    #[derive(Debug, Clone, Copy)]
    struct TestMarker;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEntity {
        id: EntityId<TestMarker>,
        value: String,
    }
    
    impl DomainEntity for TestEntity {
        type IdType = TestMarker;
        
        fn id(&self) -> EntityId<Self::IdType> {
            self.id
        }
    }
    
    // Mock repository for testing
    #[derive(Clone)]
    struct MockRepository {
        storage: Arc<Mutex<std::collections::HashMap<String, TestEntity>>>,
    }
    
    impl MockRepository {
        fn new() -> Self {
            Self {
                storage: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }
        }
    }
    
    #[async_trait]
    impl SimpleRepository<TestEntity> for MockRepository {
        async fn save(&self, aggregate: &TestEntity) -> Result<SimpleAggregateMetadata, DomainError> {
            let mut storage = self.storage.lock().await;
            storage.insert(aggregate.id.to_string(), aggregate.clone());
            
            Ok(SimpleAggregateMetadata {
                aggregate_id: aggregate.id.to_string(),
                aggregate_type: "TestEntity".to_string(),
                version: 1,
                last_modified: chrono::Utc::now(),
                subject: "test.entity".to_string(),
            })
        }
        
        async fn load(&self, id: &EntityId<TestMarker>) -> Result<Option<TestEntity>, DomainError> {
            let storage = self.storage.lock().await;
            Ok(storage.get(&id.to_string()).cloned())
        }
        
        async fn exists(&self, id: &EntityId<TestMarker>) -> Result<bool, DomainError> {
            let storage = self.storage.lock().await;
            Ok(storage.contains_key(&id.to_string()))
        }
    }
    
    #[tokio::test]
    async fn test_instrumented_save() {
        let mock_repo = MockRepository::new();
        let instrumented = InstrumentedRepository::new(mock_repo);
        
        let entity = TestEntity {
            id: EntityId::new(),
            value: "test".to_string(),
        };
        
        let result = instrumented.save(&entity).await;
        assert!(result.is_ok());
        
        // Check metrics
        assert_eq!(instrumented.metrics().get_counter("repository.save.count").await, 1);
        assert_eq!(instrumented.metrics().get_counter("repository.save.success").await, 1);
        assert_eq!(instrumented.metrics().get_counter("repository.save.error").await, 0);
    }
    
    #[tokio::test]
    async fn test_instrumented_load() {
        let mock_repo = MockRepository::new();
        let instrumented = InstrumentedRepository::new(mock_repo);
        
        let entity = TestEntity {
            id: EntityId::new(),
            value: "test".to_string(),
        };
        
        // Save first
        instrumented.save(&entity).await.unwrap();
        
        // Load existing
        let loaded = instrumented.load(&entity.id).await.unwrap();
        assert!(loaded.is_some());
        
        // Load non-existing
        let fake_id = EntityId::<TestMarker>::new();
        let not_found = instrumented.load(&fake_id).await.unwrap();
        assert!(not_found.is_none());
        
        // Check metrics
        assert_eq!(instrumented.metrics().get_counter("repository.load.count").await, 2);
        assert_eq!(instrumented.metrics().get_counter("repository.load.hit").await, 1);
        assert_eq!(instrumented.metrics().get_counter("repository.load.miss").await, 1);
    }
    
    #[tokio::test]
    async fn test_metrics_summary() {
        let mock_repo = MockRepository::new();
        let instrumented = InstrumentedRepository::new(mock_repo);
        
        // Perform various operations
        for i in 0..5 {
            let entity = TestEntity {
                id: EntityId::new(),
                value: format!("test{}", i),
            };
            instrumented.save(&entity).await.unwrap();
            instrumented.load(&entity.id).await.unwrap();
        }
        
        let summary = instrumented.metrics().summary().await;
        
        assert_eq!(summary.counters.get("repository.save.count"), Some(&5));
        assert_eq!(summary.counters.get("repository.load.count"), Some(&5));
        assert_eq!(summary.counters.get("repository.save.success"), Some(&5));
        assert_eq!(summary.counters.get("repository.load.hit"), Some(&5));
        
        // Check duration stats exist
        assert!(summary.durations.contains_key("repository.save"));
        assert!(summary.durations.contains_key("repository.load"));
    }
}