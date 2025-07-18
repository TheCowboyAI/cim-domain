//! Tests for the persistence layer

#[cfg(test)]
mod persistence_tests {
    use crate::{
        entity::EntityId,
        DomainEntity,
        persistence::*,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    
    // Test entity marker
    #[derive(Debug, Clone, Copy)]
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
            metadata: HashMap::from([
                ("region".to_string(), serde_json::json!("us-west")),
            ]),
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
        
        let err = RepositoryError::VersionConflict { expected: 5, actual: 3 };
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
    #[ignore] // Requires NATS server to be running
    async fn test_nats_simple_repository() {
        // This test demonstrates how to use the simple repository
        // It requires a NATS server running at localhost:4222
        /*
        use async_nats::connect;
        
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
        let loaded = repo.load(&entity.id).await.unwrap();
        assert!(loaded.is_some());
        
        let loaded_entity = loaded.unwrap();
        assert_eq!(loaded_entity.name, entity.name);
        assert_eq!(loaded_entity.version, entity.version);
        
        // Check exists
        let exists = repo.exists(&entity.id).await.unwrap();
        assert!(exists);
        
        // Load non-existent entity
        let non_existent_id = EntityId::<TestEntityMarker>::new();
        let not_found = repo.load(&non_existent_id).await.unwrap();
        assert!(not_found.is_none());
        */
    }
    
    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_nats_kv_repository_builder() {
        // Test the builder pattern
        /*
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
        
        // Use the repository...
        */
    }
}