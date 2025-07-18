//! Integration tests for the persistence layer

use cim_domain::{
    EntityId,
    DomainEntity,
    persistence::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Test entity marker
#[derive(Debug, Clone, Copy, PartialEq)]
struct TestEntityMarker;

// Test entity
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestEntity {
    id: EntityId<TestEntityMarker>,
    name: String,
    version: u64,
}

impl DomainEntity for TestEntity {
    type IdType = TestEntityMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl TestEntity {
    fn new(name: String) -> Self {
        Self {
            id: EntityId::new(),
            name,
            version: 1,
        }
    }
}

#[tokio::test]
#[ignore] // Requires NATS server to be running
async fn test_simple_repository_crud() {
    // Connect to NATS
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    
    // Create repository
    let repo = NatsSimpleRepository::new(
        client,
        "test-aggregates".to_string(),
        "TestEntity".to_string(),
    ).await.unwrap();
    
    // Create test entity
    let entity = TestEntity::new("Test Entity".to_string());
    let entity_id = entity.id();
    
    // Save entity
    let metadata = repo.save(&entity).await.unwrap();
    assert_eq!(metadata.aggregate_type, "TestEntity");
    assert!(metadata.version > 0);
    
    // Load entity
    let loaded: Option<TestEntity> = repo.load(&entity_id).await.unwrap();
    assert!(loaded.is_some());
    
    let loaded_entity = loaded.unwrap();
    assert_eq!(loaded_entity.name, entity.name);
    assert_eq!(loaded_entity.version, entity.version);
    
    // Check exists using the trait directly
    use cim_domain::persistence::SimpleRepository;
    let exists = <NatsSimpleRepository as SimpleRepository<TestEntity>>::exists(&repo, &entity_id).await.unwrap();
    assert!(exists);
    
    // Load non-existent entity
    let non_existent_id = EntityId::<TestEntityMarker>::new();
    let not_found: Option<TestEntity> = repo.load(&non_existent_id).await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
#[ignore] // Requires NATS server
async fn test_nats_kv_repository_with_ttl() {
    use std::time::Duration;
    use tokio::time::sleep;
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    
    // Create repository with 2 second TTL
    let repo: NatsKvRepository<TestEntity> = NatsKvRepositoryBuilder::new()
        .client(client)
        .bucket_name("test-ttl-bucket")
        .aggregate_type("TestEntity")
        .ttl_seconds(2)
        .build()
        .await
        .unwrap();
    
    // Save entity
    let entity = TestEntity::new("TTL Test".to_string());
    let entity_id = entity.id();
    
    let metadata = repo.save(&entity).await.unwrap();
    assert_eq!(metadata.aggregate_type, "TestEntity");
    
    // Load immediately - should exist
    let loaded: Option<TestEntity> = repo.load(&entity_id).await.unwrap();
    assert!(loaded.is_some());
    
    // Wait for TTL to expire
    sleep(Duration::from_secs(3)).await;
    
    // Load after TTL - should not exist
    let expired: Option<TestEntity> = repo.load(&entity_id).await.unwrap();
    assert!(expired.is_none());
}

#[tokio::test]
#[ignore] // Requires NATS server
async fn test_read_model_store() {
    use chrono::Utc;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestReadModel {
        id: String,
        count: u32,
        name: String,
    }
    
    impl ReadModel for TestReadModel {
        fn model_type() -> &'static str {
            "TestReadModel"
        }
        
        fn id(&self) -> &str {
            &self.id
        }
        
        fn apply_event(&mut self, _event: &dyn cim_domain::DomainEvent) -> Result<(), cim_domain::DomainError> {
            self.count += 1;
            Ok(())
        }
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    let store = NatsReadModelStore::new(client, "test-read-models".to_string()).await.unwrap();
    
    // Create and save read model
    let model = TestReadModel {
        id: "model-123".to_string(),
        count: 5,
        name: "Test Model".to_string(),
    };
    
    let metadata = ReadModelMetadata {
        id: model.id.clone(),
        model_type: TestReadModel::model_type().to_string(),
        schema_version: 1,
        last_updated: Utc::now(),
        last_event_position: 100,
        metadata: HashMap::new(),
    };
    
    store.save(&model, metadata.clone()).await.unwrap();
    
    // Load read model
    let loaded = store.load::<TestReadModel>(&model.id).await.unwrap();
    assert!(loaded.is_some());
    
    let (loaded_model, loaded_metadata) = loaded.unwrap();
    assert_eq!(loaded_model.id, model.id);
    assert_eq!(loaded_model.count, model.count);
    assert_eq!(loaded_metadata.last_event_position, 100);
    
    // Update projection status
    store.update_projection_status(
        TestReadModel::model_type(),
        ProjectionStatus::UpToDate,
    ).await.unwrap();
    
    // Delete read model
    store.delete(TestReadModel::model_type(), &model.id).await.unwrap();
    
    // Verify deleted
    let deleted = store.load::<TestReadModel>(&model.id).await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
#[ignore] // Requires NATS server
async fn test_query_support() {
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    
    // Create repository
    let repo: NatsKvRepository<TestEntity> = NatsKvRepositoryBuilder::new()
        .client(client)
        .bucket_name("test-query-bucket")
        .aggregate_type("TestEntity")
        .build()
        .await
        .unwrap();
    
    // Create multiple entities
    let mut entities = vec![];
    for i in 0..10 {
        let entity = TestEntity::new(format!("Entity {}", i));
        repo.save(&entity).await.unwrap();
        entities.push(entity);
    }
    
    // Build query
    let _query = QueryBuilder::new()
        .limit(5)
        .offset(2)
        .build();
    
    // In a real implementation, the repository would support querying
    // For now, we'll test the query building and pagination
    
    let total_items = entities.len();
    let pagination = Pagination::from_query(5, 2, total_items);
    
    assert_eq!(pagination.page, 1); // offset 2 / per_page 5 = 0, + 1 = page 1
    assert_eq!(pagination.per_page, 5);
    assert_eq!(pagination.total_pages, 2); // 10 items / 5 per page
    assert!(pagination.has_next());
    assert!(!pagination.has_prev());
    
    // Test query result
    let items: Vec<TestEntity> = entities.into_iter().skip(2).take(5).collect();
    let result = QueryResult::new(items.clone(), total_items, 10);
    
    assert_eq!(result.items.len(), 5);
    assert_eq!(result.total_count, total_items);
    assert!(result.has_more);
}

#[tokio::test]
async fn test_repository_error_types() {
    let err = RepositoryError::NotFound("test-123".to_string());
    assert!(err.to_string().contains("not found"));
    
    let err = RepositoryError::VersionConflict { expected: 5, actual: 3 };
    assert!(err.to_string().contains("Version conflict"));
    
    let err = RepositoryError::StorageError("NATS down".to_string());
    assert!(err.to_string().contains("Storage error"));
}

#[tokio::test]
async fn test_save_and_load_options() {
    let save_options = SaveOptions {
        expected_version: Some(3),
        create_snapshot: true,
        metadata: None,
    };
    
    assert_eq!(save_options.expected_version, Some(3));
    assert!(save_options.create_snapshot);
    assert!(save_options.metadata.is_none());
    
    let load_options = LoadOptions {
        version: Some(10),
        use_snapshot: true,
        max_events: Some(100),
    };
    
    assert_eq!(load_options.version, Some(10));
    assert!(load_options.use_snapshot);
    assert_eq!(load_options.max_events, Some(100));
}

#[tokio::test]
async fn test_aggregate_metadata() {
    use chrono::Utc;
    
    let metadata = AggregateMetadata {
        aggregate_id: "agg-123".to_string(),
        aggregate_type: "Order".to_string(),
        version: 5,
        last_modified: Utc::now(),
        subject: "orders.agg-123".to_string(),
        metadata: HashMap::from([
            ("region".to_string(), serde_json::json!("us-west")),
            ("source".to_string(), serde_json::json!("api")),
        ]),
    };
    
    assert_eq!(metadata.aggregate_id, "agg-123");
    assert_eq!(metadata.version, 5);
    assert_eq!(metadata.metadata.len(), 2);
    assert_eq!(metadata.metadata.get("region").unwrap(), &serde_json::json!("us-west"));
}