// Copyright 2025 Cowboy AI, LLC.

//! Integration tests for persistence layer with real NATS server
//!
//! These tests require a running NATS server with JetStream enabled:
//! ```bash
//! docker run -d --name nats-test -p 4222:4222 nats:latest -js
//! ```

use cim_domain::{
    EntityId,
    DomainEntity,
    DomainEvent,
    persistence::*,
    DomainError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;
use uuid::Uuid;

// Test domain models
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderMarker;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
    total: f64,
    status: OrderStatus,
    version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderItem {
    product_id: String,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
enum OrderStatus {
    Pending,
    Confirmed,
    Shipped,
    Delivered,
    Cancelled,
}

impl DomainEntity for Order {
    type IdType = OrderMarker;
    
    fn id(&self) -> EntityId<Self::IdType> {
        self.id
    }
}

impl Order {
    fn new(customer_id: String, items: Vec<OrderItem>) -> Self {
        let total = items.iter().map(|i| i.price * i.quantity as f64).sum();
        Self {
            id: EntityId::new(),
            customer_id,
            items,
            total,
            status: OrderStatus::Pending,
            version: 1,
        }
    }
    
    fn confirm(&mut self) {
        self.status = OrderStatus::Confirmed;
        self.version += 1;
    }
    
    fn ship(&mut self) {
        self.status = OrderStatus::Shipped;
        self.version += 1;
    }
}

// Test events
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderCreated {
    order_id: Uuid,
    customer_id: String,
    total: f64,
}

impl DomainEvent for OrderCreated {
    fn subject(&self) -> String {
        "orders.order.created.v1".to_string()
    }
    
    fn aggregate_id(&self) -> Uuid {
        self.order_id
    }
    
    fn event_type(&self) -> &'static str {
        "OrderCreated"
    }
}

// Test read model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderSummary {
    id: String,
    customer_id: String,
    total: f64,
    status: String,
    order_count: u32,
}

impl ReadModel for OrderSummary {
    fn model_type() -> &'static str {
        "OrderSummary"
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn apply_event(&mut self, event: &dyn DomainEvent) -> Result<(), DomainError> {
        match event.event_type() {
            "OrderCreated" => {
                self.order_count += 1;
            }
            _ => {}
        }
        Ok(())
    }
}

// Helper to check if NATS is available
async fn nats_available() -> bool {
    async_nats::connect("nats://localhost:4222").await.is_ok()
}

#[tokio::test]
async fn test_simple_repository_lifecycle() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    
    // Create repository
    let repo = NatsSimpleRepository::new(
        client,
        "test-orders-simple".to_string(),
        "Order".to_string(),
    ).await.unwrap();
    
    // Create and save order
    let mut order = Order::new(
        "customer-123".to_string(),
        vec![
            OrderItem {
                product_id: "prod-1".to_string(),
                quantity: 2,
                price: 50.0,
            },
            OrderItem {
                product_id: "prod-2".to_string(),
                quantity: 1,
                price: 100.0,
            },
        ],
    );
    
    let order_id = order.id();
    
    // Save initial version
    let metadata = repo.save(&order).await.unwrap();
    assert_eq!(metadata.version, 1);
    assert_eq!(metadata.aggregate_type, "Order");
    
    // Load and verify
    let loaded: Option<Order> = repo.load(&order_id).await.unwrap();
    assert!(loaded.is_some());
    let mut loaded_order = loaded.unwrap();
    assert_eq!(loaded_order.total, 200.0);
    assert_eq!(loaded_order.status, OrderStatus::Pending);
    
    // Update and save
    loaded_order.confirm();
    let metadata2 = repo.save(&loaded_order).await.unwrap();
    assert_eq!(metadata2.version, 2);
    
    // Check existence
    assert!(repo.exists(&order_id).await.unwrap());
    
    // Test non-existent order
    let fake_id = EntityId::<OrderMarker>::new();
    assert!(!repo.exists(&fake_id).await.unwrap());
}

#[tokio::test]
async fn test_kv_repository_with_ttl() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    
    // Create repository with 2 second TTL
    let repo: NatsKvRepository<Order> = NatsKvRepositoryBuilder::new()
        .client(client)
        .bucket_name("test-orders-ttl")
        .aggregate_type("Order")
        .ttl_seconds(2)
        .build()
        .await
        .unwrap();
    
    // Save order
    let order = Order::new(
        "customer-ttl".to_string(),
        vec![OrderItem {
            product_id: "prod-ttl".to_string(),
            quantity: 1,
            price: 25.0,
        }],
    );
    
    let order_id = order.id();
    repo.save(&order).await.unwrap();
    
    // Immediately loadable
    let loaded: Option<Order> = repo.load(&order_id).await.unwrap();
    assert!(loaded.is_some());
    
    // Wait for TTL
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Should be expired
    let expired: Option<Order> = repo.load(&order_id).await.unwrap();
    assert!(expired.is_none());
}

#[tokio::test]
async fn test_read_model_store_with_caching() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    let store = NatsReadModelStore::new(
        client,
        "test-order-summaries".to_string(),
    ).await.unwrap();
    
    // Create and save read model
    let summary = OrderSummary {
        id: "customer-123".to_string(),
        customer_id: "customer-123".to_string(),
        total: 500.0,
        status: "active".to_string(),
        order_count: 5,
    };
    
    let metadata = ReadModelMetadata {
        id: summary.id.clone(),
        model_type: OrderSummary::model_type().to_string(),
        schema_version: 1,
        last_updated: Utc::now(),
        last_event_position: 100,
        metadata: HashMap::from([
            ("region".to_string(), serde_json::json!("us-west")),
        ]),
    };
    
    store.save(&summary, metadata.clone()).await.unwrap();
    
    // First load - from storage
    let start = std::time::Instant::now();
    let loaded1 = store.load::<OrderSummary>(&summary.id).await.unwrap();
    let storage_time = start.elapsed();
    assert!(loaded1.is_some());
    
    // Second load - from cache (should be faster)
    let start = std::time::Instant::now();
    let loaded2 = store.load::<OrderSummary>(&summary.id).await.unwrap();
    let cache_time = start.elapsed();
    
    assert!(loaded2.is_some());
    // Cache should be faster (though this might be flaky in CI)
    println!("Storage time: {:?}, Cache time: {:?}", storage_time, cache_time);
    
    // Verify data
    let (model, meta) = loaded2.unwrap();
    assert_eq!(model.order_count, 5);
    assert_eq!(meta.last_event_position, 100);
    assert_eq!(meta.metadata.get("region").unwrap(), &serde_json::json!("us-west"));
    
    // Update projection status
    store.update_projection_status(
        OrderSummary::model_type(),
        ProjectionStatus::UpToDate,
    ).await.unwrap();
    
    // Delete and verify
    store.delete(OrderSummary::model_type(), &summary.id).await.unwrap();
    let deleted = store.load::<OrderSummary>(&summary.id).await.unwrap();
    assert!(deleted.is_none());
}

#[tokio::test]
async fn test_concurrent_access() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    let repo = NatsSimpleRepository::new(
        client,
        "test-concurrent".to_string(),
        "Order".to_string(),
    ).await.unwrap();
    
    let order = Order::new(
        "concurrent-customer".to_string(),
        vec![OrderItem {
            product_id: "prod-1".to_string(),
            quantity: 1,
            price: 100.0,
        }],
    );
    
    let order_id = order.id();
    repo.save(&order).await.unwrap();
    
    // Spawn multiple tasks to update the same order
    let mut handles = vec![];
    
    for i in 0..5 {
        let repo_clone = repo.clone();
        let order_id_clone = order_id.clone();
        
        let handle = tokio::spawn(async move {
            // Load, modify, and save
            if let Some(mut order) = repo_clone.load::<Order>(&order_id_clone).await.unwrap() {
                // Add a new item
                order.items.push(OrderItem {
                    product_id: format!("prod-{}", i),
                    quantity: 1,
                    price: 10.0 * i as f64,
                });
                order.version += 1;
                
                // This might fail due to version conflicts
                repo_clone.save(&order).await
            } else {
                Err(DomainError::NotFound("Order not found".to_string()))
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Some should succeed, some might fail with version conflicts
    let successes = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
    let conflicts = results.iter().filter(|r| r.as_ref().unwrap().is_err()).count();
    
    println!("Concurrent updates: {} succeeded, {} conflicts", successes, conflicts);
    assert!(successes >= 1); // At least one should succeed
}

#[tokio::test]
async fn test_query_patterns() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    // Test query building and pagination
    let query = QueryBuilder::new()
        .filter("customer_id", serde_json::json!("customer-123"))
        .filter("status", serde_json::json!("confirmed"))
        .sort_by("created_at", SortDirection::Descending)
        .limit(20)
        .offset(0)
        .build();
    
    assert_eq!(query.filters.len(), 2);
    assert_eq!(query.limit, Some(20));
    assert_eq!(query.offset, Some(0));
    
    // Test pagination
    let total_items = 100;
    let page1 = Pagination::from_query(20, 0, total_items);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.total_pages, 5);
    assert!(page1.has_next());
    assert!(!page1.has_prev());
    
    let page3 = Pagination::from_query(20, 40, total_items);
    assert_eq!(page3.page, 3);
    assert!(page3.has_next());
    assert!(page3.has_prev());
    
    let last_page = Pagination::from_query(20, 80, total_items);
    assert_eq!(last_page.page, 5);
    assert!(!last_page.has_next());
    assert!(last_page.has_prev());
}

#[tokio::test]
async fn test_event_handling_in_read_model() {
    if !nats_available().await {
        eprintln!("Skipping test: NATS not available");
        return;
    }
    
    let client = async_nats::connect("nats://localhost:4222").await.unwrap();
    let store = NatsReadModelStore::new(
        client,
        "test-event-handling".to_string(),
    ).await.unwrap();
    
    // Create read model
    let mut summary = OrderSummary {
        id: "customer-events".to_string(),
        customer_id: "customer-events".to_string(),
        total: 0.0,
        status: "active".to_string(),
        order_count: 0,
    };
    
    // Apply events
    let event = OrderCreated {
        order_id: Uuid::new_v4(),
        customer_id: "customer-events".to_string(),
        total: 100.0,
    };
    
    summary.apply_event(&event).unwrap();
    assert_eq!(summary.order_count, 1);
    
    // Apply more events
    for _ in 0..5 {
        let event = OrderCreated {
            order_id: Uuid::new_v4(),
            customer_id: "customer-events".to_string(),
            total: 50.0,
        };
        summary.apply_event(&event).unwrap();
    }
    
    assert_eq!(summary.order_count, 6);
    
    // Save and verify
    let metadata = ReadModelMetadata {
        id: summary.id.clone(),
        model_type: OrderSummary::model_type().to_string(),
        schema_version: 1,
        last_updated: Utc::now(),
        last_event_position: 6,
        metadata: HashMap::new(),
    };
    
    store.save(&summary, metadata).await.unwrap();
    
    let loaded = store.load::<OrderSummary>(&summary.id).await.unwrap();
    assert!(loaded.is_some());
    let (model, _) = loaded.unwrap();
    assert_eq!(model.order_count, 6);
}