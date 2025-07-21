// Copyright 2025 Cowboy AI, LLC.

//! Advanced persistence example using the improved repository implementations
//!
//! This example demonstrates:
//! - Using the NatsKvRepository with builder pattern
//! - Working with aggregate metadata
//! - Version tracking
//! - TTL-based expiration

use chrono::Utc;
use cim_domain::{
    persistence::{AggregateMetadata, NatsKvRepositoryBuilder, SimpleRepository},
    DomainEntity, EntityId,
};
use serde::{Deserialize, Serialize};

// Define a marker type for our domain entity
#[derive(Debug, Clone, Copy)]
struct OrderMarker;

// Define an Order aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    id: EntityId<OrderMarker>,
    customer_id: String,
    items: Vec<OrderItem>,
    status: OrderStatus,
    total: f64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderItem {
    product_id: String,
    quantity: u32,
    price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    fn new(customer_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            customer_id,
            items: Vec::new(),
            status: OrderStatus::Pending,
            total: 0.0,
            created_at: now,
            updated_at: now,
        }
    }

    fn add_item(&mut self, product_id: String, quantity: u32, price: f64) {
        self.items.push(OrderItem {
            product_id,
            quantity,
            price,
        });
        self.total = self
            .items
            .iter()
            .map(|item| item.quantity as f64 * item.price)
            .sum();
        self.updated_at = Utc::now();
    }

    fn confirm(&mut self) {
        self.status = OrderStatus::Confirmed;
        self.updated_at = Utc::now();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CIM Domain - Advanced Persistence Example");
    println!("========================================\n");

    // Connect to NATS
    println!("Connecting to NATS...");
    let client = match async_nats::connect("nats://localhost:4222").await {
        Ok(client) => {
            println!("✅ Connected to NATS successfully\n");
            client
        }
        Err(e) => {
            println!("❌ Failed to connect to NATS: {}", e);
            println!("\nPlease ensure NATS is running:");
            println!("  docker run -p 4222:4222 nats:latest -js");
            return Ok(());
        }
    };

    // Create a repository using the builder pattern
    println!("Creating order repository with custom configuration...");
    let repository: Box<dyn SimpleRepository<Order>> = Box::new(
        NatsKvRepositoryBuilder::new()
            .client(client.clone())
            .bucket_name("orders")
            .aggregate_type("Order")
            .history(20) // Keep 20 versions
            .ttl_seconds(3600) // Expire after 1 hour
            .build()
            .await?,
    );
    println!("✅ Repository created with:");
    println!("   - Bucket: orders");
    println!("   - History: 20 versions");
    println!("   - TTL: 1 hour\n");

    // Create a new order
    let mut order = Order::new("CUST-12345".to_string());
    println!("Created new order: {}", order.id);
    println!("  Customer: {}", order.customer_id);
    println!("  Status: {:?}", order.status);

    // Add items to the order
    println!("\nAdding items to order...");
    order.add_item("PROD-001".to_string(), 2, 29.99);
    order.add_item("PROD-002".to_string(), 1, 49.99);
    order.add_item("PROD-003".to_string(), 3, 15.00);

    println!("  Added {} items", order.items.len());
    println!("  Total: ${:.2}", order.total);

    // Save the order (version 1)
    println!("\nSaving order (version 1)...");
    let metadata1 = repository.save(&order).await?;
    println!("✅ Saved successfully");
    println!("  Version: {}", metadata1.version);
    println!("  Subject: {}", metadata1.subject);

    // Modify and save again (version 2)
    println!("\nConfirming order...");
    order.confirm();

    let metadata2 = repository.save(&order).await?;
    println!("✅ Order confirmed and saved");
    println!("  New version: {}", metadata2.version);
    println!("  Status: {:?}", order.status);

    // Load the order
    println!("\nLoading order from repository...");
    let loaded_order: Order = repository
        .load(&order.id)
        .await?
        .expect("Order should exist");

    println!("✅ Order loaded successfully");
    println!("  Status: {:?}", loaded_order.status);
    println!("  Items: {}", loaded_order.items.len());
    println!("  Total: ${:.2}", loaded_order.total);

    // Demonstrate version history
    println!("\n📊 Version History:");
    println!("  Version 1: Created with {} items", order.items.len());
    println!(
        "  Version {}: Status changed to {:?}",
        metadata2.version, order.status
    );

    // Create multiple orders to demonstrate bucket functionality
    println!("\n🛒 Creating additional orders...");
    for i in 1..=3 {
        let mut new_order = Order::new(format!("CUST-{:05}", i));
        new_order.add_item("PROD-100".to_string(), i, 10.0 * i as f64);

        let metadata = repository.save(&new_order).await?;
        println!(
            "  Order {} created (version {})",
            new_order.id, metadata.version
        );
    }

    // Demonstrate TTL behavior
    println!("\n⏰ TTL Information:");
    println!("  Orders in this bucket will expire after 1 hour");
    println!("  This is useful for temporary order drafts or caching");

    // Show aggregate metadata usage
    println!("\n📋 Aggregate Metadata Example:");
    let example_metadata = AggregateMetadata {
        aggregate_id: order.id.to_string(),
        aggregate_type: "Order".to_string(),
        version: metadata2.version,
        last_modified: metadata2.last_modified,
        subject: metadata2.subject.clone(),
        metadata: std::collections::HashMap::from([
            ("source".to_string(), serde_json::json!("web")),
            ("region".to_string(), serde_json::json!("us-west")),
        ]),
    };

    println!("  Aggregate ID: {}", example_metadata.aggregate_id);
    println!("  Type: {}", example_metadata.aggregate_type);
    println!("  Version: {}", example_metadata.version);
    println!("  Custom metadata: {:?}", example_metadata.metadata);

    println!("\n✅ Advanced persistence example completed successfully!");
    println!("\nKey Features Demonstrated:");
    println!("  • NatsKvRepository with builder pattern");
    println!("  • Version tracking and history");
    println!("  • TTL-based expiration");
    println!("  • Aggregate metadata");
    println!("  • Multiple aggregate instances");

    Ok(())
}
