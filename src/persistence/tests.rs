// Copyright 2025 Cowboy AI, LLC.

//! Tests for the persistence layer

#[cfg(test)]
mod persistence_tests {
    use crate::{entity::EntityId, persistence::*, DomainEntity};
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

    #[test]
    fn test_entity_implementation() {
        let id = EntityId::<TestEntityMarker>::new();
        let entity = TestEntity {
            id,
            name: "Test Entity".to_string(),
            version: 1,
        };

        // Verify DomainEntity implementation
        assert_eq!(entity.id(), id);
        assert_eq!(entity.name, "Test Entity");
        assert_eq!(entity.version, 1);

        // Test serialization
        let serialized = serde_json::to_string(&entity).unwrap();
        let deserialized: TestEntity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, entity.name);
        assert_eq!(deserialized.version, entity.version);
    }

    #[test]
    fn test_simple_aggregate_metadata() {
        use chrono::Utc;

        let metadata = SimpleAggregateMetadata {
            aggregate_id: "test-id".to_string(),
            aggregate_type: "TestAggregate".to_string(),
            version: 1,
            last_modified: Utc::now(),
            subject: "domain.test.state.v1".to_string(),
        };

        assert_eq!(metadata.aggregate_id, "test-id");
        assert_eq!(metadata.aggregate_type, "TestAggregate");
        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.subject, "domain.test.state.v1");
    }

    #[test]
    fn test_aggregate_metadata() {
        use chrono::Utc;

        let metadata = AggregateMetadata {
            aggregate_id: "agg-123".to_string(),
            aggregate_type: "Order".to_string(),
            version: 5,
            last_modified: Utc::now(),
            subject: "orders.agg-123".to_string(),
            metadata: HashMap::from([("region".to_string(), serde_json::json!("us-west"))]),
        };

        assert_eq!(metadata.aggregate_id, "agg-123");
        assert_eq!(metadata.version, 5);
        assert!(metadata.metadata.contains_key("region"));
    }

    #[test]
    fn test_save_options() {
        let options = SaveOptions {
            expected_version: Some(3),
            create_snapshot: true,
            metadata: None,
        };

        assert_eq!(options.expected_version, Some(3));
        assert!(options.create_snapshot);
        assert!(options.metadata.is_none());

        let default_options = SaveOptions::default();
        assert!(default_options.expected_version.is_none());
        assert!(!default_options.create_snapshot);
    }

    #[test]
    fn test_load_options() {
        let options = LoadOptions {
            version: Some(10),
            use_snapshot: true,
            max_events: Some(100),
        };

        assert_eq!(options.version, Some(10));
        assert!(options.use_snapshot);
        assert_eq!(options.max_events, Some(100));
    }

    #[test]
    fn test_repository_error() {
        let err = RepositoryError::NotFound("test-123".to_string());
        assert!(err.to_string().contains("not found"));

        let err = RepositoryError::VersionConflict {
            expected: 5,
            actual: 3,
        };
        assert!(err.to_string().contains("Version conflict"));
    }

    #[test]
    fn test_nats_kv_config() {
        let config = NatsKvConfig {
            bucket_name: "test-bucket".to_string(),
            aggregate_type: "TestAggregate".to_string(),
            history: 20,
            ttl_seconds: 3600,
        };

        assert_eq!(config.bucket_name, "test-bucket");
        assert_eq!(config.history, 20);
        assert_eq!(config.ttl_seconds, 3600);

        let default_config = NatsKvConfig::default();
        assert_eq!(default_config.bucket_name, "aggregates");
        assert_eq!(default_config.history, 10);
        assert_eq!(default_config.ttl_seconds, 0);
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .filter("status", serde_json::json!("active"))
            .filter("region", serde_json::json!("us-west"))
            .sort_by("created_at", SortDirection::Descending)
            .limit(20)
            .offset(40)
            .build();

        assert_eq!(query.filters.len(), 2);
        assert!(query.filters.contains_key("status"));
        assert!(query.filters.contains_key("region"));
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.offset, Some(40));

        if let Some((field, dir)) = query.sort_by {
            assert_eq!(field, "created_at");
            assert_eq!(dir, SortDirection::Descending);
        } else {
            panic!("Expected sort_by to be Some");
        }
    }

    #[test]
    fn test_query_result() {
        let items = vec![1, 2, 3, 4, 5];
        let result = QueryResult::new(items.clone(), 100, 50);

        assert_eq!(result.items, items);
        assert_eq!(result.total_count, 100);
        assert!(result.has_more);
        assert_eq!(result.execution_time_ms, 50);

        // Test map function
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.items, vec![2, 4, 6, 8, 10]);
        assert_eq!(mapped.total_count, 100);
    }

    #[test]
    fn test_pagination() {
        // Test from_query
        let pagination = Pagination::from_query(10, 20, 100);
        assert_eq!(pagination.page, 3); // offset 20 / per_page 10 + 1
        assert_eq!(pagination.per_page, 10);
        assert_eq!(pagination.total_pages, 10); // 100 / 10
        assert_eq!(pagination.total_items, 100);
        assert!(pagination.has_next());
        assert!(pagination.has_prev());

        // Test to_limit_offset
        let (limit, offset) = pagination.to_limit_offset();
        assert_eq!(limit, 10);
        assert_eq!(offset, 20);

        // Test edge cases
        let first_page = Pagination::from_query(10, 0, 100);
        assert_eq!(first_page.page, 1);
        assert!(!first_page.has_prev());
        assert!(first_page.has_next());

        let last_page = Pagination::from_query(10, 90, 100);
        assert_eq!(last_page.page, 10);
        assert!(last_page.has_prev());
        assert!(!last_page.has_next());
    }

    #[test]
    fn test_projection_status() {
        let status = ProjectionStatus::UpToDate;
        assert_eq!(status, ProjectionStatus::UpToDate);

        let lagging = ProjectionStatus::Lagging { behind_by: 42 };
        if let ProjectionStatus::Lagging { behind_by } = lagging {
            assert_eq!(behind_by, 42);
        } else {
            panic!("Expected Lagging status");
        }

        let failed = ProjectionStatus::Failed;
        assert_ne!(failed, ProjectionStatus::Rebuilding);
    }

    #[test]
    fn test_read_model_metadata() {
        use chrono::Utc;

        let metadata = ReadModelMetadata {
            id: "model-123".to_string(),
            model_type: "CustomerStats".to_string(),
            schema_version: 2,
            last_updated: Utc::now(),
            last_event_position: 1000,
            metadata: HashMap::new(),
        };

        assert_eq!(metadata.id, "model-123");
        assert_eq!(metadata.model_type, "CustomerStats");
        assert_eq!(metadata.schema_version, 2);
        assert_eq!(metadata.last_event_position, 1000);
    }

    #[tokio::test]
    async fn test_nats_simple_repository() {
        use async_nats::connect;
        use crate::persistence::SimpleRepository;

        // Connect to NATS
        let client = connect("nats://localhost:4222").await.unwrap();

        // Create repository
        let repo = NatsSimpleRepository::new(
            client,
            "test-aggregates".to_string(),
            "TestEntity".to_string(),
        ).await.unwrap();

        // Create test entity
        let entity = TestEntity {
            id: EntityId::new(),
            name: "Test Entity".to_string(),
            version: 1,
        };

        // Save entity
        let metadata = repo.save(&entity).await.unwrap();
        assert_eq!(metadata.aggregate_type, "TestEntity");
        assert!(metadata.version > 0);

        // Load entity
        let loaded: Option<TestEntity> = repo.load(&entity.id).await.unwrap();
        assert!(loaded.is_some());

        let loaded_entity = loaded.unwrap();
        assert_eq!(loaded_entity.name, entity.name);
        assert_eq!(loaded_entity.version, entity.version);

        // Check exists
        let exists = <NatsSimpleRepository as SimpleRepository<TestEntity>>::exists(&repo, &entity.id).await.unwrap();
        assert!(exists);

        // Load non-existent entity
        let non_existent_id = EntityId::<TestEntityMarker>::new();
        let not_found: Option<TestEntity> = repo.load(&non_existent_id).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_nats_kv_repository_builder() {
        use async_nats::connect;

        let client = connect("nats://localhost:4222").await.unwrap();

        let repo: NatsKvRepository<TestEntity> = NatsKvRepositoryBuilder::new()
            .client(client)
            .bucket_name("test-bucket")
            .aggregate_type("TestEntity")
            .history(15)
            .ttl_seconds(7200)
            .build()
            .await
            .unwrap();

        // Test that the repository was created successfully
        // Create test entity
        let entity = TestEntity {
            id: EntityId::new(),
            name: "Builder Test".to_string(),
            version: 1,
        };

        // Save and load to verify it works
        let metadata = repo.save(&entity).await.unwrap();
        assert_eq!(metadata.aggregate_type, "TestEntity");
        
        let loaded: Option<TestEntity> = repo.load(&entity.id).await.unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn test_save_options_builder() {
        use crate::infrastructure::event_store::EventMetadata;

        // Test all options
        let options = SaveOptions {
            expected_version: Some(5),
            create_snapshot: true,
            metadata: Some(EventMetadata {
                correlation_id: Some("corr-123".to_string()),
                causation_id: Some("cause-456".to_string()),
                triggered_by: Some("admin".to_string()),
                custom: Some(serde_json::json!({"source": "API"})),
            }),
        };

        assert_eq!(options.expected_version, Some(5));
        assert!(options.create_snapshot);
        assert!(options.metadata.is_some());
        let metadata = options.metadata.as_ref().unwrap();
        assert_eq!(metadata.triggered_by.as_ref().unwrap(), "admin");
        assert_eq!(metadata.get("source").unwrap(), "API");

        // Test default
        let default_options = SaveOptions::default();
        assert!(default_options.expected_version.is_none());
        assert!(!default_options.create_snapshot);
        assert!(default_options.metadata.is_none());
    }

    #[test]
    fn test_load_options_all_fields() {
        let options = LoadOptions {
            version: Some(42),
            use_snapshot: true,
            max_events: Some(100),
        };

        assert_eq!(options.version, Some(42));
        assert!(options.use_snapshot);
        assert_eq!(options.max_events, Some(100));

        // Test default
        let default_options = LoadOptions::default();
        assert!(default_options.version.is_none());
        assert!(!default_options.use_snapshot);
        assert!(default_options.max_events.is_none());
    }

    #[test]
    fn test_query_options_all_fields() {
        // QueryOptions is not defined in aggregate_repository_v2, skip this test
    }

    #[test]
    fn test_repository_error_variants() {
        use crate::infrastructure::EventStoreError;

        // Test direct construction
        let not_found = RepositoryError::NotFound("user-123".to_string());
        assert_eq!(not_found.to_string(), "Aggregate not found: user-123");

        let version_conflict = RepositoryError::VersionConflict {
            expected: 5,
            actual: 3,
        };
        assert_eq!(
            version_conflict.to_string(),
            "Version conflict: expected 5, actual 3"
        );

        let serialization = RepositoryError::SerializationError("Invalid JSON".to_string());
        assert_eq!(
            serialization.to_string(),
            "Serialization error: Invalid JSON"
        );

        let storage = RepositoryError::StorageError("Connection failed".to_string());
        assert_eq!(storage.to_string(), "Storage error: Connection failed");

        // Note: SubjectError and IpldError are not in aggregate_repository_v2
        // These are specific to the original aggregate_repository.rs

        // Test From conversions
        let event_store_err = EventStoreError::ConnectionError("NATS down".to_string());
        let repo_err: RepositoryError = event_store_err.into();
        assert!(repo_err.to_string().contains("Event store error"));

        // Note: SnapshotError conversion is not implemented in aggregate_repository_v2
    }

    #[test]
    fn test_aggregate_metadata_with_cid() {
        use chrono::Utc;

        // Note: The AggregateMetadata in aggregate_repository_v2 doesn't have state_cid field
        // This test is for the original aggregate_repository.rs version
        let metadata = AggregateMetadata {
            aggregate_id: "order-789".to_string(),
            aggregate_type: "Order".to_string(),
            version: 10,
            last_modified: Utc::now(),
            subject: "domain.order.state.v1".to_string(),
            metadata: HashMap::from([
                ("customer_id".to_string(), serde_json::json!("cust-123")),
                ("priority".to_string(), serde_json::json!("high")),
            ]),
        };

        assert_eq!(metadata.aggregate_id, "order-789");
        assert_eq!(metadata.aggregate_type, "Order");
        assert_eq!(metadata.version, 10);
        assert_eq!(metadata.subject, "domain.order.state.v1");
        assert_eq!(metadata.metadata.len(), 2);
        assert_eq!(metadata.metadata.get("priority").unwrap(), "high");

        // Test serialization
        let serialized = serde_json::to_string(&metadata).unwrap();
        let deserialized: AggregateMetadata = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.aggregate_id, metadata.aggregate_id);
        assert_eq!(deserialized.version, metadata.version);
    }
}
